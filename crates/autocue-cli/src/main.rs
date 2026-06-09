//! autocue-cli — 命令行入口
//!
//! VibeRustAutocue 的二进制入口。
//! 组合前后端：Engine + Render + Input → 事件循环。

use autocue_core::{
    loader, DisplayMode, EngineCommand,
    marker::MarkerManager,
    script::Script,
    scroll::ScrollState,
    tokenizer::Tokenizer,
};
use autocue_input::keyboard::KeyboardHandler;
use autocue_render::{
    LayoutEngine, Painter, RenderWindow,
    display::DisplayController,
    heading_bar::HeadingBar,
};
use clap::{Parser, Subcommand};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};

#[derive(Parser)]
#[command(name = "autocue", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Run {
        file: Option<String>,
        #[arg(long)]
        clipboard: bool,
        #[arg(long, default_value = "scroll")]
        mode: String,
        #[arg(long, default_value = "5.0")]
        speed: f64,
        #[arg(long)]
        font_size: Option<f32>,
        #[arg(long)]
        mirror: bool,
    },
    Init,
    Check { file: String },
}

// ---------------------------------------------------------------------------
// Engine — 后端核心（实现于 CLI 层，组合 autocue-core 各模块）
// ---------------------------------------------------------------------------

struct AppEngine {
    script: Option<Script>,
    chunks: Vec<autocue_core::script::Chunk>,
    scroll: Option<ScrollState>,
    markers: MarkerManager,
    tokenizer: Tokenizer,
    speed: f64,
    is_playing: bool,
    mode: DisplayMode,
    font_size: f32,
    viewport_width: f32,
    viewport_height: f32,
}

impl AppEngine {
    fn new(viewport_width: f32, viewport_height: f32, font_size: f32, speed: f64, mode: DisplayMode) -> Self {
        Self {
            script: None,
            chunks: vec![],
            scroll: None,
            markers: MarkerManager::new(),
            tokenizer: Tokenizer::new(viewport_width, viewport_height, font_size),
            speed,
            is_playing: false,
            mode,
            font_size,
            viewport_width,
            viewport_height,
        }
    }

    fn load(&mut self, script: Script) {
        self.chunks = self.tokenizer.tokenize(&script).unwrap_or_default();

        let counts: Vec<usize> = self.chunks.iter().map(|c| c.char_count).collect();
        self.scroll = Some(ScrollState::new(counts));
        self.script = Some(script);
    }

    fn send_command(&mut self, cmd: EngineCommand) {
        match cmd {
            EngineCommand::Play => self.is_playing = true,
            EngineCommand::Pause => self.is_playing = false,
            EngineCommand::TogglePlay => self.is_playing = !self.is_playing,
            EngineCommand::SeekForward(n) => {
                if let Some(ref mut s) = self.scroll {
                    s.seek_forward(n);
                }
            }
            EngineCommand::SeekBackward(n) => {
                if let Some(ref mut s) = self.scroll {
                    s.seek_backward(n);
                }
            }
            EngineCommand::SetSpeed(s) => self.speed = s,
            EngineCommand::SetMode(m) => self.mode = m,
            EngineCommand::NextHeading => { /* TODO */ }
            EngineCommand::PrevHeading => { /* TODO */ }
            EngineCommand::SetFontSize(fs) => {
                self.font_size = fs;
                // 重新分词
                self.tokenizer.relayout(self.viewport_width, self.viewport_height, fs);
                if let Some(ref script) = self.script {
                    self.chunks = self.tokenizer.tokenize(script).unwrap_or_default();
                    let counts: Vec<usize> = self.chunks.iter().map(|c| c.char_count).collect();
                    // 保留当前位置，仅更新总 chunk 数
                    if let Some(ref mut s) = self.scroll {
                        let old_idx = s.current_index;
                        *s = ScrollState::new(counts);
                        s.seek_to(old_idx.min(s.total_chunks.saturating_sub(1)));
                    }
                }
            }
        }
    }

    fn tick(&mut self, delta: f64) {
        if self.is_playing {
            if let Some(ref mut s) = self.scroll {
                s.advance(delta, self.speed);
            }
        }
    }

    fn scroll_state(&self) -> Option<&ScrollState> {
        self.scroll.as_ref()
    }

    fn chunks(&self) -> &[autocue_core::script::Chunk] {
        &self.chunks
    }

    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn current_heading(&self) -> Option<&str> {
        let idx = self.scroll.as_ref().map(|s| s.current_index).unwrap_or(0);
        self.chunks.get(idx).and_then(|c| c.heading.as_ref()).map(|h| h.text.as_str())
    }

    fn update_viewport(&mut self, w: u32, h: u32) {
        self.viewport_width = w as f32;
        self.viewport_height = h as f32;
        self.tokenizer.relayout(self.viewport_width, self.viewport_height, self.font_size);
    }
}

// ---------------------------------------------------------------------------
// Application state (winit ApplicationHandler)
// ---------------------------------------------------------------------------

struct AppState {
    window: Option<RenderWindow>,
    painter: Option<Painter>,
    layout: LayoutEngine,
    engine: AppEngine,
    keyboard: KeyboardHandler,
    display: DisplayController,
    heading_bar: HeadingBar,
    font_size: f32,
    last_frame: std::time::Instant,
}

impl ApplicationHandler for AppState {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let rw = pollster::block_on(RenderWindow::new(event_loop, 800, 600, "VibeRustAutocue"));
        match rw {
            Ok(mut window) => {
                let format = window.config.format;
                let painter = Painter::new(&window.device, format);
                self.engine.update_viewport(window.size.0, window.size.1);
                self.painter = Some(painter);
                window.window.request_redraw();
                self.window = Some(window);
                self.last_frame = std::time::Instant::now();
            }
            Err(e) => {
                tracing::error!("Failed to create window: {e}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::KeyboardInput {
                event: ref key_event, ..
            } => {
                // Esc 直接退出
                if key_event.state == ElementState::Pressed {
                    if let Key::Named(NamedKey::Escape) = &key_event.logical_key {
                        event_loop.exit();
                        return;
                    }
                }

                // 通过 KeyboardHandler 映射为 EngineCommand
                if let Some(cmd) = self.keyboard.handle_key(key_event) {
                    // 字体缩放也在本地处理（直接重排文本）
                    match &cmd {
                        EngineCommand::SetFontSize(fs) => {
                            self.font_size = *fs;
                            self.layout.set_font_size(*fs);
                        }
                        _ => {}
                    }
                    self.engine.send_command(cmd);
                    self.request_redraw();
                }
            }

            WindowEvent::Resized(new_size) => {
                if let Some(ref mut window) = self.window {
                    window.resize((new_size.width, new_size.height));
                }
                self.engine.update_viewport(new_size.width, new_size.height);
                self.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let delta = (now - self.last_frame).as_secs_f64().min(0.1);
                self.last_frame = now;

                self.engine.tick(delta);
                self.render();
            }

            _ => {}
        }
    }
}

impl AppState {
    fn new(
        font_size: f32,
        text: &str,
        script: Script,
        speed: f64,
        mode: DisplayMode,
    ) -> Self {
        let mut layout = LayoutEngine::new(font_size);
        layout.set_text(text);

        let mut engine = AppEngine::new(800.0, 600.0, font_size, speed, mode);
        engine.load(script);

        let keyboard = KeyboardHandler::new(true);
        let display = DisplayController::new(mode);
        let heading_bar = HeadingBar::new(0.6);

        Self {
            window: None,
            painter: None,
            layout,
            engine,
            keyboard,
            display,
            heading_bar,
            font_size,
            last_frame: std::time::Instant::now(),
        }
    }

    fn request_redraw(&self) {
        if let Some(ref window) = self.window {
            window.window.request_redraw();
        }
    }

    fn render(&mut self) {
        // 阶段 1: 收集显示数据（仅不可变借用）
        let heading = self.engine.current_heading().map(|s| s.to_string());
        self.heading_bar.update(heading.as_deref());

        let vp_lines = self.visible_lines();
        let mut display_text = String::new();
        if self.heading_bar.visible && !self.heading_bar.text.is_empty() {
            display_text.push_str("▸ ");
            display_text.push_str(&self.heading_bar.text);
            display_text.push_str("\n\n");
        }
        let is_playing = self.engine.is_playing();
        let engine_speed = self.engine.speed;
        let fs = self.font_size;

        if let Some(scroll) = self.engine.scroll_state() {
            let idx = scroll.current_index;
            let visible = self.display.compute_visible(scroll, self.engine.chunks(), vp_lines);
            if let Some(c) = self.engine.chunks().get(idx) {
                for i in visible.line_range {
                    if let Some(line) = c.lines.get(i) {
                        display_text.push_str(line);
                        display_text.push('\n');
                    }
                }
            }
        }
        if display_text.is_empty() {
            display_text = "— End —".into();
        }
        let status = format!(
            "  {} | speed: {:.1}c/s | size: {:.0}pt",
            if is_playing { "▶" } else { "⏸" },
            engine_speed, fs,
        );
        display_text.push_str(&status);

        // 阶段 2: 更新 layout 和渲染（此时不再借用 engine 等）
        self.layout.set_text(&display_text);

        let Some(ref mut window) = self.window else { return };
        let Some(ref mut painter) = self.painter else { return };
        let surface_tex = match window.begin_frame() {
            Ok(t) => t,
            Err(_) => return,
        };
        let view = surface_tex.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let _ = painter.draw(&window.device, &window.queue, &view, &mut self.layout, window.size.0, window.size.1);
        surface_tex.present();
    }

    fn visible_lines(&self) -> usize {
        let Some(ref window) = self.window else { return 15 };
        let heading_offset = if self.heading_bar.visible { 3 } else { 0 };
        let line_h = (self.font_size * 1.5) as u32;
        if line_h == 0 { return 15; }
        ((window.size.1 / line_h).saturating_sub(heading_offset).max(1)) as usize
    }
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "autocue=info".into()))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run { file, clipboard: _, mode, speed, font_size, mirror: _ } => {
            let fs = font_size.unwrap_or(48.0);
            let dm = DisplayMode::from_str(&mode).unwrap_or(DisplayMode::Scroll);

            let (text, script) = if let Some(ref path) = file {
                if path == "-" {
                    use std::io::Read;
                    let mut buf = String::new();
                    std::io::stdin().read_to_string(&mut buf).unwrap_or_default();
                    let s = loader::from_string("stdin", buf.clone());
                    (buf, s)
                } else {
                    match loader::load_file(std::path::Path::new(path)) {
                        Ok(s) => {
                            let txt = s.raw_text.clone();
                            (txt, s)
                        }
                        Err(e) => {
                            tracing::error!("{e}");
                            let txt = format!("Error: {e}");
                            (txt.clone(), loader::from_string("error", txt))
                        }
                    }
                }
            } else {
                let txt: String = "VibeRustAutocue\n\nWelcome! Press Space to start, Esc to exit.\n↑↓ speed  =/- font size".into();
                (txt.clone(), loader::from_string("demo", txt))
            };

            let mut app = AppState::new(fs, &text, script, speed, dm);
            let event_loop = winit::event_loop::EventLoop::new().unwrap();
            let _ = event_loop.run_app(&mut app);
        }

        Command::Init => {
            tracing::warn!("Config generation not yet implemented (Phase 4)");
        }

        Command::Check { file } => {
            match loader::load_file(std::path::Path::new(&file)) {
                Ok(script) => {
                    println!("✓ {file}");
                    println!("  Paragraphs: {}", script.paragraphs.len());
                    println!("  Headings:   {}", script.headings.len());
                    println!("  Characters: {}", script.total_chars);
                    println!("  Est. duration: {:.1}s", script.estimated_duration);
                }
                Err(e) => tracing::error!("{e}"),
            }
        }
    }
}
