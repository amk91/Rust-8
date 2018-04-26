use std::{thread, time};

extern crate rand;
use rand::{Rng, thread_rng};

extern crate termion;
use termion::raw::IntoRawMode;
use std::io::{Write, stdout, stdin};

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

//TODO: define SKP and SKPN

fn ld_delay_to_reg(vx: usize, delay_timer: &u8, V: &mut Registers)
{
	V[0] = *delay_timer;
}

//TODO: define LD_KEY

fn ld_reg_to_delay(vx: usize, delay_timer: &mut u8, V: &Registers)
{
	*delay_timer = V[vx];
}

fn ld_reg_to_sound(vx: usize, sound_timer: &mut u8, V: &Registers)
{
	*sound_timer = V[vx];
}

fn add_reg_index(vx: usize, I: &mut u16, V: &Registers)
{
	*I += V[vx] as u16;
}

fn get_op_code(memory: &Memory, pc: &mut u16) -> OpCode
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

	*pc += 2;
	op_code
}



fn main()
{
	let mut I: u16 = 0b0;
	let mut V: Registers = [0; N_REGISTERS];

	let mut sp: u8 = 0b0;
	let mut stack: Stack = [0; N_STACK];

	let mut frame_buffer: FrameBuffer = [[false; N_FRAMEBUFFER_HEIGHT]; N_FRAMEBUFFER_WIDTH];
	let mut memory: Memory = [0; N_MEMORY];

	let mut delay_timer: u8 = 0;
	let mut sound_timer: u8 = 0;
	let mut pc: u16 = 0x200;

	let cycle_time = time::Duration::from_millis(16);

	let stdout = stdout();
    let mut stdout = stdout.lock().into_raw_mode().unwrap();
    let stdin = stdin();
	let stdin = stdin.lock();

	write!(stdout, "{}", termion::clear::All);

	for i in 0..frame_buffer.len()
	{
		for j in 0..frame_buffer[i].len()
		{
			write!(stdout, "{}▓", termion::cursor::Goto(i as u16 + 1, j as u16 + 1)).unwrap();
		}
	}

	/*loop
	{
		let op = get_op_code(&memory, &mut pc);
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
						0x07 => ld_delay_to_reg(x as usize, &delay_timer, &mut V),
						0x15 => ld_reg_to_delay(x as usize, &mut delay_timer, &V),
						0x18 => ld_reg_to_sound(x as usize, &mut sound_timer, &V),
						0x1E => add_reg_index(x as usize, &mut I, &V),
						//TODO: call Fx29


						_ =>
						{

						}
					}
				}

				_ =>
				{

				},
			};
		}
	}*/
}
