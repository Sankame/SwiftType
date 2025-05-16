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
        // キーワードを削除（キーワードの長さに基づいてバックスペース）
        self.simulate_backspace(keyword_length);
        
        // クリップボードにテキストを設定
        if let Ok(mut clipboard) = Clipboard::new() {
            if let Err(e) = clipboard.set_text(text) {
                log::error!("Failed to set clipboard text: {}", e);
                return false;
            }
            
            // CTRL+Vで貼り付ける
            self.simulate_paste();
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
    fn simulate_backspace(&self, count: usize) {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, VK_BACK,
        };
        use std::thread;
        use std::time::Duration;
        
        // バックスペース数をログに記録（デバッグ用）
        log::debug!("Simulating {} backspaces", count);
        
        // 各バックスペースキー入力に対して2つの入力イベント（押下と解放）が必要
        let mut inputs: Vec<INPUT> = Vec::with_capacity(count * 2);
        
        for _ in 0..count {
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
            
            inputs.push(key_down);
            inputs.push(key_up);
        }
        
        // 入力イベントを送信（各キー入力の間に短い遅延を入れる）
        for i in (0..inputs.len()).step_by(2) {
            if i + 1 < inputs.len() {
                unsafe {
                    // バックスペースを押下
                    SendInput(&inputs[i..i+1], std::mem::size_of::<INPUT>() as i32);
                    // 短い遅延
                    thread::sleep(Duration::from_millis(10));
                    // バックスペースを解放
                    SendInput(&inputs[i+1..i+2], std::mem::size_of::<INPUT>() as i32);
                    // 少し長めの遅延
                    thread::sleep(Duration::from_millis(20));
                }
            }
        }
        
        log::debug!("Completed sending {} backspace events", count);
        
        // 最後の操作後に少し待機して、システムが処理する時間を与える
        thread::sleep(Duration::from_millis(50));
    }

    /// テキスト入力のシミュレーション (CTRL+V)
    fn simulate_paste(&self) {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
        };
        use windows::Win32::Foundation::HWND;
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Simulating paste operation (CTRL+V)");
        
        // バックスペース処理の後に少し待機してから貼り付け処理を実行
        thread::sleep(Duration::from_millis(100));
        
        // 入力イベントの配列を作成
        let mut inputs: [INPUT; 4] = unsafe { std::mem::zeroed() };
        
        // CTRL キーを押す
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki = KEYBDINPUT {
            wVk: VK_CONTROL,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        // V キーを押す
        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki = KEYBDINPUT {
            wVk: VK_V,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        // V キーを離す
        inputs[2].r#type = INPUT_KEYBOARD;
        inputs[2].Anonymous.ki = KEYBDINPUT {
            wVk: VK_V,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        // CTRL キーを離す
        inputs[3].r#type = INPUT_KEYBOARD;
        inputs[3].Anonymous.ki = KEYBDINPUT {
            wVk: VK_CONTROL,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        // 入力イベントを送信（遅延を加える）
        unsafe {
            // CTRL キーを押す
            SendInput(&inputs[0..1], std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(20));
            
            // V キーを押す
            SendInput(&inputs[1..2], std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(20));
            
            // V キーを離す
            SendInput(&inputs[2..3], std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(20));
            
            // CTRL キーを離す
            SendInput(&inputs[3..4], std::mem::size_of::<INPUT>() as i32);
        }
        
        log::debug!("Paste operation completed");
    }
} 