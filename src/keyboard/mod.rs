pub mod hook;
pub mod key;

pub use hook::KeyboardHook;
pub use key::Key;

use std::sync::{Arc, Mutex};

/// キーボード状態の共有参照型
pub type SharedKeyboardState = Arc<Mutex<KeyboardState>>;

/// キーボードの状態を管理するクラス
#[derive(Debug)]
pub struct KeyboardState {
    /// キー入力のバッファ
    buffer: Vec<char>,
    /// バッファの最大サイズ
    buffer_size: usize,
}

impl KeyboardState {
    /// 新しいキーボード状態を作成する
    /// 
    /// # 引数
    /// * `buffer_size` - バッファの最大サイズ
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(buffer_size),
            buffer_size,
        }
    }
    
    /// キー入力を処理する
    /// 
    /// # 引数
    /// * `msg` - Windowsメッセージ（WM_KEYDOWNなど）
    /// * `vk_code` - 仮想キーコード
    pub fn process_key_event(&mut self, msg: u32, vk_code: u32) {
        // WM_KEYDOWN (0x0100) または WM_SYSKEYDOWN (0x0104) の場合
        if msg == 0x0100 || msg == 0x0104 {
            if let Some(c) = Key::from_virtual_key(vk_code).to_char() {
                self.add_char(c);
            }
        }
    }
    
    /// 置換チェックを行うべきかを判断
    pub fn should_check_replacement(&self) -> bool {
        // 一定以上の文字が入力されていれば、置換チェックを行う
        self.buffer.len() >= 2
    }
    
    /// バッファに文字を追加する
    pub fn add_char(&mut self, c: char) {
        // 改行文字の場合はバッファをクリアする
        if c == '\n' || c == '\r' {
            log::debug!("Newline detected, clearing buffer");
            self.clear_buffer();
            return;
        }
        
        self.buffer.push(c);
        log::trace!("Added character '{}' to buffer, current buffer: '{}'", c, self.get_keyword_candidate());
        
        // バッファサイズを制限
        if self.buffer.len() > self.buffer_size {
            self.buffer.remove(0);
        }
    }
    
    /// バッファをクリアする
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }
    
    /// キーワード置換を行う
    /// 
    /// # 引数
    /// * `keyword` - 置換対象のキーワード
    /// * `_replacement` - 置換後のテキスト
    /// 
    /// # 戻り値
    /// 置換が成功したかどうか
    #[allow(dead_code)]
    pub fn replace_keyword(&mut self, keyword: &str, _replacement: &str) -> bool {
        // バッファから特定のキーワードを削除する
        let keyword_len = keyword.chars().count();
        if keyword_len > self.buffer.len() {
            return false;
        }
        
        // バッファの末尾がキーワードと一致するかチェック
        let start_index = self.buffer.len() - keyword_len;
        let buffer_slice: String = self.buffer[start_index..].iter().collect();
        
        if buffer_slice == keyword {
            // キーワードをバッファから削除
            self.buffer.truncate(start_index);
            log::debug!("Keyword '{}' matched and removed from buffer", keyword);
            true
        } else {
            false
        }
    }
    
    /// バッファの内容を取得する
    #[allow(dead_code)]
    pub fn get_buffer(&self) -> String {
        self.buffer.iter().collect()
    }
    
    /// 現在のキーワード候補を取得する
    pub fn get_keyword_candidate(&self) -> String {
        self.buffer.iter().collect()
    }
    
    /// キーワードが検出され置換された場合、バッファをクリアする
    #[allow(dead_code)]
    pub fn keyword_replaced(&mut self) {
        log::debug!("Keyword replacement completed, clearing buffer");
        self.clear_buffer();
    }
} 