pub mod settings;

use std::path::PathBuf;

pub use settings::Settings;

/// アプリケーションの設定を管理する構造体
#[derive(Debug, Clone)]
pub struct ConfigManager {
    settings: Settings,
    config_path: PathBuf,
}

impl ConfigManager {
    /// 新しい設定マネージャーを作成する
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = Self::get_config_dir()?;
        std::fs::create_dir_all(&config_dir)?;
        
        let config_path = config_dir.join("settings.json");
        let settings = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let mut loaded_settings: Settings = serde_json::from_str(&content)?;
            
            // 既存の日本語タイトルやカテゴリを英語に変換
            for snippet in &mut loaded_settings.snippets {
                // 日本語タイトルを英語に変換
                match snippet.name.as_str() {
                    "今日の日付 (YYYY/MM/DD)" => snippet.name = "Today's Date (YYYY/MM/DD)".to_string(),
                    "今日の日付 (YYYYMMDD)" => snippet.name = "Today's Date (YYYYMMDD)".to_string(),
                    "現在時刻" => snippet.name = "Current Time".to_string(),
                    "タイムスタンプ" => snippet.name = "Timestamp".to_string(),
                    _ => {}
                }
                
                // 日本語カテゴリを英語に変換
                match snippet.category.as_str() {
                    "日付" => snippet.category = "Date".to_string(),
                    "時間" => snippet.category = "Time".to_string(),
                    "テンプレート" => snippet.category = "Templates".to_string(),
                    _ => {}
                }
                
                // 特殊文字を含むキーワードを安全な形式に変換
                if snippet.keyword.contains('=') || snippet.keyword.contains(';') || snippet.keyword.contains(',') {
                    let original = snippet.keyword.clone();
                    snippet.keyword = snippet.keyword.replace('=', "_")
                                              .replace(';', "_")
                                              .replace(',', "_");
                    log::info!("Sanitized keyword from '{}' to '{}'", original, snippet.keyword);
                }
            }
            
            loaded_settings
        } else {
            let default_settings = Settings::default();
            let serialized = serde_json::to_string_pretty(&default_settings)?;
            std::fs::write(&config_path, serialized)?;
            default_settings
        };
        
        Ok(Self {
            settings,
            config_path,
        })
    }
    
    /// 設定を取得する
    pub fn get_settings(&self) -> &Settings {
        &self.settings
    }
    
    /// 設定を変更する
    pub fn update_settings(&mut self, mut settings: Settings) -> Result<(), Box<dyn std::error::Error>> {
        // 保存前に特殊文字を含むキーワードを安全な形式に変換
        for snippet in &mut settings.snippets {
            if snippet.keyword.contains('=') || snippet.keyword.contains(';') || snippet.keyword.contains(',') {
                let original = snippet.keyword.clone();
                snippet.keyword = snippet.keyword.replace('=', "_")
                                          .replace(';', "_")
                                          .replace(',', "_");
                log::info!("Sanitized keyword from '{}' to '{}'", original, snippet.keyword);
            }
        }
        
        self.settings = settings;
        self.save()
    }
    
    /// 設定を保存する
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = serde_json::to_string_pretty(&self.settings)?;
        
        // 親ディレクトリが存在することを確認
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        
        // ファイルに書き込み
        match std::fs::write(&self.config_path, serialized) {
            Ok(()) => {
                log::debug!("Settings saved successfully to {:?}", self.config_path);
                Ok(())
            },
            Err(e) => {
                log::error!("Failed to save settings to {:?}: {}", self.config_path, e);
                Err(Box::new(e))
            }
        }
    }
    
    /// 設定ディレクトリのパスを取得する
    fn get_config_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "Could not find config directory".to_string())?
            .join("swifttype");
        Ok(config_dir)
    }
}
