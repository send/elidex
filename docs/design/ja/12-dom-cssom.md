
# 12. DOM API & CSSOM

Object Model API（DOMとCSSOM）は、スクリプトがページコンテンツとスタイルとやり取りする主要インターフェースである。両方ともScriptSession（第13章）上に構築され、同一性マッピング、変更バッファリング、GC協調を処理する。OMプラグインは純粋にドメインロジックに集中する。

## 12.1 DOM APIアーキテクチャ

DOM APIはスクリプトエンジンとECSベースのDOMストアの橋渡しである。すべてのDOM操作はScriptSessionを経由し、同一性マッピングと変更バッファリングを処理する。DomApiHandlerプラグインは純粋にドメインロジックに集中する。

### 12.1.1 DomApiHandlerプラグイントレイト

各DOM APIメソッドはDomApiHandlerトレイトに準拠するプラグインとして実装され、他のすべてのelidexプラグインと同じデュアルディスパッチパターンに従う：

```rust
#[elidex_plugin(dispatch = "static")]
pub trait DomApiHandler: Send + Sync {
    fn method_name(&self) -> &str;
    fn spec_level(&self) -> DomSpecLevel;
    fn invoke(
        &self,
        this: EntityId,
        args: &[JsValue],
        session: &mut dyn ScriptSession,
        dom: &EcsDom,   // 読み取り専用。書き込みはsession.record_mutation()経由
    ) -> Result<JsValue>;
}

pub enum DomSpecLevel {
    Living,       // DOM Living Standard: querySelector, addEventListener等
    Legacy,       // レガシーAPI: getElementsByClassName (live), attachEvent等
    Deprecated,   // 危険: document.write, document.all等
}
```

これは各DOMメソッドが個別にトグル可能であることを意味する。elidex-appはレガシーDOM APIをコンパイル時に除外でき、elidex-browserは互換レイヤー経由でそれらを含む。

### 12.1.2 コア vs 互換 DOM API

| API | コア | 互換 | 備考 |
| --- | --- | --- | --- |
| querySelector / querySelectorAll | ✓ | — | 静的NodeList（スナップショット）を返す。主要クエリAPI。 |
| getElementById | ✓ | — | IDコンポーネントによる直接ECSルックアップ。ファストパス。 |
| addEventListener / removeEventListener | ✓ | — | ECS EventTargetコンポーネント。標準イベントフロー。 |
| createElement / createTextNode | ✓ | — | 適切なコンポーネントを持つ新しいECSエンティティを生成。 |
| appendChild / insertBefore / removeChild | ✓ | — | セッションにMutation::AppendChild等を記録。flush時に再レイアウトをトリガー。 |
| getAttribute / setAttribute | ✓ | — | セッションにMutation::SetAttributeを記録。 |
| MutationObserver | ✓ | — | セッションflushがバッファされた変更からMutationRecordsを生成。ファーストクラス。 |
| classList / dataset | ✓ | — | ECS Attributesコンポーネントに裏打ちされた便利API。 |
| innerHTML / outerHTML | ✓ | — | シリアライゼーションとフラグメント解析。フレームワークで広く使用。 |
| getElementsByClassName (live HTMLCollection) | ✗ | ✓ | ライブコレクションをセッションのregister_live_query()経由で登録。各flush時に再評価。 |
| document.write / document.writeln | ✗ | ✓ | パーサーストリームを中断。パイプラインに対して極めて破壊的。compatシムはinnerHTMLにシリアライズ。 |
| document.all | ✗ | ✓ | 有名なquirk：typeof document.all === "undefined"なのに存在する。互換のみ。 |
| element.attachEvent / detachEvent | ✗ | ✓ | IEレガシー。addEventListenerにシム。 |

> **Phase 0 サーベイ結果（第29章 §29.4）:** document.all 0%（Deprecated分類検証済み、初回削除サイクル候補）、document.write JA 12.4% / EN 5.3%（compat-only 分類が適切であることを確認）。

### 12.1.3 ECS統合パターン

ScriptSessionにより、DOM APIハンドラはECS状態を直接読み取るが、書き込みはセッションのMutation Buffer経由で行う：

```rust
// querySelector → TagType + Attributes上のECSクエリ（読み取り専用、セッション不要）
fn query_selector(root: EntityId, selector: &str, dom: &EcsDom) -> Option<EntityId> {
    let parsed = css_selector::parse(selector)?;
    dom.query::<(TreeRelation, TagType, Attributes)>()
        .descendants_of(root)
        .find(|(_, tag, attrs)| parsed.matches(tag, attrs))
}

// setAttribute → 書き込みはセッションのMutation Buffer経由
fn set_attribute(entity: EntityId, name: &str, value: &str, session: &mut dyn ScriptSession) {
    session.record_mutation(Mutation::SetAttribute(entity, name.into(), value.into()));
}

// element.style → Identity Mapが毎回同じラッパーを返すことを保証
fn get_style(entity: EntityId, session: &mut dyn ScriptSession) -> JsObjectRef {
    session.get_or_create_wrapper(entity, ComponentKind::InlineStyle)
}
```

### 12.1.4 Shadow DOMとWeb Components

Shadow DOMはWeb Componentsの基盤であり、モダンWebフレームワークで活発に使用されている。ECSはシャドウルートを別のツリースコープとしてモデル化できる：

```rust
pub struct ShadowRoot {
    mode: ShadowRootMode,  // OpenまたはClosed
    host: EntityId,         // このシャドウルートを所有する要素
}

pub struct TreeRelation {
    parent: EntityId,
    first_child: Option<EntityId>,
    next_sibling: Option<EntityId>,
    shadow_root: Option<EntityId>,  // このエンティティがシャドウツリーをホストする場合
    tree_scope: TreeScope,           // このノードが属するスコープ
}
```

Shadow DOMサポートはelidex-appにも重要。アプリケーションUIのコンポーネントカプセル化を提供する。実装はPhase 3に予定（第3章）。

## 12.2 CSSOM（CSS Object Model）

CSSOMはスクリプトからCSSルールとスタイルシートを操作するAPIで、DOM APIと同じ構造的課題（OOPラッパー ↔ ECSデータの変換）を持つ。同じScriptSessionインフラストラクチャ上に構築され、Identity Map、変更追跡、GC協調はDOMと共有される。CSSOMプラグインは純粋にCSSドメインロジックに集中する。

### 12.2.1 CssomApiHandlerプラグイントレイト

```rust
#[elidex_plugin(dispatch = "static")]
pub trait CssomApiHandler: Send + Sync {
    fn method_name(&self) -> &str;
    fn spec_level(&self) -> CssomSpecLevel;
    fn invoke(
        &self,
        this: EntityId,
        args: &[JsValue],
        session: &mut dyn ScriptSession,
        dom: &EcsDom,
    ) -> Result<JsValue>;
}

pub enum CssomSpecLevel {
    Living,       // CSSOM Living Standard（全現行API）
    // 将来: Legacy, Deprecated — 必要時のためにアーキテクチャ準備済み
}
```

### 12.2.2 CSSOM APIカバレッジ

| API | コア | 備考 |
| --- | --- | --- |
| element.style | ✓ | インラインスタイル用CSSStyleDeclaration。セッションIdentity Mapが`el.style === el.style`を保証。書き込みはセッションバッファにMutation::SetInlineStyleを記録。 |
| window.getComputedStyle() | ✓ | 読み取り専用CSSStyleDeclaration。ECSのComputedStyleコンポーネントから取得。Live：セッションflush後のスタイル再計算で自動更新。 |
| document.styleSheets | ✓ | StyleSheetList。各CSSStyleSheetはECSエンティティ。セッションIdentity Mapが安定ラッパーを提供。 |
| CSSStyleSheet.insertRule() / deleteRule() | ✓ | セッションバッファにMutation::InsertCssRule / DeleteCssRuleを記録。flush時にスタイル再計算をトリガー。 |
| CSS.supports() | ✓ | PluginRegistryへのクエリ。指定プロパティ/値のCssPropertyHandlerが存在するか確認。 |
| CSSStyleSheet()コンストラクタ | ✓ | Constructable Stylesheets。新規ECSエンティティを生成。Shadow DOMスタイリングの基盤。 |
| element.computedStyleMap() (CSS Typed OM) | ✓ | CSSNumericValueベースの型安全アクセス。文字列パースオーバーヘッドなし。P2優先度。 |
| element.style.cssText | ✓ | バルク書き込み。セッションバッファに複数のMutation::SetInlineStyleエントリを記録。 |
| getComputedStyle().getPropertyValue() | ✓ | 個別プロパティ読み取り。PluginRegistry経由でCssPropertyHandlerプラグインにディスパッチ。 |

CSSOMは現在すべてcoreでcompat APIがない。数十年にわたるレガシーAPI（document.all、attachEvent、ライブコレクション）を蓄積したDOMと異なり、CSSOMは比較的若くクリーンである。ただし、CssomSpecLevel enumはLegacy/Deprecated階層のアーキテクチャ的準備を含む——例えばIE時代の`element.currentStyle`や`element.runtimeStyle`がcompatに必要になった場合に対応可能。

> **Phase 0 サーベイ結果（第29章 §29.2, §29.5）:** width/height属性が60%以上のサイトで使用されており、presentational hints対応はP0要件。StyleSystemはスタイル解決時にこれらをCSS初期値として扱う必要がある。

### 12.2.3 スタイルシートのECSモデル

スタイルシートはECSにエンティティとして格納され、DOMノードと同じセッション仲介パターンが使える：

```rust
pub struct StyleSheetData {
    owner: StyleSheetOwner,       // <link>, <style>, またはconstructable
    rules: Vec<CssRuleEntity>,    // 各ルールもエンティティ
    disabled: bool,
    media: MediaList,
}

pub enum StyleSheetOwner {
    LinkElement(EntityId),        // <link rel="stylesheet">
    StyleElement(EntityId),       // <style>
    Constructed,                  // new CSSStyleSheet()
}
```

スクリプトがCSSOM経由でスタイルシートを変更すると（例：`sheet.insertRule()`）、変更はセッションバッファを経由する。flush時にStyleSystemは影響を受けたスタイルシートの変更通知を受け、変更されたルールにマッチする要素のみをターゲットとしたスタイル再計算をトリガーする。
