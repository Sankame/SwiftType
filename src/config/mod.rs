pub mod settings;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
            serde_json::from_str(&content)?
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
    pub fn update_settings(&mut self, settings: Settings) -> Result<(), Box<dyn std::error::Error>> {
        self.settings = settings;
        self.save()
    }
    
    /// 設定を保存する
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = serde_json::to_string_pretty(&self.settings)?;
        std::fs::write(&self.config_path, serialized)?;
        Ok(())
    }
    
    /// 設定ディレクトリのパスを取得する
    fn get_config_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "Could not find config directory".to_string())?
            .join("swifttype");
        Ok(config_dir)
    }
} 