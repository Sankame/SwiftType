pub mod formatter;

use std::sync::{Arc, Mutex};
use arboard::Clipboard;
use std::thread;
use std::time::Duration;

use crate::config::Settings;
use crate::config::settings::SnippetType;
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
    pub fn check_for_replacements(&self, buffer: &str) -> Option<(String, usize)> {
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
                    
                    // キーワードの長さを返す（正確なバックスペース数のため）
                    return Some((replacement, snippet.keyword.len()));
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
    #[allow(dead_code)]
    pub fn try_replace(&mut self, buffer: &str) -> bool {
        if let Some((replacement, keyword_length)) = self.check_for_replacements(buffer) {
            self.perform_replacement_with_backspace(&replacement, keyword_length)
        } else {
            false
        }
    }
    
    /// 置換を実行する（キーワードの長さを指定してバックスペース）
    pub fn perform_replacement_with_backspace(&self, text: &str, keyword_length: usize) -> bool {
        // キーワード削除前にログ記録
        log::debug!("Replacing keyword (length: {}) with text: '{}'", keyword_length, text);
        
        // 高リスクの長さに対する特別処理 (5-9文字)
        // このサイズ範囲は特に問題が発生しやすい
        let is_high_risk_length = keyword_length >= 5 && keyword_length <= 9;
        
        // バックスペース処理の前に少し長く待機（キーボードバッファが安定するのを待つ）
        // 高リスクの長さの場合、より長く待機
        let pre_backspace_wait = if is_high_risk_length { 300 } else { 200 };
        thread::sleep(Duration::from_millis(pre_backspace_wait));
        
        // キーワードを削除（キーワードの長さに基づいてバックスペース）
        // タイミングの問題で一行上に移動する問題があるため、
        // 長さに基づいて適切なバックスペース回数を決定
        let adjusted_length = keyword_length;
        
        if !self.simulate_backspace(adjusted_length) {
            log::error!("Failed to simulate backspace for keyword of length {}", adjusted_length);
            return false;
        }
        
        log::debug!("Backspace operation completed successfully, waiting before text input operation");
        
        // バックスペースと入力操作の間の遅延
        // 長いキーワードの場合は待機時間を長く
        let wait_time = if keyword_length > 7 { 400 } else { 300 };
        thread::sleep(Duration::from_millis(wait_time));
        
        // テキストが短い場合は直接文字入力を試みる (より高い成功率)
        if text.len() <= 50 {
            log::debug!("Attempting direct text input for text: '{}'", text);
            
            // 改良された直接文字入力メソッドを使用（日本語文字にも対応）
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
            thread::sleep(Duration::from_millis(150));
            
            // CTRL+Vで貼り付ける
            let paste_result = self.simulate_paste_simple();
            
            if !paste_result {
                log::error!("Failed to simulate paste operation");
                
                // クリップボードを元の状態に戻す (エラー無視)
                if let Some(original_text) = original_clipboard {
                    let _ = clipboard.set_text(&original_text);
                }
                
                return false;
            }
            
            // 操作完了後に少し待機
            thread::sleep(Duration::from_millis(200));
            
            log::debug!("Replacement completed successfully: '{}'", text);
            return true;
        } else {
            log::error!("Failed to access clipboard");
            return false;
        }
    }
    
    /// バックスペースキーを自動で入力する
    fn simulate_backspace(&self, count: usize) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, VK_BACK,
        };
        
        if count == 0 {
            log::debug!("No backspaces to simulate");
            return true; // 削除するものがなければ成功と見なす
        }
        
        // バックスペース数をログに記録（デバッグ用）
        log::debug!("Simulating {} backspaces", count);
        
        // 高リスクの長さに対する特別処理
        let is_high_risk_length = count >= 5 && count <= 9;
        
        // バックスペース処理前の待機時間 (高リスクの場合は長く)
        let initial_wait = if is_high_risk_length { 100 } else { 80 };
        thread::sleep(Duration::from_millis(initial_wait));
        
        let mut success = true;
        
        // カーソル位置を安定させるためにバックスペースを丁寧に実行
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
            
            // キーの押下を確実に処理してもらうための待機時間
            thread::sleep(Duration::from_millis(30));
            
            // バックスペースを解放
            let sent_up = unsafe {
                SendInput(&[key_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent_up != 1 {
                log::error!("Failed to send backspace key up event for backspace {}", i + 1);
                success = false;
            }
            
            // 次のバックスペース前の待機時間
            let wait_time = 40;
            thread::sleep(Duration::from_millis(wait_time));
        }
        
        log::debug!("Completed sending {} backspace events, success: {}", count, success);
        
        // 最後の操作後に長めに待機して、システムが処理する時間を与える
        let final_wait = if is_high_risk_length { 180 } else if count > 5 { 150 } else { 120 };
        thread::sleep(Duration::from_millis(final_wait));
        
        success
    }

    /// シンプルなテキスト貼り付け操作 (CTRL+V)
    fn simulate_paste_simple(&self) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
        };
        
        log::debug!("Simulating paste operation (CTRL+V) with improved approach");
        
        // 開始前に修飾キーをリセット（前回の失敗状態から回復するため）
        self.reset_modifier_keys();
        
        // 一貫した時間をおいて貼り付け処理を実行
        thread::sleep(Duration::from_millis(150));
        
        // 入力をまとめて準備
        let mut inputs: Vec<INPUT> = Vec::with_capacity(4);
        
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
        inputs.push(ctrl_down);
        
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
        inputs.push(v_down);
        
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
        inputs.push(v_up);
        
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
        inputs.push(ctrl_up);
        
        // すべての入力をまとめて送信
        let sent = unsafe {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent as usize != inputs.len() {
            log::error!("Failed to send paste key sequence, sent only {} of {}", sent, inputs.len());
            self.reset_modifier_keys();
            return false;
        }
        
        // 操作後に待機
        thread::sleep(Duration::from_millis(100));
        
        log::debug!("Paste operation completed via improved approach");
        
        true
    }

    /// 直接文字入力（Unicode文字対応）
    fn simulate_direct_char_input(&self, text: &str) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_UNICODE, KEYEVENTF_KEYUP, VIRTUAL_KEY,
        };
        
        log::debug!("Simulating direct char input for: '{}'", text);
        
        // IMEの状態確認は条件付きでコンパイル
        #[cfg(feature = "Win32_UI_Input_Ime")]
        let ime_active = self.check_ime_status();
        #[cfg(not(feature = "Win32_UI_Input_Ime"))]
        let ime_active = false;
        
        if ime_active {
            log::debug!("IME is active, temporarily disabling for direct input");
            self.toggle_ime(false);
            
            // IMEの状態変更が反映されるのを待つ
            thread::sleep(Duration::from_millis(100));
        }
        
        for c in text.chars() {
            // キーダウン入力を表すINPUT構造体を作成
            let mut input_down: INPUT = unsafe { std::mem::zeroed() };
            input_down.r#type = INPUT_KEYBOARD;
            input_down.Anonymous.ki = KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: c as u16,
                dwFlags: KEYEVENTF_UNICODE,
                time: 0,
                dwExtraInfo: 0,
            };
            
            // キーアップ入力を表すINPUT構造体を作成
            let mut input_up: INPUT = unsafe { std::mem::zeroed() };
            input_up.r#type = INPUT_KEYBOARD;
            input_up.Anonymous.ki = KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: c as u16,
                dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            // キーダウン入力を送信
            let sent_down = unsafe {
                SendInput(&[input_down], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent_down != 1 {
                log::error!("Failed to send unicode character down event: '{}'", c);
                // IMEの状態を元に戻す
                if ime_active {
                    self.toggle_ime(true);
                }
                return false;
            }
            
            // キーダウンとキーアップの間に小さな遅延
            thread::sleep(Duration::from_millis(10));
            
            // キーアップ入力を送信
            let sent_up = unsafe {
                SendInput(&[input_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent_up != 1 {
                log::error!("Failed to send unicode character up event: '{}'", c);
                // IMEの状態を元に戻す
                if ime_active {
                    self.toggle_ime(true);
                }
                return false;
            }
            
            // 文字間に小さな遅延
            thread::sleep(Duration::from_millis(15));
        }
        
        // IMEの状態を元に戻す
        if ime_active {
            log::debug!("Restoring IME state");
            thread::sleep(Duration::from_millis(50));
            self.toggle_ime(true);
        }
        
        log::debug!("Direct char input completed successfully");
        return true;
    }

    /// IMEの状態を確認する関数
    #[cfg(feature = "Win32_UI_Input_Ime")]
    fn check_ime_status(&self) -> bool {
        use windows::Win32::UI::Input::Ime::{ImmGetContext, ImmGetOpenStatus};
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        
        unsafe {
            let hwnd = GetForegroundWindow();
            let himc = ImmGetContext(hwnd);
            
            if himc.is_invalid() {
                log::debug!("Failed to get IMM context, assuming IME is not active");
                return false;
            }
            
            let is_open = ImmGetOpenStatus(himc);
            log::debug!("IME status: {}", is_open);
            
            is_open
        }
    }
    
    /// IMEの状態を切り替える関数
    #[cfg(feature = "Win32_UI_Input_Ime")]
    fn toggle_ime(&self, enable: bool) -> bool {
        use windows::Win32::UI::Input::Ime::{ImmGetContext, ImmSetOpenStatus};
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        
        unsafe {
            let hwnd = GetForegroundWindow();
            let himc = ImmGetContext(hwnd);
            
            if himc.is_invalid() {
                log::error!("Failed to get IMM context for toggling IME");
                return false;
            }
            
            let result = ImmSetOpenStatus(himc, enable);
            log::debug!("Set IME status to {}: {}", enable, result);
            
            result
        }
    }
    
    /// IME機能が無効な場合のダミー実装
    #[cfg(not(feature = "Win32_UI_Input_Ime"))]
    fn check_ime_status(&self) -> bool {
        log::debug!("IME feature not enabled, assuming IME is not active");
        false
    }
    
    /// IME機能が無効な場合のダミー実装
    #[cfg(not(feature = "Win32_UI_Input_Ime"))]
    fn toggle_ime(&self, _enable: bool) -> bool {
        log::debug!("IME feature not enabled, toggle operation ignored");
        true
    }

    /// モディファイアキーを強制的に解放する関数
    pub fn reset_modifier_keys(&self) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, 
            VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN,
        };
        
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
