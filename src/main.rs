use std::io::{Read, Stdin, Stdout, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{thread, fs::File};
use std::env;

extern crate rand;
use rand::Rng;

extern crate sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;

const N_REGISTERS: usize = 16;
const N_STACK: usize = 32;
const N_FRAMEBUFFER_WIDTH: usize = 64;
const N_FRAMEBUFFER_HEIGHT: usize = 32;
const N_MEMORY: usize = 4096;

const MAIN_THREAD_MS: u32 = 2;
const SEC_THREAD_MS: u32 = 17;

type Registers = [u8; N_REGISTERS];
type Stack = [u16; N_STACK];
type FrameBuffer = [[bool; N_FRAMEBUFFER_HEIGHT]; N_FRAMEBUFFER_WIDTH];
type Memory = [u8; N_MEMORY];

#[derive(Default, Debug)]
struct OpCode {
    code: u8,
    x: Option<u16>,
    y: Option<u16>,
    variant: Option<u8>,
}

type Sprite = [u8; 5];

static SPRITES: [Sprite; 16] = [
    [0xF0, 0x90, 0x90, 0x90, 0xF0],
    [0x20, 0x60, 0x20, 0x20, 0x70],
    [0xF0, 0x10, 0xF0, 0x80, 0xF0],
    [0xF0, 0x10, 0x90, 0x10, 0xF0],
    [0x90, 0x90, 0xF0, 0x10, 0x10],
    [0xF0, 0x80, 0xF0, 0x10, 0xF0],
    [0xF0, 0x80, 0xF0, 0x90, 0xF0],
    [0xF0, 0x10, 0x20, 0x40, 0x40],
    [0xF0, 0x90, 0xF0, 0x90, 0xF0],
    [0xF0, 0x90, 0x90, 0x10, 0xF0],
    [0xF0, 0x90, 0xF0, 0x90, 0x90],
    [0xE0, 0x90, 0xE0, 0x90, 0xE0],
    [0xF0, 0x80, 0x80, 0x80, 0xF0],
    [0xE0, 0x90, 0x90, 0x90, 0xE0],
    [0xF0, 0x80, 0xF0, 0x80, 0xF0],
    [0xF0, 0x80, 0xF0, 0x80, 0x80],
];

fn cls(frame_buffer: Arc<Mutex<FrameBuffer>>) {
    let mut frame_buffer = frame_buffer.lock().unwrap();
    for row in frame_buffer.iter_mut() {
        for p in row.iter_mut() {
            *p = false;
        }
    }
}

fn ret(program_counter: &mut u16, stack_pointer: &mut usize, stack: &Stack) {
    *program_counter = stack[*stack_pointer];
    *stack_pointer -= 1;
}

fn jp_addr(address: u16, program_counter: &mut u16) {
    *program_counter = address;
}

fn call_addr(
    address: u16,
    stack_pointer: &mut usize,
    program_counter: &mut u16,
    stack: &mut Stack,
) {
    *stack_pointer += 1;
    stack[*stack_pointer] = *program_counter;
    *program_counter = address;
}

fn se(vx: usize, value: u8, registers: &mut Registers, program_counter: &mut u16) {
    if registers[vx] == value {
        *program_counter += 2;
    }
}

fn sne(vx: usize, value: u8, registers: &mut Registers, program_counter: &mut u16) {
    if registers[vx] != value {
        *program_counter += 2;
    }
}

fn se_regs(vx: usize, vy: usize, registers: &mut Registers, program_counter: &mut u16) {
    if registers[vx] == registers[vy] {
        *program_counter += 2;
    }
}

fn ld(vx: usize, value: u8, registers: &mut Registers) {
    registers[vx] = value;
}

fn add(vx: usize, value: u8, registers: &mut Registers) {
    registers[vx] += value;
}

fn ld_regs(vx: usize, vy: usize, registers: &mut Registers) {
    registers[vx] = registers[vy];
}

fn or(vx: usize, vy: usize, registers: &mut Registers) {
    registers[vx] |= registers[vy];
}

fn and(vx: usize, vy: usize, registers: &mut Registers) {
    registers[vx] &= registers[vy];
}

fn xor(vx: usize, vy: usize, registers: &mut Registers) {
    registers[vx] ^= registers[vy];
}

fn add_regs(vx: usize, vy: usize, registers: &mut Registers) {
    let result = registers[vx].overflowing_add(registers[vy]);
    registers[vx] = result.0;
    registers[N_REGISTERS - 1] = result.1 as u8;
}

fn sub_regs(vx: usize, vy: usize, registers: &mut Registers) {
    let result = registers[vx].overflowing_sub(registers[vy]);
    registers[vx] = result.0;
    registers[N_REGISTERS - 1] = result.1 as u8;
}

fn shr(vx: usize, registers: &mut Registers) {
    registers[N_REGISTERS - 1] = registers[vx] & 0b0000_0001;
    registers[vx] /= 2;
}

fn subn_regs(vx: usize, vy: usize, registers: &mut Registers) {
    let result = registers[vy].overflowing_sub(registers[vx]);
    registers[vx] = result.0;
    registers[N_REGISTERS - 1] = result.1 as u8;
}

fn shl(vx: usize, registers: &mut Registers) {
    registers[N_REGISTERS - 1] = registers[vx] & 0b1000_0000;
    registers[vx] *= 2;
}

fn sne_regs(vx: usize, vy: usize, registers: &mut Registers, program_counter: &mut u16) {
    if registers[vx] != registers[vy] {
        *program_counter += 1;
    }
}

fn ld_reg_index(address: u16, index_register: &mut u16) {
    *index_register = address;
}

fn jp_v0(address: u16, program_counter: &mut u16, registers: &Registers) {
    *program_counter = u16::from(registers[0]) + address;
}

fn rnd(vx: usize, value: u8, registers: &mut Registers) {
    let mut rng = rand::thread_rng();
    let result: u8 = rng.gen::<u8>() & value;
    registers[vx] = result;
}

fn drw(
    vx: usize,
    vy: usize,
    bytes_number: u8,
    registers: &mut Registers,
    index_register: u16,
    memory: &Memory,
    frame_buffer: Arc<Mutex<FrameBuffer>>,
) {
    let mut frame_buffer = frame_buffer.lock().unwrap();
    for byte in 0..bytes_number {
        let sprite_index = memory[(index_register + u16::from(byte)) as usize] as usize;
        if sprite_index < 16 {
            let sprite = SPRITES[sprite_index];
            for j in 0..sprite.len() {
                for i in 0..8 {
                    let x = N_FRAMEBUFFER_WIDTH % (i + registers[vx] as usize);
                    let y = N_FRAMEBUFFER_HEIGHT % (j + registers[vy] as usize);

                    let old_pixel = frame_buffer[y][x];
                    let new_pixel = sprite[j] & 1 << (8 - i) > 0;

                    frame_buffer[y][x] ^= new_pixel;

                    if old_pixel && new_pixel {
                        registers[N_REGISTERS - 1] = 1;
                    }
                }
            }
        }
    }
}

fn skp(vx: usize, key: Option<u8>, registers: &Registers, program_counter: &mut u16) {
    match key {
        Some(key) => {
            if registers[vx] == key {
                *program_counter += 2;
            }
        }
        None => {}
    }
}

fn sknp(vx: usize, key: Option<u8>, registers: &Registers, program_counter: &mut u16) {
    match key {
        Some(key) => {
            if registers[vx] != key {
                *program_counter += 2;
            }
        }
        None => {}
    }
}

fn ld_delay_to_reg(vx: usize, delay_timer: Arc<Mutex<u8>>, registers: &mut Registers) {
    let delay_timer = delay_timer.lock().unwrap();
    registers[vx] = *delay_timer;
}

fn ld_key(vx: usize, registers: &mut Registers, event_pump: &mut sdl2::EventPump) {
	//TODO: get key blocking the loop

    let mut key: Option<u8> = None;
    println!("entering blocking loop");
    while key.is_none() {
		for event in event_pump.poll_iter() {
	        match event {
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
	            _ => {}
	        }
	    }
	}
    println!("exiting blocking loop");
}

fn ld_reg_to_delay(vx: usize, delay_timer: Arc<Mutex<u8>>, registers: &Registers) {
    let mut delay_timer = delay_timer.lock().unwrap();
    *delay_timer = registers[vx];
}

fn ld_reg_to_sound(vx: usize, sound_timer: Arc<Mutex<u8>>, registers: &Registers) {
    let mut sound_timer = sound_timer.lock().unwrap();
    *sound_timer = registers[vx];
}

fn add_reg_index(vx: usize, index_register: &mut u16, registers: &Registers) {
    *index_register += u16::from(registers[vx]);
}

fn ld_sprite(vx: usize, registers: &Registers, index_register: &mut u16) {
    if (registers[vx] >= 'A'.to_digit(10).unwrap() as u8
        && registers[vx] <= 'F'.to_digit(10).unwrap() as u8) || registers[vx] <= 9
    {
        *index_register = u16::from(registers[vx]);
    }
}

fn ld_bcd(vx: usize, registers: &Registers, index_register: &u16, memory: &mut Memory) {
    let h = (registers[vx] / 100) & 0b0000_0111;
    let d = (registers[vx] / 10) & 0b0000_0111;
    let u = registers[vx] & 0b0000_0111;

    memory[*index_register as usize] = h;
    memory[(*index_register + 1) as usize] = d;
    memory[(*index_register + 2) as usize] = u;
}

fn ld_x_regs(vx: usize, registers: &mut Registers, index_register: &mut u16, memory: &Memory) {
    for i in 0..=vx {
        registers[i] = memory[usize::from(*index_register) + i];
    }

    *index_register += (vx + 1) as u16;
}

fn get_op_code(memory: &Memory, program_counter: &u16) -> OpCode {
    let mut op_code: OpCode = OpCode::default();
    op_code.code = memory[*program_counter as usize] >> 4;
    match op_code.code {
        0 => op_code.variant = Some(0b0000_1111 & memory[(*program_counter + 1) as usize]),

        1 | 2 | 0xA | 0xB => {
            let mut address = u16::from(0b0000_1111 & memory[*program_counter as usize]) << 8;
            address |= u16::from(memory[(*program_counter + 1) as usize]);

            op_code.x = Some(address)
        }

        3 | 4 | 6 | 7 => {
            op_code.x = Some(u16::from(0b0000_1111 & memory[*program_counter as usize]));
            op_code.y = Some(u16::from(memory[(*program_counter + 1) as usize]))
        }

        5 | 8 | 9 => {
            op_code.x = Some(u16::from(0b0000_1111 & memory[*program_counter as usize]));
            op_code.y = Some(u16::from(
                0b1111_0000 & memory[(*program_counter + 1) as usize],
            ));
            op_code.variant = Some(0b0000_1111 & memory[(*program_counter + 1) as usize])
        }

        0xC => {
            op_code.x = Some(u16::from(0b0000_1111 & memory[*program_counter as usize]));
            op_code.y = Some(u16::from(memory[(*program_counter + 1) as usize]))
        }

        0xE | 0xF => {
            op_code.x = Some(u16::from(0b0000_1111 & memory[*program_counter as usize]));
            op_code.variant = Some(memory[(*program_counter + 1) as usize])
        }

        _ => {
        	std::process::exit(0);
        }
    };

    op_code
}

fn load_rom(filepath: &String, memory: &mut Memory) {
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

fn tick_60hz(
    delay_timer: Arc<Mutex<u8>>,
    sound_timer: Arc<Mutex<u8>>,
    frame_buffer: Arc<Mutex<FrameBuffer>>
) {
	// let sdl_context = sdl2::init().unwrap();
	// let video_subsystem = sdl_context.video().unwrap();
 //    let window = video_subsystem
 //    	.window("Rust-8", 640, 320)
 //        .position_centered()
 //        .build()
 //        .unwrap();
 //    let mut canvas = window
 //    	.into_canvas()
 //    	.target_texture()
 //    	.build()
 //    	.unwrap();

 //    loop {
 //        let thread_duration = Instant::now();
 //        {
 //            let mut delay_timer = delay_timer.lock().unwrap();
 //            if *delay_timer > 0 {
 //                *delay_timer -= 1;
 //            }
 //        }

 //        {
 //            let mut sound_timer = sound_timer.lock().unwrap();
 //            if *sound_timer > 0 {
 //                *sound_timer -= 1;
 //                //TODO: sound the buzzer
 //            }
 //        }

 //        draw(Arc::clone(&frame_buffer), &mut canvas);

 //        let thread_duration = thread_duration.elapsed().subsec_nanos() / 1_000_000;
 //        if thread_duration < SEC_THREAD_MS {
 //            let thread_duration: u64 = (SEC_THREAD_MS - thread_duration).into();
 //            thread::sleep(Duration::from_millis(thread_duration));
 //        }
 //    }
}

fn draw(frame_buffer: Arc<Mutex<FrameBuffer>>, canvas: &mut Canvas<Window>) {
    let frame_buffer = frame_buffer.lock().unwrap();
    for i in 0..frame_buffer.len() {
        for j in 0..frame_buffer[i].len() {
            let c = if !frame_buffer[i][j] { "▓" } else { " " };

            
        }
    }
}

fn main() {
    let mut index_register: u16 = 0b0;
    let mut registers: Registers = [0; N_REGISTERS];

    let mut stack_pointer: usize = 0;
    let mut stack: Stack = [0; N_STACK];

    let frame_buffer: FrameBuffer = [[false; N_FRAMEBUFFER_HEIGHT]; N_FRAMEBUFFER_WIDTH];
    let frame_buffer = Arc::new(Mutex::new(frame_buffer));
    let mut memory: Memory = [0; N_MEMORY];

    let mut program_counter: u16 = 0x200;

    let delay_timer = Arc::new(Mutex::new(u8::from(0)));
    let sound_timer = Arc::new(Mutex::new(u8::from(0)));

    {
        let delay_timer = Arc::clone(&delay_timer);
        let sound_timer = Arc::clone(&sound_timer);
        let frame_buffer = Arc::clone(&frame_buffer);
        thread::spawn(|| tick_60hz(delay_timer, sound_timer, frame_buffer));
    }

	// let args: Vec<String> = env::args().collect();
	// if args.len() < 2 {
	// 	println!("Unable to load ROM");
	// 	std::process::exit(0);
	// } else {
	// 	load_rom(&args[1], &mut memory);
	// }

	let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    loop {
        let thread_duration = Instant::now();

        let mut key: Option<u8> = None;
		for event in event_pump.poll_iter() {
            match event {
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
                _ => {}
            }
        }

        let op = get_op_code(&memory, &program_counter);
        match op.code {
            0x0 => match op.variant.unwrap() {
                0x0 => cls(Arc::clone(&frame_buffer)),
                0xE => ret(&mut program_counter, &mut stack_pointer, &stack),

                _ => {
                    //TODO: raise error
                }
            },
            0x1 => jp_addr(op.x.unwrap(), &mut program_counter),
            0x2 => call_addr(
                op.x.unwrap(),
                &mut stack_pointer,
                &mut program_counter,
                &mut stack,
            ),
            0x3 => se(
                op.x.unwrap() as usize,
                op.y.unwrap() as u8,
                &mut registers,
                &mut program_counter,
            ),
            0x4 => sne(
                op.x.unwrap() as usize,
                op.y.unwrap() as u8,
                &mut registers,
                &mut program_counter,
            ),
            0x5 => se_regs(
                op.x.unwrap() as usize,
                op.y.unwrap() as usize,
                &mut registers,
                &mut program_counter,
            ),
            0x6 => ld(op.x.unwrap() as usize, op.y.unwrap() as u8, &mut registers),
            0x7 => add(op.x.unwrap() as usize, op.y.unwrap() as u8, &mut registers),
            0x8 => match op.variant.unwrap() {
                0x0 => ld_regs(
                    op.x.unwrap() as usize,
                    op.y.unwrap() as usize,
                    &mut registers,
                ),
                0x1 => or(
                    op.x.unwrap() as usize,
                    op.y.unwrap() as usize,
                    &mut registers,
                ),
                0x2 => and(
                    op.x.unwrap() as usize,
                    op.y.unwrap() as usize,
                    &mut registers,
                ),
                0x3 => xor(
                    op.x.unwrap() as usize,
                    op.y.unwrap() as usize,
                    &mut registers,
                ),
                0x4 => add_regs(
                    op.x.unwrap() as usize,
                    op.y.unwrap() as usize,
                    &mut registers,
                ),
                0x5 => sub_regs(
                    op.x.unwrap() as usize,
                    op.y.unwrap() as usize,
                    &mut registers,
                ),
                0x6 => shr(op.x.unwrap() as usize, &mut registers),
                0x7 => subn_regs(
                    op.x.unwrap() as usize,
                    op.y.unwrap() as usize,
                    &mut registers,
                ),
                0xE => shl(op.x.unwrap() as usize, &mut registers),

                _ => {
                    //TODO: raise error
                }
            },
            0x9 => sne_regs(
                op.x.unwrap() as usize,
                op.y.unwrap() as usize,
                &mut registers,
                &mut program_counter,
            ),
            0xA => ld_reg_index(op.x.unwrap(), &mut program_counter),
            0xB => jp_v0(op.x.unwrap(), &mut program_counter, &registers),
            0xC => rnd(op.x.unwrap() as usize, op.y.unwrap() as u8, &mut registers),
            0xD => drw(
                op.x.unwrap() as usize,
                op.y.unwrap() as usize,
                op.variant.unwrap(),
                &mut registers,
                index_register,
                &memory,
                Arc::clone(&frame_buffer),
            ),
            0xE => match op.variant.unwrap() {
                0x9E => skp(
                    op.x.unwrap() as usize,
                    key,
                    &registers,
                    &mut program_counter,
                ),
                0xA1 => sknp(
                    op.x.unwrap() as usize,
                    key,
                    &registers,
                    &mut program_counter,
                ),

                _ => {
                    //TODO: raise error
                }
            },
            0xF => match op.variant.unwrap() {
                0x07 => ld_delay_to_reg(
                    op.x.unwrap() as usize,
                    Arc::clone(&delay_timer),
                    &mut registers,
                ),
                0x0A => ld_key(op.x.unwrap() as usize, &mut registers, &mut event_pump),
                0x15 => ld_reg_to_delay(
                    op.x.unwrap() as usize,
                    Arc::clone(&delay_timer),
                    &registers,
                ),
                0x18 => ld_reg_to_sound(
                    op.x.unwrap() as usize,
                    Arc::clone(&sound_timer),
                    &registers,
                ),
                0x1E => add_reg_index(op.x.unwrap() as usize, &mut index_register, &registers),
                0x29 => ld_sprite(op.x.unwrap() as usize, &registers, &mut index_register),
                0x55 => ld_bcd(
                    op.x.unwrap() as usize,
                    &registers,
                    &index_register,
                    &mut memory,
                ),
                0x65 => ld_x_regs(
                    op.x.unwrap() as usize,
                    &mut registers,
                    &mut index_register,
                    &memory,
                ),
                _ => {
                    //TODO: raise error
                }
            },

            _ => {
                //TODO: raise error
            }
        };

        program_counter += 2;

        let thread_duration = thread_duration.elapsed().subsec_nanos() / 1_000_000;
        if thread_duration < MAIN_THREAD_MS {
            let thread_duration: u64 = (MAIN_THREAD_MS - thread_duration).into();
            thread::sleep(Duration::from_millis(thread_duration));
        }
    }
}
