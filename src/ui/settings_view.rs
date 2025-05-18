use egui::{self, Ui};
// 未使用のインポートを削除
// use std::sync::{Arc, Mutex};

use crate::config::{ConfigManager, Settings};

/// 設定画面を描画する
/// 
/// # 引数
/// * `ui` - EGUIのUIコンテキスト
/// * `settings` - アプリケーションの設定
/// * `config_manager` - 設定マネージャー
/// 
/// # 戻り値
/// 設定が変更されたかどうか
#[allow(dead_code)]
pub fn render_settings_view(
    ui: &mut Ui,
    settings: &mut Settings,
    config_manager: &mut ConfigManager,
) -> bool {
    let mut changed = false;
    
    ui.heading("Application Settings");
    ui.add_space(10.0);
    
    // アプリケーションの有効/無効
    let prev_enabled = settings.enabled;
    ui.checkbox(&mut settings.enabled, "Enable SwiftType");
    if prev_enabled != settings.enabled {
        changed = true;
    }
    
    // 自動起動
    let prev_autostart = settings.start_with_system;
    ui.checkbox(&mut settings.start_with_system, "Start with system");
    if prev_autostart != settings.start_with_system {
        changed = true;
    }
    
    ui.separator();
    ui.heading("Hotkeys");
    ui.add_space(10.0);
    
    // ホットキーの設定
    ui.label("Toggle Hotkey: Not implemented yet");
    ui.label("Open Window Hotkey: Not implemented yet");
    
    ui.separator();
    
    // 保存ボタン
    if ui.button("Save Settings").clicked() {
        if changed {
            let _ = config_manager.update_settings(settings.clone());
        }
        changed = true;
    }
    
    changed
}

/// ホットキーエディタを描画する
#[allow(dead_code)]
fn render_hotkey_editor(ui: &mut Ui, hotkey: &mut Option<crate::config::settings::Hotkey>) -> bool {
    let mut changed = false;
    
    ui.horizontal(|ui| {
        if let Some(key) = hotkey {
            // 修飾キーの設定
            let mut ctrl = key.modifiers & 1 != 0;
            let mut alt = key.modifiers & 2 != 0;
            let mut shift = key.modifiers & 4 != 0;
            let mut win = key.modifiers & 8 != 0;
            
            if ui.checkbox(&mut ctrl, "Ctrl").changed() {
                if ctrl {
                    key.modifiers |= 1;
                } else {
                    key.modifiers &= !1;
                }
                changed = true;
            }
            
            if ui.checkbox(&mut alt, "Alt").changed() {
                if alt {
                    key.modifiers |= 2;
                } else {
                    key.modifiers &= !2;
                }
                changed = true;
            }
            
            if ui.checkbox(&mut shift, "Shift").changed() {
                if shift {
                    key.modifiers |= 4;
                } else {
                    key.modifiers &= !4;
                }
                changed = true;
            }
            
            if ui.checkbox(&mut win, "Win").changed() {
                if win {
                    key.modifiers |= 8;
                } else {
                    key.modifiers &= !8;
                }
                changed = true;
            }
            
            // キーコードの表示
            ui.label(format!("Key: {}", key.key_code));
        } else {
            if ui.button("Set Hotkey").clicked() {
                *hotkey = Some(crate::config::settings::Hotkey {
                    modifiers: 0,
                    key_code: 0,
                });
                changed = true;
            }
        }
        
        if hotkey.is_some() {
            if ui.button("Clear").clicked() {
                *hotkey = None;
                changed = true;
            }
        }
    });
    
    changed
} 