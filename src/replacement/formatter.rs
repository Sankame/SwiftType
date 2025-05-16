use chrono::Local;
use regex::Regex;
use std::sync::OnceLock;

/// 正規表現パターンのキャッシュ
fn date_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"\{date:([^}]+)\}").unwrap())
}

/// 動的コンテンツをフォーマットする
/// 
/// # 引数
/// * `template` - フォーマットするテンプレート文字列
/// 
/// # 戻り値
/// フォーマット済みの文字列
pub fn format_dynamic_content(template: &str) -> String {
    // yyyy/MM/ddのようなパターンが直接指定されている場合は日付として処理
    if template.contains("yyyy") || template.contains("MM") || template.contains("dd") ||
       template.contains("HH") || template.contains("mm") || template.contains("ss") {
        return format_date(template);
    }
    
    let mut result = template.to_string();
    
    // {date:...}パターンの置換
    if template.contains("{date:") {
        if let Some(date_re) = date_pattern().is_match(template).then(|| date_pattern()) {
            result = date_re.replace_all(&result, |caps: &regex::Captures| {
                let format = &caps[1];
                format_date(format)
            }).to_string();
        }
    }
    
    result
}

/// 日付をフォーマットする補助関数
fn format_date(format: &str) -> String {
    let now = Local::now();
    
    // chrono形式に変換
    let chrono_format = format
        .replace("yyyy", "%Y")
        .replace("yy", "%y")
        .replace("MM", "%m")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S");
    
    now.format(&chrono_format).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    
    #[test]
    fn test_format_static_content() {
        let result = format_dynamic_content("Hello, World!");
        assert_eq!(result, "Hello, World!");
    }
    
    #[test]
    fn test_format_date() {
        let now = Local::now();
        
        // YYYYMMDDフォーマット
        let result = format_dynamic_content("{date:yyyyMMdd}");
        let expected = now.format("%Y%m%d").to_string();
        assert_eq!(result, expected);
        
        // YYYY/MM/DDフォーマット
        let result = format_dynamic_content("{date:yyyy/MM/dd}");
        let expected = now.format("%Y/%m/%d").to_string();
        assert_eq!(result, expected);
    }
    
    #[test]
    fn test_format_time() {
        let template = "{date:HH:mm:ss}";
        let result = format_dynamic_content(template);
        assert!(result.len() == 8); // HH:MM:SS形式で8文字
        assert!(result.contains(":"));
    }
    
    #[test]
    fn test_multiple_replacements() {
        let template = "Date: {date:yyyy/MM/dd} Time: {date:HH:mm:ss}";
        let result = format_dynamic_content(template);
        assert!(result.starts_with("Date: "));
        assert!(result.contains(" Time: "));
    }
} 