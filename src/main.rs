mod support;
use support::*;

use std::time::{ Duration, Instant };
use std::thread;
use std::env;

extern crate sdl2;

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

    let mut cpu_tick_countdown = Duration::from_nanos(MAIN_THREAD_NS);
    let mut add_cpu_tick_countdown = false;
    let mut devices_tick_countdown = Duration::from_nanos(SEC_THREAD_NS);
    let mut add_devices_tick_countdown = false;
    //let mut dt = Instant::now();
    loop {
        let thread_duration = Instant::now();

        let key = get_key_from_keyboard(&mut event_pump);        

        if cpu_tick_countdown >= Duration::from_nanos(MAIN_THREAD_NS) {
            cpu.tick(key);
            cpu_tick_countdown = Duration::from_nanos(0);
        } else {
            add_cpu_tick_countdown = true;
        }

        if devices_tick_countdown >= Duration::from_nanos(SEC_THREAD_NS) {
            // println!("{} - {}", devices_tick_countdown.subsec_millis(), dt.elapsed().subsec_millis());
            // dt = Instant::now();

            display.draw(cpu.get_frame_buffer());

            if !cpu.is_sound_timer_zero() {
                //TODO: sound the buzzer
                cpu.decrease_sound_timer();
            }

            if !cpu.is_delay_timer_zero() {
                cpu.decrease_delay_timer();
            }

            devices_tick_countdown = Duration::from_nanos(0);
        } else {
            add_devices_tick_countdown = true;
        }

        let thread_duration = thread_duration.elapsed().subsec_nanos() as u64;
        if thread_duration < THREAD_SLEEP_NS {
            let duration = Duration::from_nanos(THREAD_SLEEP_NS - thread_duration);
            if add_cpu_tick_countdown {
                cpu_tick_countdown += duration;
                add_cpu_tick_countdown = false;
            }

            if add_devices_tick_countdown {
                devices_tick_countdown += duration;
                add_devices_tick_countdown = false;
            }

            thread::sleep(duration);
        } else {
            if add_cpu_tick_countdown {
                //cpu_tick_countdown += Duration::from_nanos(1_000_000);
                add_cpu_tick_countdown = false;
            }

            if add_devices_tick_countdown {
                //devices_tick_countdown += Duration::from_nanos(1_000_000);
                add_devices_tick_countdown = false;
            }

            thread::sleep(Duration::from_nanos(0));
            println!("thread duration {}", thread_duration);
        }
    }
}
