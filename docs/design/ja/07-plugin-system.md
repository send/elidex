
# 7. プラグインシステム

## 7.1 統一プラグインフレームワーク

elidexのすべての機能（HTMLタグ処理、CSSプロパティ、レイアウトアルゴリズム、ネットワークミドルウェア、JS APIバインディング）は共通トレイトに準拠するプラグインとして実装される。設計上の主要な課題は、単一のトレイト定義で2つのディスパッチモードをサポートすることである：

| モード | メカニズム | ユースケース |
| --- | --- | --- |
| 静的（Static） | enumディスパッチ。コンパイル時に解決され、コンパイラがmatchアームをインライン化しvtable間接参照を排除する。ランタイムコストゼロ。 | Cargoフィーチャーフラグで選択される組み込み機能。コアHTML5タグ、CSSプロパティ、レイアウトアルゴリズム。ナノ秒単位の性能が重要なホットパス。 |
| 動的（Dynamic） | トレイトオブジェクトディスパッチ（Box\<dyn Trait\>）。ランタイムに登録される。呼び出しごとに小さなオーバーヘッド（~数ns vtableルックアップ）。 | エンドユーザー拡張：広告ブロッカー、DevToolsミドルウェア、カスタムCSSプロパティ、実験的機能。エンジンの再コンパイル不要。 |

プラグイン作者はディスパッチモードに関係なく同一トレイトに対して実装を書く。唯一の違いは登録方法：静的（Cargoフィーチャー経由）か動的（拡張API経由）かである。

## 7.2 プラグイントレイト

全レイヤーで使用されるコアトレイト定義：

```rust
pub trait CssPropertyHandler: Send + Sync {
    fn property_name(&self) -> &str;
    fn spec_level(&self) -> SpecLevel;
    fn parse(&self, value: &str) -> Result<CssValue, ParseError>;
    fn resolve(&self, value: &CssValue, ctx: &StyleContext) -> ComputedValue;
    fn affects_layout(&self) -> bool;
    fn deprecated_by(&self) -> Option<DeprecationInfo> { None }
}
pub trait HtmlElementHandler: Send + Sync {
    fn tag_name(&self) -> &str;
    fn spec_level(&self) -> SpecLevel;
    fn default_style(&self) -> &[CssRule];
    fn create_element(&self, attrs: &Attributes) -> ElementData;
    fn parse_behavior(&self) -> ParseBehavior { ParseBehavior::Normal }
    fn accessibility_role(&self) -> Option<AccessibilityRole> { None }
}
pub trait LayoutModel: Send + Sync {
    fn name(&self) -> &str;
    fn spec_level(&self) -> SpecLevel;
    fn layout(&self, node: &LayoutNode, children: &[LayoutNode],
              constraints: &Constraints) -> LayoutResult;
}
pub trait NetworkMiddleware: Send + Sync {
    fn name(&self) -> &str;
    fn on_request(&self, req: &mut Request) -> MiddlewareAction { MiddlewareAction::Continue }
    fn on_response(&self, req: &Request, res: &mut Response) -> MiddlewareAction { MiddlewareAction::Continue }
}
```

## 7.3 デュアルディスパッチアーキテクチャ

同一トレイトが、単一のパイプラインを構成する2つのメカニズムでディスパッチされる：

### 7.3.1 静的ディスパッチ（コンパイル時）

組み込みプラグインはCargoフィーチャーフラグでゲートされたバリアントを持つenumに集約される。コンパイラはmatchをジャンプテーブルに最適化し、すべての間接参照を排除する：

```rust
// #[elidex_plugin] procマクロにより自動生成
enum CssPropertyDispatch {
    Display(DisplayHandler),
    Margin(MarginHandler),
    Padding(PaddingHandler),
    #[cfg(feature = "css-flexbox")]
    FlexDirection(FlexDirectionHandler),
    #[cfg(feature = "css-grid")]
    GridTemplateColumns(GridTemplateColumnsHandler),
    // ... フィーチャーフラグに応じて拡縮
}

// コンパイラが各アームをインライン化 → ゼロコストディスパッチ
impl CssPropertyHandler for CssPropertyDispatch {
    fn parse(&self, value: &str) -> Result<CssValue, ParseError> {
        match self {
            Self::Display(h) => h.parse(value),
            Self::Margin(h) => h.parse(value),
            Self::Padding(h) => h.parse(value),
            #[cfg(feature = "css-flexbox")]
            Self::FlexDirection(h) => h.parse(value),
            ...
        }
    }
}
```

### 7.3.2 動的ディスパッチ（ランタイム）

拡張およびユーザー提供プラグインは同一トレイトを実装するが、ランタイムにトレイトオブジェクトとして登録される：

```rust
// 拡張作者は同じトレイトを実装する
struct MyCustomProperty { ... }
impl CssPropertyHandler for MyCustomProperty { ... }

// 動的に登録（再コンパイル不要）
browser.register_extension(Box::new(MyCustomProperty::new()));
```

### 7.3.3 統一パイプライン実行

ランタイムでは、レジストリが静的ディスパッチテーブルを先にチェックし、次に動的拡張にフォールスルーしてルックアップを解決する：

```rust
pub struct PluginRegistry<T: ?Sized> {
    static_lookup: HashMap<&'static str, StaticDispatch>,  // コンパイル時
    dynamic_lookup: HashMap<String, Box<dyn T>>,           // ランタイム
}

impl<T: ?Sized> PluginRegistry<T> {
    pub fn resolve(&self, name: &str) -> Option<&dyn T> {
        // 静的を先にチェック（組み込み機能のゼロコストパス）
        if let Some(handler) = self.static_lookup.get(name) {
            return Some(handler.as_ref());
        }
        // 動的フォールバック（拡張パス）
        self.dynamic_lookup.get(name).map(|b| b.as_ref())
    }
}
```

すべてのパイプラインステージの実行フロー：

```
Input
  │
  │  静的ステージ（enumディスパッチ、コンパイラによりインライン化）
  │  ┌─ CssPropertyDispatch::Display      ← ゼロコスト
  │  ├─ CssPropertyDispatch::Margin       ← ゼロコスト
  │  ├─ CssPropertyDispatch::FlexDir...   ← ゼロコスト
  │  └─ ...
  │
  │  動的ステージ（トレイトオブジェクト、vtableルックアップ）
  │  ┌─ Box<dyn CssPropertyHandler>       ← ~数nsオーバーヘッド
  │  └─ Box<dyn CssPropertyHandler>       ← ~数nsオーバーヘッド
  │
  ▼
Output
```

## 7.4 プラグイン作者向けprocマクロ

enumディスパッチ生成のボイラープレートを排除するため、elidex-pluginクレートはprocマクロを提供し、静的ディスパッチenum、match実装、レジストリ配線を自動生成する：

```rust
#[elidex_plugin(dispatch = "static")]
pub trait CssPropertyHandler: Send + Sync {
    fn property_name(&self) -> &str;
    fn parse(&self, value: &str) -> Result<CssValue, ParseError>;
    fn resolve(&self, value: &CssValue, ctx: &StyleContext) -> ComputedValue;
}

// マクロが生成するもの：
// 1. CssPropertyHandlerDispatch enum（#[cfg(feature)]ゲート付き）
// 2. impl CssPropertyHandler for CssPropertyHandlerDispatch（matchアーム）
// 3. PluginRegistry<dyn CssPropertyHandler>統合
```

プラグイン作者はディスパッチメカニズムと直接やり取りしない。トレイトを実装すれば、フレームワークが残りを処理する。

## 7.5 全レイヤーへの適用

デュアルディスパッチパターンはelidexのすべての拡張可能レイヤーに統一的に適用される：

| レイヤー | 静的（コンパイル時） | 動的（ランタイム拡張） |
| --- | --- | --- |
| HTMLタグ | 組み込みHTML5要素ハンドラ（div, span, a, img, ...） | カスタム要素、実験的タグ |
| CSSプロパティ | 標準CSSプロパティ（display, margin, flex, grid, ...） | ユーザー定義カスタムプロパティ、実験的CSS |
| レイアウトアルゴリズム | Block、Flex、Grid、Tableレイアウト | 実験的レイアウトモデル |
| ネットワーク | HTTPローダー、キャッシュ、file://ローダー | 広告ブロッカー、DevToolsロガー、プライバシーフィルター、APIモック |
| DOM API | Living Standardメソッド（querySelector、MutationObserver、...） | レガシーDOMシム（ライブコレクション、document.write、attachEvent） |
| ECMAScript | ES2020+コア（let/const、class、async/await、modules） | Annex Bセマンティクス、var quirks、sloppy eval |
| パーサー修復 | ルールベースのエラー回復パターン | LLMフォールバック（有効時） |

この統一性は、どのレイヤーで作業しているかに関係なく、すべてのコントリビューターと拡張作者にとって単一のelidex-pluginクレート、単一のprocマクロ、単一のメンタルモデルを意味する。

## 7.6 仕様レベルとフィーチャーフラグ

各プラグインは仕様レベルを宣言し、コンパイル時およびランタイムのフィルタリングを可能にする。三層一貫性の原則を維持するため、すべての拡張可能レイヤーが同じcore/compat/deprecatedパターンに従う固有のSpecLevel enumを持つ：

```rust
// HTMLタグ — タグ単位の分類
pub enum HtmlSpecLevel {
    Html5,          // Living Standard: <div>, <span>, <a>, <img>, <video>, ...
    Legacy,         // 非推奨タグ: <center>, <font>, <marquee>, ...
    Deprecated,     // 削除予定（使用率 < 1%）
}

// DOM API — メソッド単位の分類（第12章で定義）
pub enum DomSpecLevel {
    Living,         // DOM Living Standard: querySelector, addEventListener, ...
    Legacy,         // レガシー: getElementsByClassName (live), attachEvent, ...
    Deprecated,     // 危険: document.write, document.all, ...
}

// ECMAScript — 機能単位の分類（第14章で定義）
pub enum EsSpecLevel {
    Modern,          // ES2020+: let/const, class, async/await, modules, ...
    LegacySemantics, // var hoisting, sloppy mode, ...
    AnnexB,          // JS内HTMLコメント, __proto__, ...
}

// CSS — プロパティ/値単位の分類
pub enum CssSpecLevel {
    Standard,     // 標準プロパティの標準値: display: flex, margin, ...
    Aliased,      // 標準プロパティの旧名: word-wrap → overflow-wrap
    NonStandard,  // ベンダープレフィクスまたは非標準: zoom, -webkit-appearance, ...
    Deprecated,   // かつて標準だったが削除予定（使用率 < 1%）
}

// Web API — API単位の分類（第14章で定義）
pub enum WebApiSpecLevel {
    Modern,       // モダン非同期API: Fetch, CookieStore, IndexedDB, ...
    Legacy,       // 同期/ブロッキングAPI: XMLHttpRequest, localStorage, document.cookie
    Deprecated,   // 標準から削除済み: WebSQL, ...
}
```

CssSpecLevelは特別な注意に値する。各タグが明確にcoreかlegacyかであるHTMLタグと異なり、CSSには3つの異なる非標準使用カテゴリがある：

| CssSpecLevel | 例 | 処理 |
| --- | --- | --- |
| Standard | `display: flex`, `margin: 10px` | コア。CssPropertyHandlerプラグインで直接処理。 |
| Aliased | `word-wrap: break-word` | 互換。elidex-compat-cssがコアに到達する前に`overflow-wrap: break-word`に正規化。 |
| NonStandard | `zoom: 1.5`, `-webkit-appearance: none` | 互換。elidex-compat-cssが標準の同等物に変換（`transform: scale(1.5)`, `appearance: none`）。 |
| Deprecated | 使用率駆動（クローラーデータで < 1%） | 他のすべてのレイヤーと同じ非推奨化ライフサイクル。 |

> **Phase 0 サーベイ結果（第29章 §29.3）:** CssSpecLevel分類のサーベイデータ検証:
> - `-webkit-appearance`: EN 17.6%（NonStandard — 高優先度 compat 対応が必要であることを確認）
> - `word-wrap`: EN 13.8%（Aliased — `overflow-wrap`への正規化が必要であることを確認）
> - `-webkit-box-*`: EN 12–14%（Aliased — レガシーflexbox構文の compat 対応が必要であることを確認）

用途レベルのレガシー検出（例：`float`がページレイアウト用 vs テキスト回り込み用）は意図的に行わない。構文だけからセマンティックな意図を区別するのは非現実的である。代わりに、プロパティレベルの非推奨化は標準の非推奨化ポリシー（7.7節）を通じてクローラー使用データで駆動する。

Cargoフィーチャーフラグにより、静的ディスパッチenumにコンパイルされるプラグインが制御される：

```toml
[features]
default = []                          # HTML5のみ、最小限
level-2026 = ["html5-base", "css-flexbox", "css-grid", ...] 
level-2028 = ["level-2026", "css-anchor-positioning", ...]
browser-minimal = ["level-2026", "elidex-parser-tolerant"]
browser-full = ["browser-minimal", "elidex-compat-tags", ...]
```

コンパイル時に選択されなかった機能は静的ディスパッチenumから完全に除外され、バイナリに存在しない。これがelidex-appが最小フットプリントを実現するメカニズムである：未使用のWeb機能はゼロバイト、ゼロオーバーヘッド。

## 7.7 非推奨化ポリシー

機能を削除できることはelidexの長期的な健全性の根幹である。非推奨化ライフサイクル：

| ステージ | トリガー | アクション |
| --- | --- | --- |
| 有効 | Web使用率 > 1% | 完全サポート。browser-fullに含まれる |
| 非推奨 | Web使用率が1%以下に低下（elidex-crawlerで計測） | ブラウザモードではコンソール警告、アプリモードではコンパイル時警告。新しいlevel-YYYYフィーチャーセットから除外 |
| 削除 | 非推奨化から2メジャーバージョン後 | プラグインクレートをリポジトリから削除、Cargoフィーチャーフラグ削除。必要なユーザーは動的拡張として自己メンテナンス可能。 |

elidex-crawlerツールはサーベイサイトリストに対して定期的に実行され、非推奨化の意思決定を駆動する定量的な使用データを生成する。主観的な議論を排除する。

> **Phase 0 サーベイ結果（第29章 §29.2）:** 1%閾値のサーベイデータ検証:
> - `<font>`: JA 2.0%（Legacy 維持 — 閾値超過）
> - `<center>`: JA 1.6%（Legacy 維持 — 閾値超過）
> - `document.all`: 0%（Deprecated — 初回削除サイクル候補）
> - `<blink>`: EN 0.2%（Deprecated 該当）

削除された静的プラグインはコミュニティ維持の動的拡張として存続できることに注意。デュアルディスパッチモデルにより、コアからの非推奨化は機能が使用不能になることを意味しない。ゼロコストの静的パスから動的拡張パスに移行するだけであり、まれに使用される機能には適切なトレードオフである。
