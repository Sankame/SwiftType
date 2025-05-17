pub mod formatter;

use std::sync::{Arc, Mutex};
use arboard::Clipboard;

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
    #[allow(dead_code)]
    pub fn try_replace(&mut self, buffer: &str) -> bool {
        if let Some(replacement) = self.check_for_replacements(buffer) {
            self.perform_replacement_with_backspace(&replacement, replacement.len())
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
        
        // 選択して置換する方法を常に最初に試す（高リスクな長さの場合）
        if is_high_risk_length {
            log::debug!("Using selection-based replacement approach for high risk keyword of length {}", keyword_length);
            
            // より安定した選択ベース置換の実行を2回まで試みる
            for attempt in 1..=2 {
                if attempt > 1 {
                    log::debug!("Retry attempt {} for selection-based replacement", attempt);
                    // 再試行の前に少し長めに待機
                    std::thread::sleep(std::time::Duration::from_millis(200));
                }
                
                // バックスペース処理ではなく、選択して上書きする方法を試みる
                if self.perform_selection_replacement(text, keyword_length) {
                    log::debug!("Selection-based replacement completed successfully on attempt {}", attempt);
                    return true;
                }
                
                log::warn!("Selection-based replacement failed on attempt {}", attempt);
                
                // 修飾キーをリセットして次の試行または別の方法への移行準備
                self.reset_modifier_keys();
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
            
            log::warn!("All selection-based replacement attempts failed, falling back to traditional backspace method");
        }
        
        // バックスペース処理の前に少し長く待機（キーボードバッファが安定するのを待つ）
        // 高リスクの長さの場合、より長く待機
        let pre_backspace_wait = if is_high_risk_length { 250 } else { 150 };
        std::thread::sleep(std::time::Duration::from_millis(pre_backspace_wait));
        
        // キーワードを削除（キーワードの長さに基づいてバックスペース）
        // タイミングの問題で一行上に移動する問題があるため、
        // 長さに基づいて適切なバックスペース回数を決定
        let adjusted_length = keyword_length;
        
        if !self.simulate_backspace(adjusted_length) {
            log::error!("Failed to simulate backspace for keyword of length {}", adjusted_length);
            return false;
        }
        
        log::debug!("Backspace operation completed successfully, waiting before paste operation");
        
        // カーソル安定化のための特別な処理を追加
        // これにより、バックスペース後にカーソル位置が安定するのを待つ
        if is_high_risk_length {
            log::debug!("High risk keyword length detected. Applying cursor stabilization...");
            self.stabilize_cursor_position();
            
            // 追加の安全策として、HOME+ENDキーを使ってカーソル位置を行内に確実に固定
            self.anchor_cursor_to_line();
        }
        
        // バックスペースと貼り付け操作の間の遅延
        // 長いキーワードの場合は待機時間を長く
        let wait_time = if keyword_length > 7 { 350 } else { 250 };
        std::thread::sleep(std::time::Duration::from_millis(wait_time));
        
        // テキストが短い場合は直接文字入力を試みる (より高い成功率)
        if text.len() <= 30 && text.chars().all(|c| c.is_ascii()) {
            log::debug!("Attempting direct text input for short text: '{}'", text);
            
            // 改良された直接文字入力メソッドを使用
            let direct_input_result = if is_high_risk_length {
                // 高リスクのキーワード長さでは、より安全な入力方法を使用
                self.simulate_safe_direct_input(text)
            } else {
                self.simulate_direct_char_input(text)
            };
            
            if direct_input_result {
                // 入力完了後に再度カーソル位置を安定化
                if is_high_risk_length {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    self.anchor_cursor_to_line();
                }
                
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
            
            // 高リスクの場合は、追加の安全策としてカーソル位置を行内に確実に固定
            if is_high_risk_length {
                self.anchor_cursor_to_line();
            }
            
            log::debug!("Replacement completed successfully: '{}'", text);
            return true;
        } else {
            log::error!("Failed to access clipboard");
            return false;
        }
    }
    
    /// 選択して置換するアプローチ (バックスペースの代わりに)
    fn perform_selection_replacement(&self, text: &str, keyword_length: usize) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP, 
            VK_SHIFT, VK_LEFT, VK_RIGHT,
        };
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Using selection-based replacement approach");
        
        // まず修飾キーをリセット
        self.reset_modifier_keys();
        
        // 選択操作前にカーソルを確実に行内に固定（重要な追加）
        // これにより選択前の位置が安定します
        self.anchor_cursor_to_line();
        
        // 選択前により長く待機し、キーボード状態を安定させる
        thread::sleep(Duration::from_millis(200));
        
        // カーソルを右に一度移動してから左に移動する（位置確認のため）
        self.stabilize_cursor_position();
        
        // さらに少し待機
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
        
        let sent_shift = unsafe {
            SendInput(&[shift_down], std::mem::size_of::<INPUT>() as i32)
        };
        
        if sent_shift != 1 {
            log::error!("Failed to send SHIFT key down event");
            self.reset_modifier_keys();
            return false;
        }
        
        // 一貫した左矢印の処理のため、より慎重に実行するよう修正
        let left_arrow_count = keyword_length;
        
        // SHIFTを押したままで左矢印キーを必要な回数押す (キーワード長と同じ数)
        for i in 0..left_arrow_count {
            // より長い待機時間を設定（特に最初と最後の矢印入力で）
            let arrow_wait = if i == 0 || i == left_arrow_count - 1 { 50 } else { 40 };
            thread::sleep(Duration::from_millis(arrow_wait));
            
            // 左矢印キーを押す
            let mut left_down: INPUT = unsafe { std::mem::zeroed() };
            left_down.r#type = INPUT_KEYBOARD;
            left_down.Anonymous.ki = KEYBDINPUT {
                wVk: VK_LEFT,
                wScan: 0,
                dwFlags: Default::default(),
                time: 0,
                dwExtraInfo: 0,
            };
            
            // 左矢印キーを押す
            let sent_left_down = unsafe {
                SendInput(&[left_down], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent_left_down != 1 {
                log::error!("Failed to send LEFT key down event for selection step {}", i + 1);
                self.reset_modifier_keys();
                return false;
            }
            
            // キーの押下と解放間の待機時間を増加（50ms）
            thread::sleep(Duration::from_millis(50));
            
            // 左矢印キーを離す
            let mut left_up: INPUT = unsafe { std::mem::zeroed() };
            left_up.r#type = INPUT_KEYBOARD;
            left_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_LEFT,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            // 左矢印キーを離す
            let sent_left_up = unsafe {
                SendInput(&[left_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent_left_up != 1 {
                log::error!("Failed to send LEFT key up event for selection step {}", i + 1);
                self.reset_modifier_keys();
                return false;
            }
            
            // カウンタの中間点で、選択が安定するように追加の待機
            if i == left_arrow_count / 2 {
                thread::sleep(Duration::from_millis(30));
            }
        }
        
        // SHIFTキーを離す前に少し長く待機
        thread::sleep(Duration::from_millis(100));
        
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
        
        // 選択完了後、より長い待機時間で確実に安定させる
        thread::sleep(Duration::from_millis(200));
        
        // 選択後に右矢印+左矢印で位置確認（カーソルを選択範囲内に維持する追加の安全策）
        // これにより選択が維持される
        if keyword_length > 5 {
            // 右矢印キーを一度押す
            let mut right_down: INPUT = unsafe { std::mem::zeroed() };
            right_down.r#type = INPUT_KEYBOARD;
            right_down.Anonymous.ki = KEYBDINPUT {
                wVk: VK_RIGHT,
                wScan: 0,
                dwFlags: Default::default(),
                time: 0,
                dwExtraInfo: 0,
            };
            
            // 右矢印キーを離す
            let mut right_up: INPUT = unsafe { std::mem::zeroed() };
            right_up.r#type = INPUT_KEYBOARD;
            right_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_RIGHT,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            // 右キーを押して離す
            unsafe {
                SendInput(&[right_down], std::mem::size_of::<INPUT>() as i32);
                thread::sleep(Duration::from_millis(50));
                SendInput(&[right_up], std::mem::size_of::<INPUT>() as i32);
            }
            
            thread::sleep(Duration::from_millis(50));
            
            // 左矢印キーを一度押す
            let mut left_down: INPUT = unsafe { std::mem::zeroed() };
            left_down.r#type = INPUT_KEYBOARD;
            left_down.Anonymous.ki = KEYBDINPUT {
                wVk: VK_LEFT,
                wScan: 0,
                dwFlags: Default::default(),
                time: 0,
                dwExtraInfo: 0,
            };
            
            // 左矢印キーを離す
            let mut left_up: INPUT = unsafe { std::mem::zeroed() };
            left_up.r#type = INPUT_KEYBOARD;
            left_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_LEFT,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            
            // 左キーを押して離す
            unsafe {
                SendInput(&[left_down], std::mem::size_of::<INPUT>() as i32);
                thread::sleep(Duration::from_millis(50));
                SendInput(&[left_up], std::mem::size_of::<INPUT>() as i32);
            }
            
            thread::sleep(Duration::from_millis(50));
        }
        
        // 直接テキスト入力で選択範囲を置換する前に、選択が正しく行われた可能性を高めるために
        // HOME + ENDで再度カーソルを行内に固定
        self.anchor_cursor_to_line();
        
        // 選択したテキストが存在するはず、少し待機してから置換テキストを入力
        thread::sleep(Duration::from_millis(200));
        
        // 直接テキスト入力で選択範囲を置換（選択範囲は自動的に削除される）
        // より長い文字間待機時間のsafe_direct_inputを使用
        let replacement_success = self.simulate_safe_direct_input_with_longer_waits(text);
        if !replacement_success {
            log::error!("Failed to replace selected text with direct input");
            return false;
        }
        
        // 操作完了後に適切な待機
        thread::sleep(Duration::from_millis(150));
        
        // 最終的にカーソル位置を再度確認・修正
        self.anchor_cursor_to_line();
        
        log::debug!("Selection-based replacement completed successfully");
        return true;
    }
    
    /// より長い待機時間を持つ安全な直接文字入力メソッド（選択置換専用）
    fn simulate_safe_direct_input_with_longer_waits(&self, text: &str) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_UNICODE, VIRTUAL_KEY,
        };
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Using safer direct input method with longer waits for selection replacement: '{}'", text);
        
        // 修飾キーをリセット
        self.reset_modifier_keys();
        
        // 安定化のため長めに待機
        thread::sleep(Duration::from_millis(100));
        
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
            
            // 文字間により長い遅延を追加（特に選択置換モード）
            thread::sleep(Duration::from_millis(30));
        }
        
        // 入力完了後にカーソル位置を確定するための長めの待機
        thread::sleep(Duration::from_millis(100));
        
        log::debug!("Safe direct input with longer waits completed successfully");
        return true;
    }
    
    /// カーソル位置を現在の行内に確実に固定するための関数（改良版）
    fn anchor_cursor_to_line(&self) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP,
            VK_HOME, VK_END, VK_RIGHT, VK_LEFT,
        };
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Anchoring cursor to current line with enhanced method");
        
        // まず修飾キーをリセット
        self.reset_modifier_keys();
        
        // 少し待機
        thread::sleep(Duration::from_millis(80));
        
        // HOMEキーを押す (行の先頭に移動)
        let mut home_down: INPUT = unsafe { std::mem::zeroed() };
        home_down.r#type = INPUT_KEYBOARD;
        home_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_HOME,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        // HOMEキーを離す
        let mut home_up: INPUT = unsafe { std::mem::zeroed() };
        home_up.r#type = INPUT_KEYBOARD;
        home_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_HOME,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        // HOMEキーを押して離す
        unsafe {
            SendInput(&[home_down], std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(50));
            SendInput(&[home_up], std::mem::size_of::<INPUT>() as i32);
        }
        
        thread::sleep(Duration::from_millis(80));
        
        // ENDキーを押す (行の末尾に移動)
        let mut end_down: INPUT = unsafe { std::mem::zeroed() };
        end_down.r#type = INPUT_KEYBOARD;
        end_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_END,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        // ENDキーを離す
        let mut end_up: INPUT = unsafe { std::mem::zeroed() };
        end_up.r#type = INPUT_KEYBOARD;
        end_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_END,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        // ENDキーを押して離す
        unsafe {
            SendInput(&[end_down], std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(50));
            SendInput(&[end_up], std::mem::size_of::<INPUT>() as i32);
        }
        
        thread::sleep(Duration::from_millis(80));
        
        // 追加の安定化: 左右の矢印キーを使用して位置を確認・安定化
        // 左矢印キーを押す
        let mut left_down: INPUT = unsafe { std::mem::zeroed() };
        left_down.r#type = INPUT_KEYBOARD;
        left_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_LEFT,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        // 左矢印キーを離す
        let mut left_up: INPUT = unsafe { std::mem::zeroed() };
        left_up.r#type = INPUT_KEYBOARD;
        left_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_LEFT,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        // 左キーを押して離す
        unsafe {
            SendInput(&[left_down], std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(50));
            SendInput(&[left_up], std::mem::size_of::<INPUT>() as i32);
        }
        
        thread::sleep(Duration::from_millis(50));
        
        // 右矢印キーを押す
        let mut right_down: INPUT = unsafe { std::mem::zeroed() };
        right_down.r#type = INPUT_KEYBOARD;
        right_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_RIGHT,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        // 右矢印キーを離す
        let mut right_up: INPUT = unsafe { std::mem::zeroed() };
        right_up.r#type = INPUT_KEYBOARD;
        right_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_RIGHT,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        // 右キーを押して離す
        unsafe {
            SendInput(&[right_down], std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(50));
            SendInput(&[right_up], std::mem::size_of::<INPUT>() as i32);
        }
        
        thread::sleep(Duration::from_millis(50));
        
        log::debug!("Cursor anchored to current line with enhanced method");
        true
    }
    
    /// 置換を実行する
    /// 
    /// 注: このメソッドは簡易的なラッパーで、内部的に perform_replacement_with_backspace を呼び出します
    #[allow(dead_code)]
    pub fn perform_replacement(&self, text: &str) -> bool {
        // perform_replacement_with_backspaceを使用することを推奨します
        self.perform_replacement_with_backspace(text, text.len())
    }
    
    /// カーソル位置を安定化させるための関数
    fn stabilize_cursor_position(&self) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_KEYUP,
            VK_LEFT, VK_RIGHT,
        };
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Stabilizing cursor position with left-right arrow keys");
        
        // まず修飾キーをリセット
        self.reset_modifier_keys();
        
        // 少し待機
        thread::sleep(Duration::from_millis(50));
        
        // 左キーを押す
        let mut left_down: INPUT = unsafe { std::mem::zeroed() };
        left_down.r#type = INPUT_KEYBOARD;
        left_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_LEFT,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        // 左キーを離す
        let mut left_up: INPUT = unsafe { std::mem::zeroed() };
        left_up.r#type = INPUT_KEYBOARD;
        left_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_LEFT,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        // 左キーを押して離す
        unsafe {
            SendInput(&[left_down], std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(30));
            SendInput(&[left_up], std::mem::size_of::<INPUT>() as i32);
        }
        
        thread::sleep(Duration::from_millis(50));
        
        // 右キーを押す
        let mut right_down: INPUT = unsafe { std::mem::zeroed() };
        right_down.r#type = INPUT_KEYBOARD;
        right_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_RIGHT,
            wScan: 0,
            dwFlags: Default::default(),
            time: 0,
            dwExtraInfo: 0,
        };
        
        // 右キーを離す
        let mut right_up: INPUT = unsafe { std::mem::zeroed() };
        right_up.r#type = INPUT_KEYBOARD;
        right_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_RIGHT,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        
        // 右キーを押して離す
        unsafe {
            SendInput(&[right_down], std::mem::size_of::<INPUT>() as i32);
            thread::sleep(Duration::from_millis(30));
            SendInput(&[right_up], std::mem::size_of::<INPUT>() as i32);
        }
        
        thread::sleep(Duration::from_millis(50));
        
        log::debug!("Cursor position stabilized");
        true
    }
    
    /// より安全な直接文字入力メソッド（特に高リスクのキーワード長に使用）
    fn simulate_safe_direct_input(&self, text: &str) -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_KEYBOARD, KEYBDINPUT, SendInput, KEYEVENTF_UNICODE, VIRTUAL_KEY,
        };
        use std::thread;
        use std::time::Duration;
        
        log::debug!("Using safer direct input method for high risk keyword replacement: '{}'", text);
        
        // 修飾キーをリセット
        self.reset_modifier_keys();
        
        // 安定化のため少し待機
        thread::sleep(Duration::from_millis(50));
        
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
            
            // 文字間に長めの遅延を追加（特に安全モード）
            thread::sleep(Duration::from_millis(15));
        }
        
        // 入力完了後にカーソル位置を確定
        thread::sleep(Duration::from_millis(50));
        
        log::debug!("Safe direct input completed successfully");
        return true;
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
        
        // 高リスクの長さに対する特別処理
        let is_high_risk_length = count >= 5 && count <= 9;
        
        // バックスペース処理前の待機時間 (高リスクの場合は長く)
        let initial_wait = if is_high_risk_length { 70 } else { 50 };
        thread::sleep(Duration::from_millis(initial_wait));
        
        let mut success = true;
        
        // 安定化のための関数: キーワードの長さに基づいて待機時間を調整
        let get_wait_time = |index: usize, total: usize| -> u64 {
            // 高リスクのキーワード長では、特に最初と最後のバックスペースをより慎重に
            if is_high_risk_length && (index == 0 || index == total - 1) {
                return 50; // より長い待機時間
            }
            
            // 最初と最後のバックスペースはより長く待機
            if index == 0 || index == total - 1 {
                return 40; // より長い待機時間
            }
            
            // 長いキーワードの場合は待機時間を長くする
            if total > 5 {
                return 30; // 少し長めの待機時間
            }
            
            20 // 標準的な待機時間
        };
        
        // カーソル位置を安定させるためにバックスペースを丁寧に実行
        for i in 0..count {
            // カーソル位置の安定化のため、特に最初のバックスペース前に追加の待機
            if i == 0 && count > 5 {
                thread::sleep(Duration::from_millis(30));
            }
            
            // 高リスクの長さで中間点の場合、追加の安定化
            if is_high_risk_length && i == count / 2 {
                thread::sleep(Duration::from_millis(20));
            }
            
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
            thread::sleep(Duration::from_millis(20));
            
            // バックスペースを解放
            let sent_up = unsafe {
                SendInput(&[key_up], std::mem::size_of::<INPUT>() as i32)
            };
            
            if sent_up != 1 {
                log::error!("Failed to send backspace key up event for backspace {}", i + 1);
                success = false;
            }
            
            // 次のバックスペース前の待機時間は、位置や長さによって動的に調整
            let wait_time = get_wait_time(i, count);
            thread::sleep(Duration::from_millis(wait_time));
        }
        
        log::debug!("Completed sending {} backspace events, success: {}", count, success);
        
        // 最後の操作後に長めに待機して、システムが処理する時間を与える
        // 長いキーワードの場合は待機時間を長く
        let final_wait = if is_high_risk_length { 150 } else if count > 5 { 120 } else { 100 };
        thread::sleep(Duration::from_millis(final_wait));
        
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