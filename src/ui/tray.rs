use std::sync::{Arc, Mutex};
use tray_icon::{
    menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent},
    TrayIcon, TrayIconBuilder, TrayEvent,
};
use crossbeam_channel::Receiver;
use std::io::Cursor;

use crate::config::Settings;

// メニュー項目のインデックス定数
const MENU_INDEX_SHOW: usize = 0;
const MENU_INDEX_ENABLED: usize = 1;
const MENU_INDEX_EXIT: usize = 3; // セパレータがあるため3番目

/// トレイアイコンの状態
pub struct TrayIconState {
    /// トレイアイコン
    _tray_icon: TrayIcon,
    /// メニューイベントの受信器
    menu_channel: Receiver<MenuEvent>,
    /// トレイアイコンイベントの受信器
    tray_channel: Receiver<TrayEvent>,
    /// アプリケーションの設定
    settings: Arc<Mutex<Settings>>,
    /// ウィンドウが表示されているかどうか
    pub show_window: Arc<Mutex<bool>>,
    /// アプリケーションを終了するかどうか
    pub should_exit: Arc<Mutex<bool>>,
}

impl TrayIconState {
    /// 新しいトレイアイコンを作成する
    pub fn new(settings: Arc<Mutex<Settings>>) -> Result<Self, Box<dyn std::error::Error>> {
        // トレイアイコンのメニューを作成
        let tray_menu = Menu::new();
        
        // メニュー項目を作成（シンプルな英語テキストに変更）
        let show_item = MenuItem::new("Show", true, None);
        let enabled_item = MenuItem::new("Enable", true, None);
        let exit_item = MenuItem::new("Exit", true, None);
        
        // メニューに項目を追加（インデックス順に追加）
        tray_menu.append(&show_item);      // インデックス 0
        tray_menu.append(&enabled_item);   // インデックス 1
        tray_menu.append(&PredefinedMenuItem::separator()); // インデックス 2
        tray_menu.append(&exit_item);      // インデックス 3
        
        // アイコンデータを作成（デフォルトアイコン）
        let icon = {
            // まずファイルからロードを試みる
            let icon_data = match std::fs::read("assets/icon.ico") {
                Ok(data) => data,
                Err(_) => {
                    // 1x1の透明なPNGデータを埋め込む（最小限の代替アイコン）
                    vec![
                        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82,
                        0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137,
                        0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 252, 207, 192, 0, 0,
                        3, 1, 1, 0, 242, 213, 127, 36, 0, 0, 0, 0, 73, 69, 78, 68,
                        174, 66, 96, 130
                    ]
                }
            };
            
            load_icon(&icon_data)?
        };
        
        // トレイアイコンを作成
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("SwiftType")
            .with_icon(icon)
            .build()?;
        
        // メニューとトレイイベント受信のセットアップ
        let menu_receiver = MenuEvent::receiver().clone();
        let tray_receiver = TrayEvent::receiver().clone();
        
        Ok(Self {
            _tray_icon: tray_icon,
            menu_channel: menu_receiver,
            tray_channel: tray_receiver,
            settings,
            show_window: Arc::new(Mutex::new(true)), // 初期状態ではウィンドウを表示
            should_exit: Arc::new(Mutex::new(false)),
        })
    }
    
    /// トレイアイコンのイベントを処理する
    pub fn process_events(&mut self) {
        // メニューイベントを処理
        if let Ok(event) = self.menu_channel.try_recv() {
            log::debug!("Tray menu event received: {:?}", event);
            // メニューイベントからインデックス（ID）を取得して処理
            let index = event.id as usize;
            
            match index {
                MENU_INDEX_SHOW => { // Show
                    if let Ok(mut show_window) = self.show_window.lock() {
                        *show_window = true;
                    }
                }
                MENU_INDEX_ENABLED => { // Enabled
                    if let Ok(mut settings) = self.settings.lock() {
                        settings.enabled = !settings.enabled;
                    }
                }
                MENU_INDEX_EXIT => { // Exit
                    if let Ok(mut should_exit) = self.should_exit.lock() {
                        *should_exit = true;
                    }
                }
                _ => {}
            }
        }
        
        // トレイアイコンイベントを処理
        if let Ok(event) = self.tray_channel.try_recv() {
            log::debug!("Tray icon event received: {:?}", event);
            // 必要に応じてトレイアイコンイベントを処理することができます
        }
    }
}

/// アイコンデータをロードする
fn load_icon(data: &[u8]) -> Result<tray_icon::icon::Icon, Box<dyn std::error::Error>> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load(Cursor::new(data), image::ImageFormat::Ico)?
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    
    Ok(tray_icon::icon::Icon::from_rgba(icon_rgba, icon_width, icon_height)?)
} 