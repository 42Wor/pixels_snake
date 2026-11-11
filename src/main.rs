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

/// Holds all mutable game state
struct Game {
    snake: VecDeque<Point>,
    food: Point,
    current_dir: Direction,
    next_dir: Direction,
    score: u32,
    high_score: u32,
    game_over: bool,
}

impl Game {
    fn new() -> Self {
        let mut snake = VecDeque::new();
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

        let food = Self::spawn_food(&snake);

        Game {
            snake,
            food,
            current_dir: Direction::Right,
            next_dir: Direction::Right,
            score: 0,
            high_score: 0,
            game_over: false,
        }
    }

    fn reset(&mut self) {
        let start_x = GRID_WIDTH / 2;
        let start_y = GRID_HEIGHT / 2;

        self.snake.clear();
        self.snake.push_back(Point {
            x: start_x,
            y: start_y,
        });
        self.snake.push_back(Point {
            x: start_x - 1,
            y: start_y,
        });
        self.snake.push_back(Point {
            x: start_x - 2,
            y: start_y,
        });

        self.food = Self::spawn_food(&self.snake);
        self.current_dir = Direction::Right;
        self.next_dir = Direction::Right;
        self.score = 0;
        self.game_over = false;
    }

    fn spawn_food(snake: &VecDeque<Point>) -> Point {
        let mut rng = rand::thread_rng();
        loop {
            let candidate = Point {
                x: rng.gen_range(1..GRID_WIDTH - 1),
                y: rng.gen_range(1..GRID_HEIGHT - 1),
            };
            if !snake.contains(&candidate) {
                return candidate;
            }
        }
    }

    /// Advances the game by 1 tick
    fn tick(&mut self) {
        if self.game_over {
            return;
        }

        self.current_dir = self.next_dir;
        let head = *self.snake.front().expect("Snake has no head");
        let (dx, dy) = self.current_dir.delta();

        let next_x = head.x as i32 + dx;
        let next_y = head.y as i32 + dy;

        // 1. Check Wall Collision
        if next_x <= 0
            || next_x >= (GRID_WIDTH as i32 - 1)
            || next_y <= 0
            || next_y >= (GRID_HEIGHT as i32 - 1)
        {
            self.trigger_game_over();
            return;
        }

        let new_head = Point {
            x: next_x as u32,
            y: next_y as u32,
        };

        // 2. Check Self Collision (the tail will move unless eating food)
        let eats_food = new_head == self.food;
        let body_to_check = if eats_food {
            self.snake.len()
        } else {
            self.snake.len() - 1 // Exclude the tail segment that vacates
        };

        let hit_self = self
            .snake
            .iter()
            .take(body_to_check)
            .any(|&seg| seg == new_head);

        if hit_self {
            self.trigger_game_over();
            return;
        }

        // 3. Move & Grow
        if eats_food {
            self.snake.push_front(new_head);
            self.score += 1;
            if self.score > self.high_score {
                self.high_score = self.score;
            }
            self.food = Self::spawn_food(&self.snake);
        } else {
            self.snake.push_front(new_head);
            self.snake.pop_back();
        }
    }

    fn trigger_game_over(&mut self) {
        self.game_over = true;
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let window = WindowBuilder::new()
        .with_title("Snake - Score: 0 | High: 0")
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

    let mut game = Game::new();
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
                } => match key_code {
                    // Movement Controls
                    KeyCode::ArrowUp | KeyCode::KeyW => {
                        if game.current_dir != Direction::Down {
                            game.next_dir = Direction::Up;
                        }
                    }
                    KeyCode::ArrowDown | KeyCode::KeyS => {
                        if game.current_dir != Direction::Up {
                            game.next_dir = Direction::Down;
                        }
                    }
                    KeyCode::ArrowLeft | KeyCode::KeyA => {
                        if game.current_dir != Direction::Right {
                            game.next_dir = Direction::Left;
                        }
                    }
                    KeyCode::ArrowRight | KeyCode::KeyD => {
                        if game.current_dir != Direction::Left {
                            game.next_dir = Direction::Right;
                        }
                    }
                    // Restart Control
                    KeyCode::Space | KeyCode::KeyR => {
                        if game.game_over {
                            game.reset();
                            last_tick = Instant::now();
                            window.set_title(&format!(
                                "Snake - Score: {} | High: {}",
                                game.score, game.high_score
                            ));
                        }
                    }
                    KeyCode::Escape => {
                        elwt.exit();
                    }
                    _ => (),
                },

                WindowEvent::RedrawRequested => {
                    draw_game(pixels.frame_mut(), &game);

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

                    if !game.game_over {
                        game.tick();

                        // Update window title based on state
                        if game.game_over {
                            window.set_title(&format!(
                                "GAME OVER! Score: {} | High: {} - Press SPACE to Restart",
                                game.score, game.high_score
                            ));
                        } else {
                            window.set_title(&format!(
                                "Snake - Score: {} | High: {}",
                                game.score, game.high_score
                            ));
                        }
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

fn draw_game(frame: &mut [u8], game: &Game) {
    // 1. Background & Border
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            let color = if x == 0 || x == GRID_WIDTH - 1 || y == 0 || y == GRID_HEIGHT - 1 {
                if game.game_over {
                    [231, 76, 60, 255] // Red border on death
                } else {
                    [160, 160, 160, 255] // Normal gray border
                }
            } else {
                [24, 26, 32, 255] // Dark Slate Background
            };
            set_pixel(frame, x, y, color);
        }
    }

    // 2. Draw Food (Apple)
    if !game.game_over {
        set_pixel(frame, game.food.x, game.food.y, [231, 76, 60, 255]);
    }

    // 3. Draw Snake
    let (head_color, body_color) = if game.game_over {
        // Dimmed red shades when dead
        ([180, 50, 50, 255], [120, 35, 35, 255])
    } else {
        // Vibrant greens when alive
        ([46, 204, 113, 255], [39, 174, 96, 255])
    };

    // Body segments
    for segment in game.snake.iter().skip(1) {
        set_pixel(frame, segment.x, segment.y, body_color);
    }

    // Head segment
    if let Some(head) = game.snake.front() {
        set_pixel(frame, head.x, head.y, head_color);
    }
}
