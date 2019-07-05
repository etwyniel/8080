use emulator::*;
use sdl2::{
    event::Event, keyboard::Keycode, pixels::Color, rect::Point, render::Canvas, video::Window,
};
use std::{env::args, time};

#[derive(Default)]
struct SpaceInvadersInOut {
    offset: u8,
    xy: u16,
}

impl InOutHandler for SpaceInvadersInOut {
    fn read(&mut self, port: u8) -> u8 {
        match port {
            0 => 14,
            1 => 9,
            3 => ((self.xy >> (8 - self.offset)) & 0xff) as u8,
            _ => 0,
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
    let mut emu = Emu8080::<SpaceInvadersInOut>::default();
    if let Some(filename) = args().nth(1) {
        emu.read_file_in_memory_at(&filename, 0).unwrap();
    } else {
        eprintln!("Usage: {} rom", args().next().unwrap());
        std::process::exit(1);
    }

    let mut done = 0;
    let mut n: u64 = 0;
    let mut last_interrupt = time::SystemTime::now();
    let interrupt_delay = time::Duration::new(1, 0).checked_div(60).unwrap();
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Space Invaders", WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_draw_color(Color::RGB(255, 255, 255));
    canvas.clear();
    canvas.present();
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
        if emu.pc > 0x1FFF {
            panic!("Program counter out of game rom: {:04X}", emu.pc);
        }
        if last_interrupt
            .elapsed()
            .unwrap_or_else(|_| time::Duration::new(0, 0))
            > interrupt_delay
        {
            let display_buffer = &emu.memory[0x2400..(0x2400 + (WINDOW_WIDTH * WINDOW_HEIGHT) / 8)];
            display_window(display_buffer, &mut canvas);
            canvas.present();
            emu.generate_interrupt(2);
            last_interrupt = time::SystemTime::now();
        }
        print!("#{} ", n);
        n += 1;
        done = emu.step_dis();
    }
}
