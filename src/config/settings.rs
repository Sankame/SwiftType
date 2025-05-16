use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// スニペットの種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnippetType {
    /// 静的なテキスト
    Static,
    /// 動的なコンテンツ（日付など）
    Dynamic,
}

/// スニペットの定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    /// スニペットの名前
    pub name: String,
    /// スニペットのキーワード（トリガー）
    pub keyword: String,
    /// スニペットの内容
    pub content: String,
    /// スニペットの種類
    pub snippet_type: SnippetType,
    /// スニペットのカテゴリ
    pub category: String,
    /// スニペットの有効/無効
    pub enabled: bool,
}

impl Snippet {
    /// 新しいスニペットを作成する
    pub fn new(
        name: String,
        keyword: String,
        content: String,
        snippet_type: SnippetType,
        category: String,
    ) -> Self {
        Self {
            name,
            keyword,
            content,
            snippet_type,
            category,
            enabled: true,
        }
    }
}

/// ホットキーの定義
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Hotkey {
    /// 修飾キー (Ctrl, Alt, Shift, Win)
    pub modifiers: u32,
    /// キーコード
    pub key_code: u32,
}

/// アプリケーションの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// アプリケーションの有効/無効
    pub enabled: bool,
    /// スニペットのリスト
    pub snippets: Vec<Snippet>,
    /// アプリケーションの起動時に自動的に起動するかどうか
    pub start_with_system: bool,
    /// 有効にするホットキー
    pub toggle_hotkey: Option<Hotkey>,
    /// ウィンドウを開くホットキー
    pub open_window_hotkey: Option<Hotkey>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut snippets = Vec::new();
        
        // 日付のスニペットを追加（英語表記に修正）
        snippets.push(Snippet::new(
            "Today's Date (YYYY/MM/DD)".to_string(),
            "ddate".to_string(),
            "yyyy/MM/dd".to_string(), // 直接日付フォーマットを指定
            SnippetType::Dynamic,
            "Date".to_string(),
        ));
        
        snippets.push(Snippet::new(
            "Today's Date (YYYYMMDD)".to_string(),
            "yyyymmdd".to_string(),
            "yyyyMMdd".to_string(), // 直接日付フォーマットを指定
            SnippetType::Dynamic,
            "Date".to_string(),
        ));
        
        snippets.push(Snippet::new(
            "Current Time".to_string(),
            "ttime".to_string(),
            "HH:mm:ss".to_string(), // 直接時刻フォーマットを指定
            SnippetType::Dynamic,
            "Time".to_string(),
        ));
        
        snippets.push(Snippet::new(
            "Timestamp".to_string(),
            "tstamp".to_string(),
            "yyyy-MM-dd HH:mm:ss".to_string(), // 直接タイムスタンプフォーマットを指定
            SnippetType::Dynamic,
            "Date".to_string(),
        ));
        
        snippets.push(Snippet::new(
            "Signature".to_string(),
            "sig".to_string(),
            "Best regards,\n\nJohn Doe\nEmail: example@example.com\nPhone: 555-123-4567".to_string(),
            SnippetType::Static,
            "Template".to_string(),
        ));
        
        Self {
            enabled: true,
            snippets,
            start_with_system: false,
            toggle_hotkey: None,
            open_window_hotkey: None,
        }
    }
} 