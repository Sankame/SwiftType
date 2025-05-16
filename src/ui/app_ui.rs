use egui::{self, CentralPanel, ScrollArea, TopBottomPanel, Ui};
use std::sync::{Arc, Mutex};

use crate::config::{ConfigManager, Settings};
use crate::keyboard::KeyboardState;
use crate::replacement::ReplacementEngine;
use super::{ThemeMode, constants, settings_view, snippet_editor};

/// アプリケーションのUI状態
#[derive(Debug)]
pub struct AppUiState {
    /// 設定マネージャー
    pub config_manager: Arc<Mutex<ConfigManager>>,
    /// アプリケーションの設定
    pub settings: Arc<Mutex<Settings>>,
    /// キーボードの状態
    pub keyboard_state: Arc<Mutex<KeyboardState>>,
    /// テキスト置換エンジン
    pub replacement_engine: Arc<Mutex<ReplacementEngine>>,
    /// テーマモード
    pub theme: ThemeMode,
    /// 選択中のタブ
    pub selected_tab: Tab,
    /// 選択中のスニペットのインデックス
    pub selected_snippet_index: Option<usize>,
}

/// アプリケーションのタブ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// スニペット一覧
    Snippets,
    /// アプリケーション設定
    Settings,
    /// スニペットエディタ
    Editor,
}

impl AppUiState {
    /// 新しいUI状態を作成する
    pub fn new(
        config_manager: Arc<Mutex<ConfigManager>>,
        settings: Arc<Mutex<Settings>>,
        keyboard_state: Arc<Mutex<KeyboardState>>,
        replacement_engine: Arc<Mutex<ReplacementEngine>>,
    ) -> Self {
        Self {
            config_manager,
            settings,
            keyboard_state,
            replacement_engine,
            theme: ThemeMode::Dark,
            selected_tab: Tab::Snippets,
            selected_snippet_index: None,
        }
    }
    
    /// タブを切り替える
    pub fn switch_tab(&mut self, tab: Tab) {
        self.selected_tab = tab;
    }
    
    /// テーマを切り替える
    pub fn toggle_theme(&mut self) {
        self.theme.toggle();
    }
}

/// アプリケーションのUI
pub struct AppUi {
    state: AppUiState,
}

impl AppUi {
    /// 新しいアプリケーションUIを作成する
    pub fn new(state: AppUiState) -> Self {
        Self { state }
    }
    
    /// 設定へのアクセスを提供する
    pub fn settings(&self) -> &Arc<Mutex<Settings>> {
        &self.state.settings
    }
    
    /// UIを更新する
    pub fn update(&mut self, ctx: &egui::Context) {
        super::setup_context(ctx, self.state.theme);
        
        self.render_top_panel(ctx);
        self.render_central_panel(ctx);
        self.render_bottom_panel(ctx);
    }
    
    /// 上部パネルを描画する
    fn render_top_panel(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(constants::APP_TITLE);
                ui.add_space(10.0);
                
                if ui.selectable_label(self.state.selected_tab == Tab::Snippets, "Snippets").clicked() {
                    self.state.switch_tab(Tab::Snippets);
                }
                
                if ui.selectable_label(self.state.selected_tab == Tab::Settings, "Settings").clicked() {
                    self.state.switch_tab(Tab::Settings);
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let theme_label = match self.state.theme {
                        ThemeMode::Light => "🌙 Dark",
                        ThemeMode::Dark => "☀️ Light",
                    };
                    
                    if ui.button(theme_label).clicked() {
                        self.state.toggle_theme();
                    }
                });
            });
        });
    }
    
    /// 中央パネルを描画する
    fn render_central_panel(&mut self, ctx: &egui::Context) {
        CentralPanel::default().show(ctx, |ui| {
            match self.state.selected_tab {
                Tab::Snippets => self.render_snippets_tab(ui),
                Tab::Settings => self.render_settings_tab(ui),
                Tab::Editor => self.render_editor_tab(ui),
            }
        });
    }
    
    /// スニペット一覧タブを描画する
    fn render_snippets_tab(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("Snippets");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Add Snippet").clicked() {
                    self.state.selected_snippet_index = None;
                    self.state.switch_tab(Tab::Editor);
                }
            });
        });
        
        ui.add_space(10.0);
        
        ScrollArea::vertical().show(ui, |ui| {
            // 設定を取得して所有権を得る
            let snippets = {
                if let Ok(settings) = self.state.settings.lock() {
                    settings.snippets.clone()
                } else {
                    return;
                }
            };
            
            // 更新するスニペットを格納する
            let mut updated_snippets = snippets.clone();
            let mut is_updated = false;
            
            // UI表示処理
            for (index, snippet) in snippets.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.checkbox(
                        &mut updated_snippets[index].enabled,
                        &snippet.name,
                    ).changed() {
                        is_updated = true;
                    }
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Edit").clicked() {
                            let tab_switch = {
                                self.state.selected_snippet_index = Some(index);
                                true
                            };
                            if tab_switch {
                                self.state.switch_tab(Tab::Editor);
                            }
                        }
                    });
                });
                
                ui.horizontal(|ui| {
                    ui.label(format!("Keyword: {}", snippet.keyword));
                    ui.label(format!("Category: {}", snippet.category));
                });
                
                ui.separator();
            }
            
            // 設定を更新
            if is_updated {
                // 設定のロックを取得して更新
                if let Ok(mut settings) = self.state.settings.lock() {
                    settings.snippets = updated_snippets;
                    
                    // 設定ファイルに保存
                    // 別のロックを取得するため、一時的にdroptさせる
                    drop(settings);
                    
                    if let Ok(mut config_manager) = self.state.config_manager.lock() {
                        // 再度設定を取得
                        if let Ok(settings) = self.state.settings.lock() {
                            let _ = config_manager.update_settings(settings.clone());
                        }
                    }
                }
            }
        });
    }
    
    /// 設定タブを描画する
    fn render_settings_tab(&mut self, ui: &mut Ui) {
        ui.heading("Settings");
        ui.add_space(10.0);
        
        // 元の設定値を取得
        let (mut enabled, mut start_with_system) = {
            if let Ok(settings) = self.state.settings.lock() {
                (settings.enabled, settings.start_with_system)
            } else {
                return;
            }
        };
        
        // UI要素の表示
        let enabled_changed = ui.checkbox(&mut enabled, "Enable SwiftType").changed();
        let startup_changed = ui.checkbox(&mut start_with_system, "Start with system").changed();
        
        // 変更があれば設定を更新
        if enabled_changed || startup_changed {
            if let Ok(mut settings) = self.state.settings.lock() {
                settings.enabled = enabled;
                settings.start_with_system = start_with_system;
                
                // 設定のロックを解放して保存
                drop(settings);
                
                if let Ok(mut config_manager) = self.state.config_manager.lock() {
                    if let Ok(settings) = self.state.settings.lock() {
                        let _ = config_manager.update_settings(settings.clone());
                    }
                }
            }
        }
    }
    
    /// エディタタブを描画する
    fn render_editor_tab(&mut self, ui: &mut Ui) {
        // スニペットの取得
        let snippet_to_edit = if let Some(index) = self.state.selected_snippet_index {
            // 既存のスニペットを編集
            if let Ok(settings) = self.state.settings.lock() {
                if index < settings.snippets.len() {
                    Some((settings.snippets[index].clone(), true, index))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            // 新しいスニペットを作成
            Some((crate::config::settings::Snippet::new(
                String::new(),
                String::new(),
                String::new(),
                crate::config::settings::SnippetType::Static,
                "一般".to_string(),
            ), false, 0))
        };
        
        if let Some((mut snippet, is_editing, index)) = snippet_to_edit {
            // UI表示
            if is_editing {
                ui.heading("Edit Snippet");
            } else {
                ui.heading("Create New Snippet");
            }
            
            let edited = snippet_editor::render_snippet_editor(ui, &mut snippet);
            
            // 編集されたスニペットの保存
            if edited && (!is_editing || !snippet.name.is_empty() && !snippet.keyword.is_empty()) {
                let switch_to_snippets = {
                    if let Ok(mut settings) = self.state.settings.lock() {
                        if is_editing && index < settings.snippets.len() {
                            // 既存のスニペットを更新
                            settings.snippets[index] = snippet;
                        } else if !is_editing {
                            // 新しいスニペットを追加
                            settings.snippets.push(snippet);
                        }
                        
                        // 設定のロックを解放して保存
                        drop(settings);
                        
                        if let Ok(mut config_manager) = self.state.config_manager.lock() {
                            if let Ok(settings) = self.state.settings.lock() {
                                let _ = config_manager.update_settings(settings.clone());
                            }
                        }
                        
                        // 新規作成時のみスニペット一覧に戻る
                        !is_editing
                    } else {
                        false
                    }
                };
                
                // スニペット一覧に戻る
                if switch_to_snippets {
                    self.state.switch_tab(Tab::Snippets);
                }
            }
        }
    }
    
    /// 下部パネルを描画する
    fn render_bottom_panel(&mut self, ctx: &egui::Context) {
        TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Status: ");
                
                if let Ok(settings) = self.state.settings.lock() {
                    let status = if settings.enabled {
                        "Enabled"
                    } else {
                        "Disabled"
                    };
                    
                    ui.label(status);
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("SwiftType v0.1.0");
                });
            });
        });
    }
} 