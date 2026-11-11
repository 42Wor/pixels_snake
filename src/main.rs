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

// --- Canvas & Window Sizing ---
const CANVAS_WIDTH: u32 = 240;
const CANVAS_HEIGHT: u32 = 264; // 24px HUD Bar + 240px Board
const HUD_HEIGHT: u32 = 24;

const WINDOW_WIDTH: f64 = 600.0;
const WINDOW_HEIGHT: f64 = 660.0; // 600 x 660 preserves exact 240x264 aspect ratio

const HUMAN_TICK_RATE: Duration = Duration::from_millis(90);
const WATCH_TICK_RATE: Duration = Duration::from_millis(80);

const CELL_SIZE: u32 = 4;
const COLS: u32 = CANVAS_WIDTH / CELL_SIZE; // 60 columns
const ROWS: u32 = (CANVAS_HEIGHT - HUD_HEIGHT) / CELL_SIZE; // 60 rows
const NUM_OBSTACLES: usize = 25;

// --- Color Palette (RGBA) ---
const COLOR_BG: [u8; 4] = [18, 20, 26, 255]; // Deep dark canvas
const COLOR_HUD_BG: [u8; 4] = [26, 30, 40, 255]; // Elevated HUD panel
const COLOR_HUD_DIVIDER: [u8; 4] = [45, 52, 70, 255]; // Divider line
const COLOR_BORDER_NORMAL: [u8; 4] = [120, 125, 140, 255]; // Slate border
const COLOR_BORDER_DEAD: [u8; 4] = [235, 77, 75, 255]; // Crimson red on death
const COLOR_OBSTACLE: [u8; 4] = [108, 92, 231, 255]; // Electric Indigo
const COLOR_FOOD: [u8; 4] = [255, 71, 87, 255]; // Bright Coral Apple
const COLOR_SNAKE_HEAD: [u8; 4] = [46, 213, 115, 255]; // Neon Mint Green
const COLOR_SNAKE_BODY: [u8; 4] = [38, 166, 91, 255]; // Forest Green
const COLOR_SNAKE_DEAD: [u8; 4] = [140, 45, 45, 255]; // Dim Red

// Text Colors
const COLOR_TEXT_SCORE: [u8; 4] = [46, 213, 115, 255]; // Mint
const COLOR_TEXT_BEST: [u8; 4] = [255, 211, 42, 255]; // Arcade Gold
const COLOR_TEXT_MANUAL: [u8; 4] = [140, 148, 165, 255]; // Slate Gray
const COLOR_TEXT_AI: [u8; 4] = [0, 206, 201, 255]; // Cyan
const COLOR_TEXT_OVER: [u8; 4] = [255, 71, 87, 255]; // Red

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

#[derive(Deserialize, Debug)]
struct StepCommand {
    action: u8,
    render: bool,
    reset: bool,
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

    fn step(&mut self, action: u8) -> f32 {
        if self.game_over {
            return -10.0;
        }

        self.current_dir = match action {
            1 => self.current_dir.turn_right(),
            2 => self.current_dir.turn_left(),
            _ => self.current_dir,
        };

        let head = *self.snake.front().unwrap();
        let (dx, dy) = self.current_dir.delta();
        let next_x = head.x as i32 + dx;
        let next_y = head.y as i32 + dy;

        if self.is_collision(next_x, next_y) {
            self.game_over = true;
            return -10.0;
        }

        let new_head = Point {
            x: next_x as u32,
            y: next_y as u32,
        };

        if new_head == self.food {
            self.snake.push_front(new_head);
            self.score += 1;
            if self.score > self.high_score {
                self.high_score = self.score;
            }
            self.food = Self::spawn_food(&self.snake, &self.obstacles);
            10.0
        } else {
            self.snake.push_front(new_head);
            self.snake.pop_back();
            -0.01
        }
    }

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
            if (self.food.y as i32) < (head.y as i32) {
                1.0
            } else {
                0.0
            },
            if (self.food.y as i32) > (head.y as i32) {
                1.0
            } else {
                0.0
            },
            if (self.food.x as i32) < (head.x as i32) {
                1.0
            } else {
                0.0
            },
            if (self.food.x as i32) > (head.x as i32) {
                1.0
            } else {
                0.0
            },
        ]
    }
}

// --- Built-in 3x5 Pixel Font ---
fn get_glyph(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '[' => [0b110, 0b100, 0b100, 0b100, 0b110],
        ']' => [0b011, 0b001, 0b001, 0b001, 0b011],
        ' ' => [0b000, 0b000, 0b000, 0b000, 0b000],
        'S' => [0b111, 0b100, 0b111, 0b001, 0b111],
        'C' => [0b111, 0b100, 0b100, 0b100, 0b111],
        'O' => [0b111, 0b101, 0b101, 0b101, 0b111],
        'R' => [0b111, 0b101, 0b110, 0b101, 0b101],
        'E' => [0b111, 0b100, 0b111, 0b100, 0b111],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'G' => [0b111, 0b100, 0b101, 0b101, 0b111],
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'M' => [0b101, 0b111, 0b101, 0b101, 0b101],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'N' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        _ => [0b000, 0b000, 0b000, 0b000, 0b000],
    }
}

fn draw_char(frame: &mut [u8], start_x: u32, start_y: u32, c: char, scale: u32, color: [u8; 4]) {
    let rows = get_glyph(c);
    for (dy, &row) in rows.iter().enumerate() {
        for dx in 0..3 {
            if (row & (1 << (2 - dx))) != 0 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = start_x + (dx as u32) * scale + sx;
                        let py = start_y + (dy as u32) * scale + sy;
                        if px < CANVAS_WIDTH && py < CANVAS_HEIGHT {
                            let idx = ((py * CANVAS_WIDTH + px) * 4) as usize;
                            frame[idx..idx + 4].copy_from_slice(&color);
                        }
                    }
                }
            }
        }
    }
}

fn draw_text(frame: &mut [u8], mut x: u32, y: u32, text: &str, scale: u32, color: [u8; 4]) {
    for c in text.chars() {
        draw_char(frame, x, y, c, scale, color);
        x += (3 + 1) * scale;
    }
}

struct ClientSession {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let window = WindowBuilder::new()
        .with_title("Snake Game AI Platform")
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Failed to create window");

    let mut pixels = {
        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, &window);
        Pixels::new(CANVAS_WIDTH, CANVAS_HEIGHT, surface_texture)
            .expect("Failed to initialize pixels surface")
    };

    // Eliminate black letterbox padding by matching the canvas background color
    pixels.clear_color(pixels::wgpu::Color {
        r: COLOR_BG[0] as f64 / 255.0,
        g: COLOR_BG[1] as f64 / 255.0,
        b: COLOR_BG[2] as f64 / 255.0,
        a: 1.0,
    });

    let listener = TcpListener::bind("127.0.0.1:9999").expect("Failed to bind port 9999");
    listener
        .set_nonblocking(true)
        .expect("Cannot set non-blocking");

    let mut client: Option<ClientSession> = None;
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
                    draw_all(pixels.frame_mut(), &game, client.is_some());
                    if let Err(err) = pixels.render() {
                        eprintln!("Render error: {err}");
                        elwt.exit();
                    }
                }
                _ => (),
            },

            Event::AboutToWait => {
                if client.is_none() {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            let writer = stream.try_clone().unwrap();
                            stream.set_nonblocking(false).unwrap();
                            client = Some(ClientSession {
                                reader: BufReader::new(stream),
                                writer,
                            });
                        }
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => (),
                        Err(err) => eprintln!("TCP Accept error: {err}"),
                    }
                }

                if let Some(ref mut session) = client {
                    let mut line = String::new();
                    match session.reader.read_line(&mut line) {
                        Ok(0) => {
                            client = None;
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

                                if cmd.render {
                                    window.request_redraw();
                                    std::thread::sleep(WATCH_TICK_RATE);
                                }
                            }
                        }
                        Err(_) => {
                            client = None;
                        }
                    }
                } else {
                    if last_tick.elapsed() >= HUMAN_TICK_RATE {
                        last_tick = Instant::now();
                        if !game.game_over {
                            game.step(0);
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
    let start_y = HUD_HEIGHT + cy * CELL_SIZE;

    for py in start_y..start_y + CELL_SIZE {
        for px in start_x..start_x + CELL_SIZE {
            if px < CANVAS_WIDTH && py < CANVAS_HEIGHT {
                let index = ((py * CANVAS_WIDTH + px) * 4) as usize;
                frame[index..index + 4].copy_from_slice(&color);
            }
        }
    }
}

fn draw_all(frame: &mut [u8], game: &Game, is_ai: bool) {
    // ----------------------------------------------------
    // 1. Draw HUD Header
    // ----------------------------------------------------
    for y in 0..HUD_HEIGHT {
        for x in 0..CANVAS_WIDTH {
            let index = ((y * CANVAS_WIDTH + x) * 4) as usize;
            if y == HUD_HEIGHT - 1 {
                frame[index..index + 4].copy_from_slice(&COLOR_HUD_DIVIDER);
            } else {
                frame[index..index + 4].copy_from_slice(&COLOR_HUD_BG);
            }
        }
    }

    // Left: Current Score
    let score_str = format!("SCORE:{}", game.score);
    draw_text(frame, 6, 7, &score_str, 2, COLOR_TEXT_SCORE);

    // Right: High Score
    let best_str = format!("BEST:{}", game.high_score);
    let best_width = (best_str.len() as u32) * 8;
    let best_x = CANVAS_WIDTH - best_width - 6;
    draw_text(frame, best_x, 7, &best_str, 2, COLOR_TEXT_BEST);

    // Center: Mode or GAME OVER
    if game.game_over {
        let msg = "GAME OVER";
        let msg_x = (CANVAS_WIDTH - (msg.len() as u32 * 8)) / 2;
        draw_text(frame, msg_x, 7, msg, 2, COLOR_TEXT_OVER);
    } else if is_ai {
        let msg = "[AI]";
        let msg_x = (CANVAS_WIDTH - (msg.len() as u32 * 8)) / 2;
        draw_text(frame, msg_x, 7, msg, 2, COLOR_TEXT_AI);
    } else {
        let msg = "[MANUAL]";
        let msg_x = (CANVAS_WIDTH - (msg.len() as u32 * 8)) / 2;
        draw_text(frame, msg_x, 7, msg, 2, COLOR_TEXT_MANUAL);
    }

    // ----------------------------------------------------
    // 2. Draw Game Board (Below HUD)
    // ----------------------------------------------------
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

    // Obstacles
    for obstacle in &game.obstacles {
        draw_cell(frame, obstacle.x, obstacle.y, COLOR_OBSTACLE);
    }

    // Food (Apple)
    if !game.game_over {
        draw_cell(frame, game.food.x, game.food.y, COLOR_FOOD);
    }

    // Snake
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
