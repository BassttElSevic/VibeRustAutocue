//! 全局快捷键 — winit KeyEvent → EngineCommand 映射

use autocue_core::EngineCommand;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, NamedKey};

/// 键盘处理器 — 将 winit 按键事件映射为 EngineCommand
pub struct KeyboardHandler {
    shortcuts_enabled: bool,
    /// 当前速度缓存，用于 ↑↓ 相对调节
    current_speed: f64,
    /// 当前字号缓存，用于 =/- 相对调节
    current_font_size: f32,
}

impl KeyboardHandler {
    pub fn new(shortcuts_enabled: bool) -> Self {
        Self {
            shortcuts_enabled,
            current_speed: 5.0,
            current_font_size: 48.0,
        }
    }

    /// 更新内部缓存的速度值（由 Engine 回调同步）
    pub fn update_speed(&mut self, speed: f64) {
        self.current_speed = speed;
    }

    /// 更新内部缓存的字号（由 Engine 回调同步）
    pub fn update_font_size(&mut self, fs: f32) {
        self.current_font_size = fs;
    }

    /// 处理按键事件，返回可选的 EngineCommand
    ///
    /// 返回 None 表示该按键无映射或快捷键已禁用。
    pub fn handle_key(&mut self, event: &KeyEvent) -> Option<EngineCommand> {
        if !self.shortcuts_enabled {
            return None;
        }

        // 只处理按下事件
        if event.state != ElementState::Pressed {
            return None;
        }

        let key = &event.logical_key;

        match key {
            // --- 播放控制 ---
            Key::Named(NamedKey::Space) => Some(EngineCommand::TogglePlay),

            // --- 导航 ---
            Key::Named(NamedKey::ArrowRight) => Some(EngineCommand::SeekForward(1)),
            Key::Named(NamedKey::ArrowLeft) => Some(EngineCommand::SeekBackward(1)),
            Key::Named(NamedKey::ArrowDown) => {
                self.current_speed = (self.current_speed * 0.9).max(0.5);
                Some(EngineCommand::SetSpeed(self.current_speed))
            }
            Key::Named(NamedKey::ArrowUp) => {
                self.current_speed = (self.current_speed * 1.1).min(50.0);
                Some(EngineCommand::SetSpeed(self.current_speed))
            }

            // --- 字体大小 (Phase 2 已在 CLI 层直接处理，这里保留作为后备) ---
            Key::Character(ch) if ch == "=" || ch == "+" => {
                self.current_font_size = (self.current_font_size + 2.0).min(300.0);
                Some(EngineCommand::SetFontSize(self.current_font_size))
            }
            Key::Character(ch) if ch == "-" => {
                self.current_font_size = (self.current_font_size - 2.0).max(12.0);
                Some(EngineCommand::SetFontSize(self.current_font_size))
            }
            Key::Character(ch) if ch == "0" => {
                self.current_font_size = 48.0;
                Some(EngineCommand::SetFontSize(self.current_font_size))
            }

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_space_maps_to_toggle_play() {
        let mut handler = KeyboardHandler::new(true);
        // 无法在测试中构造 winit KeyEvent（需要平台上下文）
        // 此处仅验证结构创建成功
        assert!(handler.shortcuts_enabled);
    }

    #[test]
    fn test_shortcuts_disabled() {
        let mut handler = KeyboardHandler::new(false);
        assert!(!handler.shortcuts_enabled);
    }

    #[test]
    fn test_speed_update() {
        let mut handler = KeyboardHandler::new(true);
        handler.update_speed(10.0);
        assert_eq!(handler.current_speed, 10.0);
    }

    #[test]
    fn test_font_size_update() {
        let mut handler = KeyboardHandler::new(true);
        handler.update_font_size(72.0);
        assert_eq!(handler.current_font_size, 72.0);
    }
}
