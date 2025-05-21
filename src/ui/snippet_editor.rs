use egui::{self, Ui};
use crate::config::settings::{Snippet, SnippetType};

/// キーワードのバリデーション
/// 
/// # 引数
/// * `keyword` - バリデーション対象のキーワード
/// 
/// # 戻り値
/// キーワードが有効かどうか
fn validate_keyword(keyword: &str) -> bool {
    // 特殊文字のチェック
    !keyword.contains('=') && !keyword.contains(';') && !keyword.contains(',')
}

/// スニペットエディタを描画する
/// 
/// # 引数
/// * `ui` - EGUIのUIコンテキスト
/// * `snippet` - 編集対象のスニペット
/// 
/// # 戻り値
/// スニペットが編集されたかどうか
pub fn render_snippet_editor(ui: &mut Ui, snippet: &mut Snippet) -> bool {
    let mut edited = false;
    
    ui.horizontal(|ui| {
        ui.label("Name:");
        edited |= ui.text_edit_singleline(&mut snippet.name).changed();
    });
    
    ui.horizontal(|ui| {
        ui.label("Keyword:");
        let response = ui.text_edit_singleline(&mut snippet.keyword);
        edited |= response.changed();
        
        // キーワードが変更された場合、バリデーションを行う
        if response.changed() {
            if !validate_keyword(&snippet.keyword) {
                ui.label("⚠ Keywords should not contain special characters (=, ;, ,)");
                
                // 特殊文字を自動的に置き換える
                let safe_keyword = snippet.keyword.replace('=', "_")
                                            .replace(';', "_")
                                            .replace(',', "_");
                
                if safe_keyword != snippet.keyword {
                    snippet.keyword = safe_keyword;
                    ui.label("Special characters have been replaced with '_'");
                }
            }
        }
    });
    
    ui.horizontal(|ui| {
        ui.label("Category:");
        edited |= ui.text_edit_singleline(&mut snippet.category).changed();
    });
    
    ui.horizontal(|ui| {
        ui.label("Type:");
        
        let mut is_static = snippet.snippet_type == SnippetType::Static;
        if ui.radio_value(&mut is_static, true, "Static").clicked() {
            snippet.snippet_type = SnippetType::Static;
            edited = true;
        }
        
        let mut is_dynamic = snippet.snippet_type == SnippetType::Dynamic;
        if ui.radio_value(&mut is_dynamic, true, "Dynamic").clicked() {
            snippet.snippet_type = SnippetType::Dynamic;
            edited = true;
        }
    });
    
    ui.label("Content:");
    let text_height = if snippet.content.contains('\n') { 120.0 } else { 80.0 };
    let response = ui.text_edit_multiline(&mut snippet.content);
    let text_edit_height = response.rect.height();
    if text_edit_height < text_height {
        ui.allocate_space(egui::Vec2::new(0.0, text_height - text_edit_height));
    }
    edited |= response.changed();
    
    // 動的コンテンツのヘルプ
    if snippet.snippet_type == SnippetType::Dynamic {
        ui.separator();
        ui.label("Dynamic Content Format:");
        ui.label("Use {date:format} for date and time:");
        
        ui.horizontal(|ui| {
            if ui.button("Date (YYYY/MM/DD)").clicked() {
                snippet.content += "{date:yyyy/MM/dd}";
                edited = true;
            }
            
            if ui.button("Date (YYYYMMDD)").clicked() {
                snippet.content += "{date:yyyyMMdd}";
                edited = true;
            }
            
            if ui.button("Time (HH:MM:SS)").clicked() {
                snippet.content += "{date:HH:mm:ss}";
                edited = true;
            }
        });
    }
    
    ui.separator();
    
    ui.horizontal(|ui| {
        ui.checkbox(&mut snippet.enabled, "Enabled");
        edited |= ui.button("Save").clicked();
    });
    
    edited
}
