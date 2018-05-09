use std::{thread, time::Duration, sync::Arc, sync::RwLock,
		fs::File, io, io::Read, io::Write};

extern crate rand;
use rand::{Rng};

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

fn se(vx: usize, value: u8, registers: &mut Registers,
	pc: &mut u16)
{
	if registers[vx] == value
	{
		*pc += 1;
	}
}

fn sne(vx: usize, value: u8, registers: &mut Registers,
	pc: &mut u16)
{
	if registers[vx] != value
	{
		*pc += 1;
	}
}

fn se_regs(vx: usize, vy: usize, registers: &mut Registers,
	pc: &mut u16)
{
	if registers[vx] == registers[vy]
	{
		*pc += 1;
	}
}

fn ld(vx: usize, value: u8, registers: &mut Registers)
{
	registers[vx] = value;
}

fn add(vx: usize, value: u8, registers: &mut Registers)
{
	registers[vx] += value;
}

fn ld_regs(vx: usize, vy: usize, registers: &mut Registers)
{
	registers[vx] = registers[vy];
}

fn or(vx: usize, vy: usize, registers: &mut Registers)
{
	registers[vx] |= registers[vy];
}

fn and(vx: usize, vy: usize, registers: &mut Registers)
{
	registers[vx] &= registers[vy];
}

fn xor(vx: usize, vy: usize, registers: &mut Registers)
{
	registers[vx] ^= registers[vy];
}

fn add_regs(vx: usize, vy: usize, registers: &mut Registers)
{
	let result = registers[vx].overflowing_add(registers[vy]);
	registers[vx] = result.0;
	registers[N_REGISTERS - 1] = result.1 as u8;
}

fn sub_regs(vx: usize, vy: usize, registers: &mut Registers)
{
	let result = registers[vx].overflowing_sub(registers[vy]);
	registers[vx] = result.0;
	registers[N_REGISTERS - 1] = result.1 as u8;
}

fn shr(vx: usize, registers: &mut Registers)
{
	let lsb = 0b0000_0001;
	registers[N_REGISTERS - 1] = registers[vx] & lsb;
	registers[vx] /= 2;
}

fn subn_regs(vx: usize, vy: usize, registers: &mut Registers)
{
	let result = registers[vy].overflowing_sub(registers[vx]);
	registers[vx] = result.0;
	registers[N_REGISTERS - 1] = result.1 as u8;
}

fn shl(vx: usize, registers: &mut Registers)
{
	let msb = 0b1000_0000;
	registers[N_REGISTERS - 1] = registers[vx] & msb;
	registers[vx] *= 2;
}

fn sne_regs(vx: usize, vy: usize, registers: &mut Registers,
	pc: &mut u16)
{
	if registers[vx] != registers[vy]
	{
		*pc += 1;
	}
}

fn ld_reg_index(address: u16, index_register: &mut u16)
{
	*index_register = address;
}

fn jp_v0(address: u16, pc: &mut u16,
	registers: &Registers)
{
	*pc = u16::from(registers[0]) + address;
}

fn rnd(vx: usize, value: u8, registers: &mut Registers)
{
	let mut rng = rand::thread_rng();
	let result: u8 = rng.gen::<u8>() & value;
	registers[vx] = result;
}

fn drw(vx: usize, vy: usize, bytes_number: u8,
	registers: &mut Registers, index_register: u16, memory: &Memory,
	frame_buffer: &mut FrameBuffer)
{
	for byte in 0..bytes_number
	{
		let sprite_index = memory[(index_register + u16::from(byte)) as usize] as usize;
		if sprite_index < 16
		{
			let sprite = SPRITES[sprite_index];
			for j in 0..sprite.len()
			{
				for i in 0..8
				{
					let x = N_FRAMEBUFFER_WIDTH % (i + registers[vx] as usize);
					let y = N_FRAMEBUFFER_HEIGHT % (j + registers[vy] as usize);

					let old_pixel = frame_buffer[y][x];
					let new_pixel = sprite[j] & 1 << (8 - i) > 0;

					frame_buffer[y][x] ^= new_pixel;

					if old_pixel && new_pixel
					{
						registers[0xF - 1] = 1;
					}
				}
			}
		}
	}
}

fn skp(vx: usize, key_pressed: u8,
	registers: &Registers, pc: &mut u16)
{
	if registers[vx] == key_pressed
	{
		*pc += 2;
	}
}

fn sknp(vx: usize, key_pressed: u8,
	registers: &Registers, pc: &mut u16)
{
	if registers[vx] != key_pressed
	{
		*pc += 2;
	}
}

fn ld_delay_to_reg(vx: usize, delay_timer: Arc<RwLock<u8>>, registers: &mut Registers)
{
	let delay_timer = delay_timer.read().unwrap();
	registers[vx] = *delay_timer;
}

fn ld_key(vx: usize, registers: &mut Registers,
	stdin: &mut io::Stdin, stdout: &io::Stdout)
{
	let mut buffer : [u8; 1] = [0; 1];
	stdout.lock().flush().unwrap();
	stdin.read_exact(&mut buffer).unwrap();
	registers[vx] = buffer[0];
}

fn ld_reg_to_delay(vx: usize, delay_timer: Arc<RwLock<u8>>, registers: &Registers)
{
	let mut delay_timer = delay_timer.write().unwrap();
	*delay_timer = registers[vx];
}

fn ld_reg_to_sound(vx: usize, sound_timer: Arc<RwLock<u8>>, registers: &Registers)
{
	let mut sound_timer = sound_timer.write().unwrap();
	*sound_timer = registers[vx];
}

fn add_reg_index(vx: usize, index_register: &mut u16, registers: &Registers)
{
	*index_register += u16::from(registers[vx]);
}

fn set_sprite(vx: usize, registers: &Registers, index_register: &mut u16)
{
	if registers[vx] <= 9 ||
		(registers[vx] >= 'A'.to_digit(10).unwrap() as u8 &&
		registers[vx] <= 'F'.to_digit(10).unwrap() as u8)
	{
		*index_register = u16::from(registers[vx]);
	}
}

fn store_bcd(vx: usize, registers: &Registers,
	index_register: &u16, memory: &mut Memory)
{
	let h = (registers[vx] / 100) & 0b0000_0111;
	let d = (registers[vx] / 10) & 0b0000_0111;
	let u = registers[vx] & 0b0000_0111;

	memory[*index_register as usize] = h;
	memory[(*index_register + 1) as usize] = d;
	memory[(*index_register + 2) as usize] = u;
}

fn get_op_code(memory: &Memory, pc: &u16) -> OpCode
{
	let mut op_code: OpCode = OpCode::default();
	op_code.code = memory[*pc as usize] >> 4;
	match op_code.code
	{
		0 =>
		{
			op_code.variant = Some(0b0000_1111 & memory[(*pc + 1) as usize])
		},

		1 | 2 | 0xA | 0xB =>
		{
			let mut address = u16::from((0b0000_1111 & memory[*pc as usize])) << 8;
			address |= u16::from(memory[(*pc + 1) as usize]);

			op_code.x = Some(address)
		},

		3 | 4 | 6 | 7=>
		{
			op_code.x = Some(u16::from(0b0000_1111 & memory[*pc as usize]));
			op_code.y = Some(u16::from(memory[(*pc + 1) as usize]))
		},

		5 | 8 | 9 =>
		{
			op_code.x = Some(u16::from(0b0000_1111 & memory[*pc as usize]));
			op_code.y = Some(u16::from(0b1111_0000 & memory[(*pc + 1) as usize]));
			op_code.variant = Some(0b0000_1111 & memory[(*pc + 1) as usize])
		},

		0xC =>
		{
			op_code.x = Some(u16::from(0b0000_1111 & memory[*pc as usize]));
			op_code.y = Some(u16::from(memory[(*pc + 1) as usize]))
		},

		0xE | 0xF =>
		{
			op_code.x = Some(u16::from(0b0000_1111 & memory[*pc as usize]));
			op_code.variant = Some(memory[(*pc + 1) as usize])
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
			let c = if !frame_buffer[i][j] { "▓" } else { " " };
			write!(stdout, "{}{}", termion::cursor::Goto(i as u16 + 1, j as u16 + 1), c).unwrap();
		}
	}
}

fn main()
{
	let mut index_register: u16 = 0b0;
	let mut registers: Registers = [0; N_REGISTERS];

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
		if op.x.is_some() && op.y.is_some() && op.variant.is_some()
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
				0x3 => se(x as usize, y as u8, &mut registers, &mut pc),
				0x4 => sne(x as usize, y as u8, &mut registers, &mut pc),
				0x5 => se_regs(x as usize, y as usize, &mut registers, &mut pc),
				0x6 => ld(x as usize, y as u8, &mut registers),
				0x7 => add(x as usize, y as u8, &mut registers),
				0x8 =>
				{
					match variant
					{
						0x0 => ld_regs(x as usize, y as usize, &mut registers),
						0x1 => or(x as usize, y as usize, &mut registers),
						0x2 => and(x as usize, y as usize, &mut registers),
						0x3 => xor(x as usize, y as usize, &mut registers),
						0x4 => add_regs(x as usize, y as usize, &mut registers),
						0x5 => sub_regs(x as usize, y as usize, &mut registers),
						0x6 => shr(x as usize, &mut registers),
						0x7 => subn_regs(x as usize, y as usize, &mut registers),
						0xE => shl(x as usize, &mut registers),

						_ =>
						{
							//TODO: raise error
						}
					}
				}
				0x9 => sne_regs(x as usize, y as usize, &mut registers, &mut pc),
				0xA => ld_reg_index(x, &mut pc),
				0xB => jp_v0(x, &mut pc, &registers),
				0xC => rnd(x as usize, y as u8, &mut registers),
				0xF =>
				{
					match variant
					{
						0x07 => ld_delay_to_reg(x as usize, delay_timer.clone(), &mut registers),
						0x15 => ld_reg_to_delay(x as usize, delay_timer.clone(), &registers),
						0x18 => ld_reg_to_sound(x as usize, sound_timer.clone(), &registers),
						0x1E => add_reg_index(x as usize, &mut index_register, &registers),
						

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
