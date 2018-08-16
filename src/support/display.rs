use support::type_defs::{FrameBuffer, N_FRAMEBUFFER_WIDTH, N_FRAMEBUFFER_HEIGHT};

use sdl2::Sdl;
use sdl2::video::Window;
use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::rect::Rect;

pub struct Display {
	canvas: Canvas<Window>,
	pixel_width: u32,
	pixel_height: u32,
}

impl Display {
	pub fn new(
		width: u32,
		height: u32,
		context: &Sdl
	) -> Display {
		let video_context = context.video().unwrap();
		let window = video_context.window(
			"Chip-8",
			width,
			height)
			.position_centered()
			.opengl()
			.build();
		let window = match window {
			Ok(w) => w,
			Err(err) => panic!("Unable to create window: {}", err),
		};

		let canvas = window.into_canvas().build();
		let canvas = match canvas {
			Ok(canvas) => canvas,
			Err(err) => panic!("Unable to create canvas: {}", err),
		};

		Display {
			canvas: canvas,
			pixel_width: width / N_FRAMEBUFFER_WIDTH as u32,
			pixel_height: height / N_FRAMEBUFFER_HEIGHT as u32,
		}
	}

	pub fn draw(&mut self, frame_buffer: FrameBuffer) {
		for x in 0..N_FRAMEBUFFER_WIDTH {
			for y in 0..N_FRAMEBUFFER_HEIGHT {
				if frame_buffer[x][y] {
					self.canvas.set_draw_color(Color::RGB(255, 255, 255));
				} else {
					self.canvas.set_draw_color(Color::RGB(0, 0, 0));
				}
				
				self.canvas.fill_rect(Rect::new(
					(x as u32 * self.pixel_width) as i32,
					(y as u32 * self.pixel_height) as i32,
					self.pixel_width,
					self.pixel_height
					))
				.unwrap();
			}
		}

		self.canvas.present();
	}
}
