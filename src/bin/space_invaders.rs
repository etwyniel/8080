use emulator::*;
use sdl2::{
    event::Event,
    keyboard::Keycode,
    pixels::{Color, Palette, PixelFormatEnum},
    surface::Surface,
    video::Window,
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

#[repr(u8)]
enum Port1Buttons {
    Credit = 0b0000_0001,
    Player2Start = 0b0000_0010,
    Player1Start = 0b0000_0100,
    Player1Shoot = 0b0001_0000,
    Player1Left = 0b0010_0000,
    Player1Right = 0b0100_0000,
}

#[repr(u8)]
enum  Port2Buttons {
    Tilt = 0b0000_0100,
    Player2Shoot = 0b0001_0000,
    Player2Left = 0b0010_0000,
    Player2Right = 0b0100_0000,
}

fn handle_buttons(io: &mut SpaceInvadersInOut, key: Keycode, down: bool) {
    {
        use Port1Buttons::*;
        let port1_mask = match key {
            Keycode::C => Credit as u8,
            Keycode::P => Player2Start as u8,
            Keycode::Return => Player1Start as u8,
            Keycode::Left => Player1Left as u8,
            Keycode::Right => Player1Right as u8,
            Keycode::Up => Player1Shoot as u8,
            _ => 0,
        };
        if down {
            io.port1 |= port1_mask;
        } else {
            io.port1 &= !port1_mask;
        }
    }
    {
        use Port2Buttons::*;
        let port2_mask = match key {
            Keycode::V => Player2Left as u8,
            Keycode::B => Player2Right as u8,
            Keycode::Space => Player2Shoot as u8,
            Keycode::T => Tilt as u8,
            _ => 0,
        };
        if down {
            io.port2 |= port2_mask;
        } else {
            io.port2 &= !port2_mask;
        }
    }
}

#[derive(Default)]
struct SpaceInvadersInOut {
    offset: u8,
    xy: u16,
    port1: u8,
    port2: u8,
}

impl InOutHandler for SpaceInvadersInOut {
    fn read(&mut self, port: u8) -> u8 {
        match port {
            0 => 14,
            1 => self.port1,
            2 => self.port2,
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
            6 => {
                // Debug port
                // let c = match val {
                //     0..=25 => (val + b'a') as char,
                //     26..=35 => (val - 26 + b'0') as char,
                //     36 => '<',
                //     37 => '>',
                //     38 => ' ',
                //     39 => '=',
                //     40 => '*',
                //     41 => 'λ',
                //     63 => '-',
                //     146 => {
                //         return;
                //     }
                //     _ => '?',
                // };
                // eprintln!("Wrote to debug: 0x{:02x} ({})", val, c);
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

fn init_surfaces() -> (Surface<'static>, Surface<'static>) {
    let game_surface = Surface::new(
        WINDOW_WIDTH as u32,
        WINDOW_HEIGHT as u32,
        PixelFormatEnum::Index8,
    ).expect("Could not create display surface");
    let temp_surface = Surface::new(
        WINDOW_WIDTH as u32,
        WINDOW_HEIGHT as u32,
        PixelFormatEnum::RGB888,
    ).expect("Could not create display surface");
    (game_surface, temp_surface)
}

fn init_window(video_subsystem: &sdl2::VideoSubsystem) -> Window {
    video_subsystem
        .window(
            "Space Invaders",
            (WINDOW_WIDTH * 2) as u32,
            (WINDOW_HEIGHT * 2) as u32,
        ).position_centered()
        .resizable()
        .build()
        .unwrap()
}

fn update_display(
    event_pump: &sdl2::EventPump,
    window: &Window,
    &mut (ref mut game_surface, ref mut temp_surface): &mut (Surface<'static>, Surface<'static>),
    emu: &Emu8080<SpaceInvadersInOut>,
) {
    let mut window_surface = window.surface(&event_pump).unwrap();
    let display_buffer = &emu.memory[0x2400..][..((WINDOW_WIDTH * WINDOW_HEIGHT) / 8)];
    display_window(display_buffer, game_surface);
    game_surface.blit(None, temp_surface, None).unwrap();
    temp_surface
        .blit_scaled(None, &mut window_surface, None)
        .unwrap();
    window_surface.finish().unwrap();
}

fn main() {
    let mut emu = Emu8080::<SpaceInvadersInOut>::default();
    let mut filename = None;
    let mut disassemble = false;
    for arg in args().skip(1) {
        if arg == "-d" || arg == "--disassemble" {
            disassemble = true;
        } else {
            filename = filename.or(Some(arg));
        }
    }
    if let Some(filename) = filename {
        emu.read_file_in_memory_at(&filename, 0).unwrap();
    } else {
        eprintln!("Usage: {} rom", args().next().unwrap());
        std::process::exit(1);
    }

    let mut surfaces = init_surfaces();
    surfaces
        .0
        .set_palette(&Palette::with_colors(&COLORS).unwrap())
        .expect("Could not set color palette");
    let mut cycles = 0;
    let mut n: u64 = 0;
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = init_window(&video_subsystem);
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
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => handle_buttons(&mut emu.io, keycode, true),
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => handle_buttons(&mut emu.io, keycode, false),
                _ => {}
            }
        }
        if emu.pc > 0x1FFF {
            panic!("Program counter out of game rom: {:04X}", emu.pc);
        }

        if cycles > 16666 {
            cycles -= 16667;
            if emu.int_enable {
                update_display(&event_pump, &window, &mut surfaces, &emu);
                emu.generate_interrupt(next_interrupt);
                next_interrupt = if next_interrupt == 1 { 2 } else { 1 };
            }
        }
        if disassemble {
            print!("#{} ", n);
        }
        n += 1;
        cycles += if disassemble {
            emu.step_dis()
        } else {
            emu.step()
        };
    }
}
