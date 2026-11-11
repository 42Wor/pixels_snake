use pixels::{Pixels, SurfaceTexture};
use rand::Rng;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowBuilder;

// --- Canvas & Window Constants ---
const GRID_WIDTH: u32 = 240;
const GRID_HEIGHT: u32 = 240;
const WINDOW_WIDTH: f64 = 600.0;
const WINDOW_HEIGHT: f64 = 600.0;
const TICK_RATE: Duration = Duration::from_millis(90);

// Size of each grid cell (4 = 60x60 grid)
const CELL_SIZE: u32 = 4;
const COLS: u32 = GRID_WIDTH / CELL_SIZE;
const ROWS: u32 = GRID_HEIGHT / CELL_SIZE;

const NUM_OBSTACLES: usize = 25;

// --- Color Palette (RGBA) ---
const COLOR_BG: [u8; 4] = [18, 20, 26, 255]; // Deep dark navy
const COLOR_BORDER_NORMAL: [u8; 4] = [120, 125, 140, 255]; // Cool slate border
const COLOR_BORDER_DEAD: [u8; 4] = [235, 77, 75, 255]; // Crimson red on death
const COLOR_OBSTACLE: [u8; 4] = [108, 92, 231, 255]; // Electric Indigo / Slate Blue
const COLOR_FOOD: [u8; 4] = [255, 71, 87, 255]; // Bright Coral Apple
const COLOR_SNAKE_HEAD: [u8; 4] = [46, 213, 115, 255]; // Neon Mint Green
const COLOR_SNAKE_BODY: [u8; 4] = [38, 166, 91, 255]; // Forest Green
const COLOR_SNAKE_DEAD: [u8; 4] = [140, 45, 45, 255]; // Dim Red

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Point {
    x: u32,
    y: u32,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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

struct Game {
    snake: VecDeque<Point>,
    food: Point,
    obstacles: Vec<Point>,
    current_dir: Direction,
    next_dir: Direction,
    score: u32,
    high_score: u32,
    game_over: bool,
}

impl Game {
    fn new() -> Self {
        let mut snake = VecDeque::new();
        let start_x = COLS / 2;
        let start_y = ROWS / 2;

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

        let obstacles = Self::generate_obstacles(&snake);
        let food = Self::spawn_food(&snake, &obstacles);

        Game {
            snake,
            food,
            obstacles,
            current_dir: Direction::Right,
            next_dir: Direction::Right,
            score: 0,
            high_score: 0,
            game_over: false,
        }
    }

    fn reset(&mut self) {
        let start_x = COLS / 2;
        let start_y = ROWS / 2;

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

        // Brand new obstacles on every restart
        self.obstacles = Self::generate_obstacles(&self.snake);
        self.food = Self::spawn_food(&self.snake, &self.obstacles);

        self.current_dir = Direction::Right;
        self.next_dir = Direction::Right;
        self.score = 0;
        self.game_over = false;
    }

    fn generate_obstacles(snake: &VecDeque<Point>) -> Vec<Point> {
        let mut rng = rand::thread_rng();
        let mut obstacles = Vec::with_capacity(NUM_OBSTACLES);
        let head = *snake.front().unwrap();

        while obstacles.len() < NUM_OBSTACLES {
            let candidate = Point {
                x: rng.gen_range(1..COLS - 1),
                y: rng.gen_range(1..ROWS - 1),
            };

            if snake.contains(&candidate) || obstacles.contains(&candidate) {
                continue;
            }

            // Safe runway for snake spawn
            let dist_x = (candidate.x as i32 - head.x as i32).abs();
            let dist_y = (candidate.y as i32 - head.y as i32).abs();
            if dist_x <= 5 && dist_y <= 3 {
                continue;
            }

            obstacles.push(candidate);
        }

        obstacles
    }

    fn spawn_food(snake: &VecDeque<Point>, obstacles: &[Point]) -> Point {
        let mut rng = rand::thread_rng();
        loop {
            let candidate = Point {
                x: rng.gen_range(1..COLS - 1),
                y: rng.gen_range(1..ROWS - 1),
            };

            if !snake.contains(&candidate) && !obstacles.contains(&candidate) {
                return candidate;
            }
        }
    }

    fn tick(&mut self) {
        if self.game_over {
            return;
        }

        self.current_dir = self.next_dir;
        let head = *self.snake.front().expect("Snake has no head");
        let (dx, dy) = self.current_dir.delta();

        let next_x = head.x as i32 + dx;
        let next_y = head.y as i32 + dy;

        // 1. Wall Collision
        if next_x <= 0 || next_x >= (COLS as i32 - 1) || next_y <= 0 || next_y >= (ROWS as i32 - 1)
        {
            self.game_over = true;
            return;
        }

        let new_head = Point {
            x: next_x as u32,
            y: next_y as u32,
        };

        // 2. Obstacle Collision
        if self.obstacles.contains(&new_head) {
            self.game_over = true;
            return;
        }

        // 3. Self Collision
        let eats_food = new_head == self.food;
        let body_to_check = if eats_food {
            self.snake.len()
        } else {
            self.snake.len() - 1
        };

        let hit_self = self
            .snake
            .iter()
            .take(body_to_check)
            .any(|&seg| seg == new_head);

        if hit_self {
            self.game_over = true;
            return;
        }

        // 4. Movement / Growth
        if eats_food {
            self.snake.push_front(new_head);
            self.score += 1;
            if self.score > self.high_score {
                self.high_score = self.score;
            }
            self.food = Self::spawn_food(&self.snake, &self.obstacles);
        } else {
            self.snake.push_front(new_head);
            self.snake.pop_back();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let window = WindowBuilder::new()
        .with_title("Snake Game AI Platform - Score: 0")
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
                    let _ = pixels.resize_surface(new_size.width, new_size.height);
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
                    KeyCode::Space | KeyCode::KeyR => {
                        if game.game_over {
                            game.reset();
                            last_tick = Instant::now();
                            window.set_title(&format!(
                                "Snake AI Platform - Score: {} | Best: {}",
                                game.score, game.high_score
                            ));
                        }
                    }
                    KeyCode::Escape => elwt.exit(),
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

                        if game.game_over {
                            window.set_title(&format!(
                                "GAME OVER! Score: {} | Best: {} [SPACE to Restart]",
                                game.score, game.high_score
                            ));
                        } else {
                            window.set_title(&format!(
                                "Snake AI Platform - Score: {} | Best: {}",
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

fn draw_cell(frame: &mut [u8], cx: u32, cy: u32, color: [u8; 4]) {
    let start_x = cx * CELL_SIZE;
    let start_y = cy * CELL_SIZE;

    for py in start_y..start_y + CELL_SIZE {
        for px in start_x..start_x + CELL_SIZE {
            if px < GRID_WIDTH && py < GRID_HEIGHT {
                let index = ((py * GRID_WIDTH + px) * 4) as usize;
                frame[index..index + 4].copy_from_slice(&color);
            }
        }
    }
}

fn draw_game(frame: &mut [u8], game: &Game) {
    // 1. Background & Border
    for cy in 0..ROWS {
        for cx in 0..COLS {
            let color = if cx == 0 || cx == COLS - 1 || cy == 0 || cy == ROWS - 1 {
                if game.game_over {
                    COLOR_BORDER_DEAD
                } else {
                    COLOR_BORDER_NORMAL
                }
            } else {
                COLOR_BG
            };
            draw_cell(frame, cx, cy, color);
        }
    }

    // 2. Obstacles
    for obstacle in &game.obstacles {
        draw_cell(frame, obstacle.x, obstacle.y, COLOR_OBSTACLE);
    }

    // 3. Apple / Food
    if !game.game_over {
        draw_cell(frame, game.food.x, game.food.y, COLOR_FOOD);
    }

    // 4. Snake
    let (head_color, body_color) = if game.game_over {
        (COLOR_SNAKE_DEAD, COLOR_SNAKE_DEAD)
    } else {
        (COLOR_SNAKE_HEAD, COLOR_SNAKE_BODY)
    };

    for segment in game.snake.iter().skip(1) {
        draw_cell(frame, segment.x, segment.y, body_color);
    }

    if let Some(head) = game.snake.front() {
        draw_cell(frame, head.x, head.y, head_color);
    }
}
