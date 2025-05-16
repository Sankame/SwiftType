mod app;
mod config;
mod keyboard;
mod replacement;
mod ui;
mod utils;

use eframe::egui;
use log::error;

use app::App;
use ui::constants;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ロガーを初期化（デバッグレベルで詳細なログを表示）
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    
    log::info!("Starting SwiftType application");
    
    // アプリケーションの設定を作成
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(
            constants::DEFAULT_WIDTH,
            constants::DEFAULT_HEIGHT,
        )),
        vsync: true,
        icon_data: None, // アイコンを追加したい場合はここで設定
        always_on_top: false,
        decorated: true,
        transparent: false,
        min_window_size: Some(egui::vec2(400.0, 300.0)),
        max_window_size: None,
        resizable: true,
        ..Default::default()
    };
    
    // アプリケーションを実行
    let result = eframe::run_native(
        constants::APP_TITLE,
        options,
        Box::new(|cc| Box::new(App::new(cc).expect("Failed to create app"))),
    );
    
    if let Err(err) = result {
        error!("Application error: {}", err);
        return Err(Box::new(err));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sample() {
        assert_eq!(2 + 2, 4);
    }
} 