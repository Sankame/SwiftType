pub mod app_ui;
pub mod settings_view;
pub mod snippet_editor;
pub mod tray;

use egui::{Context, Visuals};

/// UI関連の定数
pub mod constants {
    /// ウィンドウのタイトル
    pub const APP_TITLE: &str = "SwiftType";
    /// ウィンドウの幅
    pub const DEFAULT_WIDTH: f32 = 800.0;
    /// ウィンドウの高さ
    pub const DEFAULT_HEIGHT: f32 = 600.0;
}

/// テーマモード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    /// ライトモード
    Light,
    /// ダークモード
    Dark,
}

impl ThemeMode {
    /// テーマモードからEGUIのビジュアルを取得する
    pub fn to_visuals(self) -> Visuals {
        match self {
            ThemeMode::Light => Visuals::light(),
            ThemeMode::Dark => Visuals::dark(),
        }
    }
    
    /// 現在のテーマモードを切り替える
    pub fn toggle(&mut self) {
        *self = match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
    }
}

/// EGUIのコンテキストを設定する
pub fn setup_context(ctx: &Context, theme: ThemeMode) {
    ctx.set_visuals(theme.to_visuals());
    setup_fonts(ctx);
}

/// フォントを設定する
fn setup_fonts(ctx: &Context) {
    use egui::{FontFamily, FontId, TextStyle};
    
    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(22.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(14.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
    ].into();
    
    ctx.set_style(style);
} 