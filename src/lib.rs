use std::fs::File;
use std::io::prelude::*;
use std::num::Wrapping;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

pub trait InOutHandler {
    fn write<T: InOutHandler>(&mut self, port: u8, state: &mut State8080<T>);
    fn read<T: InOutHandler>(&mut self, port: u8, state: &mut State8080<T>);
}

pub struct DefaultHandler;

impl InOutHandler for DefaultHandler {
    fn write<T: InOutHandler>(&mut self, _port: u8, _state: &mut State8080<T>) {}
    fn read<T: InOutHandler>(&mut self, _port: u8, _state: &mut State8080<T>) {}
}

pub struct Flags {
    pub z: bool,
    pub s: bool,
    pub p: bool,
    pub cy: bool,
    pub ac: bool, //pad: i32
}

impl Flags {
    pub fn clear(&mut self) {
        self.z  = true;
        self.s  = false;
        self.p  = true;
        self.cy = false;
        self.ac = false;
    }
}

fn parity(mut x: u8, cy: bool) -> bool {
    let mut p = if cy { 1 } else { 0 };
    for _ in 0..8 {
        if x & 1 != 0 { p += 1 };
        x >>= 1;
    }
    p & 1 == 0
}

#[test]
fn parity_test() {
    assert!(!parity(0x99, true));
}

pub struct State8080<T: InOutHandler> {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: usize,
    pub pc: usize,
    pub memory: Vec<u8>,
    pub fl: Flags,
    pub int_enable: bool,
    pub io: T,
}

impl<T: InOutHandler> State8080<T> {
    pub fn new(io_handler: T) -> State8080<T> {
        State8080 {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
            memory: vec![0; 0x10000],
            fl: Flags {
                z: false,
                s: false,
                p: false,
                cy: false,
                ac: false,
            },
            int_enable: false,
            io: io_handler,
        }
    }

    pub fn set_register(&mut self, reg: u8, val: u8) {
        match reg {
            0 => self.b = val,
            1 => self.c = val,
            2 => self.d = val,
            3 => self.e = val,
            4 => self.h = val,
            5 => self.l = val,
            6 => { let addr = self.hl(); self.memory[addr] = val },
            7 => self.a = val,
            _ => panic!("Invalid register number"),
        }
    }

    pub fn get_register(&mut self, reg: u8) -> u8 {
        match reg {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => self.at_hl(),
            7 => self.a,
            _ => panic!("Invalid register number"),
        }
    }

    pub fn read_file_in_memory_at(
        &mut self,
        filename: &str,
        offset: usize,
    ) -> std::io::Result<usize> {
        File::open(filename)?.read(&mut self.memory[offset..])
    }

    pub fn generate_interrupt(&mut self, interrupt_num: u8) {
        self.call(u16::from(interrupt_num) << 8);
    }

    fn unimplemented_instruction(&mut self) {
        println!("Error: unimplemented instruction");
        std::process::exit(1);
    }

    fn bc(&self) -> usize {
        (usize::from(self.b) << 8) | usize::from(self.c)
    }

    fn de(&self) -> usize {
        (usize::from(self.d) << 8) | usize::from(self.e)
    }

    fn hl(&self) -> usize {
        (usize::from(self.h) << 8) | usize::from(self.l)
    }

    fn at_bc(&self) -> u8 {
        let addr = self.bc();
        self.memory[addr]
    }

    fn at_de(&self) -> u8 {
        let addr = self.de();
        self.memory[addr]
    }

    fn at_hl(&self) -> u8 {
        let addr = self.hl();
        self.memory[addr]
    }

    fn byte1(&self) -> u8 {
        self.memory[self.pc]
    }

    fn byte2(&self) -> u8 {
        self.memory[self.pc + 1]
    }

    fn word(&self) -> u16 {
        self.word_at(self.pc)
    }

    fn word_at(&self, addr: usize) -> u16 {
        (u16::from(self.memory[addr + 1]) << 8) | u16::from(self.memory[addr])
    }

    fn set_r(&mut self, res: u8) {
        self.fl.z = res == 0;
        self.fl.s = (res & 0x80) != 0;
        self.fl.p = parity(res, self.fl.cy);
    }

    fn set_flags(&mut self, res: u16) {
        self.set_r(res as u8);
        self.fl.cy = res > 0xff;
    }

    fn add(&mut self, val: u8) {
        let ans = u16::from(self.a) + u16::from(val);
        self.set_flags(ans);
        self.a = ans as u8;
    }

    fn add_op(&mut self, src: u8) {
        let ans = u16::from(self.get_register(src)) + u16::from(self.a);
        self.set_flags(ans);
        self.a = ans as u8;
    }

    fn adc(&mut self, val: u8) {
        let mut cy = if self.fl.cy { 1u8 } else { 0u8 };
        self.add(val);
        if self.fl.cy {
            cy += 1
        };
        self.add(cy);
    }

    fn sub(&mut self, val: u8) {
        let Wrapping(ans) = Wrapping(u16::from(self.a)) - Wrapping(u16::from(val));
        self.set_flags(ans);
        self.a = ans as u8;
    }

    fn sbb(&mut self, val: u8) {
        let carry = Wrapping(if self.fl.cy { 1u8 } else { 0u8 });
        let Wrapping(rhs) = Wrapping(val) + carry;
        self.sub(rhs);
    }

    fn and(&mut self, val: u8) {
        self.a &= val;
        let temp = self.a;
        self.set_r(temp);
    }

    fn xor(&mut self, val: u8) {
        self.a ^= val;
        let temp = self.a;
        self.set_r(temp);
    }

    fn or(&mut self, val: u8) {
        self.a |= val;
        let temp = self.a;
        self.set_r(temp);
    }

    fn cmp(&mut self, val: u8) {
        let ans = Wrapping(u16::from(self.a)) - Wrapping(u16::from(val));
        self.set_flags(ans.0);
    }

    fn pop(&mut self) -> u16 {
        let r = (u16::from(self.memory[self.sp + 1]) << 8) | u16::from(self.memory[self.sp]);
        self.sp += 2;
        r
    }

    fn push(&mut self, val: u16) {
        self.sp -= 2;
        self.memory[self.sp] = (val & 0xff) as u8;
        self.memory[self.sp + 1] = (val >> 8) as u8;
    }

    fn ret(&mut self) {
        self.pc = usize::from(self.pop());
    }

    fn call(&mut self, addr: u16) {
        if addr == 5 { // CPUDIAG print routine
            if self.c != 9 {
                return;
            }
            let mut s = self.de();
            while self.memory[s] != b'$' {
                print!("{}", self.memory[s] as char);
                s += 1;
            }
            println!();
        }
        else {
            let pc = self.pc as u16;
            self.push(pc + 2);
            self.pc = usize::from(addr);
        }
    }

    fn call_if(&mut self, cond: bool) {
        if cond {
            let addr = self.word();
            self.call(addr);
        } else {
            self.pc += 2;
        }
    }
}

fn disassemble8080_op(codebuffer: &[u8], pc: usize) -> usize {
    let code = &codebuffer[pc..];
    let mut opbytes = 1;
    print!("{:04X} ", pc);
    match code[0] {
        0x00 => print!("NOP"),
        0x01 => {
            print!("LXI    B,#${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0x02 => print!("STAX   B"),
        0x03 => print!("INX   B"),
        0x04 => print!("INR   B"),
        0x05 => print!("DCR   B"),
        0x06 => {
            print!("MVI    B,#${:02X}", code[1]);
            opbytes = 2;
        }
        0x07 => print!("RLC"),

        0x08 => print!("NOP"),
        0x09 => print!("DAD   B"),
        0x0a => print!("LDAX   B"),
        0x0b => print!("DCX   C"),
        0x0c => print!("INR   C"),
        0x0d => print!("DCR   C"),
        0x0e => {
            print!("MVI    C,#${:02X}", code[1]);
            opbytes = 2;
        }
        0x0f => print!("RRC"),

        0x10 => print!("NOP"),
        0x11 => {
            print!("LXI    D,#${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0x12 => print!("STAX   D"),
        0x13 => print!("INX   D"),
        0x14 => print!("INR   D"),
        0x15 => print!("DCR   D"),
        0x16 => {
            print!("MVI    D,#${:02X}", code[1]);
            opbytes = 2;
        }
        0x17 => print!("RAL"),

        0x18 => print!("NOP"),
        0x19 => print!("DAD   D"),
        0x1a => print!("LDAX   D"),
        0x1b => print!("DCX   D"),
        0x1c => print!("INR   E"),
        0x1d => print!("DCR   E"),
        0x1e => {
            print!("MVI    E,#${:02X}", code[1]);
            opbytes = 2;
        }
        0x1f => print!("RAR"),

        0x20 => print!("NOP"),
        0x21 => {
            print!("LXI    H,#${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0x22 => {
            print!("SHLD   ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0x23 => print!("INX   H"),
        0x24 => print!("INR   H"),
        0x25 => print!("DCR   H"),
        0x26 => {
            print!("MVI    H,#${:02X}", code[1]);
            opbytes = 2;
        }
        0x27 => print!("DAA"),

        0x28 => print!("NOP"),
        0x29 => print!("DAD   H"),
        0x2a => {
            print!("LHLD   ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0x2b => print!("DCX    H"),
        0x2c => print!("INR   L"),
        0x2d => print!("DCR   L"),
        0x2e => {
            print!("MVI    L,#${:02X}", code[1]);
            opbytes = 2;
        }
        0x2f => print!("CMA"),

        0x30 => print!("NOP"),
        0x31 => {
            print!("LXI   SP,#${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0x32 => {
            print!("SHLD   ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0x33 => print!("INX  SP"),
        0x34 => print!("INR   M"),
        0x35 => print!("DCR   M"),
        0x36 => {
            print!("MVI    M,#${:02X}", code[1]);
            opbytes = 2;
        }
        0x37 => print!("STC"),

        0x38 => print!("NOP"),
        0x39 => print!("DAD  SP"),
        0x3a => {
            print!("LDA    ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0x3b => print!("DCX   SP"),
        0x3c => print!("INR   A"),
        0x3d => print!("DCR   A"),
        0x3e => {
            print!("MVI    A,#${:02X}", code[1]);
            opbytes = 2;
        }
        0x3f => print!("CMC"),

        0x40 => print!("MOV   B,B"),
        0x41 => print!("MOV   B,C"),
        0x42 => print!("MOV   B,D"),
        0x43 => print!("MOV   B,E"),
        0x44 => print!("MOV   B,H"),
        0x45 => print!("MOV   B,L"),
        0x46 => print!("MOV   B,M"),
        0x47 => print!("MOV   B,A"),
        0x48 => print!("MOV   C,B"),
        0x49 => print!("MOV   C,C"),
        0x4a => print!("MOV   C,D"),
        0x4b => print!("MOV   C,E"),
        0x4c => print!("MOV   C,H"),
        0x4d => print!("MOV   C,L"),
        0x4e => print!("MOV   C,M"),
        0x4f => print!("MOV   C,A"),

        0x50 => print!("MOV   D,B"),
        0x51 => print!("MOV   D,C"),
        0x52 => print!("MOV   D,D"),
        0x53 => print!("MOV   D,E"),
        0x54 => print!("MOV   D,H"),
        0x55 => print!("MOV   D,L"),
        0x56 => print!("MOV   D,M"),
        0x57 => print!("MOV   D,A"),
        0x58 => print!("MOV   E,B"),
        0x59 => print!("MOV   E,C"),
        0x5a => print!("MOV   E,D"),
        0x5b => print!("MOV   E,E"),
        0x5c => print!("MOV   E,H"),
        0x5d => print!("MOV   E,L"),
        0x5e => print!("MOV   E,M"),
        0x5f => print!("MOV   E,A"),

        0x60 => print!("MOV   H,B"),
        0x61 => print!("MOV   H,C"),
        0x62 => print!("MOV   H,D"),
        0x63 => print!("MOV   H,E"),
        0x64 => print!("MOV   H,H"),
        0x65 => print!("MOV   H,L"),
        0x66 => print!("MOV   H,M"),
        0x67 => print!("MOV   H,A"),
        0x68 => print!("MOV   L,B"),
        0x69 => print!("MOV   L,C"),
        0x6a => print!("MOV   L,D"),
        0x6b => print!("MOV   L,E"),
        0x6c => print!("MOV   L,H"),
        0x6d => print!("MOV   L,L"),
        0x6e => print!("MOV   L,M"),
        0x6f => print!("MOV   L,A"),

        0x70 => print!("MOV   M,B"),
        0x71 => print!("MOV   M,C"),
        0x72 => print!("MOV   M,D"),
        0x73 => print!("MOV   M,E"),
        0x74 => print!("MOV   M,H"),
        0x75 => print!("MOV   M,L"),
        0x76 => print!("HLT"),
        0x77 => print!("MOV   M,A"),
        0x78 => print!("MOV   A,B"),
        0x79 => print!("MOV   A,C"),
        0x7a => print!("MOV   A,D"),
        0x7b => print!("MOV   A,E"),
        0x7c => print!("MOV   A,H"),
        0x7d => print!("MOV   A,L"),
        0x7e => print!("MOV   A,M"),
        0x7f => print!("MOV   A,A"),

        0x80 => print!("ADD   B"),
        0x81 => print!("ADD   C"),
        0x82 => print!("ADD   D"),
        0x83 => print!("ADD   E"),
        0x84 => print!("ADD   H"),
        0x85 => print!("ADD   L"),
        0x86 => print!("ADD   M"),
        0x87 => print!("ADD   A"),
        0x88 => print!("ADC   B"),
        0x89 => print!("ADC   C"),
        0x8a => print!("ADC   D"),
        0x8b => print!("ADC   E"),
        0x8c => print!("ADC   H"),
        0x8d => print!("ADC   L"),
        0x8e => print!("ADC   M"),
        0x8f => print!("ADC   A"),

        0x90 => print!("SUB   B"),
        0x91 => print!("SUB   C"),
        0x92 => print!("SUB   D"),
        0x93 => print!("SUB   E"),
        0x94 => print!("SUB   H"),
        0x95 => print!("SUB   L"),
        0x96 => print!("SUB   M"),
        0x97 => print!("SUB   A"),
        0x98 => print!("SBB   B"),
        0x99 => print!("SBB   C"),
        0x9a => print!("SBB   D"),
        0x9b => print!("SBB   E"),
        0x9c => print!("SBB   H"),
        0x9d => print!("SBB   L"),
        0x9e => print!("SBB   M"),
        0x9f => print!("SBB   A"),

        0xa0 => print!("ANA   B"),
        0xa1 => print!("ANA   C"),
        0xa2 => print!("ANA   D"),
        0xa3 => print!("ANA   E"),
        0xa4 => print!("ANA   H"),
        0xa5 => print!("ANA   L"),
        0xa6 => print!("ANA   M"),
        0xa7 => print!("ANA   A"),
        0xa8 => print!("XRA   B"),
        0xa9 => print!("XRA   C"),
        0xaa => print!("XRA   D"),
        0xab => print!("XRA   E"),
        0xac => print!("XRA   H"),
        0xad => print!("XRA   L"),
        0xae => print!("XRA   M"),
        0xaf => print!("XRA   A"),

        0xb0 => print!("ORA   B"),
        0xb1 => print!("ORA   C"),
        0xb2 => print!("ORA   D"),
        0xb3 => print!("ORA   E"),
        0xb4 => print!("ORA   H"),
        0xb5 => print!("ORA   L"),
        0xb6 => print!("ORA   M"),
        0xb7 => print!("ORA   A"),
        0xb8 => print!("CMP   B"),
        0xb9 => print!("CMP   C"),
        0xba => print!("CMP   D"),
        0xbb => print!("CMP   E"),
        0xbc => print!("CMP   H"),
        0xbd => print!("CMP   L"),
        0xbe => print!("CMP   M"),
        0xbf => print!("CMP   A"),

        0xc0 => print!("RNZ"),
        0xc1 => print!("POP   B"),
        0xc2 => {
            print!("JNZ    ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xc3 => {
            print!("JMP    ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xc4 => {
            print!("CNZ    ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xc5 => print!("PUSH  B"),
        0xc6 => {
            print!("ADI    #${:02X}", code[1]);
            opbytes = 2;
        }
        0xc7 => print!("RST   0"),
        0xc8 => print!("RZ"),
        0xc9 => print!("RET"),
        0xca => {
            print!("JZ     ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xcb => print!("NOP"),
        0xcc => {
            print!("CZ     ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xcd => {
            print!("CALL   ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xce => {
            print!("ACI    #${:02X}", code[1]);
            opbytes = 2;
        }
        0xcf => print!("RST   1"),

        0xd0 => print!("RNC"),
        0xd1 => print!("POP   D"),
        0xd2 => {
            print!("JNC    ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xd3 => {
            print!("OUT    #${:02X}", code[1]);
            opbytes = 2;
        }
        0xd4 => {
            print!("CNC    ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xd5 => print!("PUSH  D"),
        0xd6 => {
            print!("SUI    #${:02X}", code[1]);
            opbytes = 2;
        }
        0xd7 => print!("RST   2"),
        0xd8 => print!("RC"),
        0xd9 => print!("RET"),
        0xda => {
            print!("JC     ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xdb => {
            print!("IN     #${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xdc => {
            print!("CC     ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xdd => {
            print!("CALL   ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xde => {
            print!("SBI    #${:02X}", code[1]);
            opbytes = 2;
        }
        0xdf => print!("RST   3"),

        0xe0 => print!("RPO"),
        0xe1 => print!("POP   H"),
        0xe2 => {
            print!("JPO    ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xe3 => print!("XTHL"),
        0xe4 => {
            print!("CPO    ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xe5 => print!("PUSH  H"),
        0xe6 => {
            print!("ANI    #${:02X}", code[1]);
            opbytes = 2;
        }
        0xe7 => print!("RST   4"),
        0xe8 => print!("RPE"),
        0xe9 => print!("PCHL"),
        0xea => {
            print!("JPE    ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xeb => print!("XCHG"),
        0xec => {
            print!("CPE    ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xed => {
            print!("CALL   ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xee => {
            print!("XRI    #${:02X}", code[1]);
            opbytes = 2;
        }
        0xef => print!("RST   5"),

        0xf0 => print!("RP"),
        0xf1 => print!("POP   PSW"),
        0xf2 => {
            print!("JP     ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xf3 => print!("DI"),
        0xf4 => {
            print!("CP     ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xf5 => print!("PUSH  PSW"),
        0xf6 => {
            print!("ORI    #${:02X}", code[1]);
            opbytes = 2;
        }
        0xf7 => print!("RST   6"),
        0xf8 => print!("RM"),
        0xf9 => print!("SPHL"),
        0xfa => {
            print!("JM     ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xfb => print!("EI"),
        0xfc => {
            print!("CM     ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xfd => {
            print!("CALL   ${:02X}{:02X}", code[2], code[1]);
            opbytes = 3;
        }
        0xfe => {
            print!("CPI    #${:02X}", code[1]);
            opbytes = 2;
        }
        0xff => print!("RST   7"),

        _ => {}
    }

    println!();

    opbytes
}

pub fn emulate8080<T: InOutHandler>(state: &mut State8080<T>, dis: bool) -> i32 {
    assert!(state.pc < state.memory.len());
    let opcode = state.memory[state.pc];
    let mut ans: u16;
    let ans8: u8;
    let ans32: u32;
    let addr: usize;
    if dis { disassemble8080_op(&state.memory, state.pc); }
    state.pc += 1;
    match opcode {
        0x00 => {}, // NOP
        0x01 => { // LXI B,#$WORD
            state.c = state.byte1();
            state.b = state.byte2();
            state.pc += 2;
        },
        0x02 => { // STAX B
            let addr = state.bc();
            state.memory[addr] = state.a;
        },
        0x03 => { // INX B
            ans = (state.bc() + 1) as u16;
            state.b = (ans >> 8) as u8;
            state.c = (ans & 0xff) as u8;
        },
        0x04 => { // INR B
            state.b += 1;
            state.fl.z = state.b == 0;
            state.fl.s = (state.b & 0x7f) != 0;
            state.fl.p = (state.b & 1) == 0;
        },
        0x05 => { // DCR B
            let Wrapping(b) = Wrapping(state.b) - Wrapping(1);
            state.b = b;
            state.fl.z = state.b == 0;
            state.fl.s = (state.b & 0x7f) != 0;
            state.fl.p = (state.b & 1) == 0;
        },
        0x06 => { // MVI B,#$BYTE
            state.b = state.byte1();
            state.pc += 1;
        },
        0x07 => { // RLC
            ans8 = state.a >> 7;
            state.a = (state.a << 1) | ans8;
            state.fl.cy = ans8 == 1;
        },

        0x08 => {}, // NOP
        0x09 => { //DAD B
            let ans32: u32 = (state.hl() as u32) + (state.bc() as u32);
            state.h = ((ans32 >> 8) & 0xff) as u8;
            state.l = (ans32 & 0xff) as u8;
            state.fl.cy = ans32 > 0xffff
        },
        0x0a => {state.a = state.at_bc();}, // LDAX B
        0x0b => { // DCX C
            ans = (state.bc() - 1) as u16;
            state.b = (ans >> 8) as u8;
            state.c = (ans & 0xff) as u8;
        },
        0x0c => { // INR C
            state.c += 1;
            ans8 = state.c;
            state.set_r(ans8);
        },
        0x0d => { // DCR C
            let Wrapping(c) = Wrapping(state.c) - Wrapping(1);
            state.c = c;
            ans8 = state.c;
            state.set_r(ans8);
        },
        0x0e => { // MVI C,#$BYTE
            state.c = state.byte1();
            state.pc += 1;
        },
        0x0f => { // RRC
            ans8 = state.a << 7;
            state.a = (state.a >> 1) | ans8;
            state.fl.cy = ans8 > 0;
        },

        0x10 => {}, // NOP
        0x11 => { // LXI B,#$WORD
            state.e = state.byte1();
            state.d = state.byte2();
            state.pc += 2;
        },
        0x12 => {let addr = state.de(); state.memory[addr] = state.a}, // // STAX D
        0x13 => { // INX D
            ans = (state.de() + 1) as u16;
            state.d = (ans >> 8) as u8;
            state.e = (ans & 0xff) as u8;
        },
        0x14 => { // INR D
            state.d += 1;
            ans8 = state.d;
            state.set_r(ans8);
        },
        0x15 => { // DRC D
            let Wrapping(d) = Wrapping(state.d) - Wrapping(1);
            state.d = d;
            ans8 = state.d;
            state.set_r(ans8)
        },
        0x16 => { //MVI D,#$BYTE
            state.d = state.byte1();
            state.pc += 1;
        },
        0x17 => { // RAL
            let prev_carry = if state.fl.cy { 1 } else { 0 };
            state.fl.cy = (state.a >> 7) == 1;
            state.a = (state.a << 1) | prev_carry;
        },

        0x18 => {}, // NOP
        0x19 => { //DAD D
            ans32 = (state.hl() as u32) + (state.de() as u32);
            state.h = ((ans32 >> 8) & 0xff) as u8;
            state.l = (ans32 & 0xff) as u8;
            state.fl.cy = ans32 > 0xffff
        },
        0x1a => {state.a = state.at_de();}, // LDAX D
        0x1b => { // DCX D
            ans = (state.de() - 1) as u16;
            state.d = (ans >> 8) as u8;
            state.e = (ans & 0xff) as u8;
        },
        0x1c => { // INR E
            state.e += 1;
            ans8 = state.e;
            state.set_r(ans8);
        },
        0x1d => { // DCR E
            let Wrapping(e) = Wrapping(state.e) - Wrapping(1);
            state.e = e;
            ans8 = state.e;
            state.set_r(ans8);
        },
        0x1e => { // MVI E,#$BYTE
            state.e = state.byte1();
            state.pc += 1;
        },
        0x1f => { // RAR
            let prev_carry = if state.fl.cy { 1 } else { 0 };
            state.fl.cy = (state.a & 1) == 1;
            state.a = (state.a >> 1) | prev_carry;
        },

        0x20 => {}, // RIM
        0x21 => { // LXI H,#$WORD
            state.h = state.byte2();
            state.l = state.byte1();
            state.pc += 2;
        },
        0x22 => { // SHLD $WORD
            ans = state.word();
            state.memory[usize::from(ans)] = state.l;
            state.memory[usize::from(ans + 1)] = state.h;
            state.pc += 2;
        },
        0x23 => { // INX H
            ans = (state.hl() + 1) as u16;
            state.h = (ans >> 8) as u8;
            state.l = (ans & 0xff) as u8;
        },
        0x24 => { // INR H
            state.h += 1;
            ans8 = state.h;
            state.set_r(ans8);
        },
        0x25 => { // DCR H
            let Wrapping(h) = Wrapping(state.h) - Wrapping(1);
            state.h = h;
            ans8 = state.h;
            state.set_r(ans8);
        },
        0x26 => { // MVI L,#$BYTE
            state.h = state.byte1();
            state.pc += 1;
        },
        0x27 => {}, // RAA

        0x28 => {}, // NOP
        0x29 => { //DAD H
            let ans32: u32 = (state.hl() as u32) + (state.hl() as u32);
            state.h = ((ans32 >> 8) & 0xff) as u8;
            state.l = (ans32 & 0xff) as u8;
            state.fl.cy = ans32 > 0xffff
        },
        0x2a => { // LHDL $WORD
            ans = state.word();
            state.l = state.memory[usize::from(ans)];
            state.h = state.memory[usize::from(ans + 1)];
            state.pc += 2;
        },
        0x2b => { // DCX H
            ans = (state.hl() - 1) as u16;
            state.h = (ans >> 8) as u8;
            state.l = (ans & 0xff) as u8;
        },
        0x2c => { // INR L
            state.l += 1;
            ans8 = state.l;
            state.set_r(ans8);
        },
        0x2d => { // DCR L
            let Wrapping(l) = Wrapping(state.l) - Wrapping(1);
            state.l = l;
            ans8 = state.l;
            state.set_r(ans8);
        },
        0x2e => { // MVI L,#$BYTE
            state.l = state.byte1();
            state.pc += 1;
        },
        0x2f => { // CMA
            state.a = !state.a;
        },

        0x30 => {}, // NOP
        0x31 => { // LXI SP,#$WORD
            state.sp = usize::from(state.word());
            state.pc += 2;
        },
        0x32 => { // STA $WORD
            ans = state.word();
            state.memory[usize::from(ans)] = state.a;
            state.pc += 2;
        },
        0x33 => { // INX SP
            state.sp = (state.sp + 1) & 0xffff;
        },
        0x34 => { // INR M
            addr = state.hl();
            ans8 = state.memory[addr] + 1;
            state.memory[addr] = ans8;
            state.set_r(ans8);
        },
        0x35 => { // DCR M
            addr = state.hl();
            ans8 = state.memory[addr] - 1;
            state.memory[addr] = ans8;
            state.set_r(ans8);
        },
        0x36 => { //MVI M,#$BYTE
            addr = state.hl();
            state.memory[addr] = state.byte1();
            state.pc += 1;
        },
        0x37 => { // STC
            state.fl.cy = true;
        },

        0x38 => {}, // NOP
        0x39 => { // DAD SP
            ans32 = (state.hl() as u32) + (state.sp as u32);
            state.h = ((ans32 >> 8) & 0xff) as u8;
            state.l = (ans32 & 0xff) as u8;
            state.fl.cy = ans32 > 0xffff;
        },
        0x3a => { // LDA $WORD
            addr = usize::from(state.word());
            state.a = state.memory[addr];
            state.pc += 2;

        },
        0x3b => { // DCX SP
            ans = (state.sp - 1) as u16;
            state.sp = usize::from(ans);
        },
        0x3c => { // INR A
            state.a += 1;
            ans8 = state.a;
            state.set_r(ans8);
        },
        0x3d => { // DCR A
            let Wrapping(a) = Wrapping(state.a) - Wrapping(1);
            state.a = a;
            ans8 = state.a;
            state.set_r(ans8);
        },
        0x3e => { // MVI A,#$BYTE
            state.a = state.byte1();
            state.pc += 1;
        },
        0x3f => { // CMA
            state.fl.cy = !state.fl.cy;
        },

        op @ 0x40..=0x7F => { // MOV DST,SRC
            let src = op & 0b111;
            let dst = (op >> 3) & 0b111;
            let val = state.get_register(src);
            state.set_register(dst, val);
        },

        op @ 0x80..=0x87 => { // ADD REG
            let src = op & 0b111;
            state.add_op(src);
        },

        op @ 0x88..=0x8F => { // ADC REG
            let val = state.get_register(op & 0b111);
            state.adc(val);
        },

        op @ 0x90..=0x97 => { // SUB REG
            let val = state.get_register(op & 0b111);
            state.sub(val);
        },

        op @ 0x98..=0x9F => { // SBB REG
            let val = state.get_register(op & 0b111);
            state.sbb(val);
        },

        op @ 0xA0..=0xA7 => { // AND REG
            let val = state.get_register(op & 0b111);
            state.and(val);
        },

        op @ 0xA8..=0xAF => { // XOR REG
            let val = state.get_register(op & 0b111);
            state.xor(val);
        },

        op @ 0xB0..=0xB7 => { // OR REG
            let val = state.get_register(op & 0b111);
            state.or(val);
        },

        op @ 0xB8..=0xBF => { // CMP REG
            let val = state.get_register(op & 0b111);
            state.cmp(val);
        },

        0xc0 => {if !state.fl.z {state.ret()}}, // RNZ
        0xc1 => { // POP B
            ans = state.pop();
            state.b = (ans >> 8) as u8;
            state.c = (ans & 0xff) as u8;
        },
        0xc2 => {if !state.fl.z {state.pc = usize::from(state.word())} else {state.pc += 2}}, // JNZ
        0xc3 => {state.pc = usize::from(state.word());}, // JMP
        0xc4 => { // CNZ $WORD
            state.call_if(!state.fl.z)
        },
        0xc5 => {ans = state.bc() as u16; state.push(ans)} //PUSH  B,
        0xc6 => {ans8 = state.byte1(); state.add(ans8); state.pc += 1;}, // ADI #$BYTE
        0xc7 => {state.unimplemented_instruction()}, // RST 0
        0xc8 => {if state.fl.z {state.ret()}}, // RZ
        0xc9 => {state.ret()}, // RET
        0xca => {if state.fl.z {state.pc = usize::from(state.word())} else { state.pc += 2 }}, // JZ
        0xcb => {}, // NOP
        0xcc => {state.call_if(state.fl.z)}, // CZ
        0xcd => {ans = state.word(); state.call(ans)}, // CALL $WORD
        0xce => {ans8 = state.byte1(); state.adc(ans8); state.pc += 1;}, // ACI #$BYTE
        0xcf => {state.generate_interrupt(1);}, // RST 1

        0xd0 => {if !state.fl.cy {state.ret()}}, // RNC
        0xd1 => { // POP D
            ans = state.pop();
            state.d = (ans >> 8) as u8;
            state.e = (ans & 0xff) as u8;
        },
        0xd2 => {if !state.fl.cy {state.pc = usize::from(state.word())} else {state.pc += 2}}, // JNC
        0xd3 => {state.pc += 1}, // OUT
        0xd4 => {state.call_if(!state.fl.cy)}, // CNC
        0xd5 => {ans = state.de() as u16; state.push(ans)} //PUSH  D
        0xd6 => {ans8 = state.byte1(); state.sub(ans8); state.pc += 1;}, // SUI #$BYTE
        0xd7 => {state.unimplemented_instruction()}, // RST 2
        0xd8 => {if state.fl.cy {state.ret()}}, // RC
        0xd9 => {}, // NOP
        0xda => {if state.fl.cy {state.pc = usize::from(state.word())} else {state.pc += 2}}, // JC
        0xdb => {state.pc += 1}, // IN
        0xdc => {state.call_if(state.fl.cy)}, // CC
        0xdd => {ans = state.word(); state.call(ans)}, // CALL
        0xde => {ans8 = state.byte1(); state.sbb(ans8); state.pc += 1;}, // SBI #$BYTE
        0xdf => {state.unimplemented_instruction()}, // RST 3

        0xe0 => {if !state.fl.p {state.ret()}}, // RPO
        0xe1 => { // POP H
            ans = state.pop();
            state.h = (ans >> 8) as u8;
            state.l = (ans & 0xff) as u8;
        },
        0xe2 => {if !state.fl.p {state.pc = usize::from(state.word())} else {state.pc += 2}}, // JPO
        0xe3 => { // XTHL
            ans = state.pop();
            let temp = state.hl() as u16;
            state.push(temp);
            state.h = (ans >> 8) as u8;
            state.l = (ans & 0xff) as u8;
        },
        0xe4 => {state.call_if(!state.fl.p)}, // CPO
        0xe5 => {ans = state.hl() as u16; state.push(ans)} //PUSH  H
        0xe6 => {ans8 = state.byte1(); state.and(ans8); state.pc += 1;}, // ANI #$BYTE
        0xe7 => {state.unimplemented_instruction()}, // RST 4
        0xe8 => {if state.fl.p {state.ret()}}, // RPE
        0xe9 => { // PCHL
            state.pc = (usize::from(state.h) << 8) | usize::from(state.l);
        },
        0xea => {if state.fl.p {state.pc = usize::from(state.word())} else {state.pc += 2}}, // JPE
        0xeb => { // XCHG
            std::mem::swap(&mut state.d, &mut state.h);
            std::mem::swap(&mut state.e, &mut state.l);
        },
        0xec => {state.call_if(state.fl.p)}, // CPE
        0xed => {}, // NOP
        0xee => {ans8 = state.byte1(); state.xor(ans8); state.pc += 1;}, // XRI #$BYTE
        0xef => {state.unimplemented_instruction()}, // RST 4

        0xf0 => {if !state.fl.s {state.ret()}}, // RP
        0xf1 => { // POP PSW
            ans = state.pop();
            state.a = (ans >> 8) as u8;
            state.fl.s = ans & 0x80 > 0;
            state.fl.z = ans & 0x40 > 0;
            state.fl.ac = ans & 0x10 > 0;
            state.fl.p = ans & 0x04 > 0;
            state.fl.cy = ans & 0x01 > 0;
        },
        0xf2 => {if !state.fl.s {state.pc = usize::from(state.word())} else {state.pc += 2}}, // true
        0xf3 => {state.int_enable = false}, // DI
        0xf4 => {state.call_if(!state.fl.s)}, // CP
        0xf5 => { // PUSH PWS
            ans = u16::from(state.a) << 8;
            if state.fl.s {ans |= 0x80};
            if state.fl.z {ans |= 0x40};
            if state.fl.ac {ans |= 0x10};
            if state.fl.p {ans |= 0x04};
            if state.fl.cy {ans |= 0x01};
            state.push(ans);
        },
        0xf6 => {ans8 = state.byte1(); state.or(ans8); state.pc += 1;}, // ANI #$BYTE
        0xf7 => {state.unimplemented_instruction()}, // RST 6
        0xf8 => {if state.fl.s { state.ret() }}, // RM
        0xf9 => {state.sp = state.hl()}, // SPHL
        0xfa => {if state.fl.s { state.pc = usize::from(state.word()); } else { state.pc += 2 }}, // JM
        0xfb => {state.int_enable = true}, // EI
        0xfc => {state.call_if(state.fl.s)}, // CM
        0xfd => {ans = state.word(); state.call(ans)}, // CALL
        0xfe => {ans8 = state.byte1(); state.cmp(ans8); state.pc += 1;}, // CPI #$BYTE
        0xff => {state.unimplemented_instruction()}, // RST 7

        _ => {}
    };

    if dis {
        println!(
            "Registers: A: {:02X} BC: {:02X}{:02X} DE: {:02X}{:02X} HL: {:02X}{:02X}",
            state.a, state.b, state.c, state.d, state.e, state.h, state.l
        );
        println!(
            "Flags: s: {} z: {} p: {} cy: {}",
            state.fl.s, state.fl.z, state.fl.p, state.fl.cy
        );
    }

    0
}
