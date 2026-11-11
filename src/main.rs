use pixels::{Pixels, SurfaceTexture};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{TcpListener, TcpStream};
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
const HUMAN_TICK_RATE: Duration = Duration::from_millis(90);
const WATCH_TICK_RATE: Duration = Duration::from_millis(80);

const CELL_SIZE: u32 = 4;
const COLS: u32 = GRID_WIDTH / CELL_SIZE;
const ROWS: u32 = GRID_HEIGHT / CELL_SIZE;
const NUM_OBSTACLES: usize = 25;

// --- Colors ---
const COLOR_BG: [u8; 4] = [18, 20, 26, 255];
const COLOR_BORDER_NORMAL: [u8; 4] = [120, 125, 140, 255];
const COLOR_BORDER_DEAD: [u8; 4] = [235, 77, 75, 255];
const COLOR_OBSTACLE: [u8; 4] = [108, 92, 231, 255];
const COLOR_FOOD: [u8; 4] = [255, 71, 87, 255];
const COLOR_SNAKE_HEAD: [u8; 4] = [46, 213, 115, 255];
const COLOR_SNAKE_BODY: [u8; 4] = [38, 166, 91, 255];
const COLOR_SNAKE_DEAD: [u8; 4] = [140, 45, 45, 255];

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
    fn delta(self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }

    fn turn_right(self) -> Self {
        match self {
            Direction::Up => Direction::Right,
            Direction::Right => Direction::Down,
            Direction::Down => Direction::Left,
            Direction::Left => Direction::Up,
        }
    }

    fn turn_left(self) -> Self {
        match self {
            Direction::Up => Direction::Left,
            Direction::Left => Direction::Down,
            Direction::Down => Direction::Right,
            Direction::Right => Direction::Up,
        }
    }
}

// --- IPC Communication Protocols ---
#[derive(Deserialize, Debug)]
struct StepCommand {
    action: u8,   // 0: Straight, 1: Turn Right, 2: Turn Left
    render: bool, // true: update window, false: turbo mode
    reset: bool,  // true: restart game
}

#[derive(Serialize, Debug)]
struct StepResponse {
    state: Vec<f32>,
    reward: f32,
    done: bool,
    score: u32,
}

struct Game {
    snake: VecDeque<Point>,
    food: Point,
    obstacles: Vec<Point>,
    current_dir: Direction,
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

        self.obstacles = Self::generate_obstacles(&self.snake);
        self.food = Self::spawn_food(&self.snake, &self.obstacles);

        self.current_dir = Direction::Right;
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

    fn is_collision(&self, x: i32, y: i32) -> bool {
        if x <= 0 || x >= (COLS as i32 - 1) || y <= 0 || y >= (ROWS as i32 - 1) {
            return true;
        }
        let pt = Point {
            x: x as u32,
            y: y as u32,
        };
        self.obstacles.contains(&pt) || self.snake.contains(&pt)
    }

    /// Advances game one step from AI action and returns step reward
    fn step(&mut self, action: u8) -> f32 {
        if self.game_over {
            return -10.0;
        }

        // Action steering: 0 = straight, 1 = right, 2 = left
        self.current_dir = match action {
            1 => self.current_dir.turn_right(),
            2 => self.current_dir.turn_left(),
            _ => self.current_dir,
        };

        let head = *self.snake.front().unwrap();
        let (dx, dy) = self.current_dir.delta();
        let next_x = head.x as i32 + dx;
        let next_y = head.y as i32 + dy;

        // Collision Check
        if self.is_collision(next_x, next_y) {
            self.game_over = true;
            return -10.0;
        }

        let new_head = Point {
            x: next_x as u32,
            y: next_y as u32,
        };

        // Food Check
        if new_head == self.food {
            self.snake.push_front(new_head);
            self.score += 1;
            if self.score > self.high_score {
                self.high_score = self.score;
            }
            self.food = Self::spawn_food(&self.snake, &self.obstacles);
            10.0 // Positive reward
        } else {
            self.snake.push_front(new_head);
            self.snake.pop_back();
            -0.01 // Tiny step penalty to incentivize finding food quickly
        }
    }

    /// Builds the 11-feature state vector for reinforcement learning
    fn get_state(&self) -> Vec<f32> {
        let head = *self.snake.front().unwrap();

        let straight_dir = self.current_dir;
        let right_dir = self.current_dir.turn_right();
        let left_dir = self.current_dir.turn_left();

        let pt_straight = (
            head.x as i32 + straight_dir.delta().0,
            head.y as i32 + straight_dir.delta().1,
        );
        let pt_right = (
            head.x as i32 + right_dir.delta().0,
            head.y as i32 + right_dir.delta().1,
        );
        let pt_left = (
            head.x as i32 + left_dir.delta().0,
            head.y as i32 + left_dir.delta().1,
        );

        vec![
            // Danger straight, right, left (1.0 or 0.0)
            if self.is_collision(pt_straight.0, pt_straight.1) {
                1.0
            } else {
                0.0
            },
            if self.is_collision(pt_right.0, pt_right.1) {
                1.0
            } else {
                0.0
            },
            if self.is_collision(pt_left.0, pt_left.1) {
                1.0
            } else {
                0.0
            },
            // Current Direction
            if self.current_dir == Direction::Up {
                1.0
            } else {
                0.0
            },
            if self.current_dir == Direction::Down {
                1.0
            } else {
                0.0
            },
            if self.current_dir == Direction::Left {
                1.0
            } else {
                0.0
            },
            if self.current_dir == Direction::Right {
                1.0
            } else {
                0.0
            },
            // Food relative position
            if (self.food.y as i32) < (head.y as i32) {
                1.0
            } else {
                0.0
            }, // Food is Up
            if (self.food.y as i32) > (head.y as i32) {
                1.0
            } else {
                0.0
            }, // Food is Down
            if (self.food.x as i32) < (head.x as i32) {
                1.0
            } else {
                0.0
            }, // Food is Left
            if (self.food.x as i32) > (head.x as i32) {
                1.0
            } else {
                0.0
            }, // Food is Right
        ]
    }
}

struct ClientSession {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let window = WindowBuilder::new()
        .with_title("Snake Game - Human Mode (Listening on 127.0.0.1:9999 for Python)")
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

    let listener = TcpListener::bind("127.0.0.1:9999").expect("Failed to bind port 9999");
    listener
        .set_nonblocking(true)
        .expect("Cannot set non-blocking");

    let mut client: Option<ClientSession> = None;
    let mut game = Game::new();
    let mut last_tick = Instant::now();

    println!("======================================================");
    println!(" Rust Snake Server running on 127.0.0.1:9999");
    println!(" Play manually OR run your python agent script!");
    println!("======================================================");

    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),

                WindowEvent::Resized(new_size) => {
                    let _ = pixels.resize_surface(new_size.width, new_size.height);
                }

                // Keyboard controls (only active when Python is NOT connected)
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(key_code),
                            ..
                        },
                    ..
                } => {
                    if client.is_none() {
                        match key_code {
                            KeyCode::ArrowUp | KeyCode::KeyW => {
                                if game.current_dir != Direction::Down {
                                    game.current_dir = Direction::Up;
                                }
                            }
                            KeyCode::ArrowDown | KeyCode::KeyS => {
                                if game.current_dir != Direction::Up {
                                    game.current_dir = Direction::Down;
                                }
                            }
                            KeyCode::ArrowLeft | KeyCode::KeyA => {
                                if game.current_dir != Direction::Right {
                                    game.current_dir = Direction::Left;
                                }
                            }
                            KeyCode::ArrowRight | KeyCode::KeyD => {
                                if game.current_dir != Direction::Left {
                                    game.current_dir = Direction::Right;
                                }
                            }
                            KeyCode::Space | KeyCode::KeyR => {
                                if game.game_over {
                                    game.reset();
                                }
                            }
                            KeyCode::Escape => elwt.exit(),
                            _ => (),
                        }
                    }
                }

                WindowEvent::RedrawRequested => {
                    draw_game(pixels.frame_mut(), &game);
                    if let Err(err) = pixels.render() {
                        eprintln!("Render error: {err}");
                        elwt.exit();
                    }
                }
                _ => (),
            },

            Event::AboutToWait => {
                // 1. Check for incoming Python connections
                if client.is_none() {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let writer = stream.try_clone().unwrap();
                            stream.set_nonblocking(false).unwrap();
                            client = Some(ClientSession {
                                reader: BufReader::new(stream),
                                writer,
                            });
                            window.set_title("Snake Game - [AI Python Agent Active]");
                            println!("[INFO] Python Agent Connected!");
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => (),
                        Err(err) => eprintln!("TCP Accept error: {err}"),
                    }
                }

                // 2. AI Mode (Python is Driving)
                if let Some(ref mut session) = client {
                    let mut line = String::new();
                    match session.reader.read_line(&mut line) {
                        Ok(0) => {
                            println!("[INFO] Python disconnected. Returning to manual mode.");
                            client = None;
                            window.set_title("Snake Game - Human Mode (Listening on :9999)");
                        }
                        Ok(_) => {
                            if let Ok(cmd) = serde_json::from_str::<StepCommand>(&line) {
                                if cmd.reset {
                                    game.reset();
                                }

                                let reward = game.step(cmd.action);
                                let response = StepResponse {
                                    state: game.get_state(),
                                    reward,
                                    done: game.game_over,
                                    score: game.score,
                                };

                                let resp_json = serde_json::to_string(&response).unwrap() + "\n";
                                let _ = session.writer.write_all(resp_json.as_bytes());

                                // Only render if requested by Python (Watch Mode)
                                if cmd.render {
                                    window.request_redraw();
                                    std::thread::sleep(WATCH_TICK_RATE);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error reading from Python: {e}");
                            client = None;
                        }
                    }
                } else {
                    // 3. Human Mode
                    if last_tick.elapsed() >= HUMAN_TICK_RATE {
                        last_tick = Instant::now();
                        if !game.game_over {
                            game.step(0); // Move straight
                        }
                        window.request_redraw();
                    }
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

    for obstacle in &game.obstacles {
        draw_cell(frame, obstacle.x, obstacle.y, COLOR_OBSTACLE);
    }

    if !game.game_over {
        draw_cell(frame, game.food.x, game.food.y, COLOR_FOOD);
    }

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
