use support::type_defs::*;
use sdl2::EventPump;

use support::op_functions::*;
use support::get_key_from_keyboard;

use std::fs::File;
use std::io::Read;

pub struct Cpu {
    index_register: u16,
    registers: Registers,

    stack_pointer: usize,
    stack: Stack,

    program_counter: u16,
    memory: Memory,

    frame_buffer: FrameBuffer,

    delay_timer: u8,
    sound_timer: u8,
}

impl Cpu {
	pub fn new() -> Cpu {
		let mut memory: Memory = [0; N_MEMORY];
	    let mut index = 0;
	    for i in 0..N_SPRITES {
	        for j in 0..5 {
	            memory[index] = SPRITES[i][j];
	            index += 1;
	        }
	    }

		Cpu {
			index_register: 0b0,
			registers: [0; N_REGISTERS],

			stack_pointer: 0,
			stack: [0; N_STACK],

			program_counter: 0x200,
			memory: memory,

			frame_buffer: [[false; N_FRAMEBUFFER_HEIGHT]; N_FRAMEBUFFER_WIDTH],

			delay_timer: 0,
			sound_timer: 0,
		}
	}

	fn get_op_code(&self) -> OpCode {
	    let mut op_code: OpCode = OpCode::default();
	    op_code.code = self.memory[self.program_counter as usize] >> 4;
	    match op_code.code {
	        0x0 => op_code.variant = Some(0b0000_1111 & self.memory[(self.program_counter + 1) as usize]),

	        0x1 | 0x2 | 0xA | 0xB => {
	            let mut address = u16::from(0b0000_1111 & self.memory[self.program_counter as usize]) << 8;
	            address |= u16::from(self.memory[(self.program_counter + 1) as usize]);

	            op_code.x = Some(address)
	        },

	        0x3 | 0x4 | 0x6 | 0x7 => {
	            op_code.x = Some(u16::from(0b0000_1111 & self.memory[self.program_counter as usize]));
	            op_code.y = Some(u16::from(self.memory[(self.program_counter + 1) as usize]))
	        },

	        0x5 | 0x8 | 0x9 | 0xD => {
	            op_code.x = Some(u16::from(0b0000_1111 & self.memory[self.program_counter as usize]));
	            op_code.y = Some(u16::from(0b1111_0000 & self.memory[(self.program_counter + 1) as usize],
	            ));
	            op_code.variant = Some(0b0000_1111 & self.memory[(self.program_counter + 1) as usize])
	        },

	        0xC => {
	            op_code.x = Some(u16::from(0b0000_1111 & self.memory[self.program_counter as usize]));
	            op_code.y = Some(u16::from(self.memory[(self.program_counter + 1) as usize]))
	        },

	        0xE | 0xF => {
	            op_code.x = Some(u16::from(0b0000_1111 & self.memory[self.program_counter as usize]));
	            op_code.variant = Some(self.memory[(self.program_counter + 1) as usize])
	        },

	        _ => {
	        	panic!("Unable to parse OpCode {}", op_code.code);
	        },
	    };

	    op_code
	}

	pub fn get_delay_timer(&self) -> u8 {
		self.delay_timer
	}

	pub fn get_sound_timer(&self) -> u8 {
		self.sound_timer
	}

	pub fn load_rom(&mut self, filepath: &String) {
	    let mut file = File::open(filepath).expect("File not found");
	    let mut buffer: Vec<u8> = Vec::new();
	    file.read_to_end(&mut buffer)
	        .expect("Unable to read buffer");

	    let mut index = 0x200;
	    for byte in buffer.iter() {
	        if index < 4096 {
	            self.memory[index] = *byte;
	            index += 1;
	        }
	    }
	}

	pub fn tick(&mut self, event_pump: &mut EventPump) {
        let key = get_key_from_keyboard(event_pump);
        let op = self.get_op_code();

        if cfg!(debug_assertion) {
	        println!("Next op code to be executed (with key {}:", if key.is_some() { key.unwrap() } else { 0 });
	        println!("code: {:x}, x: {:x}, y: {:x}, var: {:x}",
	            op.code,
	            if op.x.is_some() { op.x.unwrap() } else { 0 },
	            if op.y.is_some() { op.y.unwrap() } else { 0 },
	            if op.variant.is_some() { op.variant.unwrap() } else { 0 }
	        );

            let mut input = String::new();
            match ::std::io::stdin().read_line(&mut input) {
                Ok(_) => {
                    if input.trim() == "q" {
                        ::std::process::exit(0);
                    }
                },
                _ => {},
        	};

    	}

        match op {
        	OpCode{ code: 0x0, variant: Some(0x0), .. } => cls(
        		&mut self.frame_buffer
        	),
        	OpCode{ code: 0x0, variant: Some(0xE), .. } => ret(
        		&mut self.program_counter,
        		&mut self.stack_pointer,
        		&self.stack
    		),
    		OpCode { code: 0x1, .. } => jp_addr(
    			op.x.unwrap(),
    			&mut self.program_counter
			),
			OpCode { code: 0x2, .. } => call_addr(
				op.x.unwrap(),
				&mut self.stack_pointer,
				&mut self.program_counter,
				&mut self.stack
			),
			OpCode { code: 0x3, .. } => se(
				op.x.unwrap() as usize,
				op.y.unwrap() as u8,
				&mut self.registers,
				&mut self.program_counter
			),
			OpCode { code: 0x4, .. } => sne(
				op.x.unwrap() as usize,
				op.y.unwrap() as u8,
				&mut self.registers,
				&mut self.program_counter
			),
			OpCode { code: 0x5, .. } => se_regs(
				op.x.unwrap() as usize,
				op.y.unwrap() as usize,
				&mut self.registers,
				&mut self.program_counter
			),
			OpCode { code: 0x6, .. } => ld(
                op.x.unwrap() as usize,
                op.y.unwrap() as u8,
                &mut self.registers
            ),
            OpCode { code: 0x7, .. } => add(
                op.x.unwrap() as usize,
                op.y.unwrap() as u8,
                &mut self.registers
            ),
            OpCode { code: 0x8, variant: Some(0x0), .. } => ld_regs(
            	op.x.unwrap() as usize,
            	op.y.unwrap() as usize,
            	&mut self.registers
            ),
            OpCode { code: 0x8, variant: Some(0x1), .. } => or(
            	op.x.unwrap() as usize,
            	op.y.unwrap() as usize,
            	&mut self.registers
            ),
            OpCode { code: 0x8, variant: Some(0x2), .. } => and(
                op.x.unwrap() as usize,
                op.y.unwrap() as usize,
                &mut self.registers,
            ),
            OpCode { code: 0x8, variant: Some(0x3), .. } => xor(
            	op.x.unwrap() as usize,
            	op.y.unwrap() as usize,
            	&mut self.registers
            ),
            OpCode { code: 0x8, variant: Some(0x4), .. } => add_regs(
            	op.x.unwrap() as usize,
            	op.y.unwrap() as usize,
            	&mut self.registers
            ),
            OpCode { code: 0x8, variant: Some(0x5), .. } => sub_regs(
            	op.x.unwrap() as usize,
            	op.y.unwrap() as usize,
            	&mut self.registers
            ),
            OpCode { code: 0x8, variant: Some(0x6), .. } => shr(
            	op.x.unwrap() as usize,
            	&mut self.registers
            ),
            OpCode { code: 0x8, variant: Some(0x7), .. } => subn_regs(
            	op.x.unwrap() as usize,
            	op.y.unwrap() as usize,
            	&mut self.registers
            ),
            OpCode { code: 0x8, variant: Some(0xE), .. } => shl(
            	op.x.unwrap() as usize,
            	&mut self.registers
            ),
            OpCode { code: 0x9, .. } => sne_regs(
                op.x.unwrap() as usize,
                op.y.unwrap() as usize,
                &mut self.registers,
                &mut self.program_counter,
            ),
            OpCode { code: 0xA, .. } => ld_reg_index(
	            op.x.unwrap(),
	            &mut self.program_counter
            ),
            OpCode { code: 0xB, .. } => jp_v0(
            	op.x.unwrap() as u16,
            	&mut self.program_counter,
            	&self.registers
            ),
            OpCode { code: 0xC, .. } => rnd(
            	op.x.unwrap() as usize,
            	op.y.unwrap() as u8,
            	&mut self.registers
            ),
            OpCode { code: 0xD, .. } => drw(
                op.x.unwrap() as usize,
                op.y.unwrap() as usize,
                op.variant.unwrap(),
                &mut self.registers,
                self.index_register,
                &self.memory,
                &mut self.frame_buffer
            ),
            OpCode { code: 0xE, variant: Some(0x9E), .. } => skp(
            	op.x.unwrap() as usize,
            	key,
            	&self.registers,
            	&mut self.program_counter
            ),
            OpCode { code: 0xE, variant: Some(0xA1), .. } => sknp(
            	op.x.unwrap() as usize,
            	key,
            	&self.registers,
            	&mut self.program_counter
            ),
            OpCode { code: 0xF, variant: Some(0x7), .. } => ld_delay_to_reg(
            	op.x.unwrap() as usize,
            	&mut self.delay_timer,
            	&mut self.registers
            ),
            OpCode { code: 0xF, variant: Some(0x0A), .. } => ld_key(
            	op.x.unwrap() as usize,
            	&mut self.registers,
            	event_pump
            ),
            OpCode { code: 0xF, variant: Some(0x15), .. } => ld_reg_to_delay(
            	op.x.unwrap() as usize,
            	&mut self.delay_timer,
            	&self.registers
            ),
            OpCode { code: 0xF, variant: Some(0x18), .. } => ld_reg_to_sound(
            	op.x.unwrap() as usize,
            	&mut self.sound_timer,
            	&mut self.registers
        	),
            OpCode { code: 0xF, variant: Some(0x1E), .. } => add_reg_index(
            	op.x.unwrap() as usize,
            	&mut self.index_register,
            	&self.registers
            ),
            OpCode { code: 0xF, variant: Some(0x29), .. } => ld_sprite(
            	op.x.unwrap() as usize,
            	&self.registers,
            	&mut self.index_register
            ),
            OpCode { code: 0xF, variant: Some(0x55), .. } => ld_bcd(
            	op.x.unwrap() as usize,
            	&self.registers,
            	&self.index_register,
            	&mut self.memory
            ),
            OpCode { code: 0xF, variant: Some(0x65), .. } => ld_x_regs(
            	op.x.unwrap() as usize,
            	&mut self.registers,
            	&mut self.index_register,
            	&self.memory
            ),
            _ => panic!("Unable to parse op {}:{}", op.code, op.variant.unwrap()),
        };

        self.program_counter += 2;
	}
}
