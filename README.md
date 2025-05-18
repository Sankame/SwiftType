# SwiftType

Rustで実装された高機能テキスト置換ツール。ショートカットキーワードを設定して素早くテキストを展開できます。

## 機能

- カスタムキーワードとテキスト置換の定義
- 動的コンテンツのサポート（日付、時刻など）
- スニペット管理（カテゴリー分類、インポート/エクスポート）
- システム統合（自動起動、トレイアイコン）
- ユーザーフレンドリーな設定UI

## 開発環境の準備

### 必要なツール

1. Rust (https://www.rust-lang.org/tools/install)

2. ビルドツール(https://visualstudio.microsoft.com/ja/visual-cpp-build-tools/)
    + 「C++によるデスクトップ開発」(Visual Studio Build Tools)のみ選択。

### プロジェクトのビルド

```bash
# プロジェクトのクローン
git clone https://github.com/yourusername/swifttype.git
cd swifttype

# 依存関係のインストールとビルド
cargo build --release

# 実行
cargo run --release
```

## プロジェクト構造

```
swifttype/
├── src/                    # ソースコード
│   ├── main.rs             # エントリーポイント
│   ├── app.rs              # アプリケーションのメインロジック
│   ├── config/             # 設定関連
│   ├── keyboard/           # キーボードフック
│   ├── replacement/        # テキスト置換エンジン
│   ├── ui/                 # ユーザーインターフェース
│   └── utils/              # ユーティリティ関数
├── tests/                  # 統合テスト
├── Cargo.toml              # 依存関係設定
└── README.md               # このファイル
```

## テスト

```bash
# すべてのテストを実行
cargo test

# 特定のテストを実行
cargo test test_name
```

## ライセンス

MIT

## 注意事項

+ このアプリケーションはキーボード入力をフックするため、セキュリティソフトによって警告される場合があります。 

+ [Settings]-[Start with system] のチェックボックスをONにした場合、下記プログラムが呼ばれます。
    + SwiftType\target\release\swifttype.exe