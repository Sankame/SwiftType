use std::sync::{Arc, Mutex};
use std::cell::Cell;
use once_cell::sync::OnceCell;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx,
    WH_KEYBOARD_LL, KBDLLHOOKSTRUCT, LLKHF_INJECTED, HHOOK, KBDLLHOOKSTRUCT_FLAGS,
};

use crate::keyboard::{KeyboardState, SharedKeyboardState};
use crate::replacement::ReplacementEngine;

// グローバル状態のためのスレッドセーフなOnceCell
static GLOBAL_KEYBOARD_STATE: OnceCell<std::sync::Weak<Mutex<KeyboardState>>> = OnceCell::new();
static GLOBAL_REPLACEMENT_ENGINE: OnceCell<std::sync::Weak<Mutex<ReplacementEngine>>> = OnceCell::new();

/// キーボードフックのコールバック関数
pub extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // HC_ACTIONは0なので、直接比較
    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    
    let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    
    // キーが注入されたものであれば無視する
    if kb.flags & KBDLLHOOKSTRUCT_FLAGS(LLKHF_INJECTED.0) != KBDLLHOOKSTRUCT_FLAGS(0) {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }
    
    // グローバルなキーボード状態を取得
    let keyboard_state = GLOBAL_KEYBOARD_STATE.get()
        .and_then(|state| state.upgrade());
    
    let replacement_engine = GLOBAL_REPLACEMENT_ENGINE.get()
        .and_then(|engine| engine.upgrade());
    
    if let (Some(keyboard_state), Some(replacement_engine)) = (keyboard_state, replacement_engine) {
        // イベントを処理
        process_key_event(keyboard_state, replacement_engine, wparam, kb);
    }
    
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// キーボードフック
pub struct KeyboardHook {
    hook: Cell<isize>,
    keyboard_state: SharedKeyboardState,
    replacement_engine: Arc<Mutex<ReplacementEngine>>,
}

impl KeyboardHook {
    /// 新しいキーボードフックを作成する
    pub fn new(
        keyboard_state: SharedKeyboardState,
        replacement_engine: Arc<Mutex<ReplacementEngine>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            hook: Cell::new(0),
            keyboard_state,
            replacement_engine,
        })
    }
    
    /// キーボードフックを開始する
    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // グローバル参照を設定（安全に初期化）
        let _ = GLOBAL_KEYBOARD_STATE.set(Arc::downgrade(&self.keyboard_state));
        let _ = GLOBAL_REPLACEMENT_ENGINE.set(Arc::downgrade(&self.replacement_engine));
        
        // キーボードフックを設定
        unsafe {
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                None,
                0,
            )?;
            
            // フックハンドルを保存（内部可変性を使用）
            self.hook.set(hook.0);
        }
        
        Ok(())
    }
}

impl Drop for KeyboardHook {
    fn drop(&mut self) {
        unsafe {
            let hook_value = self.hook.get();
            if hook_value != 0 {
                let hook_handle = HHOOK(hook_value);
                let _ = UnhookWindowsHookEx(hook_handle);
            }
        }
    }
}

/// キー入力イベントを処理する
fn process_key_event(
    keyboard_state: Arc<Mutex<KeyboardState>>,
    replacement_engine: Arc<Mutex<ReplacementEngine>>,
    wparam: WPARAM,
    kb: &KBDLLHOOKSTRUCT,
) {
    // キーボード状態を更新
    if let Ok(mut state) = keyboard_state.lock() {
        // キー入力を処理
        state.process_key_event(wparam.0 as u32, kb.vkCode);
        
        // キーワードの置換を試みる
        if let Ok(engine) = replacement_engine.lock() {
            if state.should_check_replacement() {
                // バッファから現在のキーワード候補を取得
                let keyword = state.get_keyword_candidate();
                
                // キーワードが見つかれば置換
                if !keyword.is_empty() {
                    log::debug!("Checking for replacement with keyword: '{}'", keyword);
                    if let Some(replacement) = engine.check_for_replacements(&keyword) {
                        log::debug!("Found replacement: '{}' for keyword: '{}'", replacement, keyword);
                        
                        // バッファをクリア (検出されたキーワードを消去)
                        // 注: これにより連続的な置換を防止する
                        state.clear_buffer();
                        
                        // ここでmutexをドロップして、置換処理中に他のキー入力が処理できるようにする
                        drop(state);
                        drop(engine);
                        
                        // 再度エンジンを取得して置換実行
                        if let Ok(engine) = replacement_engine.lock() {
                            // 置換実行 - 改良されたバックスペースとペースト処理を使用
                            if engine.perform_replacement_with_backspace(&replacement, keyword.len()) {
                                log::debug!("Successfully replaced '{}' with '{}'", keyword, replacement);
                            } else {
                                log::error!("Failed to replace '{}' with '{}'", keyword, replacement);
                                
                                // 置換が失敗した場合、キーボード状態を明示的にリセット
                                if let Ok(mut state) = keyboard_state.lock() {
                                    state.clear_buffer();
                                }
                                
                                // モディファイアキーをリセットして、キーボードを正常な状態に戻す
                                engine.reset_modifier_keys();
                            }
                        }
                    }
                }
            }
        }
    }
} 