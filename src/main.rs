mod support;
use support::*;

use std::env;

extern crate sdl2;

extern crate adi_clock;
use adi_clock::{ Timer, Clock };

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

    let mut cpu_clock = Clock::new();
    let mut devices_clock = Clock::new();

    loop {
        let key = get_key_from_keyboard(&mut event_pump);
        
        if cpu_clock.since() >= 0.002 {
            println!("cpu {}", cpu_clock.since());

            cpu.tick(key);

            cpu_clock = Clock::new();
        }

        if devices_clock.since() >= 0.017 {
            println!("devices {}", devices_clock.since());

            display.draw(cpu.get_frame_buffer());

            if !cpu.is_delay_timer_zero() {
                cpu.decrease_delay_timer();
            }

            if !cpu.is_sound_timer_zero() {
                //TODO: sound the buzzer
                cpu.decrease_sound_timer();
            }

            devices_clock = Clock::new();
        }

        Timer::sleep(0.001);
    }
}
