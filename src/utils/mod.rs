use std::sync::{Arc, Mutex};

/// アプリケーションの終了ハンドラ
/// 
/// # 引数
/// * `should_exit` - 終了フラグ
/// 
/// # 戻り値
/// 終了するかどうか
pub fn check_should_exit(should_exit: &Arc<Mutex<bool>>) -> bool {
    if let Ok(flag) = should_exit.lock() {
        *flag
    } else {
        false
    }
}

/// 既知のテキスト置換ツールのプロセス名リスト
const CONFLICTING_TOOLS: &[&str] = &[
    "PhraseExpress.exe",
    "TextExpander.exe",
    "Breevy.exe",
    "TypeItIn.exe",
    "AutoHotkey.exe",
    "ActiveWords.exe",
    "FastKeys.exe",
    "AutoText.exe",
    "TyperTask.exe",
];

/// 競合する可能性のあるテキスト置換ツールが実行中かどうかをチェックする
/// 
/// # 戻り値
/// 見つかった競合ツールのリスト
pub fn check_conflicting_tools() -> Vec<String> {
    use windows::Win32::System::ProcessStatus::EnumProcesses;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
    use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
    use windows::Win32::Foundation::{HANDLE, CloseHandle, BOOL};
    
    let mut found_tools = Vec::new();
    
    unsafe {
        let mut processes = [0u32; 1024];
        let mut needed: u32 = 0;
        
        // プロセスIDのリストを取得
        let enum_result = EnumProcesses(processes.as_mut_ptr(), (processes.len() * std::mem::size_of::<u32>()) as u32, &mut needed);
        if enum_result.as_bool() {
            let count = needed as usize / std::mem::size_of::<u32>();
            
            for i in 0..count {
                if processes[i] != 0 {
                    // プロセスを開く
                    let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, processes[i]);
                    
                    if let Ok(process) = process {
                        let mut name_buf = [0u16; 260]; // MAX_PATH
                        
                        // プロセス名を取得
                        let name_result = GetModuleBaseNameW(process, None, &mut name_buf);
                        if name_result != 0 {
                            let len = name_buf.iter().position(|&c| c == 0).unwrap_or(name_buf.len());
                            let process_name = String::from_utf16_lossy(&name_buf[..len]);
                            
                            // 既知の競合ツールとマッチするか確認
                            for tool in CONFLICTING_TOOLS {
                                if process_name.eq_ignore_ascii_case(tool) {
                                    found_tools.push(process_name.clone());
                                    break;
                                }
                            }
                        }
                        
                        // プロセスハンドルを閉じる
                        CloseHandle(process);
                    }
                }
            }
        }
    }
    
    found_tools
}

/// 自動起動の設定
/// 
/// # 引数
/// * `enable` - 有効にするかどうか
/// 
/// # 戻り値
/// 成功したかどうか
pub fn set_auto_startup(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    use std::env;
    use windows::Win32::UI::Shell::SHGetFolderPathW;
    use windows::Win32::UI::Shell::CSIDL_STARTUP;
    use windows::Win32::Foundation::MAX_PATH;
    
    // 実行ファイルのパスを取得
    let exe_path = match env::current_exe() {
        Ok(path) => path,
        Err(e) => {
            log::error!("Failed to get executable path: {}", e);
            return Err(Box::new(e));
        }
    };
    
    log::debug!("Executable path: {:?}", exe_path);
    
    // スタートアップフォルダのパスを取得
    let mut path_buf = [0u16; MAX_PATH as usize];
    let startup_folder = unsafe {
        match SHGetFolderPathW(
            None,
            CSIDL_STARTUP as i32,
            None,
            0,
            &mut path_buf,
        ) {
            Ok(_) => {
                let len = path_buf.iter().position(|&c| c == 0).unwrap_or(path_buf.len());
                String::from_utf16_lossy(&path_buf[..len])
            },
            Err(e) => {
                log::error!("Failed to get startup folder path: {:?}", e);
                return Err(Box::new(e));
            }
        }
    };
    
    log::debug!("Startup folder: {}", startup_folder);
    
    // ショートカットファイルのパス
    let shortcut_path = std::path::Path::new(&startup_folder).join("SwiftType.lnk");
    log::debug!("Shortcut path: {:?}", shortcut_path);
    
    if enable {
        // ショートカットを作成する
        match create_shortcut(&exe_path.to_string_lossy(), &shortcut_path.to_string_lossy()) {
            Ok(_) => {
                log::info!("Auto-startup shortcut created successfully at {:?}", shortcut_path);
            },
            Err(e) => {
                log::error!("Failed to create auto-startup shortcut: {}", e);
                return Err(e);
            }
        }
    } else {
        // ショートカットが存在する場合は削除する
        if shortcut_path.exists() {
            match std::fs::remove_file(&shortcut_path) {
                Ok(_) => {
                    log::info!("Auto-startup shortcut removed successfully from {:?}", shortcut_path);
                },
                Err(e) => {
                    log::error!("Failed to remove auto-startup shortcut: {}", e);
                    return Err(Box::new(e));
                }
            }
        } else {
            log::debug!("Auto-startup shortcut doesn't exist, nothing to remove");
        }
    }
    
    Ok(())
}

/// Windowsショートカットを作成する
/// 
/// # 引数
/// * `target_path` - ターゲットファイルのパス
/// * `shortcut_path` - ショートカットファイルのパス
/// 
/// # 戻り値
/// 成功したかどうか
fn create_shortcut(target_path: &str, shortcut_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::ptr::null_mut;
    use windows::core::{PCWSTR, ComInterface};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitialize, CLSCTX_INPROC_SERVER};
    use windows::Win32::UI::Shell::IShellLinkW;
    use windows::Win32::System::Com::IPersistFile;
    
    log::debug!("Creating shortcut: target='{}', shortcut='{}'", target_path, shortcut_path);
    
    unsafe {
        // COM を初期化
        match CoInitialize(Some(null_mut())) {
            Ok(_) => {
                log::debug!("COM initialized successfully");
            },
            Err(e) => {
                log::error!("Failed to initialize COM: {:?}", e);
                return Err(Box::new(e));
            }
        }
        
        // ShellLink オブジェクトを作成
        let shell_link: IShellLinkW = match CoCreateInstance(
            &windows::Win32::UI::Shell::ShellLink,
            None,
            CLSCTX_INPROC_SERVER
        ) {
            Ok(link) => {
                log::debug!("ShellLink object created successfully");
                link
            },
            Err(e) => {
                log::error!("Failed to create ShellLink object: {:?}", e);
                return Err(Box::new(e));
            }
        };
        
        // リンクのプロパティを設定
        let target_path_w = windows_to_wide(target_path);
        match shell_link.SetPath(PCWSTR(target_path_w.as_ptr())) {
            Ok(_) => {
                log::debug!("ShellLink path set to '{}'", target_path);
            },
            Err(e) => {
                log::error!("Failed to set ShellLink path: {:?}", e);
                return Err(Box::new(e));
            }
        }
        
        // IPersistFile インターフェースを取得
        let persist_file: IPersistFile = match shell_link.cast() {
            Ok(file) => {
                log::debug!("IPersistFile interface acquired");
                file
            },
            Err(e) => {
                log::error!("Failed to get IPersistFile interface: {:?}", e);
                return Err(Box::new(e));
            }
        };
        
        // ショートカットファイルを保存
        let shortcut_path_w = windows_to_wide(shortcut_path);
        match persist_file.Save(PCWSTR(shortcut_path_w.as_ptr()), true) {
            Ok(_) => {
                log::debug!("Shortcut file saved successfully to '{}'", shortcut_path);
            },
            Err(e) => {
                log::error!("Failed to save shortcut file: {:?}", e);
                return Err(Box::new(e));
            }
        }
    }
    
    Ok(())
}

/// 文字列をワイド文字列に変換する
fn windows_to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
} 