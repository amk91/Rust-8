mod support;
use support::*;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::thread;
use std::env;

extern crate sdl2;

fn main() {
    let mut index_register: u16 = 0b0;
    let mut registers: Registers = [0; N_REGISTERS];

    let mut stack_pointer: usize = 0;
    let mut stack: Stack = [0; N_STACK];

    let frame_buffer: FrameBuffer = [[false; N_FRAMEBUFFER_HEIGHT]; N_FRAMEBUFFER_WIDTH];
    let frame_buffer = Arc::new(Mutex::new(frame_buffer));
    let mut memory: Memory = [0; N_MEMORY];

    let mut program_counter: u16 = 0x200;
    let mut index = 0;
    for i in 0..N_SPRITES {
        for j in 0..5 {
            memory[index] = SPRITES[i][j];
            index += 1;
        }
    }

	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		println!("No valid ROM detected");
		std::process::exit(0);
	} else {
		load_rom(&args[1], &mut memory);
	}

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    loop {
        let thread_duration = Instant::now();

        let key = get_key_from_keyboard(&mut event_pump);
        let op = get_op_code(&memory, &program_counter);

        println!("Next op code to be executed (with key {}:", if key.is_some() { key.unwrap() } else { 0 });
        println!("code: {:x}, x: {:x}, y: {:x}, var: {:x}",
            op.code,
            if op.x.is_some() { op.x.unwrap() } else { 0 },
            if op.y.is_some() { op.y.unwrap() } else { 0 },
            if op.variant.is_some() { op.variant.unwrap() } else { 0 }
        );
        
        {
            let mut input = String::new();
            match std::io::stdin().read_line(&mut input) {
                Ok(_) => {
                    if input.trim() == "q" {
                        std::process::exit(0);
                    }
                },
                _ => {},
            }
        }

        match op.code {
            0x0 => match op.variant.unwrap() {
                0x0 => cls(Arc::clone(&frame_buffer)),
                0xE => ret(
                    &mut program_counter,
                    &mut stack_pointer,
                    &stack
                    ),
                _ => {
                    panic!("Unable to parse code {}", op.variant.unwrap());
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
            0x6 => ld(
                op.x.unwrap() as usize,
                op.y.unwrap() as u8,
                &mut registers
                ),
            0x7 => add(
                op.x.unwrap() as usize,
                op.y.unwrap() as u8,
                &mut registers
                ),
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
                0x6 => shr(
                    op.x.unwrap() as usize,
                    &mut registers
                    ),
                0x7 => subn_regs(
                    op.x.unwrap() as usize,
                    op.y.unwrap() as usize,
                    &mut registers,
                ),
                0xE => shl(
                    op.x.unwrap() as usize,
                    &mut registers
                    ),
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
            0xA => ld_reg_index(
                op.x.unwrap(),
                &mut program_counter
                ),
            0xB => jp_v0(
                op.x.unwrap(),
                &mut program_counter,
                &registers
                ),
            0xC => rnd(
                op.x.unwrap() as usize,
                op.y.unwrap() as u8,
                &mut registers
                ),
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
                0x0A => ld_key(
                    op.x.unwrap() as usize,
                    &mut registers,
                    &mut event_pump
                    ),
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
                0x1E => add_reg_index(
                    op.x.unwrap() as usize,
                    &mut index_register,
                    &registers
                    ),
                0x29 => ld_sprite(
                    op.x.unwrap() as usize,
                    &registers,
                    &mut index_register
                    ),
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

        let thread_duration = thread_duration.
            elapsed().subsec_nanos() / 1_000_000;
        if thread_duration < MAIN_THREAD_MS {
            let thread_duration: u64 = (MAIN_THREAD_MS - thread_duration).
                into();
            thread::sleep(Duration::from_millis(thread_duration));
        }
    }
}
