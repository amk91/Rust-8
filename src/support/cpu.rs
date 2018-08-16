use support::type_defs::*;

extern crate rand;
use support::cpu::rand::Rng;

use std::fs::File;
use std::io::Read;

pub struct Cpu {
    index_register: u16,
    registers: Registers,

    stack_pointer: usize,
    stack: Stack,

    program_counter: u16,
    jump: bool,
    memory: Memory,

    frame_buffer: FrameBuffer,

    delay_timer: u8,
    sound_timer: u8,

    wait_for_key: bool,
    key_register: usize,
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
			jump: false,
			memory: memory,

			frame_buffer: [[false; N_FRAMEBUFFER_HEIGHT]; N_FRAMEBUFFER_WIDTH],

			delay_timer: 0,
			sound_timer: 0,

			wait_for_key: false,
			key_register: 0,
		}
	}

	fn get_op_code(&self) -> OpCode {
	    let mut op_code = OpCode::default();
	    op_code.code = self.memory[self.program_counter as usize] >> 4;
	    match op_code.code {
	    	0x0 => {
	    		op_code.variant = Some(self.memory[(self.program_counter + 1) as usize]);
	    	},
	        0x1 | 0x2 | 0xA | 0xB => {
	            let mut address = u16::from(0x0F & self.memory[self.program_counter as usize]);
	            address = address << 8;
	            address |= u16::from(self.memory[(self.program_counter + 1) as usize]);

	            op_code.x = Some(address);
	        },
	        0x3 | 0x4 | 0x6 | 0x7 | 0xC => {
	            op_code.x = Some(u16::from(0x0F & self.memory[self.program_counter as usize]));
	            op_code.y = Some(u16::from(self.memory[(self.program_counter + 1) as usize]));
	        },
	        0x5 | 0x8 | 0x9 | 0xD => {
	            op_code.x = Some(u16::from(0x0F & self.memory[self.program_counter as usize]));
	            op_code.y = Some(u16::from(self.memory[self.program_counter as usize + 1] >> 4));
	            op_code.variant = Some(0x0F &self.memory[(self.program_counter + 1) as usize]);
	        },
	        0xE | 0xF => {
	            op_code.x = Some(u16::from(0x0F & self.memory[self.program_counter as usize]));
	            op_code.variant = Some(self.memory[(self.program_counter + 1) as usize]);
	        },
	        _ => {
	        	panic!("Unable to parse OpCode {}", op_code.code);
	        },
	    };

	    op_code
	}

	pub fn get_frame_buffer(&self) -> FrameBuffer {
		self.frame_buffer
	}

	pub fn decrease_delay_timer(&mut self) {
		self.delay_timer -= 1;
	}

	pub fn is_delay_timer_zero(&self) -> bool {
		self.delay_timer == 0
	}

	pub fn decrease_sound_timer(&mut self) {
		self.sound_timer -= 1;
	}

	pub fn is_sound_timer_zero(&self) -> bool {
		self.sound_timer == 0
	}

	pub fn load_rom(&mut self, filepath: &String) {
	    let mut file = File::open(filepath).expect("File not found");
	    let mut buffer = [0u8; N_MEMORY - 0x200];

	    if let Ok(_) = file.read(&mut buffer) {
		    for (i, &byte) in buffer.iter().enumerate() {
		    	if i + 0x200 < N_MEMORY {
		    		self.memory[i + 0x200] = byte;
		    	} else {
		    	    break;
		    	}
		    }
	    }
	}

	pub fn tick(&mut self, key: Option<u8>) {
		if self.wait_for_key && key.is_some() {
			self.registers[self.key_register] = key.unwrap();
			self.wait_for_key = false;
			self.key_register = 0;
		}

		if !self.wait_for_key
		{        
			let op = self.get_op_code();
	    	self.jump = false;
	        match op {
	        	OpCode { code: 0x0, variant: Some(0xE0), .. } => self.cls(),
	        	OpCode { code: 0x0, variant: Some(0xEE), .. } => self.ret(),
	    		OpCode { code: 0x1, x: Some(x), .. } => self.jp_addr(x),
				OpCode { code: 0x2, x: Some(x), .. } => self.call_addr(x),
				OpCode { code: 0x3, x: Some(x), y: Some(y), .. } => {
					self.se(x as usize, y as u8);
				},
				OpCode { code: 0x4, x: Some(x), y: Some(y), .. } => {
					self.sne(x as usize, y as u8);
				},
				OpCode { code: 0x5, x: Some(x), y: Some(y), .. } => {
					self.se_regs(x as usize, y as usize);
				},
				OpCode { code: 0x6, x: Some(x), y: Some(y), .. } => {
					self.ld(x as usize, y as u8);
				},
	            OpCode { code: 0x7, x: Some(x), y: Some(y), .. } => {
	            	self.add(x as usize, y as u8);
	            },
	            OpCode { code: 0x8, x: Some(x), y: Some(y), variant: Some(0x0) } => {
	            	self.ld_regs(x as usize, y as usize);
	            },
	            OpCode { code: 0x8, x: Some(x), y: Some(y), variant: Some(0x1) } => {
	            	self.or(x as usize, y as usize);
	            },
	            OpCode { code: 0x8, x: Some(x), y: Some(y), variant: Some(0x2) } => {
	            	self.and(x as usize, y as usize);
	            },
	            OpCode { code: 0x8, x: Some(x), y: Some(y), variant: Some(0x3) } => {
	            	self.xor(x as usize, y as usize);
	            },
	            OpCode { code: 0x8, x: Some(x), y: Some(y), variant: Some(0x4) } => {
	            	self.add_regs(x as usize, y as usize);
	            },
	            OpCode { code: 0x8, x: Some(x), y: Some(y), variant: Some(0x5) } => {
	            	self.sub_regs(x as usize, y as usize);
	            },
	            OpCode { code: 0x8, x: Some(x), variant: Some(0x6), .. } => {
	            	self.shr(x as usize);
	            },
	            OpCode { code: 0x8, x: Some(x), y: Some(y), variant: Some(0x7) } => {
	            	self.subn_regs(x as usize, y as usize);
	            },
	            OpCode { code: 0x8, x: Some(x), variant: Some(0xE), .. } => {
	            	self.shl(x as usize);
	            },
	            OpCode { code: 0x9, x: Some(x), y: Some(y), .. } => {
	            	self.sne_regs(x as usize, y as usize);
	            },
	            OpCode { code: 0xA, x: Some(x), .. } => self.ld_reg_index(x),
	            OpCode { code: 0xB, x: Some(x), .. } => self.jp_v0(x as u16),
	            OpCode { code: 0xC, x: Some(x), y: Some(y), .. } => {
	            	self.rnd(x as usize, y as u8);
	            },
	            OpCode { code: 0xD, x: Some(x), y: Some(y), variant: Some(variant) } => {
	            	self.drw(x as usize, y as usize, variant);
	            },
	            OpCode { code: 0xE, x: Some(x), variant: Some(0x9E), .. } => {
	            	self.skp(x as usize, key);
	            },
	            OpCode { code: 0xE, x: Some(x), variant: Some(0xA1), .. } => {
	            	self.sknp(x as usize, key);
	            },
	            OpCode { code: 0xF, x: Some(x), variant: Some(0x7), .. } => {
	            	self.ld_delay_to_reg(x as usize);
	            },
	            OpCode { code: 0xF, x: Some(x),	 variant: Some(0x0A), .. } => {
	            	self.ld_key(x as usize);
	            },
	            OpCode { code: 0xF, x: Some(x), variant: Some(0x15), .. } => {
	            	self.ld_reg_to_delay(x as usize);
	            },
	            OpCode { code: 0xF, x: Some(x), variant: Some(0x18), .. } => {
	            	self.ld_reg_to_sound(x as usize);
	            },
	            OpCode { code: 0xF, x: Some(x), variant: Some(0x1E), .. } => {
	            	self.add_reg_index(x as usize);
	            },
	            OpCode { code: 0xF, x: Some(x), variant: Some(0x29), .. } => {
	            	self.ld_sprite(x as usize);
	            },
	            OpCode { code: 0xF, x: Some(x), variant: Some(0x33), .. } => {
	            	self.ld_bcd(x as usize);
	            },
	            OpCode { code: 0xF, x: Some(x), variant: Some(0x55), .. } => {
	            	self.ld_regs_to_mem(x as usize);
	            },
	            OpCode { code: 0xF, x: Some(x), variant: Some(0x65), .. } => {
	            	self.ld_mem_to_regs(x as usize);
	            },
	            _ => { },
	        };

	        if !self.jump {
	        	self.program_counter += 2;
	        }
    	}
	}

	fn cls(&mut self) {
	    for row in self.frame_buffer.iter_mut() {
	        for p in row.iter_mut() {
	            *p = false;
	        }
	    }
	}

	fn ret(&mut self) {
	    self.stack_pointer -= 1;
	    self.program_counter = self.stack[self.stack_pointer];

	    self.jump = true;
	}

	fn jp_addr(&mut self, address: u16) {
	    self.program_counter = address;

	    self.jump = true;
	}

	fn call_addr(&mut self, address: u16) {
	    self.stack[self.stack_pointer] = self.program_counter + 2;
	    self.stack_pointer += 1;
	    self.program_counter = address;
	    
	    self.jump = true;
	}

	fn se(&mut self, vx: usize, value: u8) {
	    if self.registers[vx] == value {
	        self.program_counter += 2;
	    }
	}

	fn sne(&mut self, vx: usize, value: u8) {
	    if self.registers[vx] != value {
	        self.program_counter += 2;
	    }
	}

	fn se_regs(&mut self, vx: usize, vy: usize) {
	    if self.registers[vx] == self.registers[vy] {
	        self.program_counter += 2;
	    }
	}

	fn ld(&mut self, vx: usize, value: u8) {
	    self.registers[vx] = value;
	}

	fn add(&mut self, vx: usize, value: u8) {
	    self.registers[vx] = (self.registers[vx] as u16 + value as u16) as u8;
	}

	fn ld_regs(&mut self, vx: usize, vy: usize) {
	    self.registers[vx] = self.registers[vy];
	}

	fn or(&mut self, vx: usize, vy: usize) {
	    self.registers[vx] |= self.registers[vy];
	}

	fn and(&mut self, vx: usize, vy: usize) {
	    self.registers[vx] &= self.registers[vy];
	}

	fn xor(&mut self, vx: usize, vy: usize) {
	    self.registers[vx] ^= self.registers[vy];
	}

	fn add_regs(&mut self, vx: usize, vy: usize) {
	    let result = self.registers[vx].overflowing_add(self.registers[vy]);
	    self.registers[vx] = result.0;
	    self.registers[N_REGISTERS - 1] = result.1 as u8;
	}

	fn sub_regs(&mut self, vx: usize, vy: usize) {
	    let result = self.registers[vx].overflowing_sub(self.registers[vy]);
	    self.registers[vx] = result.0;
	    self.registers[N_REGISTERS - 1] = result.1 as u8;
	}

	fn shr(&mut self, vx: usize) {
	    self.registers[0xF] = self.registers[vx] & 0b0000_0001;
	    self.registers[vx] >>= 1;
	}

	fn subn_regs(&mut self, vx: usize, vy: usize) {
	    let result = self.registers[vy].overflowing_sub(self.registers[vx]);
	    self.registers[vx] = result.0;
	    self.registers[0x0F] = result.1 as u8;
	}

	fn shl(&mut self, vx: usize) {
	    self.registers[0xF] = self.registers[vx] & 0b1000_0000;
	    self.registers[vx] <<= 1;
	}

	fn sne_regs(&mut self, vx: usize, vy: usize) {
	    if self.registers[vx] != self.registers[vy] {
	        self.program_counter += 2;
	    }
	}

	fn ld_reg_index(&mut self, address: u16) {
	    self.index_register = address;
	}

	fn jp_v0(&mut self, address: u16) {
	    self.program_counter = u16::from(self.registers[0]) + address;
	    self.jump = true;
	}

	fn rnd(&mut self, vx: usize, value: u8) {
	    let mut rng = rand::thread_rng();
	    self.registers[vx] = rng.gen::<u8>() & value;
	}

	fn drw(&mut self, vx: usize, vy: usize, bytes_number: u8) {
		self.registers[0xF] = 0;
		for row in 0..bytes_number {
			let y = usize::from((self.registers[vy] + row) % N_FRAMEBUFFER_HEIGHT as u8);
			let byte = self.memory[self.index_register as usize + row as usize];
			for bit in 0..8 {
				let x = usize::from((self.registers[vx] + bit) % N_FRAMEBUFFER_WIDTH as u8);
				let new_pixel = (byte >> (7 - bit) & 1) == 1;
				self.registers[0xF] |= u8::from(new_pixel & self.frame_buffer[x][y]);
				self.frame_buffer[x][y] ^= new_pixel;
			}
		}
	}

	fn skp(&mut self, vx: usize, key: Option<u8>) {
	    match key {
	        Some(key) => {
	            if self.registers[vx] == key {
	                self.program_counter += 2;
	            }
	        }
	        None => { }
	    }
	}

	fn sknp(&mut self, vx: usize, key: Option<u8>) {
	    match key {
	        Some(key) => {
	            if self.registers[vx] != key {
	                self.program_counter += 2;
	            }
	        }
	        None => { }
	    }
	}

	fn ld_delay_to_reg(&mut self, vx: usize) {
	    self.registers[vx] = self.delay_timer;
	}

	fn ld_key(&mut self, vx: usize) {
		self.wait_for_key = true;
		self.key_register = vx;
	}

	fn ld_reg_to_delay(&mut self, vx: usize) {
	    self.delay_timer = self.registers[vx];
	}

	fn ld_reg_to_sound(&mut self, vx: usize) {
	    self.sound_timer = self.registers[vx];
	}

	fn add_reg_index(&mut self, vx: usize) {
	    self.index_register += u16::from(self.registers[vx]);
	}

	fn ld_sprite(&mut self, vx: usize) {
	    self.index_register = u16::from(self.registers[vx] * 5);
	}

	fn ld_bcd(&mut self, vx: usize) {
	    let h = self.registers[vx] / 100;
	    let d = (self.registers[vx] % 100) / 10;
	    let u = self.registers[vx] % 10;

	    self.memory[self.index_register as usize] = h;
	    self.memory[(self.index_register + 1) as usize] = d;
	    self.memory[(self.index_register + 2) as usize] = u;
	}

	fn ld_regs_to_mem(&mut self, vx: usize) {
		for i in 0..vx + 1 {
			self.memory[usize::from(self.index_register) + i] = self.registers[i];
		}
	}

	fn ld_mem_to_regs(&mut self, vx: usize) {
	    for i in 0..vx + 1 {
	        self.registers[i] = self.memory[usize::from(self.index_register) + i];
	    }
	}
}
