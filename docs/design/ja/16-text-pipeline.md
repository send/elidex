
# 16. テキスト＆フォントパイプライン

## 16.1 概要

テキストはブラウザエンジンで最も複雑なレンダリングプリミティブ。パース（文字エンコーディング）、DOM（テキストノード）、スタイリング（フォント選択、色）、レイアウト（行分割、bidi）、ペイント（グリフラスタライゼーション）のすべてのパイプラインステージにまたがる。Elidexのテキストパイプラインは初日からファーストクラスのCJKおよび双方向サポートを設計。

```
ECS内テキストノード
  │
  ├── フォント解決（fontdb：システムフォント + Webフォント）
  ├── アイテマイゼーション（スクリプト、言語、フォントでランに分割）
  ├── シェーピング（rustybuzz：コードポイント → 配置済みグリフ）
  ├── BiDi並べ替え（unicode-bidi：論理順 → 視覚順）
  ├── 行分割（ICU4X：Unicode行分割 + 辞書ベースCJK）
  ├── レイアウト（インラインフォーマッティングコンテキスト：ラン → 行 → ボックス）
  └── ペイント（Vello：グリフアウトラインまたはグリフアトラス → GPU）
```

## 16.2 フォント解決

### 16.2.1 フォントマッチング

CSS Fonts仕様のカスケードに従う：

```rust
pub struct FontResolver {
    system_db: fontdb::Database,
    web_fonts: HashMap<String, Vec<FontFace>>,
}

pub struct FontFace {
    pub family: String,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub stretch: FontStretch,
    pub unicode_range: Vec<UnicodeRange>,
    pub source: FontSource,
}

pub enum FontSource {
    System(fontdb::ID),
    WebFont { url: Url, data: Option<Bytes> },
}
```

マッチングアルゴリズム（CSS Fonts Level 4）：ファミリ名でフィルタ → スタイル（italic > oblique > normal） → ウェイト（最近接マッチ） → ストレッチ。マッチなしの場合、スタック内の次ファミリへ。最終フォールバック：プラットフォームデフォルト（San Francisco / Segoe UI / Noto Sans）。

### 16.2.2 総称フォントファミリ

| 総称 | macOS | Windows | Linux |
| --- | --- | --- | --- |
| `serif` | Times New Roman | Times New Roman | Noto Serif |
| `sans-serif` | Helvetica Neue | Segoe UI | Noto Sans |
| `monospace` | Menlo | Cascadia Mono | Noto Sans Mono |
| `system-ui` | San Francisco | Segoe UI | システムデフォルト |

`sans-serif`のCJKフォールバックチェーン：ヒラギノ角ゴシック（macOS） → 游ゴシック（Windows） → Noto Sans CJK（Linux）。

### 16.2.3 Webフォント（@font-face）

`font-display`によるロード挙動制御：**swap**（デフォルト：フォールバックで即時レンダリング、ロード後スワップ）、**block**（3秒まで不可視）、**fallback**（100msブロック後フォールバック、3s内ロードでスワップ）、**optional**（短ブロック後永続フォールバック）。

Coreフォーマット：WOFF2（主要、Brotliベース）、WOFF、OpenType/TrueType。

## 16.3 テキストアイテマイゼーション

テキストを「ラン」に分割 — 同じフォント、スクリプト、言語、方向を共有するセグメント：

```rust
pub struct TextRun {
    pub text: Range<usize>,
    pub font: FontKey,
    pub script: Script,
    pub language: Language,
    pub direction: Direction,
    pub level: BidiLevel,
}
```

ステップ：スクリプト検出（ICU4X ScriptExtensions） → フォントフォールバック（プライマリフォントにカバレッジがない場合ランを分割） → BiDi分析（unicode-bidi埋め込みレベル） → 言語タグ付け（`lang`属性継承）。

## 16.4 シェーピング

**rustybuzz**（純粋RustのHarfBuzzポート）でUnicodeコードポイントを配置済みグリフに変換：

```rust
pub struct ShapedGlyph {
    pub glyph_id: u16,
    pub x_advance: f32,
    pub y_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
    pub cluster: u32,
}
```

処理：リガチャ（fi、ffl、アラビア語連結形）、カーニング、複雑スクリプト（アラビア語文脈フォーム、デーバナーガリー結合子、タイ語マーク配置）、OpenType機能（CSS `font-feature-settings`、`font-variant-*`経由の`kern`、`liga`、`calt`、`smcp`等）。

シェーピングはrayonプール上で独立テキストラン間の並列処理を実行。

## 16.5 双方向テキスト（BiDi）

Unicode双方向アルゴリズムが`unicode-bidi`クレート経由でLTR/RTL混在テキストを処理：

1. **分析**（アイテマイゼーション中）：文字 + 明示的オーバーライド（`dir`、`unicode-bidi` CSS）から埋め込みレベルを計算。
2. **行分割**（レイアウト中）：論理順で行を分割。
3. **並べ替え**（レイアウトとペイント間）：行ごとに視覚順を計算。奇数レベルのランを反転。

CSS：`direction: rtl`、`unicode-bidi: embed | bidi-override | isolate | isolate-override | plaintext`。

## 16.6 行分割

### 16.6.1 アルゴリズム

ICU4X `LineSegmenter`とUnicode UAX #14規則に言語固有オーバーライドを加えて使用。

### 16.6.2 CJK行分割

CJK固有規則：CJK文字間は一般に分割可能（スペース不要）、句読点配置制約（開き括弧で行末不可、閉じ括弧で行頭不可）、禁則処理（日本語）、日本語単語境界の辞書ベースセグメンテーション。

CSS：`word-break: normal | break-all | keep-all`、`overflow-wrap: normal | break-word | anywhere`、`line-break: auto | loose | normal | strict`。

### 16.6.3 ハイフネーション

自動ハイフネーション（`hyphens: auto`）はLiangのアルゴリズム（`hyphenation`クレート）経由。言語固有パターン。ソフトハイフン（`&shy;`）は常に尊重。

## 16.7 縦書き

日本語・中国語コンテンツのため初日からサポート：

```rust
pub enum WritingMode {
    HorizontalTb,
    VerticalRl,      // 日本語デフォルト
    VerticalLr,      // モンゴル語
}
```

レイアウトエンジンは物理的x/yではなく抽象的**インライン/ブロック**座標を使用。writing-modeがペイント時のマッピングを決定：

| | Horizontal-TB | Vertical-RL | Vertical-LR |
| --- | --- | --- | --- |
| インライン軸 | x | y | y |
| ブロック軸 | y | x | x |

CSS：`writing-mode`、`text-orientation: mixed | upright | sideways`、`text-combine-upright`（縦中横）。

## 16.8 ルビ注釈

ルビ（`<ruby>`）がふりがな・ピンイン等の注音を提供。水平時は上、縦書き時は右にアノテーション配置。`ruby-position`で制御。オーバーハング規則で隣接空白へのはみ出しを許可。

## 16.9 グリフラスタライゼーション

2パス：小テキスト（≤48px）は**グリフアトラス**（CPUラスタライズ → テクスチャアトラス → GPU blit）でキャッシュ効率化、大テキスト（>48px）はGPU上で**Velloベクターアウトライン**を直接使用。

サブピクセルポジショニング：最大4水平サブピクセル位置でラスタライズ。LCDサブピクセルレンダリング：Windows（ClearType）、Linux（設定可能）、macOS（Mojave以降無効）。

テキストデコレーション（`underline`、`overline`、`line-through`）：フォントメトリクスで配置。ディセンダースキッピング（`text-decoration-skip-ink: auto`）はグリフアウトラインでギャップを計算。

## 16.10 文字エンコーディング

Core：UTF-8のみ。Compat層（`elidex-compat-charset`）がShift_JIS、EUC-JP、ISO-2022-JP、GB2312/GBK/GB18030、EUC-KR、ISO-8859-*、Windows-1252を処理。`<meta charset>`、HTTP `Content-Type`、BOM、エンコーディングスニッフィングで検出。

## 16.11 elidex-appテキスト

| 側面 | elidex-browser | elidex-app |
| --- | --- | --- |
| Webフォント | フルサポート | フルサポート |
| レガシーエンコーディング | Compat層 | 除外（UTF-8のみ） |
| 縦書き | フルサポート | フルサポート |
| ルビ | フルサポート | フルサポート |
| ハイフネーションデータ | 一般言語をバンドル | 設定可能 |
