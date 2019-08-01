use emulator::*;
use sdl2::{
    event::Event,
    keyboard::Keycode,
    pixels::{Color, Palette, PixelFormatEnum},
    surface::Surface,
};
use std::env::args;

const COLORS: [Color; 4] = [
    Color {
        r: 0x00,
        g: 0x00,
        b: 0x00,
        a: 0xff,
    },
    Color {
        r: 0xff,
        g: 0xff,
        b: 0xff,
        a: 0xff,
    },
    Color {
        r: 0x00,
        g: 0xff,
        b: 0x00,
        a: 0xff,
    },
    Color {
        r: 0xff,
        g: 0x00,
        b: 0x00,
        a: 0xff,
    },
];

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

fn display_window(display_buffer: &[u8], surface: &mut Surface) {
    let pitch = surface.pitch() as usize;
    let dest_pixels = surface
        .without_lock_mut()
        .expect("Could not retrieve surface pixels");
    for x in 0..WINDOW_WIDTH {
        for y in 0..(WINDOW_HEIGHT / 8) {
            let pixels = display_buffer[x * WINDOW_HEIGHT / 8 + y];
            for i in 0..8 {
                let line = WINDOW_HEIGHT - y * 8 - i - 1;
                let color = if line < 50 {
                    3
                } else if line > 180 && line < 230 {
                    2
                } else {
                    1
                };
                if pixels & (1 << i) > 0 {
                    dest_pixels[line * pitch + x] = color;
                } else {
                    dest_pixels[line * pitch + x] = 0;
                }
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

    let mut temp_surface = Surface::new(
        WINDOW_WIDTH as u32,
        WINDOW_HEIGHT as u32,
        PixelFormatEnum::Index8,
    ).expect("Could not create display surface");
    temp_surface
        .set_palette(&Palette::with_colors(&COLORS).unwrap())
        .expect("Could not set color palette");
    let mut cycles = 0;
    let mut n: u64 = 0;
    // let mut last_interrupt = time::SystemTime::now();
    // let interrupt_delay = time::Duration::new(1, 0).checked_div(60).unwrap();
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Space Invaders", WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)
        .position_centered()
        .build()
        .unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut next_interrupt = 1;
    loop {
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
        // if last_interrupt
        //     .elapsed()
        //     .unwrap_or_else(|_| time::Duration::new(0, 0))
        //     > interrupt_delay
        if cycles > 1666 {
            cycles %= 1666;
            if emu.int_enable {
                let mut window_surface = window.surface(&event_pump).unwrap();
                let display_buffer = &emu.memory[0x2400..][..((WINDOW_WIDTH * WINDOW_HEIGHT) / 8)];
                display_window(display_buffer, &mut temp_surface);
                temp_surface
                    .blit_scaled(None, &mut window_surface, None)
                    .unwrap();
                window_surface.finish().unwrap();
                emu.generate_interrupt(next_interrupt);
                next_interrupt = if next_interrupt == 1 { 2 } else { 1 };
                // last_interrupt = time::SystemTime::now();
            }
        }
        print!("#{} ", n);
        n += 1;
        cycles += emu.step_dis();
    }
}
