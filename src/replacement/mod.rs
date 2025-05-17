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
        
        // バックスペース処理の前に少し長めに待機（キーボードバッファが安定するのを待つ）
        std::thread::sleep(std::time::Duration::from_millis(200));
        
        // キーワードを削除（キーワードの長さに基づいてバックスペース）
        if !self.simulate_backspace(keyword_length) {
            log::error!("Failed to simulate backspace for keyword of length {}", keyword_length);
            return false;
        }
        
        // バックスペースとクリップボード操作の間に十分な遅延を設ける
        std::thread::sleep(std::time::Duration::from_millis(300));
        
        // クリップボードにテキストを設定
        if let Ok(mut clipboard) = Clipboard::new() {
            if let Err(e) = clipboard.set_text(text) {
                log::error!("Failed to set clipboard text: {}", e);
                return false;
            }
            
            // CTRL+Vで貼り付ける
            if !self.simulate_paste() {
                log::error!("Failed to simulate paste operation");
                return false;
            }
            
            // 操作完了後に少し待機
            std::thread::sleep(std::time::Duration::from_millis(200));
            
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
        thread::sleep(Duration::from_millis(100));
        
        let mut success = true;
        
        // 各バックスペースを個別に送信し、遅延を挟む
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
            
            if sent_down == 0 {
                log::error!("Failed to send backspace key down event for backspace {}", i + 1);
                success = false;
            }
            
            // 短い遅延
            thread::sleep(Duration::from_millis(40));
            
            // バックスペースを解放
            let sent_up = unsafe {
                SendInput(&[key_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent_up == 0 {
                log::error!("Failed to send backspace key up event for backspace {}", i + 1);
                success = false;
            }
            
            // 次のバックスペース前に少し長めの遅延
            thread::sleep(Duration::from_millis(60));
            
            log::debug!("Sent backspace {} of {}", i + 1, count);
        }
        
        log::debug!("Completed sending {} backspace events, success: {}", count, success);
        
        // 最後の操作後に十分待機して、システムが処理する時間を与える
        thread::sleep(Duration::from_millis(200));
        
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
        
        // バックスペース処理の後に十分待機してから貼り付け処理を実行
        thread::sleep(Duration::from_millis(300));
        
        // すべての入力を個別に送信し、状態を確認
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
        
        if sent_ctrl_down == 0 {
            log::error!("Failed to send CTRL key down event");
            success = false;
        }
        
        thread::sleep(Duration::from_millis(60));
        
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
        
        if sent_v_down == 0 {
            log::error!("Failed to send V key down event");
            success = false;
        }
        
        thread::sleep(Duration::from_millis(60));
        
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
        
        if sent_v_up == 0 {
            log::error!("Failed to send V key up event");
            success = false;
        }
        
        thread::sleep(Duration::from_millis(60));
        
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
        
        if sent_ctrl_up == 0 {
            log::error!("Failed to send CTRL key up event");
            success = false;
        }
        
        thread::sleep(Duration::from_millis(60));
        
        log::debug!("Paste operation completed: {}", if success { "success" } else { "failed" });
        
        success
    }
} 