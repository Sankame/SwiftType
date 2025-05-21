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
                // まず元のキーワードで直接比較
                if buffer.ends_with(&snippet.keyword) {
                    log::debug!("Found matching keyword (direct): '{}' for snippet: '{}'", 
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
                
                // 元の比較で見つからない場合のみ、正規化して比較
                let normalized_buffer = buffer.replace('=', "_")
                                             .replace(';', "_")
                                             .replace(',', "_");
                let normalized_keyword = snippet.keyword.replace('=', "_")
                                                      .replace(';', "_")
                                                      .replace(',', "_");
                
                if normalized_buffer.ends_with(&normalized_keyword) {
                    log::debug!("Found matching keyword (normalized): '{}' for snippet: '{}'", 
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
        
        // 安全のため、キーワード長に上限を設ける
        let safe_length = std::cmp::min(keyword_length, 20); // 最大20文字に制限
        if safe_length < keyword_length {
            log::warn!("Limiting keyword length from {} to {}", keyword_length, safe_length);
        }
        
        // 高リスクの長さに対する特別処理 (5-9文字)
        let is_high_risk_length = safe_length >= 5 && safe_length <= 9;
        
        // 短いキーワードの場合は特別な処理
        let is_short_keyword = safe_length <= 2;
        
        // バックスペース処理の前に少し待機
        // 短いキーワードの場合はより長く待機
        let pre_backspace_wait = if is_short_keyword {
            400 // 短いキーワードは長めに待機
        } else if is_high_risk_length {
            300
        } else {
            200
        };
        thread::sleep(Duration::from_millis(pre_backspace_wait));
        
        // 例外処理を追加
        let backspace_result = std::panic::catch_unwind(|| {
            // キーワードを削除（キーワードの長さに基づいてバックスペース）
            if !self.simulate_backspace(safe_length, is_short_keyword) {
                log::error!("Failed to simulate backspace for keyword of length {}", safe_length);
                return false;
            }
            true
        });
        
        // パニックが発生した場合は失敗として扱う
        let backspace_success = match backspace_result {
            Ok(success) => success,
            Err(_) => {
                log::error!("Panic occurred during backspace operation");
                false
            }
        };
        
        if !backspace_success {
            return false;
        }
        
        log::debug!("Backspace operation completed successfully, waiting before text input operation");
        
        // バックスペースと入力操作の間の遅延
        // 短いキーワードの場合はより長く待機
        let wait_time = if is_short_keyword {
            600 // 短いキーワードは長めに待機
        } else if safe_length > 7 {
            400
        } else {
            300
        };
        thread::sleep(Duration::from_millis(wait_time));
        
        // テキストが短い場合は直接文字入力を試みる (より高い成功率)
        if text.len() <= 50 {
            log::debug!("Attempting direct text input for text: '{}'", text);
            
            // 例外処理を追加
            let input_result = std::panic::catch_unwind(|| {
                // 改良された直接文字入力メソッドを使用（日本語文字にも対応）
                let direct_input_result = self.simulate_direct_char_input(text);
                
                if direct_input_result {
                    log::debug!("Direct text input completed successfully");
                    return true;
                }
                
                log::warn!("Direct text input failed, falling back to clipboard method");
                false
            });
            
            // 直接入力が成功した場合は終了
            match input_result {
                Ok(true) => return true,
                Ok(false) => {}, // クリップボード方式にフォールバック
                Err(_) => {
                    log::error!("Panic occurred during direct text input");
                    // クリップボード方式にフォールバック
                }
            }
        }
        
        // クリップボード操作を例外処理で囲む
        let clipboard_result = std::panic::catch_unwind(|| {
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
        });
        
        match clipboard_result {
            Ok(result) => result,
            Err(_) => {
                log::error!("Panic occurred during clipboard operation");
                false
            }
        }
    }
    
    /// バックスペースキーを自動で入力する
    fn simulate_backspace(&self, count: usize, is_short_keyword: bool) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, VK_BACK,
        };
        
        if count == 0 {
            log::debug!("No backspaces to simulate");
            return true; // 削除するものがなければ成功と見なす
        }
        
        // バックスペース数をログに記録（デバッグ用）
        log::debug!("Simulating {} backspaces", count);
        
        // 安全のため、バックスペース数に上限を設ける
        let safe_count = std::cmp::min(count, 20); // 最大20回に制限
        if safe_count < count {
            log::warn!("Limiting backspace count from {} to {}", count, safe_count);
        }
        
        // 高リスクの長さに対する特別処理
        let is_high_risk_length = safe_count >= 5 && safe_count <= 9;
        
        // バックスペース処理前の待機時間
        // 短いキーワードの場合はより長く待機
        let initial_wait = if is_short_keyword {
            100 // 短いキーワードは長めに待機
        } else if is_high_risk_length {
            50
        } else {
            40
        };
        thread::sleep(Duration::from_millis(initial_wait));
        
        // 例外処理を追加
        let success = match std::panic::catch_unwind(|| {
            // カーソル位置を安定させるためにバックスペースを丁寧に実行
            for i in 0..safe_count {
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
                    return false;
                }
                
                // キーの押下を確実に処理してもらうための待機時間
                // 短いキーワードの場合はより長く待機
                let key_down_wait = if is_short_keyword { 40 } else { 20 };
                thread::sleep(Duration::from_millis(key_down_wait));
                
                // バックスペースを解放
                let sent_up = unsafe {
                    SendInput(&[key_up], std::mem::size_of::<INPUT>() as i32)
                };
                
                if sent_up != 1 {
                    log::error!("Failed to send backspace key up event for backspace {}", i + 1);
                    return false;
                }
                
                // 次のバックスペース前の待機時間
                // 短いキーワードの場合はより長く待機
                let between_backspace_wait = if is_short_keyword { 50 } else { 20 };
                thread::sleep(Duration::from_millis(between_backspace_wait));
            }
            
            // すべて成功
            true
        }) {
            Ok(result) => {
                log::debug!("Completed sending {} backspace events, success: {}", safe_count, result);
                result
            },
            Err(_) => {
                log::error!("Panic occurred during backspace simulation");
                false
            }
        };
        
        // 最後の操作後の待機時間
        // 短いキーワードの場合はより長く待機
        let final_wait = if is_short_keyword {
            200 // 短いキーワードは長めに待機
        } else if is_high_risk_length {
            100
        } else if safe_count > 5 {
            80
        } else {
            60
        };
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
        
        // IMEの状態確認
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
        
        // 短いテキストの場合は特に慎重に処理
        let is_short_text = text.len() <= 3;
        let char_delay = if is_short_text { 30 } else { 15 };
        
        // 入力前に少し待機（特に短いテキストの場合）
        if is_short_text {
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
            thread::sleep(Duration::from_millis(char_delay));
            
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
            let between_char_delay = if is_short_text { 30 } else { 15 };
            thread::sleep(Duration::from_millis(between_char_delay));
        }
        
        // IMEの状態を元に戻す
        if ime_active {
            log::debug!("Restoring IME state");
            thread::sleep(Duration::from_millis(50));
            self.toggle_ime(true);
        }
        
        // 入力後に少し待機（特に短いテキストの場合）
        if is_short_text {
            thread::sleep(Duration::from_millis(100));
        }
        
        log::debug!("Direct char input completed successfully");
        return true;
    }

    /// IMEの状態を確認する関数
    #[cfg(feature = "Win32_UI_Input_Ime")]
    fn check_ime_status(&self) -> bool {
        use windows::Win32::UI::Input::Ime::{ImmGetContext, ImmGetOpenStatus};
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        use windows::Win32::Globalization::HIMC;
        use windows::Win32::Foundation::BOOL;
        
        unsafe {
            let hwnd = GetForegroundWindow();
            let himc = ImmGetContext(hwnd);
            
            if himc.is_invalid() {
                log::debug!("Failed to get IMM context, assuming IME is not active");
                return false;
            }
            
            let is_open = ImmGetOpenStatus(himc);
            log::debug!("IME status: {:?}", is_open);
            
            is_open.into()
        }
    }
    
    /// IMEの状態を切り替える関数
    #[cfg(feature = "Win32_UI_Input_Ime")]
    fn toggle_ime(&self, enable: bool) -> bool {
        use windows::Win32::UI::Input::Ime::{ImmGetContext, ImmSetOpenStatus};
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        use windows::Win32::Globalization::HIMC;
        use windows::Win32::Foundation::BOOL;
        
        unsafe {
            let hwnd = GetForegroundWindow();
            let himc = ImmGetContext(hwnd);
            
            if himc.is_invalid() {
                log::error!("Failed to get IMM context for toggling IME");
                return false;
            }
            
            let result = ImmSetOpenStatus(himc, enable);
            log::debug!("Set IME status to {}: {:?}", enable, result);
            
            result.into()
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
