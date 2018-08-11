mod support;
use support::*;

use std::time::{Duration, Instant};
use std::thread;
use std::env;

extern crate sdl2;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut cpu = Cpu::new();
    let mut display = Display::new(640, 320, &sdl_context);

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("No valid ROM detected");
        std::process::exit(0);
    } else {
        cpu.load_rom(&args[1]);
    }

    let mut cpu_tick_countdown = MAIN_THREAD_MS;
    let mut devices_tick_countdown = SEC_THREAD_MS;
    'running: loop {
        let thread_duration = Instant::now();

        if cpu_tick_countdown <= 0 {
            cpu.tick(&mut event_pump);
            cpu_tick_countdown = MAIN_THREAD_MS;
        } else {
            cpu_tick_countdown -= 1;
        }

        if devices_tick_countdown <= 0 {
            display.draw(cpu.get_frame_buffer());

            if !cpu.is_delay_timer_zero() {
                cpu.decrease_delay_timer();
            }

            if !cpu.is_sound_timer_zero() {
                cpu.decrease_sound_timer();
                //TODO: sound the buzz
            }

            devices_tick_countdown = SEC_THREAD_MS;
        } else {
            devices_tick_countdown -= 1;
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                _ => { },
            }
        }

        let thread_duration = thread_duration.
            elapsed().subsec_nanos() / 1_000_000;

        if thread_duration < 1 {
            thread::sleep(Duration::from_millis(u64::from(1 - thread_duration)));
        }
    }
}
