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
        0x23 => print!("INX   B"),
        0x24 => print!("INR   H"),
        0x25 => print!("DCR   H"),
        0x26 => {print!("MVI    H,#${:02x}", code[1]); opbytes = 2;},
        0x27 => print!("DAA"),

        0x28 => print!("NOP"),
        0x29 => print!("DAD   D"),
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
        0xcb => {print!("JMP    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xcc => {print!("CZ     ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xcd => {print!("CALL   ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xce => {print!("ACI    #${:02x}", code[1]); opbytes = 2;},
        0xcf => print!("RST   1"),

        0xd0 => print!("RNC"),
        0xd1 => print!("POP   D"),
        0xd2 => {print!("JNC    ${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
        0xd3 => {print!("OUT    #${:02x}{:02x}", code[2], code[1]); opbytes = 3;},
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
    match opcode {
        0x00 => {},
        0x01 => {
            state.c = state.memory[state.pc + 1];
            state.b = state.memory[state.pc + 2];
            state.pc += 2;
        },
        0x02 => {
            let addr = state.bc();
            state.memory[addr] = state.a;
        },
        0x03 => {
            ans = (state.bc() + 1) as u16;
            state.b = (ans >> 8) as u8;
            state.c = (ans & 0xff) as u8;
        },
        0x04 => {
            state.b += 1;
            state.fl.z = state.b == 0;
            state.fl.s = (state.b & 0x7f) != 0;
            state.fl.p = (state.b & 1) == 0;
        },
        0x05 => {
            state.b -= 1;
            state.fl.z = state.b == 0;
            state.fl.s = (state.b & 0x7f) != 0;
            state.fl.p = (state.b & 1) == 0;
        },
        0x06 => {
            state.b = state.memory[state.pc + 1];
            state.pc += 1;
        },
        0x07 => {
            let ans8: u8 = state.a >> 7;
            state.a = (state.a << 1) | ans8;
            state.fl.cy = ans8 == 1;
        },

        0x08 => {},
        0x09 => {
            let ans32: u32 = (state.hl() as u32) + (state.bc() as u32);
            state.h = ((ans32 >> 8) & 0xff) as u8;
            state.l = (ans32 & 0xff) as u8;
            state.fl.cy = ans32 > 0xffff
        },
        0x0a => {state.a = state.at_bc();},
        0x0b => {
            ans = (state.bc() - 1) as u16;
            state.b = (ans >> 8) as u8;
            state.c = (ans & 0xff) as u8;
        },
        0x0c => {
            state.c += 1;
            state.set_r(state.c);
        },
        0x0d => {
            state.c -= 1;
            state.set_r(state.c);
        },
        0x0e => {
            state.c = state.memory[state.pc + 1];
            state.pc += 1;
        },
        0x0f => {
            let ans8: u8 = state.a << 7;
            state.a = (state.a >> 1) | ans8;
            state.fl.cy = ans8 > 0;
        },

        0x10 => {},
        0x11 => {
            state.e = state.byte2();
            state.d = state.byte1();
            state.pc += 2;
        },
        0x12 => {state.a = state.at_bc()},
        0x13 => {
            ans = (state.de() + 1) as u16;
            state.d = (ans >> 8) as u8;
            state.e = (ans & 0xff) as u8;
        },
        0x14 => {
            state.d += 1;
            state.set_r(state.d);
        },
        0x15 => print!("DCR   B"),
        0x16 => {print!("MVI    B,#${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0x17 => print!("RLC"),

        0x18 => print!("NOP"),
        0x19 => print!("DAD   D"),
        0x1a => print!("LDAX   D"),
        0x1b => print!("DCX   D"),
        0x1c => print!("INR   E"),
        0x1d => print!("DCR   E"),
        0x1e => {print!("MVI    E,#${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0x1f => print!("RAR"),

        0x20 => print!("NOP"),
        0x21 => {print!("LXI    H,#${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0x22 => {print!("SHLD   ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0x23 => print!("INX   B"),
        0x24 => print!("INR   H"),
        0x25 => print!("DCR   H"),
        0x26 => {print!("MVI    H,#${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0x27 => print!("DAA"),

        0x28 => print!("NOP"),
        0x29 => print!("DAD   D"),
        0x2a => {print!("LHLD   ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0x2b => print!("DCX    H"),
        0x2c => print!("INR   L"),
        0x2d => print!("DCR   L"),
        0x2e => {print!("MVI    L,#${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0x2f => print!("CMA"),

        0x30 => print!("NOP"),
        0x31 => {print!("LXI   SP,#${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0x32 => {print!("SHL    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0x33 => print!("INX  SP"),
        0x34 => print!("INR   M"),
        0x35 => print!("DCR   M"),
        0x36 => {print!("MVI    M,#${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0x37 => print!("STC"),

        0x38 => print!("NOP"),
        0x39 => print!("DAD  SP"),
        0x3a => {print!("LDA    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0x3b => print!("DCX   SP"),
        0x3c => print!("INR   A"),
        0x3d => print!("DCR   A"),
        0x3e => {print!("MVI    A,#${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
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
        0xc2 => {print!("JNZ    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xc3 => {print!("JMP    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xc4 => {print!("CNZ    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xc5 => print!("PUSH  B"),
        0xc6 => {print!("ADI    #${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0xc7 => print!("RST   0"),
        0xc8 => print!("RZ"),
        0xc9 => print!("RET"),
        0xca => {print!("JZ     ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xcb => {print!("JMP    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xcc => {print!("CZ     ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xcd => {print!("CALL   ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xce => {print!("ACI    #${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0xcf => print!("RST   1"),

        0xd0 => print!("RNC"),
        0xd1 => print!("POP   D"),
        0xd2 => {print!("JNC    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xd3 => {print!("OUT    #${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xd4 => {print!("CNC    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xd5 => print!("PUSH  D"),
        0xd6 => {print!("SUI    #${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0xd7 => print!("RST   2"),
        0xd8 => print!("RC"),
        0xd9 => print!("RET"),
        0xda => {print!("JC     ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xdb => {print!("IN     #${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xdc => {print!("CC     ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xdd => {print!("CALL   ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xde => {print!("SBI    #${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0xdf => print!("RST   3"),

        0xe0 => print!("RPO"),
        0xe1 => print!("POP   H"),
        0xe2 => {print!("JPO    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xe3 => print!("XTHL"),
        0xe4 => {print!("CPO    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xe5 => print!("PUSH  H"),
        0xe6 => {print!("ANI    #${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0xe7 => print!("RST   4"),
        0xe8 => print!("RPE"),
        0xe9 => print!("PCHL"),
        0xea => {print!("JPE    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xeb => print!("XCHG"),
        0xec => {print!("CPE    ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xed => {print!("CALL   ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xee => {print!("XRI    #${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0xef => print!("RST   5"),

        0xf0 => print!("RP"),
        0xf1 => print!("POP   PSW"),
        0xf2 => {print!("JP     ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xf3 => print!("DI"),
        0xf4 => {print!("CP     ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xf5 => print!("PUSH  PSW"),
        0xf6 => {print!("ORI    #${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0xf7 => print!("RST   6"),
        0xf8 => print!("RM"),
        0xf9 => print!("SPHL"),
        0xfa => {print!("JM     ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xfb => print!("EI"),
        0xfc => {print!("CM     ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xfd => {print!("CALL   ${:02x}{:02x}", state.memory[state.pc + 2], state.memory[state.pc + 1]); opbytes = 3;},
        0xfe => {print!("CPI    #${:02x}", state.memory[state.pc + 1]); opbytes = 2;},
        0xff => print!("RST   7"),

        _ => {}
    }
}

