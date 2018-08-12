pub const N_REGISTERS: usize = 16;
pub const N_STACK: usize = 32;
pub const N_FRAMEBUFFER_WIDTH: usize = 64;
pub const N_FRAMEBUFFER_HEIGHT: usize = 32;
pub const N_MEMORY: usize = 4096;
pub const N_SPRITES: usize = 16;

pub const THREAD_SLEEP_NS: u64 = 2_000_000;
pub const MAIN_THREAD_NS: u64 = 2_000_000;
pub const SEC_THREAD_NS: u64 = 17_000_000;

pub type Registers = [u8; N_REGISTERS];
pub type Stack = [u16; N_STACK];
pub type FrameBuffer = [[bool; N_FRAMEBUFFER_HEIGHT]; N_FRAMEBUFFER_WIDTH];
pub type Memory = [u8; N_MEMORY];

#[derive(Default, Debug)]
pub struct OpCode {
    pub code: u8,
    pub x: Option<u16>,
    pub y: Option<u16>,
    pub variant: Option<u8>,
}

pub type Sprite = [u8; 5];

pub static SPRITES: [Sprite; N_SPRITES] = [
    [0xF0, 0x90, 0x90, 0x90, 0xF0],
    [0x20, 0x60, 0x20, 0x20, 0x70],
    [0xF0, 0x10, 0xF0, 0x80, 0xF0],
    [0xF0, 0x10, 0xF0, 0x10, 0xF0],
    [0x90, 0x90, 0xF0, 0x10, 0x10],
    [0xF0, 0x80, 0xF0, 0x10, 0xF0],
    [0xF0, 0x80, 0xF0, 0x90, 0xF0],
    [0xF0, 0x10, 0x20, 0x40, 0x40],
    [0xF0, 0x90, 0xF0, 0x90, 0xF0],
    [0xF0, 0x90, 0xF0, 0x10, 0xF0],
    [0xF0, 0x90, 0xF0, 0x90, 0x90],
    [0xE0, 0x90, 0xE0, 0x90, 0xE0],
    [0xF0, 0x80, 0x80, 0x80, 0xF0],
    [0xE0, 0x90, 0x90, 0x90, 0xE0],
    [0xF0, 0x80, 0xF0, 0x80, 0xF0],
    [0xF0, 0x80, 0xF0, 0x80, 0x80],
];
