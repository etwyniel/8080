#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

struct Flags {
    z: bool,
    s: bool,
    p: bool,
    cy: bool,
    ac: bool,
    pad: i32
}

pub struct State8080 {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    sp: usize,
    pc: usize,
    memory: Vec<u8>,
    fl: Flags,
    int_enable: u8
}

impl State8080 {
    fn unimplemented_instruction(&mut self) {
        println!("Error: unimplemented instruction");
        std::process::exit(1);
    }

    fn bc(&self) -> usize {
        usize::from((self.b << 8) | self.c)
    }

    fn de(&self) -> usize {
        usize::from((self.d << 8) | self.e)
    }

    fn hl(&self) -> usize {
        usize::from((self.h << 8) | self.l)
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
        self.memory[self.pc + 1]
    }

    fn byte2(&self) -> u8 {
        self.memory[self.pc + 2]
    }

     fn word(&self) -> u16 {
         self.word_at(self.pc + 1)
     }

     fn word_at(&self, addr: usize) -> u16 {
         (u16::from(self.memory[addr + 1]) << 8) | u16::from(self.memory[addr])
     }

     fn set_r(&mut self, res: u8) {
        self.fl.z = res == 0;
        self.fl.s = (res & 0x7f) != 0;
        self.fl.p = (res & 1) == 0;
     }

     fn set_flags(&mut self, res: u16) {
         self.fl.cy = res > 0xff;
         self.set_r(res as u8);
     }

     fn add(&mut self, val: u8) {
        let ans = u16::from(self.a) + u16::from(val);
        self.set_flags(ans);
        self.a = ans as u8;
     }

     fn adc(&mut self, val: u8) {
         let mut cy = if self.fl.cy {1u8} else {0u8};
         self.add(val);
         if self.fl.cy {cy += 1};
         self.add(cy);
     }

     fn sub(&mut self, val: u8) {
        let ans = u16::from(self.a) - u16::from(val);
        self.set_flags(ans);
        self.a = ans as u8;
     }

     fn sbb(&mut self, val: u8) {
         let mut cy = if self.fl.cy {1u8} else {0u8};
         self.sub(val);
         if self.fl.cy {cy += 1};
         self.sub(cy);
     }

     fn and(&mut self, val: u8) {
         self.a = self.a & val;
         let temp = self.a;
         self.set_r(temp);
     }

     fn xor(&mut self, val: u8) {
         self.a = self.a ^ val;
         let temp = self.a;
         self.set_r(temp);
     }

     fn or(&mut self, val: u8) {
         self.a = self.a | val;
         let temp = self.a;
         self.set_r(temp);
     }

     fn cmp(&mut self, val: u8) {
        let ans = u16::from(self.a) - u16::from(val);
        self.set_flags(ans);
     }

     fn pop(&mut self) -> u16 {
         let r = (u16::from(self.memory[self.sp+1]) << 8) | u16::from(self.memory[self.sp]);
         self.sp += 2;
         r
     }

     fn push(&mut self, val: u16) {
         self.memory[self.sp] = (val & 0xff) as u8;
         self.memory[self.sp + 1] = (val >> 8) as u8;
         self.sp -= 2;
     }

     fn ret(&mut self) {
         self.pc = usize::from(self.pop());
     }

     fn call(&mut self, addr: u16) {
         let pc = self.pc as u16;
         self.push(pc);
         self.pc = usize::from(addr);
     }
}

fn disassemble8080_op(codebuffer: &[u8], pc: usize) -> usize {
    let code = &codebuffer[pc..];
    let mut opbytes = 1;
    print!("{:04x} ", pc);
    match code[0] {
        0x00 => print!("NOP"),
        0x01 => {print!("LXI    B,#${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0x02 => print!("STAX   B"),
        0x03 => print!("INX   B"),
        0x04 => print!("INR   B"),
        0x05 => print!("DCR   B"),
        0x06 => {print!("MVI    B,#${:02x}", code[1]); opbytes = 2;},
        0x07 => print!("RLC"),

        0x08 => print!("NOP"),
        0x09 => print!("DAD   B"),
        0x0a => print!("LDAX   B"),
        0x0b => print!("DCX   C"),
        0x0c => print!("INR   C"),
        0x0d => print!("DCR   C"),
        0x0e => {print!("MVI    C,#${:02x}", code[1]); opbytes = 2;},
        0x0f => print!("RRC"),

        0x10 => print!("NOP"),
        0x11 => {print!("LXI    B,#${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0x12 => print!("STAX   B"),
        0x13 => print!("INX   D"),
        0x14 => print!("INR   D"),
        0x15 => print!("DCR   D"),
        0x16 => {print!("MVI    B,#${:02x}", code[1]); opbytes = 2;},
        0x17 => print!("RLC"),

        0x18 => print!("NOP"),
        0x19 => print!("DAD   D"),
        0x1a => print!("LDAX   D"),
        0x1b => print!("DCX   D"),
        0x1c => print!("INR   E"),
        0x1d => print!("DCR   E"),
        0x1e => {print!("MVI    E,#${:02x}", code[1]); opbytes = 2;},
        0x1f => print!("RAR"),

        0x20 => print!("NOP"),
        0x21 => {print!("LXI    H,#${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0x22 => {print!("SHLD   ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0x23 => print!("INX   H"),
        0x24 => print!("INR   H"),
        0x25 => print!("DCR   H"),
        0x26 => {print!("MVI    H,#${:02x}", code[1]); opbytes = 2;},
        0x27 => print!("DAA"),

        0x28 => print!("NOP"),
        0x29 => print!("DAD   H"),
        0x2a => {print!("LHLD   ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0x2b => print!("DCX    H"),
        0x2c => print!("INR   L"),
        0x2d => print!("DCR   L"),
        0x2e => {print!("MVI    L,#${:02x}", code[1]); opbytes = 2;},
        0x2f => print!("CMA"),

        0x30 => print!("NOP"),
        0x31 => {print!("LXI   SP,#${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0x32 => {print!("SHL    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0x33 => print!("INX  SP"),
        0x34 => print!("INR   M"),
        0x35 => print!("DCR   M"),
        0x36 => {print!("MVI    M,#${:02x}", code[1]); opbytes = 2;},
        0x37 => print!("STC"),

        0x38 => print!("NOP"),
        0x39 => print!("DAD  SP"),
        0x3a => {print!("LDA    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0x3b => print!("DCX   SP"),
        0x3c => print!("INR   A"),
        0x3d => print!("DCR   A"),
        0x3e => {print!("MVI    A,#${:02x}", code[1]); opbytes = 2;},
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
        0xc2 => {print!("JNZ    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xc3 => {print!("JMP    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xc4 => {print!("CNZ    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xc5 => print!("PUSH  B"),
        0xc6 => {print!("ADI    #${:02x}", code[1]); opbytes = 2;},
        0xc7 => print!("RST   0"),
        0xc8 => print!("RZ"),
        0xc9 => print!("RET"),
        0xca => {print!("JZ     ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xcb => {print!("NOP")},
        0xcc => {print!("CZ     ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xcd => {print!("CALL   ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xce => {print!("ACI    #${:02x}", code[1]); opbytes = 2;},
        0xcf => print!("RST   1"),

        0xd0 => print!("RNC"),
        0xd1 => print!("POP   D"),
        0xd2 => {print!("JNC    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xd3 => {print!("OUT    #${:02x}", code[1]); opbytes = 2;},
        0xd4 => {print!("CNC    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xd5 => print!("PUSH  D"),
        0xd6 => {print!("SUI    #${:02x}", code[1]); opbytes = 2;},
        0xd7 => print!("RST   2"),
        0xd8 => print!("RC"),
        0xd9 => print!("RET"),
        0xda => {print!("JC     ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xdb => {print!("IN     #${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xdc => {print!("CC     ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xdd => {print!("CALL   ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xde => {print!("SBI    #${:02x}", code[1]); opbytes = 2;},
        0xdf => print!("RST   3"),

        0xe0 => print!("RPO"),
        0xe1 => print!("POP   H"),
        0xe2 => {print!("JPO    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xe3 => print!("XTHL"),
        0xe4 => {print!("CPO    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xe5 => print!("PUSH  H"),
        0xe6 => {print!("ANI    #${:02x}", code[1]); opbytes = 2;},
        0xe7 => print!("RST   4"),
        0xe8 => print!("RPE"),
        0xe9 => print!("PCHL"),
        0xea => {print!("JPE    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xeb => print!("XCHG"),
        0xec => {print!("CPE    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xed => {print!("CALL   ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xee => {print!("XRI    #${:02x}", code[1]); opbytes = 2;},
        0xef => print!("RST   5"),

        0xf0 => print!("RP"),
        0xf1 => print!("POP   PSW"),
        0xf2 => {print!("JP     ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xf3 => print!("DI"),
        0xf4 => {print!("CP     ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xf5 => print!("PUSH  PSW"),
        0xf6 => {print!("ORI    #${:02x}", code[1]); opbytes = 2;},
        0xf7 => print!("RST   6"),
        0xf8 => print!("RM"),
        0xf9 => print!("SPHL"),
        0xfa => {print!("JM     ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xfb => print!("EI"),
        0xfc => {print!("CM     ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xfd => {print!("CALL   ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xfe => {print!("CPI    #${:02x}", code[1]); opbytes = 2;},
        0xff => print!("RST   7"),

        _ => {}
    }

    opbytes
}

pub fn emulate8080(state: &mut State8080) {
    let opcode = state.memory[state.pc];
    let mut opbytes = 1;
    let mut ans: u16;
    let mut ans8: u8;
    let ans32: u32;
    let addr: usize;
    match opcode {
        0x00 => {}, // NOP
        0x01 => { // LXI B,#$WORD
            state.c = state.memory[state.pc + 1];
            state.b = state.memory[state.pc + 2];
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
            state.b -= 1;
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
            state.c -= 1;
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
            state.e = state.byte2();
            state.d = state.byte1();
            state.pc += 2;
        },
        0x12 => {state.a = state.at_bc()}, // // STAX B
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
            state.d -= 1;
            ans8 = state.d;
            state.set_r(ans8)
        },
        0x16 => { //MVI D,#$BYTE
            state.b = state.byte1();
        },
        0x17 => { // RAL
            state.fl.cy = (state.a >> 7) == 1;
            state.a = (state.a >> 1) | (if state.fl.cy {1u8} else {0u8});
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
            state.e -= 1;
            ans8 = state.e;
            state.set_r(ans8);
        },
        0x1e => { // MVI E,#$BYTE
            state.e = state.byte1();
            state.pc += 1;
        },
        0x1f => { // RAL
            state.fl.cy = (state.a << 7) > 0;
            state.a = (state.a << 1) | (if state.fl.cy {0x80u8} else {0u8});
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
        },
        0x23 => { // INX H
            ans = (state.hl() + 1) as u16;
            state.h = (ans >> 8) as u8;
            state.l = (ans & 0xff) as u8;
        },
        0x24 => { // INR L
            state.l += 1;
            ans8 = state.l;
            state.set_r(ans8);
        },
        0x25 => { // DCR L
            state.l -= 1;
            ans8 = state.l;
            state.set_r(ans8);
        },
        0x26 => { // MVI L,#$BYTE
            state.l = state.byte1();
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
            state.l -= 1;
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
            state.memory[addr] = state.a;

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
            state.a -= 1;
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

        0x40 => {state.b = state.b}, // MOV B,B
        0x41 => {state.b = state.c}, // MOV B,C
        0x42 => {state.b = state.d}, // MOV B,D
        0x43 => {state.b = state.e}, // MOV B,E
        0x44 => {state.b = state.h}, // MOV B,H
        0x45 => {state.b = state.l}, // MOV B,L
        0x46 => {state.b = state.at_hl()}, // MOV B,M
        0x47 => {state.b = state.a}, // MOV B,A
        0x48 => {state.c = state.b}, // MOV C,B
        0x49 => {state.c = state.b}, // MOV C,C
        0x4a => {state.c = state.b}, // MOV C,D
        0x4b => {state.c = state.b}, // MOV C,E
        0x4c => {state.c = state.b}, // MOV C,H
        0x4d => {state.c = state.b}, // MOV C,L
        0x4e => {state.c = state.at_hl()}, // MOV C,M
        0x4f => {state.c = state.b}, // MOV C,A

        0x50 => {state.d = state.b}, // MOV D,B
        0x51 => {state.d = state.c}, // MOV D,C
        0x52 => {state.d = state.d}, // MOV D,D
        0x53 => {state.d = state.e}, // MOV D,E
        0x54 => {state.d = state.h}, // MOV D,H
        0x55 => {state.d = state.l}, // MOV D,L
        0x56 => {state.d = state.at_hl()}, // MOV D,M
        0x57 => {state.d = state.a}, // MOV D,A
        0x58 => {state.e = state.b}, // MOV E,B
        0x59 => {state.e = state.b}, // MOV E,C
        0x5a => {state.e = state.b}, // MOV E,D
        0x5b => {state.e = state.b}, // MOV E,E
        0x5c => {state.e = state.b}, // MOV E,H
        0x5d => {state.e = state.b}, // MOV E,L
        0x5e => {state.e = state.at_hl()}, // MOV E,M
        0x5f => {state.e = state.b}, // MOV E,A

        0x60 => {state.h = state.b}, // MOV H,B
        0x61 => {state.h = state.c}, // MOV H,C
        0x62 => {state.h = state.d}, // MOV H,D
        0x63 => {state.h = state.e}, // MOV H,E
        0x64 => {state.h = state.h}, // MOV H,H
        0x65 => {state.h = state.l}, // MOV H,L
        0x66 => {state.h = state.at_hl()}, // MOV H,M
        0x67 => {state.h = state.a}, // MOV H,A
        0x68 => {state.l = state.b}, // MOV L,B
        0x69 => {state.l = state.b}, // MOV L,C
        0x6a => {state.l = state.b}, // MOV L,D
        0x6b => {state.l = state.b}, // MOV L,E
        0x6c => {state.l = state.b}, // MOV L,H
        0x6d => {state.l = state.b}, // MOV L,L
        0x6e => {state.l = state.at_hl()}, // MOV L,M
        0x6f => {state.l = state.b}, // MOV L,A

        0x70 => {addr = state.hl(); state.memory[addr] = state.b}, // MOV M,B
        0x71 => {addr = state.hl(); state.memory[addr] = state.c}, // MOV M,C
        0x72 => {addr = state.hl(); state.memory[addr] = state.d}, // MOV M,D
        0x73 => {addr = state.hl(); state.memory[addr] = state.e}, // MOV M,E
        0x74 => {addr = state.hl(); state.memory[addr] = state.h}, // MOV M,H
        0x75 => {addr = state.hl(); state.memory[addr] = state.l}, // MOV M,L
        0x76 => {}, // HLT
        0x77 => {addr = state.hl(); state.memory[addr] = state.a}, // MOV M,A
        0x78 => {state.a = state.b}, // MOV A,B
        0x79 => {state.a = state.b}, // MOV A,C
        0x7a => {state.a = state.b}, // MOV A,D
        0x7b => {state.a = state.b}, // MOV A,E
        0x7c => {state.a = state.b}, // MOV A,H
        0x7d => {state.a = state.b}, // MOV A,L
        0x7e => {state.a = state.at_hl()}, // MOV A,M
        0x7f => {state.a = state.b}, // MOV A,A

        0x80 => {ans8 = state.b; state.add(ans8)}, // ADD B
        0x81 => {ans8 = state.c; state.add(ans8)}, // ADD C
        0x82 => {ans8 = state.d; state.add(ans8)}, // ADD D
        0x83 => {ans8 = state.e; state.add(ans8)}, // ADD E
        0x84 => {ans8 = state.h; state.add(ans8)}, // ADD H
        0x85 => {ans8 = state.l; state.add(ans8)}, // ADD L
        0x86 => {ans8 = state.at_hl(); state.add(ans8)}, // ADD M
        0x87 => {ans8 = state.a; state.add(ans8)}, // ADD A
        0x88 => {ans8 = state.b; state.adc(ans8)}, // ADC B
        0x89 => {ans8 = state.c; state.adc(ans8)}, // ADC C
        0x8a => {ans8 = state.d; state.adc(ans8)}, // ADC D
        0x8b => {ans8 = state.e; state.adc(ans8)}, // ADC E
        0x8c => {ans8 = state.h; state.adc(ans8)}, // ADC H
        0x8d => {ans8 = state.l; state.adc(ans8)}, // ADC L
        0x8e => {ans8 = state.at_hl(); state.adc(ans8)}, // ADC M
        0x8f => {ans8 = state.a; state.adc(ans8)}, // ADC A

        0x90 => {ans8 = state.b; state.sub(ans8)}, // SUB B
        0x91 => {ans8 = state.c; state.sub(ans8)}, // SUB C
        0x92 => {ans8 = state.d; state.sub(ans8)}, // SUB D
        0x93 => {ans8 = state.e; state.sub(ans8)}, // SUB E
        0x94 => {ans8 = state.h; state.sub(ans8)}, // SUB H
        0x95 => {ans8 = state.l; state.sub(ans8)}, // SUB L
        0x96 => {ans8 = state.at_hl(); state.sub(ans8)}, // SUB M
        0x97 => {ans8 = state.a; state.sub(ans8)}, // SUB A
        0x98 => {ans8 = state.b; state.sbb(ans8)}, // SBB B
        0x99 => {ans8 = state.c; state.sbb(ans8)}, // SBB C
        0x9a => {ans8 = state.d; state.sbb(ans8)}, // SBB D
        0x9b => {ans8 = state.e; state.sbb(ans8)}, // SBB E
        0x9c => {ans8 = state.h; state.sbb(ans8)}, // SBB H
        0x9d => {ans8 = state.l; state.sbb(ans8)}, // SBB L
        0x9e => {ans8 = state.at_hl(); state.sbb(ans8)}, // SBB M
        0x9f => {ans8 = state.a; state.sbb(ans8)}, // SBB A

        0xa0 => {ans8 = state.b; state.and(ans8)}, // ANA B
        0xa1 => {ans8 = state.c; state.and(ans8)}, // ANA C
        0xa2 => {ans8 = state.d; state.and(ans8)}, // ANA D
        0xa3 => {ans8 = state.e; state.and(ans8)}, // ANA E
        0xa4 => {ans8 = state.h; state.and(ans8)}, // ANA H
        0xa5 => {ans8 = state.l; state.and(ans8)}, // ANA L
        0xa6 => {ans8 = state.at_hl(); state.and(ans8)}, // ANA M
        0xa7 => {ans8 = state.a; state.and(ans8)}, // ANA A
        0xa8 => {ans8 = state.b; state.xor(ans8)}, // XRA B
        0xa9 => {ans8 = state.c; state.xor(ans8)}, // XRA C
        0xaa => {ans8 = state.d; state.xor(ans8)}, // XRA D
        0xab => {ans8 = state.e; state.xor(ans8)}, // XRA E
        0xac => {ans8 = state.h; state.xor(ans8)}, // XRA H
        0xad => {ans8 = state.l; state.xor(ans8)}, // XRA L
        0xae => {ans8 = state.at_hl(); state.xor(ans8)}, // XRA M
        0xaf => {ans8 = state.a; state.xor(ans8)}, // XRA A

        0xb0 => {ans8 = state.b; state.or(ans8)}, // ORA B
        0xb1 => {ans8 = state.c; state.or(ans8)}, // ORA C
        0xb2 => {ans8 = state.d; state.or(ans8)}, // ORA D
        0xb3 => {ans8 = state.e; state.or(ans8)}, // ORA E
        0xb4 => {ans8 = state.h; state.or(ans8)}, // ORA H
        0xb5 => {ans8 = state.l; state.or(ans8)}, // ORA L
        0xb6 => {ans8 = state.at_hl(); state.or(ans8)}, // ORA M
        0xb7 => {ans8 = state.a; state.or(ans8)}, // ORA A
        0xb8 => {ans8 = state.b; state.cmp(ans8)}, // CMP B
        0xb9 => {ans8 = state.c; state.cmp(ans8)}, // CMP C
        0xba => {ans8 = state.d; state.cmp(ans8)}, // CMP D
        0xbb => {ans8 = state.e; state.cmp(ans8)}, // CMP E
        0xbc => {ans8 = state.h; state.cmp(ans8)}, // CMP H
        0xbd => {ans8 = state.l; state.cmp(ans8)}, // CMP L
        0xbe => {ans8 = state.at_hl(); state.cmp(ans8)}, // CMP M
        0xbf => {ans8 = state.a; state.cmp(ans8)}, // CMP A

        0xc0 => {if !state.fl.z {state.ret()}}, // RNZ
        0xc1 => { // POP B
            ans = state.pop();
            state.b = (ans & 0xff) as u8;
            state.c = (ans >> 8) as u8
        },
        0xc2 => {if !state.fl.z {state.pc = usize::from(state.word())}}, // JNZ
        0xc3 => {state.pc = usize::from(state.word())}, // JMP
        0xc4 => { // CNZ $WORD
            if !state.fl.z {ans = state.word(); state.call(ans)}
        },
        0xc5 => {ans = state.bc() as u16; state.push(ans)} //PUSH  B,
        0xc6 => {ans8 = state.byte1(); state.add(ans8)}, // ADI #$BYTE
        0xc7 => {state.unimplemented_instruction()}, // RST 0
        0xc8 => {if state.fl.z {state.ret()}}, // RZ
        0xc9 => {state.ret()}, // RET
        0xca => {if state.fl.z {state.pc = usize::from(state.word())}}, // JZ
        0xcb => {}, // NOP
        0xcc => {if state.fl.z {ans = state.word(); state.call(ans)}}, // CZ
        0xcd => {ans = state.word(); state.call(ans)}, // CALL $WORD
        0xce => {ans8 = state.byte1(); state.adc(ans8)}, // ACI #$BYTE
        0xcf => {state.unimplemented_instruction()}, // RST 1

        0xd0 => {if !state.fl.cy {state.ret()}}, // RNC
        0xd1 => { // POP D
            ans = state.pop();
            state.d = (ans & 0xff) as u8;
            state.e = (ans >> 8) as u8
        },
        0xd2 => {if !state.fl.cy {state.pc = usize::from(state.word())}}, // JNC
        0xd3 => {state.pc += 1}, // OUT
        0xd4 => {if !state.fl.cy {ans = state.word(); state.call(ans)}}, // CNC
        0xd5 => {ans = state.de() as u16; state.push(ans)} //PUSH  D
        0xd6 => {ans8 = state.byte1(); state.sub(ans8)}, // SUI #$BYTE
        0xd7 => {state.unimplemented_instruction()}, // RST 2
        0xd8 => {if state.fl.cy {state.ret()}}, // RC
        0xd9 => {}, // NOP
        0xda => {if state.fl.cy {state.pc = usize::from(state.word())}}, // JC
        0xdb => {state.pc += 1}, // IN
        0xdc => {if state.fl.cy {ans = state.word(); state.call(ans)}}, // CC
        0xdd => {ans = state.word(); state.call(ans)}, // CALL
        0xde => {ans8 = state.byte1(); state.sbb(ans8)}, // SBI #$BYTE
        0xdf => {state.unimplemented_instruction()}, // RST 3

        0xe0 => {if state.fl.p {state.ret()}}, // RPO
        0xe1 => { // POP H
            ans = state.pop();
            state.h = (ans & 0xff) as u8;
            state.l = (ans >> 8) as u8
        },
        0xe2 => {if state.fl.p {state.pc = usize::from(state.word())}}, // JPO
        0xe3 => { // XTHL
            ans = state.pop();
            let temp = state.hl() as u16;
            state.push(temp);
            state.h = (ans & 0xff) as u8;
            state.l = (ans >> 8) as u8;
        },
        0xe4 => {if state.fl.p {ans = state.word(); state.call(ans)}}, // CPO
        0xe5 => {ans = state.hl() as u16; state.push(ans)} //PUSH  H
        0xe6 => {ans8 = state.byte1(); state.and(ans8)}, // ANI #$BYTE
        0xe7 => {state.unimplemented_instruction()}, // RST 4
        0xe8 => {if !state.fl.p {state.ret()}}, // RPE
        0xe9 => { // PCHL
            state.pc = (usize::from(state.h) << 8) | usize::from(state.l);
        },
        0xea => {if !state.fl.p {state.pc = usize::from(state.word())}}, // JPE
        0xeb => { // XTHL
            ans = state.de() as u16;
            let temp = state.hl() as u16;
            state.d = (temp & 0xff) as u8;
            state.e = (temp >> 8) as u8;
            state.h = (ans & 0xff) as u8;
            state.l = (ans >> 8) as u8;
        },
        0xec => {if !state.fl.p {ans = state.word(); state.call(ans)}}, // CPE
        0xed => {}, // NOP
        0xee => {ans8 = state.byte1(); state.xor(ans8)}, // XRI #$BYTE
        0xef => {state.unimplemented_instruction()}, // RST 4

        0xf0 => {if state.fl.s {state.ret()}}, // RP
        0xf1 => { // POP PSW
            ans = state.pop();
            state.a = (ans >> 8) as u8;
            state.fl.s = ans & 0x80 > 0;
            state.fl.z = ans & 0x40 > 0;
            state.fl.ac = ans & 0x10 > 0;
            state.fl.p = ans & 0x04 > 0;
            state.fl.cy = ans & 0x01 > 0;
        },
        0xf2 => {if !state.fl.s {state.pc = usize::from(state.word())}}, // JP
        0xf3 => {state.int_enable = 0}, // DI
        0xf4 => {if !state.fl.s {ans = state.word(); state.call(ans)}}, // CP
        0xf5 => { // PUSH PWS
            ans = u16::from(state.a) << 8;
            if state.fl.s {ans = ans | 0x80};
            if state.fl.z {ans = ans | 0x40};
            if state.fl.ac {ans = ans | 0x10};
            if state.fl.p {ans = ans | 0x04};
            if state.fl.cy {ans = ans | 0x01};
            state.push(ans);
        },
        0xf6 => {ans8 = state.byte1(); state.or(ans8)}, // ANI #$BYTE
        0xf7 => {state.unimplemented_instruction()}, // RST 6
        0xf8 => {state.unimplemented_instruction()}, // RM
        0xf9 => {state.sp = state.hl()}, // SPHL
        0xfa => {state.unimplemented_instruction()}, // JM
        0xfb => {state.int_enable = 1}, // EI
        0xfc => {state.unimplemented_instruction()}, // CM
        0xfd => {ans = state.word(); state.call(ans)}, // CALL
        0xfe => {ans8 = state.byte1(); state.cmp(ans8)}, // CPI #$BYTE
        0xff => {state.unimplemented_instruction()}, // RST 7

        _ => {}
    };
    state.pc += 1;
}

