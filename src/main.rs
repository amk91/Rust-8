use std::{thread, time::Duration, sync::Arc, sync::RwLock,
		env, fs::File,
		io, io::Read, io::Write,
		prelude::*};

extern crate rand;
use rand::{Rng, thread_rng};

extern crate termion;
use termion::raw::IntoRawMode;

extern crate termios;
use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};

extern crate adi_clock;

const N_REGISTERS: usize = 16;
const N_STACK: usize = 32;
const N_FRAMEBUFFER_WIDTH: usize = 64;
const N_FRAMEBUFFER_HEIGHT: usize = 32;
const N_MEMORY: usize = 4096;
const N_SPRITE_BYTES: usize = 5;

type Registers = [u8; N_REGISTERS];
type Stack = [u16; N_STACK];
type FrameBuffer = [[bool; N_FRAMEBUFFER_HEIGHT]; N_FRAMEBUFFER_WIDTH];
type Memory = [u8; N_MEMORY];

#[derive(Default)]
struct OpCode
{
	code: u8,
	x: Option<u16>,
	y: Option<u16>,
	variant: Option<u8>,
}

type Sprite = [u8; 5];

static SPRITES: [Sprite; 16] =
[
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
	[0xF0, 0x80, 0xF0, 0x80, 0x80]
];

fn cls(frame_buffer: &mut FrameBuffer)
{
	for row in frame_buffer.iter_mut()
	{
		for p in row.iter_mut()
		{
			*p = false;
		}
	}
}

fn ret(pc: &mut u16, sp: &mut u8, stack: &Stack)
{
	*pc = stack[*sp as usize];
	*sp -= 2;
}

fn jp_addr(address: u16, pc: &mut u16)
{
	*pc = address;
}

fn call_addr(address: u16, sp: &mut u8,
	pc: &mut u16, stack: &mut Stack)
{
	*sp += 2;
	stack[*sp as usize] = *pc;
	*pc = address;
}

fn se(vx: usize, value: u8, V: &mut Registers,
	pc: &mut u16)
{
	if V[vx] == value
	{
		*pc += 1;
	}
}

fn sne(vx: usize, value: u8, V: &mut Registers,
	pc: &mut u16)
{
	if V[vx] != value
	{
		*pc += 1;
	}
}

fn se_regs(vx: usize, vy: usize, V: &mut Registers,
	pc: &mut u16)
{
	if V[vx] == V[vy]
	{
		*pc += 1;
	}
}

fn ld(vx: usize, value: u8, V: &mut Registers)
{
	V[vx] = value;
}

fn add(vx: usize, value: u8, V: &mut Registers)
{
	V[vx] += value;
}

fn ld_regs(vx: usize, vy: usize, V: &mut Registers)
{
	V[vx] = V[vy];
}

fn or(vx: usize, vy: usize, V: &mut Registers)
{
	V[vx] |= V[vy];
}

fn and(vx: usize, vy: usize, V: &mut Registers)
{
	V[vx] &= V[vy];
}

fn xor(vx: usize, vy: usize, V: &mut Registers)
{
	V[vx] ^= V[vy];
}

fn add_regs(vx: usize, vy: usize, V: &mut Registers)
{
	let result = V[vx].overflowing_add(V[vy]);
	V[vx] = result.0;
	V[N_REGISTERS - 1] = result.1 as u8;
}

fn sub_regs(vx: usize, vy: usize, V: &mut Registers)
{
	let result = V[vx].overflowing_sub(V[vy]);
	V[vx] = result.0;
	V[N_REGISTERS - 1] = result.1 as u8;
}

fn shr(vx: usize, V: &mut Registers)
{
	let lsb = 0b00000001;
	V[N_REGISTERS - 1] = V[vx] & lsb;
	V[vx] /= 2;
}

fn subn_regs(vx: usize, vy: usize, V: &mut Registers)
{
	let result = V[vy].overflowing_sub(V[vx]);
	V[vx] = result.0;
	V[N_REGISTERS - 1] = result.1 as u8;
}

fn shl(vx: usize, V: &mut Registers)
{
	let msb = 0b10000000;
	V[N_REGISTERS - 1] = V[vx] & msb;
	V[vx] *= 2;
}

fn sne_regs(vx: usize, vy: usize, V: &mut Registers,
	pc: &mut u16)
{
	if V[vx] != V[vy]
	{
		*pc += 1;
	}
}

fn ld_reg_index(address: u16, I: &mut u16)
{
	*I = address;
}

fn jp_v0(address: u16, pc: &mut u16,
	V: &Registers)
{
	*pc = V[0] as u16 + address;
}

fn rnd(vx: usize, value: u8, V: &mut Registers)
{
	let mut rng = rand::thread_rng();
	let result: u8 = rng.gen::<u8>() & value;
	V[vx] = result;
}

fn drw(vx: usize, vy: usize, bytes_number: u8,
	V: &mut Registers, I: u16, memory: &Memory,
	frame_buffer: &mut FrameBuffer)
{
	let x = V[vx] as usize;
	let y = V[vy] as usize;

	for byte in 0..bytes_number
	{
		let sprite_index = memory[(I + byte as u16) as usize] as usize;
		if sprite_index >= 0 && sprite_index < 16
		{
			let sprite = SPRITES[sprite_index];
			for j in 0..sprite.len()
			{
				for i in 0..8
				{
					let old_pixel = frame_buffer[j][i];
					let new_pixel = sprite[j] & (1 << 8 - i) > 0;

					let x_index = N_FRAMEBUFFER_WIDTH % i;
					let y_index = N_FRAMEBUFFER_HEIGHT % j;

					frame_buffer[y_index][x_index] ^= new_pixel;

					if old_pixel == true && new_pixel == true
					{
						V[0xF - 1] = 1;
					}
				}
			}
		}
	}
}

fn skp(vx: usize, key_pressed: u8,
	V: &Registers, pc: &mut u16)
{
	if V[vx] == key_pressed
	{
		*pc += 2;
	}
}

fn sknp(vx: usize, key_pressed: u8,
	V: &Registers, pc: &mut u16)
{
	if V[vx] != key_pressed
	{
		*pc += 2;
	}
}

fn ld_delay_to_reg(vx: usize, delay_timer: Arc<RwLock<u8>>, V: &mut Registers)
{
	let delay_timer = delay_timer.read().unwrap();
	V[0] = *delay_timer;
}

fn ld_key(vx: usize, V: &mut Registers,
	stdin: &mut io::Stdin, stdout: &io::Stdout)
{
	let mut buffer : [u8; 1] = [0; 1];
	stdout.lock().flush().unwrap();
	stdin.read_exact(&mut buffer).unwrap();
	V[vx] = buffer[0];
}

fn ld_reg_to_delay(vx: usize, delay_timer: Arc<RwLock<u8>>, V: &Registers)
{
	let mut delay_timer = delay_timer.write().unwrap();
	*delay_timer = V[vx];
}

fn ld_reg_to_sound(vx: usize, sound_timer: Arc<RwLock<u8>>, V: &Registers)
{
	let mut sound_timer = sound_timer.write().unwrap();
	*sound_timer = V[vx];
}

fn add_reg_index(vx: usize, I: &mut u16, V: &Registers)
{
	*I += V[vx] as u16;
}

fn set_sprite(vx: usize, V: &Registers, I: &mut u16)
{
	if V[vx] >= 0 && V[vx] <= 9
	{
		*I = V[vx] as u16;
	}
	else if V[vx] >= 'A'.to_digit(10).unwrap() as u8 &&
		V[vx] <= 'F'.to_digit(10).unwrap() as u8
	{
		*I = V[vx] as u16;
	}
}

fn store_bcd(vx: usize, V: &Registers,
	I: &u16, memory: &mut Memory)
{
	let h = V[vx] / 100 & 0b00000111;
	let d = V[vx] / 10 & 0b00000111;
	let u = V[vx] & 0b00000111;

	memory[*I as usize] = h;
	memory[(*I + 1) as usize] = d;
	memory[(*I + 2) as usize] = u;
}

fn get_op_code(memory: &Memory, pc: &u16) -> OpCode
{
	let mut op_code: OpCode = OpCode::default();
	op_code.code = memory[*pc as usize] >> 4;
	match op_code.code
	{
		0 =>
		{
			op_code.variant = Some(0b00001111 & memory[(*pc + 1) as usize])
		},

		1 | 2 | 0xA | 0xB =>
		{
			let mut address = ((0b00001111 & memory[*pc as usize]) as u16) << 8;
			address |= memory[(*pc + 1) as usize] as u16;

			op_code.x = Some(address)
		},

		3 | 4 | 6 | 7=>
		{
			op_code.x = Some((0b00001111 & memory[*pc as usize]) as u16);
			op_code.y = Some(memory[(*pc + 1) as usize] as u16)
		},

		5 | 8 | 9 =>
		{
			op_code.x = Some((0b00001111 & memory[*pc as usize]) as u16);
			op_code.y = Some((0b11110000 & memory[(*pc + 1) as usize]) as u16);
			op_code.variant = Some(0b00001111 & memory[(*pc + 1) as usize])
		},

		0xC =>
		{
			op_code.x = Some((0b00001111 & memory[*pc as usize]) as u16);
			op_code.y = Some(memory[(*pc + 1) as usize] as u16)
		},

		0xE | 0xF =>
		{
			op_code.x = Some((0b00001111 & memory[*pc as usize]) as u16);
			op_code.variant = Some(memory[(*pc + 1) as usize]);
		},

		_ => { },
	};

	op_code
}

fn load_rom(filepath: String, memory: &mut Memory)
{
	let mut file = File::open(filepath).expect("File not found");
	let mut buffer: Vec<u8> = Vec::new();
	file.read_to_end(&mut buffer);

	let mut index = 0x200;
	for byte in buffer.iter()
	{
		if index < 4096
		{
			memory[index] = *byte;
			index += 1;
		}
	}
}

fn tick_60hz(delay_timer: Arc<RwLock<u8>>,
	sound_timer: Arc<RwLock<u8>>)
{
	loop
	{
		{
			let mut delay_timer = delay_timer.write().unwrap();
			if *delay_timer > 0
			{
				*delay_timer -= 1;
			}
		}

		{
			let mut sound_timer = sound_timer.write().unwrap();
			if *sound_timer > 0
			{
				*sound_timer -= 1;
				//TODO: sound the buzzer
			}
		}

		thread::sleep(Duration::from_millis(17));
	}
}

fn draw(frame_buffer: &FrameBuffer, stdout: &io::Stdout)
{
    let mut stdout = stdout.lock().into_raw_mode().unwrap();
	for i in 0..frame_buffer.len()
	{
		for j in 0..frame_buffer[i].len()
		{
			let mut c = " ";
			if !frame_buffer[i][j]
			{
				c = "▓";
			}

			write!(stdout, "{}{}", termion::cursor::Goto(i as u16 + 1, j as u16 + 1), c).unwrap();
		}
	}
}

fn main()
{
	let mut I: u16 = 0b0;
	let mut V: Registers = [0; N_REGISTERS];

	let mut sp: u8 = 0b0;
	let mut stack: Stack = [0; N_STACK];

	let mut frame_buffer: FrameBuffer = [[false; N_FRAMEBUFFER_HEIGHT]; N_FRAMEBUFFER_WIDTH];
	let mut memory: Memory = [0; N_MEMORY];

	let mut pc: u16 = 0x200;

	// Init termios and termion data
	let stdin = 0;
	let termios = Termios::from_fd(stdin).unwrap();
	let mut new_termios = termios.clone();
	new_termios.c_lflag &= !(ICANON | ECHO);
	tcsetattr(stdin, TCSANOW, &mut new_termios).unwrap();
	let stdout = io::stdout();
	let mut reader = io::stdin();

	let delay_timer = Arc::new(RwLock::new(0 as u8));
	let shared_delay_timer = delay_timer.clone();

	let sound_timer = Arc::new(RwLock::new(0 as u8));
	let shared_sound_timer = sound_timer.clone();

	thread::spawn(|| tick_60hz(shared_delay_timer, shared_sound_timer));

	loop
	{
		let op = get_op_code(&memory, &pc);
		if op.x.is_none() || op.y.is_none() || op.variant.is_none()
		{
			//TODO: raise error
		}
		else
		{
			let x = op.x.unwrap();
			let y = op.y.unwrap();
			let variant = op.variant.unwrap();
			match op.code
			{
				0x0 =>
				{
					match variant
					{
						0x0 => cls(&mut frame_buffer),
						0xE => ret(&mut pc, &mut sp, &stack),

						_ =>
						{
							//TODO: raise error
						}
					};
				},
				0x1 => jp_addr(x, &mut pc),
				0x2 => call_addr(x, &mut sp, &mut pc, &mut stack),
				0x3 => se(x as usize, y as u8, &mut V, &mut pc),
				0x4 => sne(x as usize, y as u8, &mut V, &mut pc),
				0x5 => se_regs(x as usize, y as usize, &mut V, &mut pc),
				0x6 => ld(x as usize, y as u8, &mut V),
				0x7 => add(x as usize, y as u8, &mut V),
				0x8 =>
				{
					match variant
					{
						0x0 => ld_regs(x as usize, y as usize, &mut V),
						0x1 => or(x as usize, y as usize, &mut V),
						0x2 => and(x as usize, y as usize, &mut V),
						0x3 => xor(x as usize, y as usize, &mut V),
						0x4 => add_regs(x as usize, y as usize, &mut V),
						0x5 => sub_regs(x as usize, y as usize, &mut V),
						0x6 => shr(x as usize, &mut V),
						0x7 => subn_regs(x as usize, y as usize, &mut V),
						0xE => shl(x as usize, &mut V),

						_ =>
						{
							//TODO: raise error
						}
					}
				}
				0x9 => sne_regs(x as usize, y as usize, &mut V, &mut pc),
				0xA => ld_reg_index(x, &mut pc),
				0xB => jp_v0(x, &mut pc, &mut V),
				0xC => rnd(x as usize, y as u8, &mut V),
				0xF =>
				{
					match variant
					{
						0x07 => ld_delay_to_reg(x as usize, delay_timer.clone(), &mut V),
						0x15 => ld_reg_to_delay(x as usize, delay_timer.clone(), &V),
						0x18 => ld_reg_to_sound(x as usize, sound_timer.clone(), &V),
						0x1E => add_reg_index(x as usize, &mut I, &V),
						

						_ =>
						{

						}
					}
				}

				_ =>
				{

				},
			};

			pc += 2;
		}

		thread::sleep(Duration::from_millis(2));
	}

	// Reset stdin to original termios data
	tcsetattr(stdin, TCSANOW, & termios).unwrap();
}
