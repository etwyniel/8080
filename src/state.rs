use std::default::Default;

#[derive(Default)]
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

pub struct State8080 {
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
}

impl Default for State8080 {
    fn default() -> Self {
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
            fl: Default::default(),
            int_enable: false,
        }
    }
}

impl State8080 {
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
        let neg = op & 0b1000 == 0;
        let res = match (op >> 4) & 0b11 {
            0 => self.fl.z,
            1 => self.fl.cy,
            2 => self.fl.p,
            3 => self.fl.s,
            _ => unreachable!(),
        };
        neg ^ res
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

    pub fn byte1(&self) -> u8 {
        self.memory[self.pc]
    }

    pub fn byte2(&self) -> u8 {
        self.memory[self.pc + 1]
    }

    pub fn word(&self) -> u16 {
        self.word_at(self.pc)
    }

    pub fn word_at(&self, addr: usize) -> u16 {
        (u16::from(self.memory[addr + 1]) << 8) | u16::from(self.memory[addr])
    }

    pub fn set_r(&mut self, res: u8) {
        self.fl.z = res == 0;
        self.fl.s = (res & 0x80) != 0;
        self.fl.p = parity(res, self.fl.cy);
    }

    pub fn set_flags(&mut self, res: u16) {
        self.set_r(res as u8);
        self.fl.cy = res > 0xff;
    }
}
