use pixels::{Pixels, SurfaceTexture};
use std::time::{Duration, Instant};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

const GRID_WIDTH: u32 = 24;
const GRID_HEIGHT: u32 = 24;
const WINDOW_WIDTH: f64 = 600.0;
const WINDOW_HEIGHT: f64 = 600.0;

// The game advances one tile every 120 ms
const TICK_RATE: Duration = Duration::from_millis(120);

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let window = WindowBuilder::new()
        .with_title("Snake - Milestone 2: Moving Dot")
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Failed to create window");

    let mut pixels = {
        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, &window);
        Pixels::new(GRID_WIDTH, GRID_HEIGHT, surface_texture)
            .expect("Failed to initialize pixels surface")
    };

    // --- State: Dot Position & Direction ---
    let mut dot_x: u32 = 1;
    let mut dot_y: u32 = GRID_HEIGHT / 2;

    let dir_x: i32 = 1; // 1 = moving right
    let dir_y: i32 = 0;

    // --- State: Timing ---
    let mut last_tick = Instant::now();

    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),

                WindowEvent::Resized(new_size) => {
                    if let Err(err) = pixels.resize_surface(new_size.width, new_size.height) {
                        eprintln!("Resize surface failed: {err}");
                        elwt.exit();
                    }
                }

                WindowEvent::RedrawRequested => {
                    draw_grid(pixels.frame_mut(), dot_x, dot_y);

                    if let Err(err) = pixels.render() {
                        eprintln!("pixels.render failed: {err}");
                        elwt.exit();
                    }
                }
                _ => (),
            },

            Event::AboutToWait => {
                // Check if enough time has passed to advance the game
                if last_tick.elapsed() >= TICK_RATE {
                    last_tick = Instant::now();

                    // Advance position
                    dot_x = (dot_x as i32 + dir_x) as u32;
                    dot_y = (dot_y as i32 + dir_y) as u32;

                    // Temporary boundary wrap: if it reaches the right wall, loop back to x=1
                    if dot_x >= GRID_WIDTH - 1 {
                        dot_x = 1;
                    }

                    // Trigger a redraw since coordinates changed
                    window.request_redraw();
                }
            }

            _ => (),
        }
    });
}

fn draw_grid(frame: &mut [u8], dot_x: u32, dot_y: u32) {
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let index = ((y * GRID_WIDTH + x) * 4) as usize;

            let color = if x == 0 || x == GRID_WIDTH - 1 || y == 0 || y == GRID_HEIGHT - 1 {
                // Wall Border: Light Gray
                [160, 160, 160, 255]
            } else if x == dot_x && y == dot_y {
                // Moving Dot: Green
                [46, 204, 113, 255]
            } else {
                // Background
                [24, 26, 32, 255]
            };

            frame[index..index + 4].copy_from_slice(&color);
        }
    }
}
