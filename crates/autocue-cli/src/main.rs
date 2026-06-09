//! autocue-cli — 命令行入口
//!
//! VibeRustAutocue 的二进制入口，负责解析 CLI 参数并编排各层。

use clap::{Parser, Subcommand};

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
    Check {
        /// Path to the script file
        file: String,
    },
}

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
            clipboard,
            mode,
            speed,
            font_size,
            mirror,
        } => {
            tracing::info!(
                "Starting autocue: mode={mode}, speed={speed}, clipboard={clipboard}, mirror={mirror}"
            );
            if let Some(ref f) = file {
                tracing::info!("Script file: {f}");
            }
            if let Some(fs) = font_size {
                tracing::info!("Font size override: {fs}pt");
            }
            // TODO: Phase 3 — wire up engine + render + input event loop
            tracing::warn!("Teleprompter window not yet implemented (Phase 3)");
        }
        Command::Init => {
            tracing::info!("Generating default autocue.toml...");
            // TODO: Phase 4 — write default config
            tracing::warn!("Config generation not yet implemented (Phase 4)");
        }
        Command::Check { file } => {
            tracing::info!("Checking script: {file}");
            // TODO: Phase 1 — implement loader and validation
            tracing::warn!("Script validation not yet implemented (Phase 1)");
        }
    }
}
