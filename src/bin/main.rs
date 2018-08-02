extern crate emulator;

use emulator::*;
use std::{thread, time};

fn main() {
    let mut state = State8080::new();
    state.read_file_in_memory_at("invaders.rom", 0).unwrap();
    //state.read_file_in_memory_at("invaders.h", 0).unwrap();
    //state.read_file_in_memory_at("invaders.g", 0x800).unwrap();
    //state.read_file_in_memory_at("invaders.f", 0x1000).unwrap();
    //state.read_file_in_memory_at("invaders.e", 0x1800).unwrap();

    let mut done = 0;
    let mut n: u64 = 0;
    while done == 0 {
        print!("#{} ", n);
        n += 1;
        done = emulate8080(&mut state);
        //thread::sleep(time::Duration::from_millis(20));
    }
}
