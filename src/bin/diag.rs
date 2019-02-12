use emulator::{DefaultHandler, State8080, emulate8080};

pub static DIAG_BYTES: &'static [u8] = include_bytes!("cpudiag.bin");

pub fn run_diag() {
    let mut state = State8080::new(DefaultHandler);
    state.memory[0x100..(0x100 + DIAG_BYTES.len())].copy_from_slice(DIAG_BYTES);
    state.memory[368] = 0x7;
    //Skip DAA test
    state.memory[0x59c] = 0xc3; //JMP
    state.memory[0x59d] = 0xc2;
    state.memory[0x59e] = 0x05;

    state.pc = 0x100;
    while emulate8080(&mut state, false) == 0 {
        if state.pc == 0x0689 { // Error procedure
            eprintln!("\x1b[1;31mDiagnostic failed\x1b[0m");
            return;
        } else if state.pc == 0 { // End of program
            return;
        }
    }
}

fn main() {
    run_diag();
}
