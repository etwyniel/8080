use emulator::*;
use sdl2::{
    event::Event, keyboard::Keycode, pixels::Color, rect::Point, render::Canvas, video::Window,
};
use std::{env::args, time};

struct SpaceInvadersInOut {
    offset: u8,
    xy: u16,
}

impl InOutHandler for SpaceInvadersInOut {
    fn read(&mut self, port: u8) -> u8 {
        match port {
            0 => 14,
            1 => 9,
            3 => {
                ((self.xy >> (8 - self.offset)) & 0xff) as u8
            }
            _ => { 0 }
        }
    }

    fn write(&mut self, port: u8, val: u8) {
        match port {
            4 => {
                self.xy = (self.xy >> 8) | (u16::from(val) << 8);
            }
            2 => {
                self.offset = val & 0x7;
            }
            _ => {}
        }
    }
}

const WINDOW_WIDTH: usize = 224;
const WINDOW_HEIGHT: usize = 256;

fn display_window(display_buffer: &[u8], canvas: &mut Canvas<Window>) {
    for x in 0..WINDOW_WIDTH {
        for y in 0..(WINDOW_HEIGHT / 8) {
            let pixels = display_buffer[x * WINDOW_HEIGHT / 8 + y];
            for i in 0..8 {
                if pixels & (1 << i) > 0 {
                    canvas.set_draw_color(Color::RGB(255, 255, 255));
                } else {
                    canvas.set_draw_color(Color::RGB(0, 0, 0));
                }
                canvas
                    .draw_point(Point::new(x as i32, (WINDOW_HEIGHT - y * 8 - i - 1) as i32))
                    .unwrap();
            }
        }
    }
}

fn main() {
    let mut state = State8080::new(SpaceInvadersInOut {
        offset: 0,
        xy: 0x0000,
    });
    if let Some(filename) = args().nth(1) {
        state.read_file_in_memory_at(&filename, 0).unwrap();
    } else {
        eprintln!("Usage: {} rom", args().next().unwrap());
        std::process::exit(1);
    }
    //state.read_file_in_memory_at("invaders.h", 0).unwrap();
    //state.read_file_in_memory_at("invaders.g", 0x800).unwrap();
    //state.read_file_in_memory_at("invaders.f", 0x1000).unwrap();
    //state.read_file_in_memory_at("invaders.e", 0x1800).unwrap();

    let mut done = 0;
    // let mut n: u64 = 0;
    let mut last_interrupt = time::SystemTime::now();
    // let mut framecount: usize = 0;
    let interrupt_delay = time::Duration::new(1, 0).checked_div(60).unwrap();
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Space Invaders", WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    let mut event_pump = sdl_context.event_pump().unwrap();
    while done == 0 {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    return;
                }
                _ => {}
            }
        }
        if state.pc > 0x1FFF {
            panic!("Program counter out of game rom: {:04X}", state.pc);
        }
        if last_interrupt
            .elapsed()
            .unwrap_or_else(|_| time::Duration::new(0, 0))
            > interrupt_delay
        {
            let display_buffer =
                &state.memory[0x2400..(0x2400 + (WINDOW_WIDTH * WINDOW_HEIGHT) / 8)];
            display_window(display_buffer, &mut canvas);
            canvas.present();
            state.generate_interrupt(2);
            last_interrupt = time::SystemTime::now();
            // framecount += 1;
        }
        // print!("#{} ", n);
        // n += 1;
        done = emulate8080(&mut state, false);
        // thread::sleep(time::Duration::from_millis(1000));
    }
}
