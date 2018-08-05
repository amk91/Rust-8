use support::type_defs::*;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::Read;
use std::thread;

extern crate sdl2;
use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::keyboard::Keycode;

pub fn get_op_code(memory: &Memory, program_counter: &u16) -> OpCode {
    let mut op_code: OpCode = OpCode::default();
    op_code.code = memory[*program_counter as usize] >> 4;
    match op_code.code {
        0x0 => op_code.variant = Some(0b0000_1111 & memory[(*program_counter + 1) as usize]),

        0x1 | 0x2 | 0xA | 0xB => {
            let mut address = u16::from(0b0000_1111 & memory[*program_counter as usize]) << 8;
            address |= u16::from(memory[(*program_counter + 1) as usize]);

            op_code.x = Some(address)
        },

        0x3 | 0x4 | 0x6 | 0x7 => {
            op_code.x = Some(u16::from(0b0000_1111 & memory[*program_counter as usize]));
            op_code.y = Some(u16::from(memory[(*program_counter + 1) as usize]))
        },

        0x5 | 0x8 | 0x9 | 0xD => {
            op_code.x = Some(u16::from(0b0000_1111 & memory[*program_counter as usize]));
            op_code.y = Some(u16::from(0b1111_0000 & memory[(*program_counter + 1) as usize],
            ));
            op_code.variant = Some(0b0000_1111 & memory[(*program_counter + 1) as usize])
        },

        0xC => {
            op_code.x = Some(u16::from(0b0000_1111 & memory[*program_counter as usize]));
            op_code.y = Some(u16::from(memory[(*program_counter + 1) as usize]))
        },

        0xE | 0xF => {
            op_code.x = Some(u16::from(0b0000_1111 & memory[*program_counter as usize]));
            op_code.variant = Some(memory[(*program_counter + 1) as usize])
        },

        _ => {
        	panic!("Unable to parse OpCode {}", op_code.code);
        },
    };

    op_code
}

pub fn load_rom(filepath: &String, memory: &mut Memory) {
    let mut file = File::open(filepath).expect("File not found");
    let mut buffer: Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)
        .expect("Unable to read buffer");

    let mut index = 0x200;
    for byte in buffer.iter() {
        if index < 4096 {
            memory[index] = *byte;
            index += 1;
        }
    }
}

pub fn tick_60hz(
    delay_timer: Arc<Mutex<u8>>,
    sound_timer: Arc<Mutex<u8>>,
    frame_buffer: Arc<Mutex<FrameBuffer>>,
    sdl_context: sdl2::Sdl
) {
    loop {
        let thread_duration = Instant::now();
        {
            let mut delay_timer = delay_timer.lock().unwrap();
            if *delay_timer > 0 {
                *delay_timer -= 1;
            }
        }

        {
            let mut sound_timer = sound_timer.lock().unwrap();
            if *sound_timer > 0 {
                *sound_timer -= 1;
                //TODO: sound the buzzer
            }
        }

        draw(Arc::clone(&frame_buffer), &mut sdl_context);

        let thread_duration = thread_duration.elapsed().subsec_nanos() / 1_000_000;
        if thread_duration < SEC_THREAD_MS {
            let thread_duration: u64 = (SEC_THREAD_MS - thread_duration).into();
            thread::sleep(Duration::from_millis(thread_duration));
        }
    }
}

pub fn draw(
    frame_buffer: Arc<Mutex<FrameBuffer>>,
    sdl_context: &mut sdl2::Sdl,
) {
    let mut event_pump = sdl_context.event_pump().unwrap();

    let frame_buffer = frame_buffer.lock().unwrap();
    for i in 0..frame_buffer.len() {
        for j in 0..frame_buffer[i].len() {
            
        }
    }
}

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
