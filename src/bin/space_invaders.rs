extern crate emulator;
//extern crate space_invaders;

use emulator::*;
use std::{thread, time};

struct SpaceInvadersInOut {
    offset: u8,
    xy: u16,
}

impl InOutHandler for SpaceInvadersInOut {
    fn read<T: InOutHandler>(&mut self, port: u8, state: &mut State8080<T>) {
        match port {
            3 => {
                state.a = ((self.xy >> (8 - self.offset)) & 0xff) as u8;
            }
            _ => {}
        }
    }
    fn write<T: InOutHandler>(&mut self, port: u8, state: &mut State8080<T>) {
        match port {
            4 => {
                self.xy = (self.xy >> 8) | (u16::from(state.a) << 8);
            }
            2 => {
                self.offset = state.a & 0x7;
            }
            _ => {}
        }
    }
}

fn main() {
    let mut state = State8080::new(SpaceInvadersInOut {
        offset: 0,
        xy: 0x0000,
    });
    state.read_file_in_memory_at("invaders.rom", 0).unwrap();
    //state.read_file_in_memory_at("invaders.h", 0).unwrap();
    //state.read_file_in_memory_at("invaders.g", 0x800).unwrap();
    //state.read_file_in_memory_at("invaders.f", 0x1000).unwrap();
    //state.read_file_in_memory_at("invaders.e", 0x1800).unwrap();

    let mut done = 0;
    let mut n: u64 = 0;
    let mut last_interrupt = time::SystemTime::now();
    let interrupt_delay = time::Duration::new(1, 0).checked_div(60).unwrap();
    while done == 0 {
        if state.int_enable
            && last_interrupt
                .elapsed()
                .unwrap_or_else(|_| time::Duration::new(0, 0)) > interrupt_delay
        {
            state.generate_interrupt(2);
            last_interrupt = time::SystemTime::now();
        }
        print!("#{} ", n);
        n += 1;
        done = emulate8080(&mut state);
        thread::sleep(time::Duration::from_millis(0));
    }
}
