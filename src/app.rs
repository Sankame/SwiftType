use std::sync::{Arc, Mutex};
use eframe;
use std::sync::atomic::{AtomicBool, Ordering};
use once_cell::sync::Lazy;

use crate::config::ConfigManager;
use crate::keyboard::{KeyboardHook, KeyboardState};
use crate::replacement::ReplacementEngine;
use crate::ui::app_ui::{AppUi, AppUiState};
use crate::ui::tray::TrayIconState;
use crate::utils;

// 競合ツール検出時の警告表示フラグ
static SHOW_CONFLICT_WARNING: AtomicBool = AtomicBool::new(false);
// 検出された競合ツールの名前
static CONFLICTING_TOOL_NAMES: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

/// アプリケーション本体
pub struct App {
    /// UIの状態
    ui: AppUi,
    /// トレイアイコンの状態
    tray_state: Option<TrayIconState>,
    /// キーボードフック
    _keyboard_hook: KeyboardHook,
}

impl App {
    /// アプリケーションを初期化する
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, Box<dyn std::error::Error>> {
        // 競合するツールをチェック
        let conflicting_tools = utils::check_conflicting_tools();
        if !conflicting_tools.is_empty() {
            // 競合するツールが見つかった場合の警告メッセージを設定
            log::warn!("Conflicting text expansion tools found: {:?}", conflicting_tools);
            
            // 初期化後、最初のフレーム更新で警告を表示するためのフラグをセット
            SHOW_CONFLICT_WARNING.store(true, std::sync::atomic::Ordering::SeqCst);
            CONFLICTING_TOOL_NAMES.lock().unwrap().extend(conflicting_tools);
        }
        
        // 設定を読み込む
        let config_manager = Arc::new(Mutex::new(ConfigManager::new()?));
        
        // 設定を取得
        let settings = {
            let config_manager_guard = config_manager.lock().unwrap();
            let settings = config_manager_guard.get_settings().clone();
            Arc::new(Mutex::new(settings))
        };
        
        // キーボード状態を作成
        let keyboard_state = Arc::new(Mutex::new(KeyboardState::new(100)));
        
        // 置換エンジンを作成
        let replacement_engine = Arc::new(Mutex::new(ReplacementEngine::new(Arc::clone(&settings))));
        
        // UI状態を作成
        let ui_state = AppUiState::new(
            Arc::clone(&config_manager),
            Arc::clone(&settings),
            Arc::clone(&keyboard_state),
            Arc::clone(&replacement_engine),
        );
        let ui = AppUi::new(ui_state);
        
        // キーボードフックを作成
        let keyboard_hook = KeyboardHook::new(
            Arc::clone(&keyboard_state),
            Arc::clone(&replacement_engine),
        )?;
        
        // キーボードフックを開始
        keyboard_hook.start()?;
        
        // トレイアイコンを作成
        let tray_state = TrayIconState::new(Arc::clone(&settings)).ok();
        
        Ok(Self {
            ui,
            tray_state,
            _keyboard_hook: keyboard_hook,
        })
    }
    
    /// 自動起動の設定を更新する
    pub fn update_auto_startup(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(settings) = self.ui.settings().lock() {
            utils::set_auto_startup(settings.start_with_system)?;
        }
        Ok(())
    }
}

impl eframe::App for App {
    /// フレームを更新する
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // 競合ツールの警告を表示
        if SHOW_CONFLICT_WARNING.load(Ordering::SeqCst) {
            if let Ok(conflicting_tools) = CONFLICTING_TOOL_NAMES.lock() {
                if !conflicting_tools.is_empty() {
                    // 警告ダイアログを表示
                    egui::Window::new("Warning: Conflicting Tools Detected")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .show(ctx, |ui| {
                            ui.label("The following text expansion tools are currently running:");
                            ui.spacing();
                            
                            for tool in conflicting_tools.iter() {
                                ui.label(format!("• {}", tool));
                            }
                            
                            ui.spacing();
                            ui.label("Running multiple text expansion tools simultaneously may cause conflicts and unexpected behavior.");
                            ui.label("It is recommended to close these applications before using SwiftType.");
                            
                            ui.spacing();
                            if ui.button("Understood").clicked() {
                                // 警告を閉じる
                                SHOW_CONFLICT_WARNING.store(false, Ordering::SeqCst);
                            }
                        });
                }
            }
        }
        
        // トレイアイコンのイベントを処理
        if let Some(tray_state) = &mut self.tray_state {
            tray_state.process_events();
            
            // 終了フラグをチェック
            if utils::check_should_exit(&tray_state.should_exit) {
                frame.close();
                return;
            }
            
            // ウィンドウの表示/非表示を切り替え
            if let Ok(show_window) = tray_state.show_window.lock() {
                if !*show_window {
                    frame.set_visible(false);
                    return;
                } else {
                    frame.set_visible(true);
                }
            }
        }
        
        // UIを更新
        self.ui.update(ctx);
        
        // 自動再描画を設定
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }
} 