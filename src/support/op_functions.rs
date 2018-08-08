use support::type_defs::*;
use support::get_key_from_keyboard;

extern crate rand;
use support::op_functions::rand::prelude::*;

extern crate sdl2;
use sdl2::EventPump;

pub fn cls(frame_buffer: &mut FrameBuffer) {
    for row in frame_buffer.iter_mut() {
        for p in row.iter_mut() {
            *p = false;
        }
    }
}

pub fn ret(
    program_counter: &mut u16,
    stack_pointer: &mut usize,
    stack: &Stack
) {
    *program_counter = stack[*stack_pointer];
    *stack_pointer -= 1;
}

pub fn jp_addr(address: u16, program_counter: &mut u16) {
    *program_counter = address;
}

pub fn call_addr(
    address: u16,
    stack_pointer: &mut usize,
    program_counter: &mut u16,
    stack: &mut Stack,
) {
    *stack_pointer += 1;
    stack[*stack_pointer] = *program_counter;
    *program_counter = address;
}

pub fn se(
    vx: usize,
    value: u8,
    registers: &mut Registers,
    program_counter: &mut u16
) {
    if registers[vx] == value {
        *program_counter += 2;
    }
}

pub fn sne(
    vx: usize,
    value: u8,
    registers: &mut Registers,
    program_counter: &mut u16
) {
    if registers[vx] != value {
        *program_counter += 2;
    }
}

pub fn se_regs(
    vx: usize,
    vy: usize,
    registers: &mut Registers,
    program_counter: &mut u16
) {
    if registers[vx] == registers[vy] {
        *program_counter += 2;
    }
}

pub fn ld(vx: usize, value: u8, registers: &mut Registers) {
    registers[vx] = value;
}

pub fn add(vx: usize, value: u8, registers: &mut Registers) {
    registers[vx] += value;
}

pub fn ld_regs(vx: usize, vy: usize, registers: &mut Registers) {
    registers[vx] = registers[vy];
}

pub fn or(vx: usize, vy: usize, registers: &mut Registers) {
    registers[vx] |= registers[vy];
}

pub fn and(vx: usize, vy: usize, registers: &mut Registers) {
    registers[vx] &= registers[vy];
}

pub fn xor(vx: usize, vy: usize, registers: &mut Registers) {
    registers[vx] ^= registers[vy];
}

pub fn add_regs(vx: usize, vy: usize, registers: &mut Registers) {
    let result = registers[vx].overflowing_add(registers[vy]);
    registers[vx] = result.0;
    registers[N_REGISTERS - 1] = result.1 as u8;
}

pub fn sub_regs(
    vx: usize,
    vy: usize,
    registers: &mut Registers
) {
    let result = registers[vx].overflowing_sub(registers[vy]);
    registers[vx] = result.0;
    registers[N_REGISTERS - 1] = result.1 as u8;
}

pub fn shr(vx: usize, registers: &mut Registers) {
    registers[N_REGISTERS - 1] = registers[vx] & 0b0000_0001;
    registers[vx] /= 2;
}

pub fn subn_regs(
    vx: usize,
    vy: usize,
    registers: &mut Registers
) {
    let result = registers[vy].overflowing_sub(registers[vx]);
    registers[vx] = result.0;
    registers[N_REGISTERS - 1] = result.1 as u8;
}

pub fn shl(
    vx: usize,
    registers: &mut Registers
) {
    registers[N_REGISTERS - 1] = registers[vx] & 0b1000_0000;
    registers[vx] *= 2;
}

pub fn sne_regs(
    vx: usize,
    vy: usize,
    registers: &mut Registers,
    program_counter: &mut u16
) {
    if registers[vx] != registers[vy] {
        *program_counter += 1;
    }
}

pub fn ld_reg_index(address: u16, index_register: &mut u16) {
    *index_register = address;
}

pub fn jp_v0(
    address: u16,
    program_counter: &mut u16,
    registers: &Registers
) {
    *program_counter = u16::from(registers[0]) + address;
}

pub fn rnd(
    vx: usize,
    value: u8,
    registers: &mut Registers
) {
    let mut rng = rand::thread_rng();
    let result: u8 = rng.gen::<u8>() & value;
    registers[vx] = result;
}

pub fn drw(
    vx: usize,
    vy: usize,
    bytes_number: u8,
    registers: &mut Registers,
    index_register: u16,
    memory: &Memory,
    frame_buffer: &mut FrameBuffer,
) {
    for i in 0..bytes_number {
        let byte = memory[(index_register + u16::from(i)) as usize];
        let y = registers[vy + usize::from(i)] as usize;
        for bit in 0..8 {
            let x = registers[vx + bit] as usize;
            let old_pixel = frame_buffer[x][y];
            let new_pixel = (byte & (1 << (8 - bit))) > 0;
            frame_buffer[x][y] ^= new_pixel;

            if new_pixel == old_pixel {
                registers[N_REGISTERS - 1] = 1;
            }
        }
    }
}

pub fn skp(
    vx: usize,
    key: Option<u8>,
    registers: &Registers,
    program_counter: &mut u16
) {
    match key {
        Some(key) => {
            if registers[vx] == key {
                *program_counter += 2;
            }
        }
        None => {}
    }
}

pub fn sknp(
    vx: usize,
    key: Option<u8>,
    registers: &Registers,
    program_counter: &mut u16
) {
    match key {
        Some(key) => {
            if registers[vx] != key {
                *program_counter += 2;
            }
        }
        None => {}
    }
}

pub fn ld_delay_to_reg(
    vx: usize,
    delay_timer: &mut u8,
    registers: &mut Registers
) {
    registers[vx] = *delay_timer;
}

pub fn ld_key(
    vx: usize,
    registers: &mut Registers,
    event_pump: &mut EventPump
) {
    let mut key: Option<u8> = None;
    while key.is_none() {
        key = get_key_from_keyboard(event_pump);
    }

    match key {
        None => print!("LD_KEY: Key is None out of the loop"),
        Some(k) => registers[vx] = k,
    }
}

pub fn ld_reg_to_delay(
    vx: usize,
    delay_timer: &mut u8,
    registers: &Registers
) {
    *delay_timer = registers[vx];
}

pub fn ld_reg_to_sound(
    vx: usize,
    sound_timer: &mut u8,
    registers: &Registers
) {
    *sound_timer = registers[vx];
}

pub fn add_reg_index(
    vx: usize,
    index_register: &mut u16,
    registers: &Registers
) {
    *index_register += u16::from(registers[vx]);
}

pub fn ld_sprite(
    vx: usize,
    registers: &Registers,
    index_register: &mut u16
) {
    if (registers[vx] >= 'A'.to_digit(10).unwrap() as u8
        && registers[vx] <= 'F'.to_digit(10).unwrap() as u8)
       || registers[vx] <= 9
    {
        *index_register = u16::from(registers[vx]);
    }
}

pub fn ld_bcd(
    vx: usize,
    registers: &Registers,
    index_register: &u16,
    memory: &mut Memory
) {
    let h = (registers[vx] / 100) & 0b0000_0111;
    let d = (registers[vx] / 10) & 0b0000_0111;
    let u = registers[vx] & 0b0000_0111;

    memory[*index_register as usize] = h;
    memory[(*index_register + 1) as usize] = d;
    memory[(*index_register + 2) as usize] = u;
}

pub fn ld_x_regs(
    vx: usize,
    registers: &mut Registers,
    index_register: &mut u16,
    memory: &Memory
) {
    for i in 0..=vx {
        registers[i] = memory[usize::from(*index_register) + i];
    }

    *index_register += (vx + 1) as u16;
}
