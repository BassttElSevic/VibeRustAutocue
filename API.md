# VibeRustAutocue — API 接口文档

> 本文档记录项目中所有公开的类型、trait、函数及其用法和实现要点。
> 版本随代码同步更新。当前版本: **draft v0.1**

---

## 目录

1. [autocue-core — 核心引擎](#1-autocue-core--核心引擎)
2. [autocue-config — 配置层](#2-autocue-config--配置层)
3. [autocue-render — 渲染层](#3-autocue-render--渲染层)
4. [autocue-input — 输入控制](#4-autocue-input--输入控制)
5. [autocue-cli — CLI 入口](#5-autocue-cli--cli-入口)
6. [前后端通信契约](#6-前后端通信契约)

---

## 1. autocue-core — 核心引擎

> **crate 定位**: 后端。不依赖任何 GUI crate。所有类型和 trait 在此定义。

### 1.1 Error 类型

```rust
/// autocue_core::Error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to load script: {0}")]
    Load(String),

    #[error("tokenization error: {0}")]
    Tokenize(String),

    #[error("scroll error: {0}")]
    Scroll(String),
}

/// autocue_core::Result<T>
pub type Result<T> = std::result::Result<T, Error>;
```

**用法**：所有 core 模块的公开函数返回 `Result<T>`，调用方用 `?` 传播。

---

### 1.2 数据模型 (`script.rs`)

#### DisplayMode

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayMode {
    /// 匀速滚动 — 文本自动向上流动
    Scroll,
    /// 逐块推进 — 一次一屏，手动切块
    Chunk,
    /// 聚焦行 — 只高亮当前句
    Focus,
}

impl DisplayMode {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self>;
    /// 转为配置字符串
    pub fn as_str(&self) -> &'static str;
}
```

#### Heading

```rust
/// Markdown 标题节点
#[derive(Debug, Clone)]
pub struct Heading {
    /// 层级: 1 = `#`, 2 = `##`, 3 = `###`
    pub level: u8,
    /// 标题文本（不含 `#` 前缀）
    pub text: String,
    /// 在原稿中的起始行号 (0-based)
    pub line: usize,
}
```

#### Paragraph

```rust
/// 文稿中的一个段落
#[derive(Debug, Clone)]
pub struct Paragraph {
    /// 段落文本
    pub text: String,
    /// 是否为空行
    pub is_blank: bool,
    /// 关联的标题（最内层 `##`），无标题则为 None
    pub heading: Option<Heading>,
}
```

#### Chunk

```rust
/// 分词后适合屏幕显示的"块"
#[derive(Debug, Clone)]
pub struct Chunk {
    /// 块包含的行（已按屏幕宽度断行）
    pub lines: Vec<String>,
    /// 预估朗读时长 (秒)，基于字数 ÷ 速度
    pub duration_estimate: f64,
    /// 所属标题
    pub heading: Option<Heading>,
    /// 在原稿中的起始字符偏移
    pub start_offset: usize,
}
```

#### Script

```rust
/// 解析后的完整文稿
#[derive(Debug, Clone)]
pub struct Script {
    /// 文件路径或来源标识
    pub source: String,
    /// 原始全文
    pub raw_text: String,
    /// 段落列表
    pub paragraphs: Vec<Paragraph>,
    /// 标题层级结构
    pub headings: Vec<Heading>,
    /// 总字符数
    pub total_chars: usize,
    /// 预估总时长 (秒)
    pub estimated_duration: f64,
}
```

#### Marker

```rust
/// 书签 / 标记点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marker {
    /// 书签名称
    pub name: String,
    /// 在文稿中的字符偏移
    pub offset: usize,
    /// 创建时间
    pub created_at: std::time::SystemTime,
}
```

---

### 1.3 Engine trait (`lib.rs`)

前后端通信的核心契约。定义在 `autocue-core`，由 CLI 编排层提供具体实现。

```rust
/// 引擎对外暴露的只读状态快照
#[derive(Debug, Clone)]
pub struct EngineState {
    /// 当前显示的 Chunk
    pub current_chunk: Chunk,
    /// 全局进度 0.0 ~ 1.0
    pub progress: f32,
    /// 当前速度 (字/秒)
    pub speed: f64,
    /// 是否正在播放
    pub is_playing: bool,
    /// 当前所属标题 (.md)
    pub current_heading: Option<String>,
    /// 当前展示模式
    pub mode: DisplayMode,
    /// 当前字号 (pt)
    pub font_size: f32,
}

/// 前端向后端发送的命令
#[derive(Debug, Clone)]
pub enum EngineCommand {
    Play,
    Pause,
    TogglePlay,
    /// 前进 n 句
    SeekForward(usize),
    /// 后退 n 句
    SeekBackward(usize),
    /// 设置速度 (字/秒)
    SetSpeed(f64),
    /// 切换展示模式
    SetMode(DisplayMode),
    /// 跳到下一个标题
    NextHeading,
    /// 跳到上一个标题
    PrevHeading,
    /// 调节字号
    SetFontSize(f32),
}

/// 引擎 trait
///
/// # 实现者
/// CLI 编排层提供具体 struct 实现此 trait
///
/// # 调用者
/// - `autocue-render` 通过 `engine.state()` 获取渲染数据
/// - `autocue-input` 通过 `engine.send_command()` 发送操作
pub trait Engine {
    /// 获取当前只读状态
    fn state(&self) -> &EngineState;

    /// 发送操作命令
    fn send_command(&mut self, cmd: EngineCommand);

    /// 每帧 tick，delta 为距上一帧的秒数
    ///
    /// 内部更新滚动位置，重新计算 EngineState
    fn tick(&mut self, delta: f64);

    /// 加载文稿
    fn load(&mut self, script: Script) -> Result<()>;

    /// 根据当前窗口宽度重新分词
    fn relayout(&mut self, viewport_width: f32, font_size: f32) -> Result<()>;
}
```

**使用示例**：

```rust
// CLI 编排层
let mut engine = MyEngine::new(config);
engine.load(script)?;
engine.send_command(EngineCommand::Play);

// 事件循环
loop {
    let delta = frame_timer.elapsed();
    engine.tick(delta);
    let state = engine.state();
    renderer.draw(state);  // 前端消费
}
```

---

### 1.4 文稿加载器 (`loader.rs`)

```rust
/// 从文件路径加载文稿，自动检测格式
///
/// 支持格式: .txt, .md, .docx
/// 对于未知扩展名，默认按 .txt 处理
pub fn load_file(path: &std::path::Path) -> Result<Script>;

/// 从系统剪贴板加载文稿
///
/// 依赖 `arboard` crate，跨平台
pub fn load_clipboard() -> Result<Script>;

/// 从 stdin 读取文稿
pub fn load_stdin() -> Result<Script>;

/// 从原始字符串创建 Script（用于测试和程序化调用）
pub fn from_string(source: &str, text: String) -> Script;
```

**实现要点**：
- `.txt` — 直接 `std::fs::read_to_string`，按双换行分段
- `.md` — 同上，额外用正则 `^(#{1,6})\s+(.+)$` 解析标题，构建 `headings[]`
- `.docx` — 用 `quick-xml` 解析 ZIP 内的 `word/document.xml`，提取 `<w:p>` → `<w:t>` 文本节点
- 所有 loader 输出统一的 `Script` 结构

---

### 1.5 分词器 (`tokenizer.rs`)

```rust
/// 分词器
pub struct Tokenizer {
    /// 视口宽度 (像素)
    viewport_width: f32,
    /// 字号
    font_size: f32,
}

impl Tokenizer {
    /// 创建分词器
    pub fn new(viewport_width: f32, font_size: f32) -> Self;

    /// 将 Script 切分为 Chunk 列表
    ///
    /// 按屏幕宽度、字体大小进行中英文智能断行。
    /// 每个 Chunk 的 lines 数量取决于视口能容纳的行数。
    pub fn tokenize(&self, script: &Script) -> Result<Vec<Chunk>>;

    /// 更新视口参数并重新分词
    pub fn relayout(&mut self, viewport_width: f32, font_size: f32);
}
```

**实现要点**：
- 中文按字断行（CJK 字符边界判断用 Unicode 范围：`\u{4E00}-\u{9FFF}` 等）
- 英文按词断行（空格边界）
- 每行最大字符宽度 = `viewport_width / (font_size * 0.6)` 估算
- 每 Chunk 行数 = `viewport_height / (font_size * line_spacing)` 估算
- `duration_estimate` = chunk 总字符数 ÷ 当前速度

---

### 1.6 滚动状态机 (`scroll.rs`)

```rust
/// 滚动状态
pub struct ScrollState {
    /// 当前所在 Chunk 索引
    pub current_index: usize,
    /// 当前 Chunk 内的行偏移
    pub line_offset: f32,
    /// 总 Chunk 数
    pub total_chunks: usize,
}

impl ScrollState {
    pub fn new(total_chunks: usize) -> Self;

    /// 向前滚动 delta 秒
    pub fn advance(&mut self, delta: f64, speed: f64);

    /// 跳转到指定 Chunk
    pub fn seek_to(&mut self, index: usize);

    /// 进度 0.0 ~ 1.0
    pub fn progress(&self) -> f32;
}
```

**实现要点**：
- `advance(delta, speed)`: 计算经过的字符数 = `delta * speed`，根据每个 Chunk 的字符数判断跨越了几个 Chunk
- 边界处理：到达末尾后 `current_index` 不再增长，外部检查 `progress() >= 1.0` 触发 `Finished` 状态
- `line_offset` 用于平滑滚动动画（匀速滚动模式下的像素级偏移）

---

### 1.7 书签管理 (`marker.rs`)

```rust
/// 书签管理器
pub struct MarkerManager {
    markers: Vec<Marker>,
}

impl MarkerManager {
    pub fn new() -> Self;

    /// 添加书签
    pub fn add(&mut self, name: String, offset: usize);

    /// 删除书签
    pub fn remove(&mut self, name: &str) -> Option<Marker>;

    /// 查找指定偏移之后的下一个书签
    pub fn next_after(&self, offset: usize) -> Option<&Marker>;

    /// 查找指定偏移之前的上一个书签
    pub fn prev_before(&self, offset: usize) -> Option<&Marker>;

    /// 列出所有书签
    pub fn list(&self) -> &[Marker];
}
```

---

## 2. autocue-config — 配置层

> **crate 定位**: 后端。纯数据解析，不依赖 GUI。

### 2.1 Config 结构体

```rust
/// 完整配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub window: WindowConfig,
    pub display: DisplayConfig,
    pub heading_bar: HeadingBarConfig,
    pub scroll: ScrollConfig,
    pub input: InputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub transparent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub font_family: String,
    pub font_size: f32,
    pub font_size_min: f32,
    pub font_size_max: f32,
    pub font_size_step: f32,
    pub line_spacing: f32,
    pub default_mode: DisplayMode,
    pub mirror: bool,
    pub bg_color: String,
    pub fg_color: String,
    pub highlight_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadingBarConfig {
    pub enabled: bool,
    pub font_size_ratio: f32,
    pub bg_color: String,
    pub fg_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollConfig {
    pub default_speed: f64,
    pub smooth: bool,
    pub margin_top: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub keyboard_shortcuts: bool,
    pub websocket_enabled: bool,
    pub websocket_bind: String,
}
```

### 2.2 公开函数

```rust
/// 从文件加载配置
pub fn load_config(path: &std::path::Path) -> Result<Config>;

/// 获取默认配置
pub fn default_config() -> Config;

/// 合并 CLI 参数到配置
pub fn merge_cli_args(config: &mut Config, args: &CliArgs);

/// 验证配置合法性
pub fn validate(config: &Config) -> Result<()>;
```

### 2.3 主题 (`theme.rs`)

```rust
/// 预设主题
pub enum ThemePreset {
    /// 黑底白字（默认）
    Dark,
    /// 白底黑字
    Light,
    /// 绿底白字（经典提词器配色）
    Classic,
}

impl ThemePreset {
    /// 应用主题到 Config
    pub fn apply(&self, config: &mut Config);
}
```

---

## 3. autocue-render — 渲染层

> **crate 定位**: 前端。依赖 winit, wgpu, cosmic-text。

### 3.1 窗口 (`window.rs`)

```rust
/// winit 窗口封装
pub struct Window {
    window: winit::window::Window,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl Window {
    /// 创建窗口和 wgpu 上下文
    pub async fn new(window_config: &WindowConfig, title: &str) -> Result<Self>;

    /// 获取窗口尺寸
    pub fn size(&self) -> (u32, u32);

    /// 设置全屏
    pub fn set_fullscreen(&mut self, fullscreen: bool);

    /// 开始一帧渲染
    pub fn begin_frame(&mut self) -> Result<wgpu::SurfaceTexture>;
}
```

### 3.2 文本排版 (`layout.rs`)

```rust
/// cosmic-text 排版引擎
pub struct LayoutEngine {
    font_system: cosmic_text::FontSystem,
    buffer: cosmic_text::Buffer,
}

impl LayoutEngine {
    pub fn new() -> Self;

    /// 排版一个 Chunk
    pub fn layout(&mut self, chunk: &Chunk, font_size: f32, fg_color: [f32; 4]);

    /// 设置字体
    pub fn set_font_family(&mut self, family: &str);

    /// 获取渲染用的 glyph 数据
    pub fn glyph_run(&self) -> &[GlyphRun];
}
```

### 3.3 绘制器 (`painter.rs`)

```rust
/// wgpu 绘制器
pub struct Painter {
    pipeline: wgpu::RenderPipeline,
    // ...
}

impl Painter {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self;

    /// 将 LayoutEngine 的输出绘制到屏幕
    pub fn draw(&mut self, view: &wgpu::TextureView, layout: &LayoutEngine);
}
```

### 3.4 展示模式调度 (`display.rs`)

```rust
/// 展示模式控制器
pub struct DisplayController {
    mode: DisplayMode,
}

impl DisplayController {
    pub fn new(mode: DisplayMode) -> Self;

    /// 计算当前应渲染的文本区域
    ///
    /// 根据模式和 EngineState 决定哪些行可见、哪些高亮。
    pub fn compute_visible(
        &self,
        chunk: &Chunk,
        state: &EngineState,
        viewport_lines: usize,
    ) -> VisibleRegion;
}

/// 计算出的可见区域
pub struct VisibleRegion {
    /// 可见行索引范围
    pub line_range: std::ops::Range<usize>,
    /// 高亮行索引
    pub highlight_line: Option<usize>,
    /// 滚动偏移 (像素)
    pub scroll_offset: f32,
}
```

**实现要点**：
- **Scroll 模式**: 阅读线固定在 `margin_top` 位置，`scroll_offset` 用 `line_offset` 平滑插值
- **Chunk 模式**: `line_range` 覆盖整个 Chunk，无高亮行
- **Focus 模式**: `highlight_line` = 当前句所在行，上下各显示 context_lines 行

### 3.5 标题叠加条 (`heading_bar.rs`)

```rust
/// .md 标题叠加条
pub struct HeadingBar {
    visible: bool,
    text: String,
    config: HeadingBarConfig,
}

impl HeadingBar {
    pub fn new(config: &HeadingBarConfig) -> Self;

    /// 更新当前标题
    pub fn update(&mut self, heading: Option<&str>);

    /// 渲染到屏幕顶部
    pub fn draw(&self, layout: &mut LayoutEngine, viewport_width: f32);
}
```

**实现要点**：
- 固定在视口顶部，高度 = `font_size * font_size_ratio * 1.5`
- 背景用 `bg_color` 指定的半透明色
- 正文区域向下偏移标题栏高度

### 3.6 镜像翻转 (`mirror.rs`)

```rust
/// 镜像模式控制
pub struct MirrorMode {
    enabled: bool,
}

impl MirrorMode {
    pub fn new(enabled: bool) -> Self;
    pub fn toggle(&mut self);
    pub fn is_enabled(&self) -> bool;

    /// 应用到 wgpu 渲染管线
    pub fn apply_to_pipeline(&self, pipeline: &mut wgpu::RenderPipeline);
}
```

**实现要点**：
- 在片元着色器中翻转 UV 坐标：`tex_coords.x = 1.0 - tex_coords.x`
- 也可以通过投影矩阵的 `scale(-1.0, 1.0, 1.0)` 实现

---

## 4. autocue-input — 输入控制

> **crate 定位**: 前端。处理键盘和远程输入，映射为 EngineCommand。

### 4.1 键盘处理 (`keyboard.rs`)

```rust
/// 键盘事件 → EngineCommand 映射
pub struct KeyboardHandler {
    shortcuts_enabled: bool,
}

impl KeyboardHandler {
    pub fn new(shortcuts_enabled: bool) -> Self;

    /// 将 winit KeyEvent 映射为可选的 EngineCommand
    ///
    /// 返回 None 表示该按键无映射
    pub fn handle_key(&self, event: &winit::event::KeyEvent) -> Option<EngineCommand>;
}
```

**按键映射表**（定义在 `handle_key` 内部）：

| winit Key | Modifiers | EngineCommand |
|-----------|-----------|---------------|
| `Space` | — | `TogglePlay` |
| `ArrowRight` | — | `SeekForward(1)` |
| `ArrowLeft` | — | `SeekBackward(1)` |
| `ArrowUp` | — | `SetSpeed(speed * 1.1)` |
| `ArrowDown` | — | `SetSpeed(speed * 0.9)` |
| `Equal` | Ctrl | `SetFontSize(font_size + step)` |
| `Minus` | Ctrl | `SetFontSize(font_size - step)` |
| `Equal` | Ctrl+Shift | `SetFontSize(font_size + 10.0)` |
| `Minus` | Ctrl+Shift | `SetFontSize(font_size - 10.0)` |
| `Digit0` | Ctrl | `SetFontSize(default)` |
| `Digit1` | Ctrl | `SetMode(Scroll)` |
| `Digit2` | Ctrl | `SetMode(Chunk)` |
| `Digit3` | Ctrl | `SetMode(Focus)` |
| `ArrowUp` | Ctrl | `PrevHeading` |
| `ArrowDown` | Ctrl | `NextHeading` |
| `ArrowLeft` | Ctrl | `SeekBackward(prev_marker)` |
| `ArrowRight` | Ctrl | `SeekForward(next_marker)` |
| `KeyR` | Ctrl | *镜像切换 (前端本地处理)* |
| `F11` | — | *全屏切换 (前端本地处理)* |
| `Escape` | — | *退出 (前端本地处理)* |

### 4.2 远程控制 (`remote.rs`)

```rust
/// WebSocket 远程控制服务
pub struct RemoteServer {
    bind_addr: String,
}

impl RemoteServer {
    pub fn new(bind_addr: &str) -> Self;

    /// 启动 WebSocket 服务（非阻塞，在后台任务中运行）
    pub fn start(&self) -> Result<tokio::sync::mpsc::Receiver<EngineCommand>>;
}
```

**WebSocket 指令格式 (JSON)**：

```json
{"cmd": "play"}
{"cmd": "pause"}
{"cmd": "toggle_play"}
{"cmd": "seek", "n": 1}
{"cmd": "seek", "n": -1}
{"cmd": "set_speed", "value": 6.0}
{"cmd": "set_mode", "value": "scroll"}
{"cmd": "set_font_size", "value": 72}
{"cmd": "next_heading"}
{"cmd": "prev_heading"}
```

---

## 5. autocue-cli — CLI 入口

> **crate 定位**: 编排层。组合前后端，启动事件循环。

### 5.1 CLI 参数结构

```rust
#[derive(Parser)]
#[command(name = "autocue", version, about)]
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
```

### 5.2 启动流程

```
main()
  ├── 初始化 tracing subscriber
  ├── 解析 CLI 参数 (clap)
  ├── 加载配置 (autocue-config)
  │   └── 合并 CLI 覆盖
  ├── 创建 Engine 实现 (autocue-core)
  │   ├── 加载 Script (loader)
  │   └── 分词 (tokenizer)
  ├── 创建 Window (autocue-render)
  ├── 创建 KeyboardHandler (autocue-input)
  │
  └── 事件循环 (winit EventLoop)
      ├── WindowEvent::RedrawRequested
      │   ├── engine.tick(delta)
      │   ├── DisplayController::compute_visible()
      │   └── Painter::draw()
      │
      ├── WindowEvent::KeyboardInput
      │   ├── KeyboardHandler::handle_key() → EngineCommand
      │   └── engine.send_command(cmd)
      │
      └── WindowEvent::CloseRequested
          └── 退出
```

---

## 6. 前后端通信契约

### 6.1 依赖规则

```
✅ 允许的依赖方向:
   render → core    (前端依赖后端 trait)
   input → core     (前端依赖后端 trait)
   cli → core       (编排层拥有后端实现)
   cli → render     (编排层拥有前端实现)
   cli → input      (编排层拥有前端实现)

❌ 禁止的依赖方向:
   core → render    (后端不碰 GUI)
   core → input     (后端不碰 GUI)
   config → render  (后端不碰 GUI)
   config → input   (后端不碰 GUI)
```

### 6.2 数据流方向

```
命令流:  Input ──(EngineCommand)──→ Engine
状态流:  Engine ──(EngineState)───→ Render
```

### 6.3 可替换性验证

换用不同 GUI 框架时，只需重写 `autocue-render`：

| 方案 | 需改动的 crate | 不动 |
|------|---------------|------|
| winit + wgpu → egui + wgpu | `autocue-render` | core, config, input |
| winit + wgpu → iced | `autocue-render` | core, config, input |
| winit + wgpu → TUI (ratatui) | `autocue-render` + `autocue-cli` | core, config |

---

> 本文档随代码实现同步更新。当模块中新增公开类型/函数时，立即在此文档中补充。
