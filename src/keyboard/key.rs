/// キーコードを表す構造体
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key(pub u32);

impl Key {
    /// 仮想キーコードからキーを作成する
    pub fn from_virtual_key(vk: u32) -> Self {
        Self(vk)
    }
    
    /// キーをキャラクターに変換する
    pub fn to_char(&self) -> Option<char> {
        // 基本的なASCIIマッピング
        match self.0 {
            0x08 => None, // バックスペース
            0x09 => None, // タブ
            0x0D => None, // エンター
            0x1B => None, // ESC
            0x20 => Some(' '), // スペース
            
            // 数字
            0x30..=0x39 => Some((b'0' + (self.0 - 0x30) as u8) as char),
            
            // アルファベット (大文字として扱う)
            0x41..=0x5A => Some((b'a' + (self.0 - 0x41) as u8) as char),
            
            // テンキー
            0x60..=0x69 => Some((b'0' + (self.0 - 0x60) as u8) as char),
            
            // 演算子
            0x6A => Some('*'), // テンキー *
            0x6B => Some('+'), // テンキー +
            0x6D => Some('-'), // テンキー -
            0x6E => Some('.'), // テンキー .
            0x6F => Some('/'), // テンキー /
            
            // その他の記号
            0xBA => Some(';'),
            0xBB => Some('='),
            0xBC => Some(','),
            0xBD => Some('-'),
            0xBE => Some('.'),
            0xBF => Some('/'),
            0xC0 => Some('@'),
            0xDB => Some('['),
            0xDC => Some('\\'),
            0xDD => Some(']'),
            0xDE => Some('\''),
            0xE2 => Some('_'),
            
            // その他はキャラクターに変換できない
            _ => None,
        }
    }
} 