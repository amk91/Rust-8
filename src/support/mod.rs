mod type_defs;
mod cpu;
mod display;

pub use self::type_defs::*;
pub use self::cpu::*;
pub use self::display::*;

extern crate sdl2;
use sdl2::EventPump;
use sdl2::event::Event;
use sdl2::keyboard::{ Keycode, Scancode };

pub fn get_key_from_keyboard(event_pump: &mut EventPump) -> Option<u8> {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                ::std::process::exit(0)
            },
            _ => { },
        }
    }

    let mut pressed_key: Option<u8> = None;
    for key in event_pump.keyboard_state().scancodes() {
        match key {
            ( Scancode::Num1, true ) => pressed_key = Some(0x1),
            ( Scancode::Num2, true ) => pressed_key = Some(0x2),
            ( Scancode::Num3, true ) => pressed_key = Some(0x3),
            ( Scancode::Num4, true ) => pressed_key = Some(0xC),
            ( Scancode::Q, true ) => pressed_key = Some(0x4),
            ( Scancode::W, true ) => pressed_key = Some(0x5),
            ( Scancode::E, true ) => pressed_key = Some(0x6),
            ( Scancode::R, true ) => pressed_key = Some(0xD),
            ( Scancode::A, true ) => pressed_key = Some(0x7),
            ( Scancode::S, true ) => pressed_key = Some(0x8),
            ( Scancode::D, true ) => pressed_key = Some(0x9),
            ( Scancode::F, true ) => pressed_key = Some(0xE),
            ( Scancode::Z, true ) => pressed_key = Some(0xA),
            ( Scancode::X, true ) => pressed_key = Some(0x0),
            ( Scancode::C, true ) => pressed_key = Some(0xB),
            ( Scancode::V, true ) => pressed_key = Some(0xF),

            _ => { },
        }
    }

    pressed_key
}
