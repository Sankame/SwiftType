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
        
        // バックスペース処理の前に少し長く待機（キーボードバッファが安定するのを待つ）
        std::thread::sleep(std::time::Duration::from_millis(250));
        
        // キーワードを削除（キーワードの長さに基づいてバックスペース）
        // 注: タイミングの問題で余分に一文字削除されることがある。それを防ぐため、
        // 実際のキーワード長より1少ない回数のバックスペースを実行
        let adjusted_length = if keyword_length > 0 { keyword_length } else { 0 };
        
        if !self.simulate_backspace(adjusted_length) {
            log::error!("Failed to simulate backspace for keyword of length {}", adjusted_length);
            return false;
        }
        
        log::debug!("Backspace operation completed successfully, waiting before paste operation");
        
        // バックスペースと貼り付け操作の間の遅延を大幅に増加（一行上に上がる問題を防止）
        std::thread::sleep(std::time::Duration::from_millis(350));
        
        // テキストが短い場合は直接文字入力を試みる (より高い成功率)
        if text.len() <= 30 && text.chars().all(|c| c.is_ascii()) {
            log::debug!("Attempting direct text input for short text: '{}'", text);
            let direct_input_result = self.simulate_direct_char_input(text);
            
            if direct_input_result {
                log::debug!("Direct text input completed successfully");
                return true;
            }
            
            log::warn!("Direct text input failed, falling back to clipboard method");
        }
        
        // クリップボードにテキストを設定
        if let Ok(mut clipboard) = Clipboard::new() {
            // 既存のクリップボード内容を保存（あとで復元するため）
            let original_clipboard = clipboard.get_text().ok();
            
            log::debug!("Setting clipboard text: '{}'", text);
            if let Err(e) = clipboard.set_text(text) {
                log::error!("Failed to set clipboard text: {}", e);
                return false;
            }
            
            // クリップボード設定後に少し待機
            std::thread::sleep(std::time::Duration::from_millis(100));
            
            // CTRL+Vで貼り付ける
            let paste_result = self.simulate_paste_simple();
            
            if !paste_result {
                log::error!("Failed to simulate paste operation, trying alternative method");
                
                // 代替方法としてHWNDを使った別の貼り付け方法を試す
                let alt_paste_result = self.simulate_paste_alternative();
                
                if !alt_paste_result {
                    log::error!("All paste methods failed");
                    
                    // クリップボードを元の状態に戻す (エラー無視)
                    if let Some(original_text) = original_clipboard {
                        let _ = clipboard.set_text(&original_text);
                    }
                    
                    return false;
                }
            }
            
            // 操作完了後に少し待機
            std::thread::sleep(std::time::Duration::from_millis(150));
            
            log::debug!("Replacement completed successfully: '{}'", text);
            return true;
        } else {
            log::error!("Failed to access clipboard");
            return false;
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
        
        // バックスペース処理前に少し長く待機
        thread::sleep(Duration::from_millis(100));
        
        let mut success = true;
        
        // 各バックスペースを個別に送信し、確実に処理されるようにする
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
            
            // キーの押下を確実に処理してもらうため、待機時間を増加
            thread::sleep(Duration::from_millis(40));
            
            // バックスペースを解放
            let sent_up = unsafe {
                SendInput(&[key_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent_up != 1 {
                log::error!("Failed to send backspace key up event for backspace {}", i + 1);
                success = false;
            }
            
            // 次のバックスペース前に長めに待機（各キー入力を確実に処理してもらうため）
            thread::sleep(Duration::from_millis(40));
        }
        
        log::debug!("Completed sending {} backspace events, success: {}", count, success);
        
        // 最後の操作後に長めに待機して、システムが処理する時間を与える
        thread::sleep(Duration::from_millis(150));
        
        success
    }

    /// シンプルなテキスト貼り付け操作 (CTRL+V)
    fn simulate_paste_simple(&self) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
        };
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Simulating paste operation (CTRL+V) with simple approach");
        
        // 開始前に修飾キーをリセット（前回の失敗状態から回復するため）
        self.reset_modifier_keys();
        
        // 一貫した時間をおいて貼り付け処理を実行
        thread::sleep(Duration::from_millis(100));
        
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
        
        // CTRLキーを押す
        let sent_ctrl_down = unsafe {
            SendInput(&[ctrl_down], std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent_ctrl_down != 1 {
            log::error!("Failed to send CTRL key down event");
            self.reset_modifier_keys();
            return false;
        }
        
        // CTRL押下後に待機
        thread::sleep(Duration::from_millis(30));
        
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
            
            // CTRLキーを解放して回復
            let mut ctrl_up: INPUT = unsafe { std::mem::zeroed() };
            ctrl_up.r#type = INPUT_KEYBOARD;
            ctrl_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_CONTROL,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            unsafe {
                SendInput(&[ctrl_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            self.reset_modifier_keys();
            return false;
        }
        
        // Vキーを押した直後に待機
        thread::sleep(Duration::from_millis(30));
        
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
            
            // CTRLキーを解放して回復
            let mut ctrl_up: INPUT = unsafe { std::mem::zeroed() };
            ctrl_up.r#type = INPUT_KEYBOARD;
            ctrl_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_CONTROL,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            unsafe {
                SendInput(&[ctrl_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            self.reset_modifier_keys();
            return false;
        }
        
        // Vキーを離した後に待機
        thread::sleep(Duration::from_millis(30));
        
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
            self.reset_modifier_keys();
            return false;
        }
        
        // 操作後に待機
        thread::sleep(Duration::from_millis(50));
        
        log::debug!("Paste operation completed via step-by-step approach");
        
        true
    }

    /// 直接文字入力（ASCII文字限定）
    fn simulate_direct_char_input(&self, text: &str) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_UNICODE, VIRTUAL_KEY,
        };
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Simulating direct char input for: '{}'", text);
        
        for c in text.chars() {
            // キー入力を表すINPUT構造体を作成
            let mut input: INPUT = unsafe { std::mem::zeroed() };
            input.r#type = INPUT_KEYBOARD;
            input.Anonymous.ki = KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: c as u16,
                dwFlags: KEYEVENTF_UNICODE,
                time: 0,
                dwExtraInfo: 0,
            };
            
            // キー入力を送信
            let sent = unsafe {
                SendInput(&[input], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent != 1 {
                log::error!("Failed to send unicode character: '{}'", c);
                return false;
            }
            
            // 文字間に小さな遅延
            thread::sleep(Duration::from_millis(5));
        }
        
        log::debug!("Direct char input completed successfully");
        return true;
    }

    /// 代替貼り付け方法
    fn simulate_paste_alternative(&self) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, KEYEVENTF_EXTENDEDKEY, VK_SHIFT, VK_INSERT,
        };
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Trying alternative paste operation (SHIFT+INSERT)");
        
        // 開始前に修飾キーをリセット
        self.reset_modifier_keys();
        
        // SHIFT+INSERTの組み合わせを試す (多くのWindowsアプリで貼り付けとして機能)
        thread::sleep(Duration::from_millis(100));
        
        // SHIFT キーを押す
        let mut shift_down: INPUT = unsafe { std::mem::zeroed() };
        shift_down.r#type = INPUT_KEYBOARD;
        shift_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_SHIFT,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        let sent_shift_down = unsafe {
            SendInput(&[shift_down], std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent_shift_down != 1 {
            log::error!("Failed to send SHIFT key down event");
            self.reset_modifier_keys();
            return false;
        }
        
        thread::sleep(Duration::from_millis(30));
        
        // INSERT キーを押す
        let mut insert_down: INPUT = unsafe { std::mem::zeroed() };
        insert_down.r#type = INPUT_KEYBOARD;
        insert_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_INSERT,
            wScan: 0,
            dwFlags: KEYEVENTF_EXTENDEDKEY, // 拡張キーとして送信
            time: 0,
            dwExtraInfo: 0,
        };
        
        let sent_insert_down = unsafe {
            SendInput(&[insert_down], std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent_insert_down != 1 {
            log::error!("Failed to send INSERT key down event");
            
            // SHIFTを解放
            let mut shift_up: INPUT = unsafe { std::mem::zeroed() };
            shift_up.r#type = INPUT_KEYBOARD;
            shift_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_SHIFT,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            unsafe {
                SendInput(&[shift_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            self.reset_modifier_keys();
            return false;
        }
        
        thread::sleep(Duration::from_millis(30));
        
        // INSERT キーを離す
        let mut insert_up: INPUT = unsafe { std::mem::zeroed() };
        insert_up.r#type = INPUT_KEYBOARD;
        insert_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_INSERT,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY,
            time: 0,
            dwExtraInfo: 0,
        };
        
        let sent_insert_up = unsafe {
            SendInput(&[insert_up], std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent_insert_up != 1 {
            log::error!("Failed to send INSERT key up event");
            
            // SHIFTを解放
            let mut shift_up: INPUT = unsafe { std::mem::zeroed() };
            shift_up.r#type = INPUT_KEYBOARD;
            shift_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_SHIFT,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            unsafe {
                SendInput(&[shift_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            self.reset_modifier_keys();
            return false;
        }
        
        thread::sleep(Duration::from_millis(30));
        
        // SHIFT キーを離す
        let mut shift_up: INPUT = unsafe { std::mem::zeroed() };
        shift_up.r#type = INPUT_KEYBOARD;
        shift_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_SHIFT,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        let sent_shift_up = unsafe {
            SendInput(&[shift_up], std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent_shift_up != 1 {
            log::error!("Failed to send SHIFT key up event");
            self.reset_modifier_keys();
            return false;
        }
        
        thread::sleep(Duration::from_millis(50));
        
        log::debug!("Alternative paste operation (SHIFT+INSERT) completed");
        return true;
    }

    /// モディファイアキーを強制的に解放する関数
    pub fn reset_modifier_keys(&self) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, 
            VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN,
        };
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Resetting all modifier keys to released state");
        
        let modifiers = [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN];
        let mut inputs: Vec<INPUT> = Vec::with_capacity(modifiers.len());
        
        // すべてのモディファイアキーを離す状態にする
        for &vk in &modifiers {
            let mut key_up: INPUT = unsafe { std::mem::zeroed() };
            key_up.r#type = INPUT_KEYBOARD;
            key_up.Anonymous.ki = KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            inputs.push(key_up);
        }
        
        // モディファイアキーをすべて解放
        let sent = unsafe {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent as usize != inputs.len() {
            log::error!("Failed to reset modifier keys, sent only {} of {}", sent, inputs.len());
            return false;
        }
        
        // 少し待機して確実にキー状態が反映されるようにする
        thread::sleep(Duration::from_millis(50));
        
        log::debug!("All modifier keys have been reset");
        true
    }
} 