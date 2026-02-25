
# 27. テスト戦略

## 27.1 概要

elidexのテストには多層アプローチが必要：標準準拠（WPT）、パフォーマンスベンチマーク、パーサー/コーデックセキュリティのファズテスト、クロスプロセス連携の統合テスト、レンダリング正確性のビジュアルリグレッションテスト。

## 27.2 Web Platform Tests（WPT）

WPTは数万のテストケースを持つ業界標準の準拠スイート。

### 27.2.1 プラグインごとのWPTマッピング

各プラグインクレートが担当するWPTテストIDを宣言。プラグイン追加で自動的にCIにテスト追加。プラグイン削除で影響するWPTテストを正確に表示。

### 27.2.2 サブセットトラッキング

elidexはWPT 100%パス率を目指さない。ターゲットサブセットを明示的に定義・追跡：

| カテゴリ | 目標 | 備考 |
| --- | --- | --- |
| HTMLパース | >95% | Core + Compatパーサー合算 |
| CSS（サポートプロパティ） | >90% | elidexのCSSプロパティレジストリ内のみ |
| DOM API（Core） | >85% | querySelector、mutation、events |
| Fetch API | >90% | 標準HTTP、ストリーミング |
| Web Animations | >80% | WAAPI統合モデル |
| Canvas 2D | >80% | Velloバックエンド |
| Compat層API | >70% | レガシーAPIはベストエフォート |

### 27.2.3 CI統合

全PRでWPT実行。追跡サブセット内の新規失敗テストはマージをブロック（意図的に非サポートの領域を除く）。ダッシュボードでパス率推移を追跡。

## 27.3 パフォーマンスベンチマーク

ChromiumとFirefoxに対する自動ベンチマーク：

| メトリクス | ベンチマーク | 目標 |
| --- | --- | --- |
| パーススループット | HTML/CSSパース時間（標準ドキュメント） | Chromiumの80%以内 |
| スタイル解決 | 大規模DOMツリーのスタイル計算 | Chromiumより高速（並列化） |
| レイアウト | 代表的ページ（ニュース、アプリ、テーブル） | Chromiumと競合 |
| ファーストペイント | URL → 最初のピクセル | ウォーム<200ms、コールド<500ms |
| メモリフットプリント | タブごとのピーク・定常状態 | Chromiumの50%未満（ECSの利点） |
| バイナリサイズ | ブラウザ・アプリ構成 | ブラウザ<30MB、アプリ最小<15MB |

### 27.3.1 ベンチマーク基盤

```rust
#[bench]
fn style_resolution_1000_nodes(b: &mut Bencher) {
    let world = create_test_dom(1000);
    b.iter(|| {
        StyleSystem::compute(&world);
    });
}
```

`criterion`を使用した統計的に厳密なベンチマーク（ウォームアップ、反復、信頼区間）。CI上で結果を追跡しリグレッション検出（5%超でマージブロック）。

## 27.4 ファズテスト

セキュリティクリティカルなパーサーを継続的にファズテスト：

| 対象 | ファザー | コーパス |
| --- | --- | --- |
| HTMLパーサー | cargo-fuzz (libFuzzer) | クロールWebページ、WPTフィクスチャ |
| CSSパーサー | cargo-fuzz | トップ1万サイトのCSS |
| SVGパーサー | cargo-fuzz | Web由来SVG + エッジケース |
| 画像デコーダー | cargo-fuzz | 不正画像（PNG, JPEG, WebP, AVIF） |
| メディアデマクサー | cargo-fuzz | 切断/破損メディアファイル |
| URLパーサー | cargo-fuzz | RFCエッジケース、IDN、punycode |
| HTTPヘッダーパーサー | cargo-fuzz | 不正ヘッダー |

CIインフラ上で24/7実行。クラッシュは自動トリアージ・起票。

## 27.5 ビジュアルリグレッションテスト

レンダリング正確性をスクリーンショット比較で検証：

```
テストケース（HTML + CSS）
  → elidexがヘッドレスビットマップにレンダリング（Vello CPUバックエンド）
  → リファレンス画像とピクセル単位比較
  → 閾値超過のdiff → レビュー対象
```

リファレンス画像はリポジトリにコミット。プラットフォーム固有のレンダリング差異（フォントヒンティング、サブピクセルレンダリング）はプラットフォーム別リファレンスセットと許容閾値で対応。

## 27.6 統合テスト

クロスプロセス連携テスト：

| 領域 | テストアプローチ |
| --- | --- |
| ナビゲーション（Ch. 9） | ヘッドレスブラウザでページロード、ドキュメント状態遷移を検証 |
| IPC（Ch. 5） | Renderer + Browserプロセスを起動、メッセージプロトコルを検証 |
| bfcache | ナビゲーション往復後、DOM状態保持を検証 |
| 権限プロンプト | テストハーネス経由でプロンプト応答をシミュレート |
| メディア再生 | テストメディアファイルをロード、A/V同期を許容範囲内で検証 |
| OPFS | SyncAccessHandle経由でwrite/read、データ整合性を検証 |
| Webフォント | WOFF2ロード、グリフレンダリングがリファレンスと一致を検証 |

## 27.7 ユニットテスト

全クレートに内部ロジックのユニットテスト。ECSアーキテクチャによりユニットテストが容易：必要なコンポーネントで最小Worldを作成、システムを実行、出力をアサート。

```rust
#[test]
fn css_cascade_specificity() {
    let mut world = World::new();
    // 2つのマッチするルールを持つ<div class="foo">を作成
    let entity = world.spawn((TagType::Div, Attributes::new(), ...));
    // 既知の詳細度でスタイルルールを追加
    StyleSystem::compute(&world);
    let style = world.get::<ComputedStyle>(entity).unwrap();
    assert_eq!(style.color, Color::RED); // より高い詳細度のルールが勝つ
}
```

## 27.8 elidex-appテスト

Embedding API（Ch. 26）がテスト専用のヘッドレスモードを提供：

```rust
#[test]
fn app_loads_content() {
    let engine = Engine::builder()
        .with_config(EngineConfig { process_mode: ProcessMode::SingleProcess, .. })
        .build().unwrap();

    let view = engine.create_view(ViewConfig {
        content: ViewContent::Html("<h1>Hello</h1>".into()),
        surface: SurfaceConfig::Headless { width: 800, height: 600 },
        ..Default::default()
    });

    // ロード完了を待機
    let result = view.evaluate_script("document.querySelector('h1').textContent").await;
    assert_eq!(result.unwrap().as_str(), "Hello");
}
```
