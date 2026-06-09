//! autocue-cli — 命令行入口
//!
//! VibeRustAutocue 的二进制入口，负责解析 CLI 参数并编排各层。

use autocue_render::{LayoutEngine, Painter, RenderWindow};
use clap::{Parser, Subcommand};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};

/// A cross-platform teleprompter (AutoCue) built in Rust.
#[derive(Parser)]
#[command(name = "autocue", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the teleprompter with a script file
    Run {
        /// Path to the script file (use "-" for stdin, or --clipboard)
        file: Option<String>,

        /// Read script from clipboard instead of file
        #[arg(long, conflicts_with = "file")]
        clipboard: bool,

        /// Display mode: scroll, chunk, or focus
        #[arg(long, default_value = "scroll")]
        mode: String,

        /// Scroll speed in characters per second
        #[arg(long, default_value = "5.0")]
        speed: f64,

        /// Font size in points
        #[arg(long)]
        font_size: Option<f32>,

        /// Enable mirror mode for teleprompter glass
        #[arg(long)]
        mirror: bool,
    },

    /// Generate a default autocue.toml config file
    Init,

    /// Validate a script file
    Check { file: String },
}

// ---------------------------------------------------------------------------
// Application state (winit ApplicationHandler)
// ---------------------------------------------------------------------------

struct AppState {
    window: Option<RenderWindow>,
    painter: Option<Painter>,
    layout: LayoutEngine,
    font_size: f32,
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let render_window = pollster::block_on(RenderWindow::new(
            event_loop,
            800,
            600,
            "VibeRustAutocue — Teleprompter",
        ));

        match render_window {
            Ok(rw) => {
                let format = rw.config.format;
                let painter = Painter::new(&rw.device, format);
                self.painter = Some(painter);
                self.window = Some(rw);
                // 请求首帧
                if let Some(ref window) = self.window {
                    window.window.request_redraw();
                }
            }
            Err(e) => {
                tracing::error!("Failed to create window: {e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: key,
                        ..
                    },
                ..
            } => {
                match key {
                    Key::Named(NamedKey::Escape) => {
                        event_loop.exit();
                    }
                    // 字体缩放
                    Key::Character(ref ch) if ch == "=" || ch == "+" => {
                        // Ctrl+= 增大字号
                        // 在 winit 中需要检查 modifier
                        self.font_size = (self.font_size + 2.0).min(300.0);
                        self.layout.set_font_size(self.font_size);
                        self.request_redraw();
                    }
                    Key::Character(ref ch) if ch == "-" => {
                        self.font_size = (self.font_size - 2.0).max(12.0);
                        self.layout.set_font_size(self.font_size);
                        self.request_redraw();
                    }
                    Key::Character(ref ch) if ch == "0" => {
                        self.font_size = 48.0;
                        self.layout.set_font_size(self.font_size);
                        self.request_redraw();
                    }
                    _ => {}
                }
            }

            WindowEvent::Resized(new_size) => {
                if let Some(ref mut window) = self.window {
                    window.resize((new_size.width, new_size.height));
                }
                self.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                self.render();
            }

            _ => {}
        }
    }
}

impl AppState {
    fn new(font_size: f32, text: &str) -> Self {
        let mut layout = LayoutEngine::new(font_size);
        layout.set_text(text);
        Self {
            window: None,
            painter: None,
            layout,
            font_size,
        }
    }

    fn request_redraw(&self) {
        if let Some(ref window) = self.window {
            window.window.request_redraw();
        }
    }

    fn render(&mut self) {
        let Some(ref mut window) = self.window else {
            return;
        };
        let Some(ref mut painter) = self.painter else {
            return;
        };

        let surface_texture = match window.begin_frame() {
            Ok(t) => t,
            Err(_) => return,
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let _ = painter.draw(
            &window.device,
            &window.queue,
            &view,
            &mut self.layout,
            window.size.0,
            window.size.1,
        );

        surface_texture.present();
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "autocue=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run {
            file,
            clipboard: _,
            mode: _,
            speed: _,
            font_size,
            mirror: _,
        } => {
            let fs = font_size.unwrap_or(48.0);

            let text = if let Some(ref path) = file {
                if path == "-" {
                    tracing::info!("Reading from stdin...");
                    use std::io::Read;
                    let mut buf = String::new();
                    std::io::stdin()
                        .read_to_string(&mut buf)
                        .unwrap_or_default();
                    buf
                } else {
                    tracing::info!("Loading: {path}");
                    std::fs::read_to_string(path).unwrap_or_else(|e| {
                        tracing::error!("Failed to read {path}: {e}");
                        String::from("Failed to load file.")
                    })
                }
            } else {
                tracing::info!("No file specified — showing demo text.");
                String::from(
                    "VibeRustAutocue\n\n\
                     Welcome! This is a cross-platform teleprompter.\n\n\
                     Press Ctrl+= and Ctrl+- to adjust font size.\n\
                     Press Esc to exit.\n\n\
                     Happy presenting! 🎤",
                )
            };

            tracing::info!(
                "Starting autocue: font_size={fs}, text_length={}",
                text.len()
            );

            let mut app = AppState::new(fs, &text);

            let event_loop = winit::event_loop::EventLoop::new().unwrap();
            let _ = event_loop.run_app(&mut app);
        }

        Command::Init => {
            tracing::info!("Generating default autocue.toml...");
            tracing::warn!("Config generation not yet implemented (Phase 4)");
        }

        Command::Check { file } => {
            tracing::info!("Checking script: {file}");
            match autocue_core::loader::load_file(std::path::Path::new(&file)) {
                Ok(script) => {
                    println!("✓ File: {file}");
                    println!("  Paragraphs: {}", script.paragraphs.len());
                    println!("  Headings:   {}", script.headings.len());
                    println!("  Characters: {}", script.total_chars);
                    println!("  Est. duration: {:.1}s", script.estimated_duration);
                }
                Err(e) => {
                    tracing::error!("{e}");
                }
            }
        }
    }
}
