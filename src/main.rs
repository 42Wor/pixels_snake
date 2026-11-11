use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

// Virtual grid resolution (in cells / virtual pixels)
const GRID_WIDTH: u32 = 24;
const GRID_HEIGHT: u32 = 24;

// Physical window display dimensions
const WINDOW_WIDTH: f64 = 600.0;
const WINDOW_HEIGHT: f64 = 600.0;

fn main() {
    // 1. Create the winit event loop
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    // 2. Build the window
    let window = WindowBuilder::new()
        .with_title("Snake - Milestone 1")
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Failed to create window");

    // 3. Initialize the pixels frame buffer
    let mut pixels = {
        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, &window);
        Pixels::new(GRID_WIDTH, GRID_HEIGHT, surface_texture)
            .expect("Failed to initialize pixels surface")
    };

    // 4. Run the application event loop
    let _ = event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => match event {
                // Exit when the user closes the window
                WindowEvent::CloseRequested => elwt.exit(),

                // Handle surface resize if window changes
                WindowEvent::Resized(new_size) => {
                    if let Err(err) = pixels.resize_surface(new_size.width, new_size.height) {
                        eprintln!("Resize surface failed: {err}");
                        elwt.exit();
                    }
                }

                // Render the frame when requested
                WindowEvent::RedrawRequested => {
                    draw_grid(pixels.frame_mut());

                    if let Err(err) = pixels.render() {
                        eprintln!("pixels.render failed: {err}");
                        elwt.exit();
                    }
                }
                _ => (),
            },

            // Continuously request a redraw
            Event::AboutToWait => {
                window.request_redraw();
            }

            _ => (),
        }
    });
}

/// Draws the static border, background, and initial snake dot to the buffer.
fn draw_grid(frame: &mut [u8]) {
    for y in 0..GRID_HEIGHT {
        for x in 0..GRID_WIDTH {
            // Each pixel has 4 bytes: [R, G, B, A]
            let index = ((y * GRID_WIDTH + x) * 4) as usize;

            let color = if x == 0 || x == GRID_WIDTH - 1 || y == 0 || y == GRID_HEIGHT - 1 {
                // Wall Border: Light Gray
                [160, 160, 160, 255]
            } else if x == GRID_WIDTH / 2 && y == GRID_HEIGHT / 2 {
                // Starting Snake Head Dot: Bright Green
                [46, 204, 113, 255]
            } else {
                // Playable Grid Space: Dark Background
                [24, 26, 32, 255]
            };

            frame[index..index + 4].copy_from_slice(&color);
        }
    }
}
