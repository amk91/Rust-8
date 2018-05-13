use std::io::{Read, Stdin, Stdout, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{thread, fs::File};

extern crate rand;
use rand::Rng;

extern crate termion;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

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

#[derive(Default)]
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
    	},
    	None => {},
    }

}

fn sknp(vx: usize, key: Option<u8>, registers: &Registers, program_counter: &mut u16) {
    match key {
    	Some(key) => {
		    if registers[vx] != key {
		        *program_counter += 2;
		    }
    	},
    	None => {},
    }
}

fn ld_delay_to_reg(vx: usize, delay_timer: Arc<Mutex<u8>>, registers: &mut Registers) {
    let delay_timer = delay_timer.lock().unwrap();
    registers[vx] = *delay_timer;
}

fn ld_key(vx: usize, registers: &mut Registers) {
    // let stdin = std::io::stdin();
    // for c in stdin.keys() {
    // 	match c.unwrap() {
    // 		Key::Char(c) => println!("Ok"),
    // 		_ => {}
    // 	}
    // }

    // let mut stdout = std::io::stdout().into_raw_mode().unwrap();
    // stdout.flush().unwrap();
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

        _ => {}
    };

    op_code
}

fn load_rom(filepath: String, memory: &mut Memory) {
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
	let stdout = std::io::stdout();

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

        draw(Arc::clone(&frame_buffer), &stdout);

        let thread_duration = thread_duration.elapsed().subsec_nanos() / 1_000_000;
        if thread_duration < SEC_THREAD_MS {
        	let thread_duration: u64 = (SEC_THREAD_MS - thread_duration).into();
        	thread::sleep(Duration::from_millis(thread_duration));
        }
    }
}

fn draw(frame_buffer: Arc<Mutex<FrameBuffer>>, stdout: &Stdout) {
    let mut stdout = stdout.lock().into_raw_mode().unwrap();
    let frame_buffer = frame_buffer.lock().unwrap();
    for i in 0..frame_buffer.len() {
        for j in 0..frame_buffer[i].len() {
            let c = if !frame_buffer[i][j] { "▓" } else { " " };
            write!(
                stdout,
                "{}{}",
                termion::cursor::Goto(i as u16 + 1, j as u16 + 1),
                c
            ).unwrap();
        }
    }

    write!(stdout, "{}", termion::cursor::Hide).unwrap();
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

    // Setup of async input handler
    let mut stdin = termion::async_stdin().bytes();

    loop {
    	let thread_duration = Instant::now();

    	let key_pressed = stdin.next();
    	let mut key: Option<u8> = None;
    	match key_pressed {
    		Some(key_pressed) => {
    			match key_pressed.unwrap() {
    				b'1' => key = Some(b'1'),
    				b'2' => key = Some(b'2'),
    				b'3' => key = Some(b'3'),
    				b'4' => key = Some(b'C'),
    				b'q' => key = Some(b'4'),
    				b'w' => key = Some(b'5'),
    				b'e' => key = Some(b'6'),
    				b'r' => key = Some(b'D'),
    				b'a' => key = Some(b'7'),
    				b's' => key = Some(b'8'),
    				b'd' => key = Some(b'9'),
    				b'f' => key = Some(b'E'),
    				b'z' => key = Some(b'A'),
    				b'x' => key = Some(b'0'),
    				b'c' => key = Some(b'B'),
    				b'v' => key = Some(b'F'),
    				_ => key = None,
    			}
    		}
    		_ => key = None,
    	};

        let op = get_op_code(&memory, &program_counter);
        if op.x.is_some() && op.y.is_some() && op.variant.is_some() {
            let x = op.x.unwrap();
            let y = op.y.unwrap();
            let variant = op.variant.unwrap();
            match op.code {
                0x0 => {
                    match variant {
                        0x0 => cls(Arc::clone(&frame_buffer)),
                        0xE => ret(&mut program_counter, &mut stack_pointer, &stack),

                        _ => {
                            //TODO: raise error
                        }
                    };
                }
                0x1 => jp_addr(x, &mut program_counter),
                0x2 => call_addr(x, &mut stack_pointer, &mut program_counter, &mut stack),
                0x3 => se(x as usize, y as u8, &mut registers, &mut program_counter),
                0x4 => sne(x as usize, y as u8, &mut registers, &mut program_counter),
                0x5 => se_regs(x as usize, y as usize, &mut registers, &mut program_counter),
                0x6 => ld(x as usize, y as u8, &mut registers),
                0x7 => add(x as usize, y as u8, &mut registers),
                0x8 => {
                    match variant {
                        0x0 => ld_regs(x as usize, y as usize, &mut registers),
                        0x1 => or(x as usize, y as usize, &mut registers),
                        0x2 => and(x as usize, y as usize, &mut registers),
                        0x3 => xor(x as usize, y as usize, &mut registers),
                        0x4 => add_regs(x as usize, y as usize, &mut registers),
                        0x5 => sub_regs(x as usize, y as usize, &mut registers),
                        0x6 => shr(x as usize, &mut registers),
                        0x7 => subn_regs(x as usize, y as usize, &mut registers),
                        0xE => shl(x as usize, &mut registers),

                        _ => {
                            //TODO: raise error
                        }
                    }
                }
                0x9 => sne_regs(x as usize, y as usize, &mut registers, &mut program_counter),
                0xA => ld_reg_index(x, &mut program_counter),
                0xB => jp_v0(x, &mut program_counter, &registers),
                0xC => rnd(x as usize, y as u8, &mut registers),
                0xD => drw(
                    x as usize,
                    y as usize,
                    variant,
                    &mut registers,
                    index_register,
                    &memory,
                    Arc::clone(&frame_buffer),
                ),
                0xE => {
                    match variant {
                        0x9E => skp(x as usize, key, &registers, &mut program_counter),
                        0xA1 => sknp(x as usize, key, &registers, &mut program_counter),

                        _ => {
                            //TODO: raise error
                        }
                    }
                }
                0xF => match variant {
                    0x07 => ld_delay_to_reg(x as usize, Arc::clone(&delay_timer), &mut registers),
                    0x0A => ld_key(x as usize, &mut registers),
                    0x15 => ld_reg_to_delay(x as usize, Arc::clone(&delay_timer), &registers),
                    0x18 => ld_reg_to_sound(x as usize, Arc::clone(&sound_timer), &registers),
                    0x1E => add_reg_index(x as usize, &mut index_register, &registers),
                    0x29 => ld_sprite(x as usize, &registers, &mut index_register),
                    0x55 => ld_bcd(x as usize, &registers, &index_register, &mut memory),
                    0x65 => ld_x_regs(x as usize, &mut registers, &mut index_register, &memory),
                    _ => {
                        //TODO: raise error
                    }
                },

                _ => {
                    //TODO: raise error
                }
            };

            program_counter += 2;
        }

        let thread_duration = thread_duration.elapsed().subsec_nanos() / 1_000_000;
        if thread_duration < MAIN_THREAD_MS {
        	let thread_duration: u64 = (MAIN_THREAD_MS - thread_duration).into();
        	thread::sleep(Duration::from_millis(thread_duration));
        }
    }
}
