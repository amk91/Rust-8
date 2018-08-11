mod type_defs;
mod cpu;
mod display;

pub use self::type_defs::*;
pub use self::cpu::*;
pub use self::display::*;

extern crate sdl2;
use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;

pub fn get_key_from_keyboard(event_pump: &mut EventPump) -> Option<u8> {
    let mut key: Option<u8> = None;
    let event = event_pump.poll_event();
    match event {
        None => { },
        Some(e) => match e {
            Event::KeyDown { keycode: Some(Keycode::Num1), .. } => key = Some(0x1),
            Event::KeyDown { keycode: Some(Keycode::Num2), .. } => key = Some(0x2),
            Event::KeyDown { keycode: Some(Keycode::Num3), .. } => key = Some(0x3),
            Event::KeyDown { keycode: Some(Keycode::Num4), .. } => key = Some(0xC),
            Event::KeyDown { keycode: Some(Keycode::Q), .. } => key = Some(0x4),
            Event::KeyDown { keycode: Some(Keycode::W), .. } => key = Some(0x5),
            Event::KeyDown { keycode: Some(Keycode::E), .. } => key = Some(0x6),
            Event::KeyDown { keycode: Some(Keycode::R), .. } => key = Some(0xD),
            Event::KeyDown { keycode: Some(Keycode::A), .. } => key = Some(0x7),
            Event::KeyDown { keycode: Some(Keycode::S), .. } => key = Some(0x8),
            Event::KeyDown { keycode: Some(Keycode::D), .. } => key = Some(0x9),
            Event::KeyDown { keycode: Some(Keycode::F), .. } => key = Some(0xE),
            Event::KeyDown { keycode: Some(Keycode::Z), .. } => key = Some(0xA),
            Event::KeyDown { keycode: Some(Keycode::X), .. } => key = Some(0x0),
            Event::KeyDown { keycode: Some(Keycode::C), .. } => key = Some(0xB),
            Event::KeyDown { keycode: Some(Keycode::V), .. } => key = Some(0xF),
            
            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                ::std::process::exit(0);
            },

            _ => { },
        },
    }

    key
}
