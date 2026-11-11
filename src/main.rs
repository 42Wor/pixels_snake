use pixels::{Pixels, SurfaceTexture};
use rand::Rng;
use std::collections::VecDeque;
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
const TICK_RATE: Duration = Duration::from_millis(110);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Point {
    x: u32,
    y: u32,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    fn delta(self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
}

/// Spawns an apple on a random tile not occupied by the snake
fn spawn_food(snake: &VecDeque<Point>) -> Point {
    let mut rng = rand::thread_rng();
    loop {
        let candidate = Point {
            x: rng.gen_range(1..GRID_WIDTH - 1),
            y: rng.gen_range(1..GRID_HEIGHT - 1),
        };

        // Ensure the food doesn't spawn inside the snake's body
        if !snake.contains(&candidate) {
            return candidate;
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let window = WindowBuilder::new()
        .with_title("Snake - Score: 0")
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

    // --- State: Snake Body ---
    let mut snake: VecDeque<Point> = VecDeque::new();
    let start_x = GRID_WIDTH / 2;
    let start_y = GRID_HEIGHT / 2;

    snake.push_back(Point {
        x: start_x,
        y: start_y,
    });
    snake.push_back(Point {
        x: start_x - 1,
        y: start_y,
    });
    snake.push_back(Point {
        x: start_x - 2,
        y: start_y,
    });

    // --- State: Food & Score ---
    let mut food = spawn_food(&snake);
    let mut score: u32 = 0;

    // --- State: Direction & Timer ---
    let mut current_dir = Direction::Right;
    let mut next_dir = Direction::Right;
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

                    if let Some(dir) = requested_dir {
                        if dir != current_dir.opposite() {
                            next_dir = dir;
                        }
                    }
                }

                WindowEvent::RedrawRequested => {
                    draw_game(pixels.frame_mut(), &snake, &food);

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
                    current_dir = next_dir;

                    // 1. Calculate new head position
                    let head = *snake.front().expect("Snake has no head");
                    let (dx, dy) = current_dir.delta();
                    let mut new_x = head.x as i32 + dx;
                    let mut new_y = head.y as i32 + dy;

                    // Temporary wrap-around within borders
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

                    let new_head = Point {
                        x: new_x as u32,
                        y: new_y as u32,
                    };

                    // 2. Growth logic: check if the new head touches the food
                    if new_head == food {
                        // Grow: add new head, KEEP the tail
                        snake.push_front(new_head);
                        score += 1;
                        food = spawn_food(&snake);
                        window.set_title(&format!("Snake - Score: {score}"));
                    } else {
                        // Normal move: add new head, drop tail
                        snake.push_front(new_head);
                        snake.pop_back();
                    }

                    window.request_redraw();
                }
            }

            _ => (),
        }
    });
}

fn set_pixel(frame: &mut [u8], x: u32, y: u32, color: [u8; 4]) {
    let index = ((y * GRID_WIDTH + x) * 4) as usize;
    frame[index..index + 4].copy_from_slice(&color);
}

fn draw_game(frame: &mut [u8], snake: &VecDeque<Point>, food: &Point) {
    // 1. Background and Border
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let color = if x == 0 || x == GRID_WIDTH - 1 || y == 0 || y == GRID_HEIGHT - 1 {
                [160, 160, 160, 255] // Gray border
            } else {
                [24, 26, 32, 255] // Dark slate background
            };
            set_pixel(frame, x, y, color);
        }
    }

    // 2. Food (Bright Red Apple)
    set_pixel(frame, food.x, food.y, [231, 76, 60, 255]);

    // 3. Snake Body (Dark Green)
    for segment in snake.iter().skip(1) {
        set_pixel(frame, segment.x, segment.y, [39, 174, 96, 255]);
    }

    // 4. Snake Head (Bright Green)
    if let Some(head) = snake.front() {
        set_pixel(frame, head.x, head.y, [46, 204, 113, 255]);
    }
}
