use pixels::{Pixels, SurfaceTexture};
use std::time::{Duration, Instant};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowBuilder;

const GRID_WIDTH: u32 = 24;
const GRID_HEIGHT: u32 = 24;
const WINDOW_WIDTH: f64 = 600.0;
const WINDOW_HEIGHT: f64 = 600.0;
const TICK_RATE: Duration = Duration::from_millis(120);

#[derive(Copy, Clone, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Returns the exact opposite direction.
    fn opposite(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    /// Returns the coordinate offset (dx, dy).
    fn delta(self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let window = WindowBuilder::new()
        .with_title("Snake - Milestone 3: Input Control")
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

    // --- State: Dot Position ---
    let mut dot_x: u32 = GRID_WIDTH / 2;
    let mut dot_y: u32 = GRID_HEIGHT / 2;

    // --- State: Directions ---
    let mut current_dir = Direction::Right;
    let mut next_dir = Direction::Right;

    // --- State: Timer ---
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

                // Handle keyboard events (WASD, Arrows, Escape)
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(key_code),
                            ..
                        },
                    ..
                } => {
                    let requested_dir = match key_code {
                        KeyCode::ArrowUp | KeyCode::KeyW => Some(Direction::Up),
                        KeyCode::ArrowDown | KeyCode::KeyS => Some(Direction::Down),
                        KeyCode::ArrowLeft | KeyCode::KeyA => Some(Direction::Left),
                        KeyCode::ArrowRight | KeyCode::KeyD => Some(Direction::Right),
                        KeyCode::Escape => {
                            elwt.exit();
                            None
                        }
                        _ => None,
                    };

                    // Prevent reversing directly onto itself
                    if let Some(dir) = requested_dir {
                        if dir != current_dir.opposite() {
                            next_dir = dir;
                        }
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
                if last_tick.elapsed() >= TICK_RATE {
                    last_tick = Instant::now();

                    // Apply the confirmed direction for this tick
                    current_dir = next_dir;

                    // Calculate new coordinates
                    let (dx, dy) = current_dir.delta();
                    let mut new_x = dot_x as i32 + dx;
                    let mut new_y = dot_y as i32 + dy;

                    // Temporary boundary wrapping within the walls (1 ..= GRID - 2)
                    if new_x < 1 {
                        new_x = GRID_WIDTH as i32 - 2;
                    } else if new_x > (GRID_WIDTH as i32 - 2) {
                        new_x = 1;
                    }

                    if new_y < 1 {
                        new_y = GRID_HEIGHT as i32 - 2;
                    } else if new_y > (GRID_HEIGHT as i32 - 2) {
                        new_y = 1;
                    }

                    dot_x = new_x as u32;
                    dot_y = new_y as u32;

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
                // Border walls
                [160, 160, 160, 255]
            } else if x == dot_x && y == dot_y {
                // Controlled Dot
                [46, 204, 113, 255]
            } else {
                // Background
                [24, 26, 32, 255]
            };

            frame[index..index + 4].copy_from_slice(&color);
        }
    }
}
