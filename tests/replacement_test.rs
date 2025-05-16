use std::sync::{Arc, Mutex};
use swifttype::config::Settings;
use swifttype::config::settings::{Snippet, SnippetType};
use swifttype::replacement::ReplacementEngine;
use swifttype::replacement::formatter::format_dynamic_content;

#[test]
fn test_format_dynamic_content() {
    let result = format_dynamic_content("Today is {date:yyyy/MM/dd}");
    assert!(result.starts_with("Today is "));
    assert!(result.contains("/"));
    
    let result = format_dynamic_content("Plain text without format");
    assert_eq!(result, "Plain text without format");
}

#[test]
fn test_check_for_replacements() {
    // テスト用の設定を作成
    let mut settings = Settings::default();
    settings.enabled = true;
    settings.snippets = vec![
        Snippet::new(
            "Test Snippet 1".to_string(),
            "test1".to_string(),
            "Replacement 1".to_string(),
            SnippetType::Static,
            "Test".to_string(),
        ),
        Snippet::new(
            "Test Snippet 2".to_string(),
            "test2".to_string(),
            "Replacement 2".to_string(),
            SnippetType::Static,
            "Test".to_string(),
        ),
    ];
    
    let settings = Arc::new(Mutex::new(settings));
    let engine = ReplacementEngine::new(settings);
    
    // キーワードが含まれていない場合
    let result = engine.check_for_replacements("This is a test");
    assert!(result.is_none());
    
    // キーワードが含まれている場合
    let result = engine.check_for_replacements("This is a test1");
    assert!(result.is_some());
    let (keyword, replacement) = result.unwrap();
    assert_eq!(keyword, "test1");
    assert_eq!(replacement, "Replacement 1");
    
    // 別のキーワードが含まれている場合
    let result = engine.check_for_replacements("Another test2");
    assert!(result.is_some());
    let (keyword, replacement) = result.unwrap();
    assert_eq!(keyword, "test2");
    assert_eq!(replacement, "Replacement 2");
} 