pub mod formatter;

use std::sync::{Arc, Mutex};
use arboard::Clipboard;

use crate::config::Settings;
use crate::config::settings::{Snippet, SnippetType};
use formatter::format_dynamic_content;

/// テキスト置換エンジン
#[derive(Debug)]
pub struct ReplacementEngine {
    settings: Arc<Mutex<Settings>>,
}

impl ReplacementEngine {
    /// 新しい置換エンジンを作成する
    pub fn new(settings: Arc<Mutex<Settings>>) -> Self {
        Self { settings }
    }
    
    /// テキストバッファから置換対象のキーワードを検索する
    pub fn check_for_replacements(&self, buffer: &str) -> Option<String> {
        if let Ok(settings) = self.settings.lock() {
            if !settings.enabled {
                return None;
            }
            
            // バッファ内容をログに記録（デバッグ用）
            log::debug!("Checking buffer for replacements: '{}'", buffer);
            
            // 有効なスニペットだけを検索
            for snippet in settings.snippets.iter().filter(|s| s.enabled) {
                if buffer.ends_with(&snippet.keyword) {
                    log::debug!("Found matching keyword: '{}' for snippet: '{}'", 
                               snippet.keyword, snippet.name);
                    
                    let replacement = match snippet.snippet_type {
                        SnippetType::Static => snippet.content.clone(),
                        SnippetType::Dynamic => {
                            let result = format_dynamic_content(&snippet.content);
                            log::debug!("Formatted dynamic content: '{}' -> '{}'", 
                                       snippet.content, result);
                            result
                        }
                    };
                    
                    return Some(replacement);
                }
            }
        }
        
        None
    }
    
    /// キーワードを置換しようと試みる
    /// 
    /// # 引数
    /// * `buffer` - 置換対象のバッファ文字列
    /// 
    /// # 戻り値
    /// 置換が成功したかどうか
    pub fn try_replace(&mut self, buffer: &str) -> bool {
        if let Some(replacement) = self.check_for_replacements(buffer) {
            self.perform_replacement(&replacement)
        } else {
            false
        }
    }
    
    /// 置換を実行する（キーワードの長さを指定してバックスペース）
    pub fn perform_replacement_with_backspace(&self, text: &str, keyword_length: usize) -> bool {
        // キーワード削除前にログ記録
        log::debug!("Replacing keyword (length: {}) with text: '{}'", keyword_length, text);
        
        // バックスペース処理の前に少し待機（キーボードバッファが安定するのを待つ）
        // 安定性のため100msに戻す
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        // キーワードを削除（キーワードの長さに基づいてバックスペース）
        if !self.simulate_backspace(keyword_length) {
            log::error!("Failed to simulate backspace for keyword of length {}", keyword_length);
            return false;
        }
        
        // バックスペースとクリップボード操作の間に遅延を設ける
        // 安定性のため150msに調整
        std::thread::sleep(std::time::Duration::from_millis(150));
        
        // クリップボードにテキストを設定
        if let Ok(mut clipboard) = Clipboard::new() {
            log::debug!("Setting clipboard text: '{}'", text);
            if let Err(e) = clipboard.set_text(text) {
                log::error!("Failed to set clipboard text: {}", e);
                return false;
            }
            
            // クリップボード設定後に少し待機
            std::thread::sleep(std::time::Duration::from_millis(50));
            
            // CTRL+Vで貼り付ける前にもう一度クリップボードの内容を確認
            match clipboard.get_text() {
                Ok(clipboard_text) => {
                    if clipboard_text != text {
                        log::error!("Clipboard text mismatch: expected '{}', got '{}'", text, clipboard_text);
                        // それでも続行
                    }
                },
                Err(e) => {
                    log::warn!("Could not verify clipboard text: {}", e);
                    // エラーでも続行
                }
            }
            
            // CTRL+Vで貼り付ける
            if !self.simulate_paste() {
                log::error!("Failed to simulate paste operation");
                return false;
            }
            
            // 操作完了後に少し待機
            // 安定性のため100msに調整
            std::thread::sleep(std::time::Duration::from_millis(100));
            
            log::debug!("Replacement completed successfully: '{}'", text);
            true
        } else {
            log::error!("Failed to access clipboard");
            false
        }
    }
    
    /// 置換を実行する
    pub fn perform_replacement(&self, text: &str) -> bool {
        // 注: このメソッドは互換性のために残しますが、キーワードを正しく削除するには
        // perform_replacement_with_backspaceを使用することを推奨します
        self.perform_replacement_with_backspace(text, text.len())
    }
    
    /// バックスペースキーを自動で入力する
    fn simulate_backspace(&self, count: usize) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, VK_BACK,
        };
        use std::thread;
        use std::time::Duration;
        
        if count == 0 {
            log::debug!("No backspaces to simulate");
            return true; // 削除するものがなければ成功と見なす
        }
        
        // バックスペース数をログに記録（デバッグ用）
        log::debug!("Simulating {} backspaces", count);
        
        // バックスペース処理前に少し待機
        // 安定性のため60msに調整
        thread::sleep(Duration::from_millis(60));
        
        let mut success = true;
        
        // 安定性向上のため、各バックスペースを個別に送信
        for i in 0..count {
            // バックスペースキーを押す
            let mut key_down: INPUT = unsafe { std::mem::zeroed() };
            key_down.r#type = INPUT_KEYBOARD;
            key_down.Anonymous.ki = KEYBDINPUT {
                wVk: VK_BACK,
                wScan: 0,
                dwFlags: Default::default(),
                time: 0,
                dwExtraInfo: 0,
            };
            
            // バックスペースキーを離す
            let mut key_up: INPUT = unsafe { std::mem::zeroed() };
            key_up.r#type = INPUT_KEYBOARD;
            key_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_BACK,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            // バックスペースを押下
            let sent_down = unsafe {
                SendInput(&[key_down], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent_down != 1 {
                log::error!("Failed to send backspace key down event for backspace {}", i + 1);
                success = false;
            }
            
            // 短い遅延 (30ms)
            thread::sleep(Duration::from_millis(30));
            
            // バックスペースを解放
            let sent_up = unsafe {
                SendInput(&[key_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent_up != 1 {
                log::error!("Failed to send backspace key up event for backspace {}", i + 1);
                success = false;
            }
            
            // 次のバックスペース前に少し待機 (40ms)
            thread::sleep(Duration::from_millis(40));
            
            log::debug!("Sent backspace {} of {}", i + 1, count);
        }
        
        log::debug!("Completed sending {} backspace events, success: {}", count, success);
        
        // 最後の操作後に待機して、システムが処理する時間を与える
        // 安定性のため100msに調整
        thread::sleep(Duration::from_millis(100));
        
        success
    }

    /// テキスト入力のシミュレーション (CTRL+V)
    fn simulate_paste(&self) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
        };
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Simulating paste operation (CTRL+V)");
        
        // バックスペース処理の後に少し待機してから貼り付け処理を実行
        // 安定性のため120msに調整
        thread::sleep(Duration::from_millis(120));
        
        // 個別キー送信に戻して安定性を確保
        let mut success = true;
        
        // CTRL キーを押す
        let mut ctrl_down: INPUT = unsafe { std::mem::zeroed() };
        ctrl_down.r#type = INPUT_KEYBOARD;
        ctrl_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_CONTROL,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        let sent_ctrl_down = unsafe {
            SendInput(&[ctrl_down], std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent_ctrl_down != 1 {
            log::error!("Failed to send CTRL key down event");
            success = false;
        }
        
        // 安定化のため少し待機
        thread::sleep(Duration::from_millis(50));
        
        // V キーを押す
        let mut v_down: INPUT = unsafe { std::mem::zeroed() };
        v_down.r#type = INPUT_KEYBOARD;
        v_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_V,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        let sent_v_down = unsafe {
            SendInput(&[v_down], std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent_v_down != 1 {
            log::error!("Failed to send V key down event");
            success = false;
        }
        
        // 安定化のため少し待機
        thread::sleep(Duration::from_millis(50));
        
        // V キーを離す
        let mut v_up: INPUT = unsafe { std::mem::zeroed() };
        v_up.r#type = INPUT_KEYBOARD;
        v_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_V,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        let sent_v_up = unsafe {
            SendInput(&[v_up], std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent_v_up != 1 {
            log::error!("Failed to send V key up event");
            success = false;
        }
        
        // 安定化のため少し待機
        thread::sleep(Duration::from_millis(50));
        
        // CTRL キーを離す
        let mut ctrl_up: INPUT = unsafe { std::mem::zeroed() };
        ctrl_up.r#type = INPUT_KEYBOARD;
        ctrl_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_CONTROL,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        let sent_ctrl_up = unsafe {
            SendInput(&[ctrl_up], std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent_ctrl_up != 1 {
            log::error!("Failed to send CTRL key up event");
            success = false;
        }
        
        // クリップボード操作後に少し待機
        thread::sleep(Duration::from_millis(80));
        
        log::debug!("Paste operation completed: {}", if success { "success" } else { "failed" });
        
        success
    }
} 