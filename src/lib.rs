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
    fn read(&mut self, port: u8) -> u8;
    fn write(&mut self, port: u8, val: u8);
}

pub struct DefaultHandler;

impl InOutHandler for DefaultHandler {
    fn read(&mut self, _port: u8) -> u8 {
        0
    }
    fn write(&mut self, _port: u8, _val: u8) {}
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
        self.z = true;
        self.s = false;
        self.p = true;
        self.cy = false;
        self.ac = false;
    }
}

fn parity(mut x: u8, cy: bool) -> bool {
    let mut p = if cy { 1 } else { 0 };
    for _ in 0..8 {
        if x & 1 != 0 {
            p += 1
        };
        x >>= 1;
    }
    p & 1 == 0
}

#[test]
fn parity_test() {
    assert!(!parity(0x99, true));
}

type Instruction<T> = fn(&mut State8080<T>, u8) -> usize;

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
    pub assignments: [Instruction<T>; 16],
    pub branching: [Instruction<T>; 16],
    pub instr_compact: [Instruction<T>; 32],
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
            memory: vec![0xFF; 0x10000],
            fl: Flags {
                z: false,
                s: false,
                p: false,
                cy: false,
                ac: false,
            },
            int_enable: false,
            io: io_handler,
            assignments: [
                // NOP
                |_, _| 0,               // 0x0
                Self::lxi,              // 0x1
                Self::stax,             // 0x2
                Self::inx,              // 0x3
                Self::inr,              // 0x4
                Self::dcr,              // 0x5
                Self::mvi,              // 0x6
                Self::assignment_extra, // 0x7
                // NOP
                |_, _| 0,               // 0x8
                Self::dad,              // 0x9
                Self::ldax,             // 0xa
                Self::dcx,              // 0xb
                Self::inr,              // 0xc
                Self::dcr,              // 0xd
                Self::mvi,              // 0xe
                Self::assignment_extra, // 0xf
            ],
            branching: [
                Self::ret_instr, // 0x0
                Self::pop_instr, // 0x1
                Self::jmp_instr, // 0x2
                |state, op| {
                    match (op >> 4) & 3 {
                        0 => state.jmp_instr(op),
                        1 => {
                            state.io.write(state.byte1(), state.a);
                            1
                        }
                        2 => {
                            let val = state.pop();
                            state.push(state.hl() as u16);
                            state.h = (val >> 8) as u8;
                            state.l = val as u8;
                            0
                        },
                        3 => {state.int_enable = false; 0},
                        _ => unreachable!(),
                    }
                }, // 0x3
                Self::call_instr, // 0x4
                Self::push_instr, // 0x5
                Self::immediate,  // 0x6
                Self::rst,        // 0x7
                Self::ret_instr,  // 0x8
                |state, op| {
                    match (op >> 4) & 3 {
                        0 => state.ret_instr(op),
                        1 => 0,
                        2 => {state.pc = (usize::from(state.h) << 8) | usize::from(state.l); 0},
                        3 => {state.sp = state.hl(); 0},
                        _ => unreachable!(),
                    }
                }, // 0x9
                Self::jmp_instr,  // 0xa
                |state, op| {
                    match (op >> 4) & 3 {
                        0 => 0,
                        1 => {state.a = state.io.read(state.byte1()); 1},
                        2 => {
                            std::mem::swap(&mut state.d, &mut state.h);
                            std::mem::swap(&mut state.e, &mut state.l);
                            0
                        }
                        3 => {state.int_enable = true; 0},
                        _ => unreachable!(),
                    }
                },         // 0xb
                Self::call_instr, // 0xc
                Self::call_instr, // 0xd
                Self::immediate,  // 0xe
                Self::rst,        // 0xf
            ],
            instr_compact: [
                Self::assignment, // 0x00..0x08
                Self::assignment, // 0x08..0x10
                Self::assignment, // 0x10..0x18
                Self::assignment, // 0x18..0x20
                Self::assignment, // 0x20..0x28
                Self::assignment, // 0x28..0x30
                Self::assignment, // 0x30..0x38
                Self::assignment, // 0x38..0x40
                Self::mov,        // 0x40..0x48
                Self::mov,        // 0x48..0x50
                Self::mov,        // 0x50..0x58
                Self::mov,        // 0x58..0x60
                Self::mov,        // 0x60..0x68
                Self::mov,        // 0x68..0x70
                Self::mov,        // 0x70..0x78
                Self::mov,        // 0x78..0x80
                Self::add_instr,  // 0x80..0x88
                Self::adc_instr,  // 0x88..0x90
                Self::sub_instr,  // 0x90..0x98
                Self::sbb_instr,  // 0x98..0xa0
                Self::and_instr,  // 0xa0..0xa8
                Self::xor_instr,  // 0xa8..0xb0
                Self::or_instr,   // 0xb0..0xb8
                Self::cmp_instr,  // 0xb8..0xc0
                Self::branch,     // 0xc0..0xc8
                Self::branch,     // 0xc8..0xd0
                Self::branch,     // 0xd0..0xd8
                Self::branch,     // 0xd8..0xe0
                Self::branch,     // 0xe0..0xe8
                Self::branch,     // 0xe8..0xf0
                Self::branch,     // 0xf0..0xf8
                Self::branch,     // 0xf8..0x100
            ],
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
            6 => {
                let addr = self.hl();
                self.memory[addr] = val
            }
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

    pub fn get_long(&self, op: u8) -> usize {
        match (op >> 4) & 3 {
            0 => self.bc(),
            1 => self.de(),
            2 => self.hl(),
            3 => self.sp,
            _ => panic!("Invalid long register"),
        }
    }

    pub fn get_flag(&self, op: u8) -> bool {
        if op & 1 == 1 {
            return true;
        }
        let neg = op & (1 << 3) == 0;
        let res = match (op >> 4) & 0b11 {
            0 => self.fl.z,
            1 => self.fl.cy,
            2 => self.fl.p,
            3 => self.fl.s,
            _ => unreachable!(),
        };
        if neg {
            !res
        } else {
            res
        }
    }

    pub fn set_long(&mut self, op: u8, (low, high): (u8, u8)) {
        match (op >> 4) & 3 {
            0 => {
                self.b = high;
                self.c = low
            }
            1 => {
                self.d = high;
                self.e = low
            }
            2 => {
                self.h = high;
                self.l = low
            }
            3 => self.sp = usize::from(high) << 8 | usize::from(low),
            _ => unreachable!(),
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
        if self.int_enable {
            // println!("* Generating interrupt {}", interrupt_num);
            self.int_enable = false;
            self.push(self.pc as u16);
            self.pc = usize::from(interrupt_num << 3);
        }
    }

    pub fn bc(&self) -> usize {
        (usize::from(self.b) << 8) | usize::from(self.c)
    }

    pub fn de(&self) -> usize {
        (usize::from(self.d) << 8) | usize::from(self.e)
    }

    pub fn hl(&self) -> usize {
        (usize::from(self.h) << 8) | usize::from(self.l)
    }

    pub fn at_bc(&self) -> u8 {
        let addr = self.bc();
        self.memory[addr]
    }

    pub fn at_de(&self) -> u8 {
        let addr = self.de();
        self.memory[addr]
    }

    pub fn at_hl(&self) -> u8 {
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

    pub fn ret(&mut self) {
        self.pc = usize::from(self.pop());
    }

    pub fn call(&mut self, addr: u16) {
        let pc = self.pc as u16;
        self.push(pc + 2);
        self.pc = usize::from(addr);
    }

    fn call_instr(&mut self, op: u8) -> usize {
        if self.get_flag(op) {
            self.call(self.word());
            0
        } else {
            2
        }
    }

    fn ret_instr(&mut self, op: u8) -> usize {
        if self.get_flag(op) {
            self.ret();
        }
        0
    }

    fn jmp_instr(&mut self, op: u8) -> usize {
        if self.get_flag(op) {
            self.pc = self.word() as usize;
            0
        } else {
            2
        }
    }

    fn lxi(&mut self, op: u8) -> usize {
        self.set_long(op, (self.byte1(), self.byte2()));
        2
    }

    fn ldax(&mut self, op: u8) -> usize {
        if op == 0x2A {
            let addr = self.word() as usize;
            self.l = self.memory[addr];
            self.h = self.memory[addr + 1];
            2
        } else if op == 0x3A {
            let addr = self.word() as usize;
            self.a = self.memory[addr];
            2
        } else {
            let addr = self.get_long(op);
            self.a = self.memory[addr];
            0
        }
    }

    fn stax(&mut self, op: u8) -> usize {
        if op == 0x22 { // SHLD
            let addr = self.word();
            self.memory[usize::from(addr)] = self.l;
            self.memory[usize::from(addr + 1)] = self.h;
            2
        } else if op == 0x32 {
            let addr = self.word() as usize;
            self.memory[addr] = self.a;
            2
        } else {
            let addr = self.get_long(op);
            self.memory[addr] = self.a;
            0
        }
    }

    fn inx(&mut self, op: u8) -> usize {
        let val = self.get_long(op) + 1;
        self.set_long(op, (val as u8, (val >> 8) as u8));
        0
    }

    fn inr(&mut self, op: u8) -> usize {
        let reg = (op >> 3) & 7;
        let Wrapping(val) = Wrapping(self.get_register(reg)) + Wrapping(1);
        self.set_register(reg, val);
        0
    }

    fn dcr(&mut self, op: u8) -> usize {
        let reg = (op >> 3) & 7;
        let Wrapping(val) = Wrapping(self.get_register(reg)) - Wrapping(1);
        self.set_register(reg, val);
        0
    }

    fn mvi(&mut self, op: u8) -> usize {
        let reg = (op >> 3) & 7;
        self.set_register(reg, self.byte1());
        1
    }

    fn dad(&mut self, op: u8) -> usize {
        let Wrapping(val) = Wrapping(self.hl()) + Wrapping(self.get_long(op));
        self.set_long(0x20, (val as u8, (val >> 8) as u8));
        self.fl.cy = val > 0xFFFF;
        0
    }

    fn dcx(&mut self, op: u8) -> usize {
        let Wrapping(val) = Wrapping(self.get_long(op)) - Wrapping(1);
        self.set_long(op, (val as u8, (val >> 8) as u8));
        0
    }

    fn mov(&mut self, op: u8) -> usize {
        let src = op & 0b111;
        let dst = (op >> 3) & 0b111;
        let val = self.get_register(src);
        self.set_register(dst, val);
        0
    }

    fn add_instr(&mut self, op: u8) -> usize {
        let src = op & 0b111;
        let val = u16::from(self.get_register(src)) + u16::from(self.a);
        self.set_flags(val);
        self.a = val as u8;
        0
    }

    fn adc_instr(&mut self, op: u8) -> usize {
        let val = self.get_register(op & 0b111);
        let mut cy = if self.fl.cy { 1u8 } else { 0u8 };
        self.add(val);
        if self.fl.cy {
            cy += 1
        };
        self.add(cy);
        0
    }

    fn sub_instr(&mut self, op: u8) -> usize {
        let val = self.get_register(op & 0b111);
        self.sub(val);
        0
    }

    fn sbb_instr(&mut self, op: u8) -> usize {
        let val = self.get_register(op & 0b111);
        let carry = Wrapping(if self.fl.cy { 1u8 } else { 0u8 });
        let Wrapping(rhs) = Wrapping(val) + carry;
        self.sub(rhs);
        0
    }

    fn and_instr(&mut self, op: u8) -> usize {
        let val = self.get_register(op & 0b111);
        self.and(val);
        0
    }

    fn xor_instr(&mut self, op: u8) -> usize {
        let val = self.get_register(op & 0b111);
        self.xor(val);
        0
    }

    fn or_instr(&mut self, op: u8) -> usize {
        let val = self.get_register(op & 0b111);
        self.or(val);
        0
    }

    fn cmp_instr(&mut self, op: u8) -> usize {
        let val = self.get_register(op & 0b111);
        self.cmp(val);
        0
    }

    fn pop_instr(&mut self, op: u8) -> usize {
        let val = self.pop();
        if op ==  0xf1 { // POP PSW
            self.a = (val >> 8) as u8;
            self.fl.s = val & 0x80 > 0;
            self.fl.z = val & 0x40 > 0;
            self.fl.ac = val & 0x10 > 0;
            self.fl.p = val & 0x04 > 0;
            self.fl.cy = val & 0x01 > 0;
        } else {
            self.set_long(op, (val as u8, (val >> 8) as u8));
        }
        0
    }

    fn push_instr(&mut self, op: u8) -> usize {
        let mut val = 0;
        if op == 0xf5 { // PUSH PSW
            val = u16::from(self.a) << 8;
            if self.fl.s {val |= 0x80};
            if self.fl.z {val |= 0x40};
            if self.fl.ac {val |= 0x10};
            if self.fl.p {val |= 0x04};
            if self.fl.cy {val |= 0x01};
        } else {
            val = self.get_long(op) as u16;
        }
        self.push(val);
        0
    }

    fn rst(&mut self, op: u8) -> usize {
        let num = (op >> 3) & 0b111;
        self.generate_interrupt(num);
        0
    }

    fn assignment(&mut self, op: u8) -> usize {
        let func = self.assignments[(op & 0xf) as usize];
        func(self, op)
    }

    fn assignment_extra(&mut self, op: u8) -> usize {
        if op & (1 << 5) != 0 {
            self.flagop(op)
        } else {
            self.rot(op)
        }
    }

    fn branch(&mut self, op: u8) -> usize {
        let func = self.branching[(op & 0xf) as usize];
        func(self, op)
    }

    fn rot(&mut self, op: u8) -> usize {
        match op >> 3 {
            0 => { // RLC
                let bit = self.a >> 7;
                self.a = (self.a << 1) | bit;
                self.fl.cy = bit == 1;
            }
            1 => { // RRC
                let bit = self.a << 7;
                self.a = (self.a >> 1) | bit;
                self.fl.cy = bit > 0;
            }
            2 => { // RAL
                let prev_carry = if self.fl.cy { 1 } else { 0 };
                self.fl.cy = (self.a >> 7) == 1;
                self.a = (self.a << 1) | prev_carry;
            }
            3 => { // RAR
                let prev_carry = if self.fl.cy { 1 } else { 0 };
                self.fl.cy = (self.a & 1) == 1;
                self.a = (self.a >> 1) | prev_carry;
            }
            _ => panic!("Invalid rotation operation"),
        };
        0
    }

    fn flagop(&mut self, op: u8) -> usize {
        match (op >> 3) & 3 {
            0 => {}, // DAA
            1 => self.a = !self.a, // CMA
            2 => self.fl.cy = true, // STC
            3 => self.fl.cy = !self.fl.cy, // CMC
            _ => unreachable!(),
        };
        0
    }

    fn immediate(&mut self, op: u8) -> usize {
        let val = self.byte1();
        match (op >> 3) & 7 {
            0 => self.add(val),
            1 => self.adc(val),
            2 => self.sub(val),
            3 => self.sbb(val),
            4 => self.and(val),
            5 => self.xor(val),
            6 => self.or(val),
            7 => self.cmp(val),
            _ => unreachable!(),
        };
        1
    }
}

fn disassemble8080_op(codebuffer: &[u8], pc: usize) -> usize {
    let code = &codebuffer[pc..];
    let mut opbytes = 1;
    print!("{:04X} {:02X} ", pc, code[0]);
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
            print!("STA   ${:02X}{:02X}", code[2], code[1]);
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
            opbytes = 2;
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
    if dis {
        disassemble8080_op(&state.memory, state.pc);
    }

    state.pc += 1;
    // let func = state.instructions[opcode as usize];
    let func = state.instr_compact[(opcode >> 3) as usize];
    state.pc += func(state, opcode);

    if dis {
        println!(
            "Registers: A: {:02X} BC: {:02X}{:02X} DE: {:02X}{:02X} HL: {:02X}{:02X}, SP: {:02X}",
            state.a, state.b, state.c, state.d, state.e, state.h, state.l, state.sp,
        );
        println!(
            "Flags: s: {} z: {} p: {} cy: {}",
            state.fl.s, state.fl.z, state.fl.p, state.fl.cy
        );
    }

    0
}
