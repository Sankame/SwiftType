use swifttype::keyboard::KeyboardState;

#[test]
fn test_keyboard_state() {
    // キーボード状態の作成
    let mut keyboard_state = KeyboardState::new(10);
    
    // 初期状態は空のバッファ
    assert_eq!(keyboard_state.get_buffer(), "");
    
    // キーの追加
    keyboard_state.add_char('a');
    assert_eq!(keyboard_state.get_buffer(), "a");
    
    keyboard_state.add_char('b');
    keyboard_state.add_char('c');
    assert_eq!(keyboard_state.get_buffer(), "abc");
    
    // キーワードのチェック
    assert!(keyboard_state.check_keyword("abc"));
    assert!(keyboard_state.check_keyword("bc"));
    assert!(keyboard_state.check_keyword("c"));
    assert!(!keyboard_state.check_keyword("d"));
    assert!(!keyboard_state.check_keyword("ab"));
    
    // バッファのクリア
    keyboard_state.clear_buffer();
    assert_eq!(keyboard_state.get_buffer(), "");
    
    // バッファのサイズ制限のテスト
    for i in 0..15 {
        keyboard_state.add_char((b'a' + (i % 26)) as char);
    }
    
    // 最大サイズは10なので、最初の5文字は切り捨てられる
    assert_eq!(keyboard_state.get_buffer().len(), 10);
    assert_eq!(keyboard_state.get_buffer(), "fghijklmno");
}

#[test]
fn test_replace_keyword() {
    let mut keyboard_state = KeyboardState::new(20);
    
    // キーボードバッファに「hello world」を追加
    for c in "hello world".chars() {
        keyboard_state.add_char(c);
    }
    
    // 「world」を削除して置換
    let replaced = keyboard_state.replace_keyword("world", "");
    assert!(replaced);
    assert_eq!(keyboard_state.get_buffer(), "hello ");
    
    // 存在しないキーワードは置換できない
    let replaced = keyboard_state.replace_keyword("not_found", "");
    assert!(!replaced);
    assert_eq!(keyboard_state.get_buffer(), "hello ");
} 