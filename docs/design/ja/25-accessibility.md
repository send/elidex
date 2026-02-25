
# 25. アクセシビリティ

## 25.1 概要

アクセシビリティ（a11y）はelidexのコンテンツをスクリーンリーダー、スイッチデバイス、音声制御等の支援技術で利用可能にする。a11yシステムはECS DOMとレイアウト結果からアクセシビリティツリーを導出し、AccessKit経由でプラットフォームAPIに公開。

```
ECS DOM (TreeRelation, TagType, Attributes, ComputedStyle, LayoutBox)
  │
  ▼
A11ySystem（ECSシステム）
  │  読み取り：セマンティックロール、ARIA属性、テキスト内容、バウンディング矩形
  │  書き込み：AccessibilityTreeリソース
  ▼
AccessKitアダプタ
  │  プラットフォームAPIに変換
  ▼
プラットフォーム (NSAccessibility / UI Automation / AT-SPI2)
  │
  ▼
支援技術 (VoiceOver, NVDA, Orca)
```

## 25.2 アクセシビリティツリー

### 25.2.1 ツリー構築

A11ySystemはDOMコンポーネントを読み取りアクセシビリティツリーを構築するECSシステム：

```rust
pub struct A11ySystem;

impl A11ySystem {
    pub fn build_tree(&self, world: &World) -> AccessibilityTree {
        let mut tree = AccessibilityTree::new();

        for (entity, tag, attrs, style, layout) in
            world.query::<(Entity, &TagType, &Attributes, &ComputedStyle, Option<&LayoutBox>)>()
        {
            // a11yから隠された要素をスキップ
            if attrs.get("aria-hidden") == Some("true") || style.display == Display::None {
                continue;
            }

            let role = self.compute_role(tag, attrs);
            let name = self.compute_accessible_name(entity, world, attrs);
            let bounds = layout.map(|l| l.bounding_rect());

            tree.add_node(AccessibilityNode {
                entity,
                role,
                name,
                description: attrs.get("aria-description").map(String::from),
                value: self.compute_value(tag, attrs),
                state: self.compute_state(attrs),
                bounds,
                children: children_of(world, entity).collect(),
                actions: self.compute_actions(tag, attrs),
            });
        }

        tree
    }
}
```

### 25.2.2 ロールマッピング

各HTML要素プラグインがデフォルトARIAロールを宣言：

| 要素 | デフォルトロール | 備考 |
| --- | --- | --- |
| `<button>` | button | |
| `<a href>` | link | href属性がある場合のみ |
| `<input type="text">` | textbox | |
| `<input type="checkbox">` | checkbox | |
| `<img alt="...">` | img | `alt=""`→プレゼンテーショナル（非表示） |
| `<nav>` | navigation | ランドマーク |
| `<main>` | main | ランドマーク |
| `<h1>`–`<h6>` | heading | aria-level付き |
| `<table>` | table | |
| `<ul>`、`<ol>` | list | |
| `<li>` | listitem | |

明示的ARIAロール（`role="..."`）がデフォルトロールを上書き。無効なロール値は無視。

### 25.2.3 アクセシブル名の計算

ACCNAME（Accessible Name and Description Computation）アルゴリズムに従う：

1. `aria-labelledby` → 参照先要素のテキストを連結
2. `aria-label` → 直接使用
3. ネイティブラベル（`<label for>`、`alt`、`title`、`placeholder`）
4. テキスト内容（`<button>`、`<a>`等の要素）
5. `title`属性（最終手段）

## 25.3 プラットフォーム統合

### 25.3.1 AccessKit

`accesskit`クレートがクロスプラットフォームa11y抽象化を提供：

| プラットフォーム | ネイティブAPI | AccessKitアダプタ |
| --- | --- | --- |
| macOS | NSAccessibility | accesskit_macos |
| Windows | UI Automation | accesskit_windows |
| Linux | AT-SPI2 (D-Bus) | accesskit_unix |

ツリー更新モデル：各フレーム（またはDOM変更時）にelidexが変更ノードを含む`TreeUpdate`を送信。AccessKitがプラットフォーム固有API呼び出しに変換。

### 25.3.2 更新戦略

毎フレームの完全ツリー再構築は高コスト。代わりにA11ySystemがダーティノードを追跡：DOM変更（ノード追加/削除/属性変更）→ マークダーティ、レイアウト変更（バウンディング矩形変更）→ マークダーティ、可視性に影響するスタイル変更 → マークダーティ。ダーティサブツリーのみ再評価。

## 25.4 フォーカス管理

フォーカスはECSリソースとして追跡：

```rust
pub struct FocusState {
    pub focused_entity: Option<EntityId>,
    pub focus_visible: bool,
}
```

フォーカス順はDOM順（デフォルト）または`tabindex`。フォーカス変更時にA11ySystemがAccessKitに通知し、プラットフォームフォーカスイベントを発火。スクリーンリーダーが新しくフォーカスされた要素をアナウンス。

モーダルのフォーカストラッピング：`<dialog>`と`role="dialog"`要素がTab循環を子孫に制限。

## 25.5 ライブリージョン

ARIAライブリージョン（`aria-live="polite|assertive"`）が動的コンテンツ変更をアナウンス：

```rust
pub struct LiveRegionAnnouncement {
    pub text: String,
    pub priority: LivePriority,
}

pub enum LivePriority {
    Polite,     // 現在のスピーチ後にアナウンス
    Assertive,  // 現在のスピーチを中断
}
```

ライブリージョン内のコンテンツ変更時、A11ySystemが変更テキストを抽出しAccessKitにアナウンスを送信。

## 25.6 ハイコントラスト＆強制カラー

`prefers-color-scheme`と`forced-colors`メディアクエリをCSSに公開。`forced-colors: active`モードでブラウザがページ色をシステムカラースキームで上書き（Windowsハイコントラスト）。レンダリングパイプラインが`color-scheme`とシステムカラー（`Canvas`、`CanvasText`、`LinkText`等）を尊重。

## 25.7 elidex-appアクセシビリティ

| 側面 | elidex-browser | elidex-app |
| --- | --- | --- |
| A11yツリー | フル、自動 | フル、自動 |
| プラットフォーム統合 | AccessKit | AccessKit |
| フォーカス管理 | 標準 | 標準 + Embedding API経由でアプリ管理可能 |
| ライブリージョン | フルサポート | フルサポート |
| ハイコントラスト | 尊重 | 尊重 |
