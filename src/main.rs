mod support;
use support::*;

use std::time::{Duration, Instant};
use std::thread;
use std::env;

extern crate sdl2;

fn main() {
    let mut cpu = Cpu::new();

	let args: Vec<String> = env::args().collect();
	if args.len() < 2 {
		println!("No valid ROM detected");
		std::process::exit(0);
	} else {
		cpu.load_rom(&args[1]);
	}

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut cpu_tick_countdown = MAIN_THREAD_MS;
    let mut devices_tick_countdown = SEC_THREAD_MS;
    loop {

        let thread_duration = Instant::now();

        if cpu_tick_countdown <= 0 {
            cpu.tick(&mut event_pump);
            cpu_tick_countdown = MAIN_THREAD_MS;
        } else {
            cpu_tick_countdown -= 1;
        }

        if devices_tick_countdown <= 0 {
            //TODO: call display and sound tick functions
            devices_tick_countdown = SEC_THREAD_MS;
        } else {
            devices_tick_countdown -= 1;
        }

        let thread_duration = thread_duration.
            elapsed().subsec_nanos() / 1_000_000;

        if thread_duration < 1 {
            thread::sleep(Duration::from_millis(u64::from(1 - thread_duration)));
        }
    }
}
