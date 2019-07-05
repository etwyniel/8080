use std::fs::File;
use std::io::prelude::*;
use std::num::Wrapping;
use std::ops::{Deref, DerefMut};

pub mod dis;
pub mod state;

use dis::disassemble8080_op;
use state::*;

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

#[derive(Default)]
pub struct Emu8080<T: InOutHandler = DefaultHandler> {
    pub state: State8080,
    io: T,
}

impl<T: InOutHandler> Deref for Emu8080<T> {
    type Target = State8080;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T: InOutHandler> DerefMut for Emu8080<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

type Instruction<T> = fn(&mut Emu8080<T>, u8) -> usize;

impl<T: InOutHandler> Emu8080<T> {
    pub fn new(io_handler: T) -> Self {
        Emu8080 {
            state: State8080 {
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
            },
            io: io_handler,
        }
    }

    pub fn set_register(&mut self, reg: u8, val: u8) {
        self.state.set_register(reg, val)
    }

    pub fn set_long(&mut self, op: u8, val: (u8, u8)) {
        self.state.set_long(op, val)
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
        let sp = self.sp;
        self.memory[sp] = (val & 0xff) as u8;
        self.memory[sp + 1] = (val >> 8) as u8;
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
        if op == 0x22 {
            // SHLD
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
        if op == 0xf1 {
            // POP PSW
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
        let mut val;
        if op == 0xf5 {
            // PUSH PSW
            val = u16::from(self.a) << 8;
            if self.fl.s {
                val |= 0x80
            };
            if self.fl.z {
                val |= 0x40
            };
            if self.fl.ac {
                val |= 0x10
            };
            if self.fl.p {
                val |= 0x04
            };
            if self.fl.cy {
                val |= 0x01
            };
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
        Self::ASSIGNMENTS[(op & 0xf) as usize](self, op)
    }

    fn assignment_extra(&mut self, op: u8) -> usize {
        if op & (1 << 5) != 0 {
            self.flagop(op)
        } else {
            self.rot(op)
        }
    }

    fn branch(&mut self, op: u8) -> usize {
        Self::BRANCHING[(op & 0xf) as usize](self, op)
    }

    fn rot(&mut self, op: u8) -> usize {
        match op >> 3 {
            0 => {
                // RLC
                let bit = self.a >> 7;
                self.a = (self.a << 1) | bit;
                self.fl.cy = bit == 1;
            }
            1 => {
                // RRC
                let bit = self.a << 7;
                self.a = (self.a >> 1) | bit;
                self.fl.cy = bit > 0;
            }
            2 => {
                // RAL
                let prev_carry = if self.fl.cy { 1 } else { 0 };
                self.fl.cy = (self.a >> 7) == 1;
                self.a = (self.a << 1) | prev_carry;
            }
            3 => {
                // RAR
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
            0 => {}                        // DAA
            1 => self.a = !self.a,         // CMA
            2 => self.fl.cy = true,        // STC
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
    const ASSIGNMENTS: [Instruction<T>; 16] = [
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
    ];
    const BRANCHING: [Instruction<T>; 16] = [
        Self::ret_instr, // 0x0
        Self::pop_instr, // 0x1
        Self::jmp_instr, // 0x2
        |emu, op| match (op >> 4) & 3 {
            0 => emu.jmp_instr(op),
            1 => {
                emu.io.write(emu.byte1(), emu.a);
                1
            }
            2 => {
                let val = emu.pop();
                emu.push(emu.hl() as u16);
                emu.h = (val >> 8) as u8;
                emu.l = val as u8;
                0
            }
            3 => {
                emu.int_enable = false;
                0
            }
            _ => unreachable!(),
        }, // 0x3
        Self::call_instr, // 0x4
        Self::push_instr, // 0x5
        Self::immediate, // 0x6
        Self::rst,       // 0x7
        Self::ret_instr, // 0x8
        |emu, op| match (op >> 4) & 3 {
            0 => emu.ret_instr(op),
            1 => 0,
            2 => {
                emu.pc = (usize::from(emu.h) << 8) | usize::from(emu.l);
                0
            }
            3 => {
                emu.sp = emu.hl();
                0
            }
            _ => unreachable!(),
        }, // 0x9
        Self::jmp_instr, // 0xa
        |emu, op| match (op >> 4) & 3 {
            0 => 0,
            1 => {
                emu.a = emu.io.read(emu.byte1());
                1
            }
            2 => {
                std::mem::swap(&mut emu.state.d, &mut emu.state.h);
                std::mem::swap(&mut emu.state.e, &mut emu.state.l);
                0
            }
            3 => {
                emu.int_enable = true;
                0
            }
            _ => unreachable!(),
        }, // 0xb
        Self::call_instr, // 0xc
        Self::call_instr, // 0xd
        Self::immediate, // 0xe
        Self::rst,       // 0xf
    ];
    const INSTR_COMPACT: [Instruction<T>; 32] = [
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
    ];

    pub fn step(&mut self) -> i32 {
        assert!(self.pc < self.memory.len());
        let opcode = self.memory[self.pc];

        self.pc += 1;
        self.pc += Self::INSTR_COMPACT[(opcode >> 3) as usize](self, opcode);

        0
    }

    pub fn step_dis(&mut self) -> i32 {
        disassemble8080_op(&self.memory, self.pc);

        let res = self.step();

        println!(
            "Registers: A: {:02X} BC: {:02X}{:02X} DE: {:02X}{:02X} HL: {:02X}{:02X}, SP: {:02X}",
            self.a, self.b, self.c, self.d, self.e, self.h, self.l, self.sp,
        );
        println!(
            "Flags: s: {} z: {} p: {} cy: {}",
            self.fl.s, self.fl.z, self.fl.p, self.fl.cy
        );

        res
    }
}
