mod type_defs;
mod op_functions;
mod cpu;

pub use self::type_defs::*;
pub use self::op_functions::*;
pub use self::cpu::*;

extern crate sdl2;
use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;

pub fn get_key_from_keyboard(event_pump: &mut EventPump) -> Option<u8> {
    let mut key: Option<u8> = None;
    let event = event_pump.poll_event();
    match event {
        None => {},
        Some(e) => match e {
            Event::KeyDown { keycode: Some(Keycode::Kp1), .. } => key = Some(b'1'),
            Event::KeyDown { keycode: Some(Keycode::Kp2), .. } => key = Some(b'2'),
            Event::KeyDown { keycode: Some(Keycode::Kp3), .. } => key = Some(b'3'),
            Event::KeyDown { keycode: Some(Keycode::Kp4), .. } => key = Some(b'C'),
            Event::KeyDown { keycode: Some(Keycode::Q), .. } => key = Some(b'4'),
            Event::KeyDown { keycode: Some(Keycode::W), .. } => key = Some(b'5'),
            Event::KeyDown { keycode: Some(Keycode::E), .. } => key = Some(b'6'),
            Event::KeyDown { keycode: Some(Keycode::R), .. } => key = Some(b'D'),
            Event::KeyDown { keycode: Some(Keycode::A), .. } => key = Some(b'7'),
            Event::KeyDown { keycode: Some(Keycode::S), .. } => key = Some(b'8'),
            Event::KeyDown { keycode: Some(Keycode::D), .. } => key = Some(b'9'),
            Event::KeyDown { keycode: Some(Keycode::F), .. } => key = Some(b'E'),
            Event::KeyDown { keycode: Some(Keycode::Z), .. } => key = Some(b'A'),
            Event::KeyDown { keycode: Some(Keycode::X), .. } => key = Some(b'0'),
            Event::KeyDown { keycode: Some(Keycode::C), .. } => key = Some(b'B'),
            Event::KeyDown { keycode: Some(Keycode::V), .. } => key = Some(b'F'),
            _ => key = None,
        },
    }

    key
}
