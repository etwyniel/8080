use emulator::{DefaultHandler, Emu8080};

pub static DIAG_BYTES: &'static [u8] = include_bytes!("cpudiag.bin");

#[test]
pub fn run_diag() {
    let mut emu = Emu8080::new(DefaultHandler);
    emu.memory[0x100..(0x100 + DIAG_BYTES.len())].copy_from_slice(DIAG_BYTES);
    emu.memory[368] = 0x7;
    //Skip DAA test
    emu.memory[0x59c] = 0xc3; //JMP
    emu.memory[0x59d] = 0xc2;
    emu.memory[0x59e] = 0x05;

    emu.pc = 0x100;
    while emu.step_dis() == 0 {
        if emu.pc == 0x0689 {
            // Error procedure
            eprintln!("\x1b[1;31mDiagnostic failed\x1b[0m");
            return;
        } else if emu.pc == 0x069B {
            // Success procedure
            eprintln!("\x1b[1;32mDiagnostic successful\x1b[0m");
            return;
        } else if emu.pc == 5 {
            // Print routine
            if emu.c == 9 {
                let mut s = emu.de();
                while emu.memory[s] != b'$' {
                    print!("{}", emu.memory[s] as char);
                    s += 1;
                }
                println!();
            }
            emu.ret();
        } else if emu.pc == 0 {
            // End of program
            return;
        }
    }
}
