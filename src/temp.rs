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
    pub instructions: [Instruction<T>; 256],
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
            instructions: [
                // NOP
                |_state, _op| 0, // 0x00
                // LXI B,#$WORD
                Self::lxi, // 0x01
                // STAX B
                Self::stax, // 0x02
                // INX B
                Self::inx, // 0x03
                // INR B
                Self::inr, // 0x04
                // DCR B
                Self::dcr, // 0x05
                // MVI B,#$BYTE
                Self::mvi, // 0x06
                // RLC
                |_state, _op| {
                    let bit = _state.a >> 7;
                    _state.a = (_state.a << 1) | bit;
                    _state.fl.cy = bit == 1;
                    0
                }, // 0x07
                // NOP
                |_state, _op| 0, // 0x08
                // DAD B
                Self::dad, // 0x09
                // LDAX B
                Self::ldax, // 0x0a
                // DCX B
                Self::dcx, // 0x0b
                // INR C
                Self::inr, // 0x0c
                // DCR C
                Self::dcr, // 0x0d
                // MVI C,#$BYTE
                Self::mvi, // 0x0e
                // RRC
                |_state, _op| {
                    let bit = _state.a << 7;
                    _state.a = (_state.a >> 1) | bit;
                    _state.fl.cy = bit > 0;
                    0
                }, // 0x0f
                // NOP
                |_state, _op| 0, // 0x10
                // LXI D,#$WORD
                Self::lxi, // 0x11
                // STAX D
                Self::stax, // 0x12
                // INX D
                Self::inx, // 0x13
                // INR D
                Self::inr, // 0x14
                // DCR D
                Self::dcr, // 0x15
                // MVI D,#$BYTE
                Self::mvi, // 0x16
                // RAL
                |_state, _op| {
                    let prev_carry = if _state.fl.cy { 1 } else { 0 };
                    _state.fl.cy = (_state.a >> 7) == 1;
                    _state.a = (_state.a << 1) | prev_carry;
                    0
                }, // 0x17
                // NOP
                |_state, _op| 0, // 0x18
                // DAD D
                Self::dad, // 0x19
                // LDAX D
                Self::ldax, // 0x1a
                // DCX D
                Self::dcx, // 0x1b
                // INR E
                Self::inr, // 0x1c
                // DCR E
                Self::dcr, // 0x1d
                // MVI E,#$BYTE
                Self::mvi, // 0x1e
                // RAR
                |_state, _op| {
                    // RAR
                    let prev_carry = if _state.fl.cy { 1 } else { 0 };
                    _state.fl.cy = (_state.a & 1) == 1;
                    _state.a = (_state.a >> 1) | prev_carry;
                    0
                }, // 0x1f
                // RIM
                |_state, _op| 0, // 0x20
                // LXI H,#$WORD
                Self::lxi, // 0x21
                // SHLD $WORD
                |_state, _op| {
                    let val = _state.word();
                    _state.memory[usize::from(val)] = _state.l;
                    _state.memory[usize::from(val + 1)] = _state.h;
                    2
                }, // 0x22
                // INX H
                Self::inx, // 0x23
                // INR H
                Self::inr, // 0x24
                // DCR H
                Self::dcr, // 0x25
                // MVI L,#$BYTE
                Self::mvi, // 0x26
                // RAA
                |_state, _op| 0, // 0x27
                // NOP
                |_state, _op| 0, // 0x28
                // DAD H
                Self::dad, // 0x29
                // LHLD $WORD
                |_state, _op| {
                    let addr = _state.word();
                    _state.l = _state.memory[usize::from(addr)];
                    _state.h = _state.memory[usize::from(addr + 1)];
                    2
                }, // 0x2a
                // DCX H
                Self::dcx, // 0x2b
                // INR L
                Self::inr, // 0x2c
                //DCR L
                Self::dcr, // 0x2d
                // MVI L,#$BYTE
                Self::mvi, // 0x2e
                // CMA
                |_state, _op| {
                    _state.a = !_state.a;
                    0
                }, // 0x2f
                // NOP
                |_state, _op| 0, // 0x30
                // LXI SP,#$WORD
                Self::lxi, // 0x31
                // STA $WORD
                |_state, _op| {
                    let addr = _state.word();
                    _state.memory[usize::from(addr)] = _state.a;
                    2
                }, // 0x32
                // INX SP
                Self::inx, // 0x33
                // INR M
                Self::inr, // 0x34
                // DCR M
                Self::dcr, // 0x35
                // MVI M,#$BYTE
                Self::mvi, // 0x36
                // STC
                |_state, _op| {
                    _state.fl.cy = true;
                    0
                }, // 0x37
                // NOP
                |_state, _op| 0, // 0x38
                // DAD SP
                Self::dad, // 0x39
                // LDA $WORD
                |_state, _op| {
                    let addr = usize::from(_state.word());
                    _state.a = _state.memory[addr];
                    2
                }, // 0x3a
                // DCX SP
                Self::dcx, // 0x3b
                // INR A
                Self::inr, // 0x3c
                // DCR A
                Self::dcr, // 0x3d
                // MVI A,#$BYTE
                Self::mvi, // 0x3e
                // CMC
                |_state, _op| {
                    _state.fl.cy = !_state.fl.cy;
                    0
                }, // 0x3f
                // MOV DST,SRC
                Self::mov, // 0x40
                Self::mov, // 0x41
                Self::mov, // 0x42
                Self::mov, // 0x43
                Self::mov, // 0x44
                Self::mov, // 0x45
                Self::mov, // 0x46
                Self::mov, // 0x47
                Self::mov, // 0x48
                Self::mov, // 0x49
                Self::mov, // 0x4a
                Self::mov, // 0x4b
                Self::mov, // 0x4c
                Self::mov, // 0x4d
                Self::mov, // 0x4e
                Self::mov, // 0x4f
                Self::mov, // 0x50
                Self::mov, // 0x51
                Self::mov, // 0x52
                Self::mov, // 0x53
                Self::mov, // 0x54
                Self::mov, // 0x55
                Self::mov, // 0x56
                Self::mov, // 0x57
                Self::mov, // 0x58
                Self::mov, // 0x59
                Self::mov, // 0x5a
                Self::mov, // 0x5b
                Self::mov, // 0x5c
                Self::mov, // 0x5d
                Self::mov, // 0x5e
                Self::mov, // 0x5f
                Self::mov, // 0x60
                Self::mov, // 0x61
                Self::mov, // 0x62
                Self::mov, // 0x63
                Self::mov, // 0x64
                Self::mov, // 0x65
                Self::mov, // 0x66
                Self::mov, // 0x67
                Self::mov, // 0x68
                Self::mov, // 0x69
                Self::mov, // 0x6a
                Self::mov, // 0x6b
                Self::mov, // 0x6c
                Self::mov, // 0x6d
                Self::mov, // 0x6e
                Self::mov, // 0x6f
                Self::mov, // 0x70
                Self::mov, // 0x71
                Self::mov, // 0x72
                Self::mov, // 0x73
                Self::mov, // 0x74
                Self::mov, // 0x75
                Self::mov, // 0x76
                Self::mov, // 0x77
                Self::mov, // 0x78
                Self::mov, // 0x79
                Self::mov, // 0x7a
                Self::mov, // 0x7b
                Self::mov, // 0x7c
                Self::mov, // 0x7d
                Self::mov, // 0x7e
                Self::mov, // 0x7f
                // ADD REG
                Self::add_instr, // 0x80
                Self::add_instr, // 0x81
                Self::add_instr, // 0x82
                Self::add_instr, // 0x83
                Self::add_instr, // 0x84
                Self::add_instr, // 0x85
                Self::add_instr, // 0x86
                Self::add_instr, // 0x87
                // ADC REG
                Self::adc_instr, // 0x88
                Self::adc_instr, // 0x89
                Self::adc_instr, // 0x8a
                Self::adc_instr, // 0x8b
                Self::adc_instr, // 0x8c
                Self::adc_instr, // 0x8d
                Self::adc_instr, // 0x8e
                Self::adc_instr, // 0x8f
                // SUB REG
                Self::sub_instr, // 0x90
                Self::sub_instr, // 0x91
                Self::sub_instr, // 0x92
                Self::sub_instr, // 0x93
                Self::sub_instr, // 0x94
                Self::sub_instr, // 0x95
                Self::sub_instr, // 0x96
                Self::sub_instr, // 0x97
                // SBB REG
                Self::sbb_instr, // 0x98
                Self::sbb_instr, // 0x99
                Self::sbb_instr, // 0x9a
                Self::sbb_instr, // 0x9b
                Self::sbb_instr, // 0x9c
                Self::sbb_instr, // 0x9d
                Self::sbb_instr, // 0x9e
                Self::sbb_instr, // 0x9f
                // AND REG
                Self::and_instr, // 0xa0
                Self::and_instr, // 0xa1
                Self::and_instr, // 0xa2
                Self::and_instr, // 0xa3
                Self::and_instr, // 0xa4
                Self::and_instr, // 0xa5
                Self::and_instr, // 0xa6
                Self::and_instr, // 0xa7
                // XOR REG
                Self::xor_instr, // 0xa8
                Self::xor_instr, // 0xa9
                Self::xor_instr, // 0xaa
                Self::xor_instr, // 0xab
                Self::xor_instr, // 0xac
                Self::xor_instr, // 0xad
                Self::xor_instr, // 0xae
                Self::xor_instr, // 0xaf
                // OR REG
                Self::or_instr, // 0xb0
                Self::or_instr, // 0xb1
                Self::or_instr, // 0xb2
                Self::or_instr, // 0xb3
                Self::or_instr, // 0xb4
                Self::or_instr, // 0xb5
                Self::or_instr, // 0xb6
                Self::or_instr, // 0xb7
                // CMP REG
                Self::cmp_instr, // 0xb8
                Self::cmp_instr, // 0xb9
                Self::cmp_instr, // 0xba
                Self::cmp_instr, // 0xbb
                Self::cmp_instr, // 0xbc
                Self::cmp_instr, // 0xbd
                Self::cmp_instr, // 0xbe
                Self::cmp_instr, // 0xbf
                // RNZ
                Self::ret_instr, // 0xc0
                // POP B
                Self::pop_instr, // 0xc1
                // JNZ
                Self::jmp_instr, // 0xc2
                // JMP
                Self::jmp_instr, // 0xc3
                // CNZ $WORD
                Self::call_instr, // 0xc4
                // PUSH B
                Self::push_instr, // 0xc5
                // ADI #$BYTE
                |_state, _op| {
                    let val = _state.byte1();
                    _state.add(val);
                    1
                }, // 0xc6
                // RST 0
                Self::rst, // 0xc7
                // RZ
                Self::ret_instr, // 0xc8
                // RET
                Self::ret_instr, // 0xc9
                // JZ
                Self::jmp_instr, // 0xca
                // NOP
                |_state, _op| 0, // 0xcb
                // CZ
                Self::call_instr, // 0xcc
                // CALL $WORD
                Self::call_instr, // 0xcd
                // ACI #$BYTE
                |_state, _op| {
                    let val = _state.byte1();
                    _state.adc(val);
                    1
                }, // 0xce
                // RST
                Self::rst, // 0xcf
                // RNC
                Self::ret_instr, // 0xd0
                // POP D
                Self::pop_instr, // 0xd1
                // JNC
                Self::jmp_instr, // 0xd2
                // OUT
                |_state, _op| {
                    _state.io.write(_state.byte1(), _state.a);
                    1
                }, // 0xd3
                // CNC
                Self::call_instr, // 0xd4
                // PUSH D
                Self::push_instr, // 0xd5
                // SUI #$BYTE
                |_state, _op| {
                    let val = _state.byte1();
                    _state.sub(val);
                    1
                }, // 0xd6
                // RST 2
                Self::rst, // 0xd7
                // RC
                Self::ret_instr, // 0xd8
                // NOP
                |_state, _op| 0, // 0xd9
                // JC
                Self::jmp_instr, // 0xda
                // IN
                |_state, _op| {
                    _state.a = _state.io.read(_state.byte1());
                    1
                }, // 0xdb
                // CC
                Self::call_instr, // 0xdc
                // CALL
                Self::call_instr, // 0xdd
                // SBI #$BYTE
                |_state, _op| {
                    let val = _state.byte1();
                    _state.sbb(val);
                    1
                }, // 0xde
                // RST 3
                Self::rst, // 0xdf
                // RPO
                Self::ret_instr, // 0xe0
                // POP H
                Self::pop_instr, // 0xe1
                // JPO
                Self::jmp_instr, // 0xe2
                // XTHL
                |_state, _op| {
                    let val = _state.pop();
                    _state.push(_state.hl() as u16);
                    _state.h = (val >> 8) as u8;
                    _state.l = val as u8;
                    0
                }, // 0xe3
                // CPO
                Self::call_instr, // 0xe4
                // PUSH H
                Self::push_instr, // 0xe5
                // ANI #$BYTE
                |_state, _op| {
                    let val = _state.byte1();
                    _state.and(val);
                    1
                }, // 0xe6
                // RST 4
                Self::rst, // 0xe7
                // RPE
                Self::ret_instr, // 0xe8
                // PCHL
                |_state, _op| {
                    _state.pc = (usize::from(_state.h) << 8) | usize::from(_state.l);
                    0
                }, // 0xe9
                // JPE
                Self::jmp_instr, // 0xea
                // XCHG
                |_state, _op| {
                    std::mem::swap(&mut _state.d, &mut _state.h);
                    std::mem::swap(&mut _state.e, &mut _state.l);
                    0
                }, // 0xeb
                // CPE
                Self::call_instr, // 0xec
                // NOP
                |_state, _op| 0, // 0xed
                // XRI #$BYTE
                |_state, _op| {
                    let val = _state.byte1();
                    _state.xor(val);
                    1
                }, // 0xee
                // RST 5
                Self::rst, // 0xef
                // RP
                Self::ret_instr, // 0xf0
                // POP PSW
                |_state, _op| {
                    let val = _state.pop();
                    _state.a = (val >> 8) as u8;
                    _state.fl.s = val & 0x80 > 0;
                    _state.fl.z = val & 0x40 > 0;
                    _state.fl.ac = val & 0x10 > 0;
                    _state.fl.p = val & 0x04 > 0;
                    _state.fl.cy = val & 0x01 > 0;
                    0
                }, // 0xf1
                // JP $WORD
                Self::jmp_instr, // 0xf2
                // DI
                |_state, _op| {
                    _state.int_enable = false;
                    0
                }, // 0xf3
                // CP
                Self::call_instr, // 0xf4
                // PUSH PSW
                |_state, _op| {
                    let mut val = u16::from(_state.a) << 8;
                    if _state.fl.s {val |= 0x80};
                    if _state.fl.z {val |= 0x40};
                    if _state.fl.ac {val |= 0x10};
                    if _state.fl.p {val |= 0x04};
                    if _state.fl.cy {val |= 0x01};
                    _state.push(val);
                    0
                }, // 0xf5
                // ORI #$BYTE
                |_state, _op| {
                    let val = _state.byte1();
                    _state.or(val);
                    1
                }, // 0xf6
                // RST 6
                Self::rst, // 0xf7
                // RM
                Self::ret_instr, // 0xf8
                // SPHL
                |_state, _op| {
                    _state.sp = _state.hl();
                    0
                }, // 0xf9
                // JM
                Self::jmp_instr, // 0xfa
                |_state, _op| {
                    _state.int_enable = true;
                    0
                }, // 0xfb
                // CM
                Self::call_instr, // 0xfc
                // CALL
                Self::call_instr, // 0xfd
                // CPI #$BYTE
                |_state, _op| {
                    let val = _state.byte1();
                    _state.cmp(val);
                    1
                }, // 0xfe
                // RST 7
                Self::rst, // 0xff
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
        let mut neg = op & (1 << 3) == 0;
        let res = match (op >> 4) & 0b11 {
            0 => self.fl.z,
            1 => self.fl.cy,
            2 => self.fl.p,
            3 => self.fl.s,
            _ => panic!("Invalid flag number"),
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
            _ => panic!("Invalid long register"),
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

    pub fn ret(&mut self) {
        self.pc = usize::from(self.pop());
    }

    pub fn call(&mut self, addr: u16) {
        let pc = self.pc as u16;
        self.push(pc + 2);
        self.pc = usize::from(addr);
    }

    fn call_if(&mut self, cond: bool) {
        if cond {
            let addr = self.word();
            self.call(addr);
        } else {
            self.pc += 2;
        }
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
        let addr = self.get_long(op);
        self.a = self.memory[addr];
        0
    }

    fn stax(&mut self, op: u8) -> usize {
        let addr = self.get_long(op);
        self.memory[addr] = self.a;
        0
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
        self.set_long(op, (val as u8, (val >> 8) as u8));
        0
    }

    fn push_instr(&mut self, op: u8) -> usize {
        let val = self.get_long(op);
        self.push(val as u16);
        0
    }

    fn rst(&mut self, op: u8) -> usize {
        let num = (op >> 3) & 0b111;
        self.generate_interrupt(num);
        0
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
    let mut ans: u16;
    let ans8: u8;
    let ans32: u32;
    let addr: usize;
    if dis {
        disassemble8080_op(&state.memory, state.pc);
    }

    state.pc += 1;
    let func = state.instructions[opcode as usize];
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
