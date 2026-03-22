# Layout Full Spec Compliance Plan (Pattern B)

Flex / Grid / Table を CSS 仕様フルカバーまで引き上げ、
CSS Fragmentation L3 / Multi-column / Paged Media も完全対応する計画。

## 現状サマリー

| Module | Coverage | Lines (prod) | Tests | Target |
|--------|----------|-------------|-------|--------|
| Flex   | ~75-80%  | 1,158       | 34    | CSS Flexbox L1 100% |
| Grid   | ~55-60%  | 1,581       | 34    | CSS Grid L1 100% + L2 subgrid |
| Table  | ~55-60%  | 1,381       | 47    | CSS 2.1 §17 100% |
| Fragmentation | 0% | 0          | 0     | CSS Fragmentation L3 |
| Multi-column  | 0% | 0          | 0     | CSS Multi-column L1 |
| Paged Media   | 0% | 0          | 0     | CSS Paged Media L3 |

> Table の行数は lib.rs 720 + algo.rs 420 + grid.rs 241 = 1,381 lines。
> Flex の coverage は auto minimum size (§4.5)、blockification (§4.2)、
> cross-size definiteness (§9.9) の欠落により従来見積もりより低い。
> Grid の coverage は justify-content/align-content track distribution (§10.5)、
> full track sizing algorithm (§12.3-12.6) の欠落により同様。

---

## 全体最適化方針

### 原則 1: シグネチャ変更は 1 回

レイアウト関数は現在統一的なパターンに従う:

```rust
// 現在のシグネチャ (4 モジュール共通)
pub fn layout_X(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    layout_child: ChildLayoutFn,
) -> LayoutBox

// ChildLayoutFn 型 (elidex-layout-block/src/lib.rs:55)
pub type ChildLayoutFn = fn(&mut EcsDom, Entity, &LayoutInput<'_>) -> LayoutBox;
```

Fragmentation 対応で `LayoutInput` にフィールドを追加し、戻り値を `LayoutOutcome` に変更する。
これを G1 で 1 回だけ行い、以降は最終シグネチャで全コードを書く。

> **命名:** `LayoutResult` は elidex-plugin の `layout_types.rs` に既存の型
> (`LayoutResult { bounds, margin, padding, border }`) があるため、衝突を避けて `LayoutOutcome` とする。

### 原則 2: ComputedStyle 変更は 1 回

現在 61 フィールド。18 新プロパティを G2 で一括追加 (詳細は G2 参照)。
CSS パーサー、style resolve も同時に実装。elidex-plugin の破壊的変更が 1 回で済む。

### 原則 3: 基盤と消費者を同時に作る

Intrinsic Sizing (A-1) と消費者 (flex-basis:content, fit-content(), auto minimum size) を同時実装。
Baseline (A-2) と消費者 (flex/grid/table baseline) を同時実装。
API 設計のやり直しがゼロになる。

---

## 実行計画: 11 グループ

### 依存関係

```
G1 (型基盤 + blockification + 簡単修正)
 │
G2 (CSS プロパティ一括 18 props) ────────────────────┐
 │                                                    │
G3 (Intrinsic Sizing + auto min size + flex-basis:content + fit-content + track sizing)
 │
G4 (Baseline + flex/grid/table baseline + vertical-align + cross-size definiteness)
 │
 ├── G5 (Grid Named Features + shorthand) ──┐
 │                                           │
 ├── G6 (Table 残り + anonymous objects)     │
 │                                           │
 └── G7 (Subgrid + Writing Mode) ← G5
     │
G8 (Block Fragmentation + break propagation + best-break + box-decoration-break) ← G2
 │
G9 (Multi-column) ← G8
 │
G10 (Flex/Grid/Table Fragmentation) ← G8
 │
G11 (Paged Media + margin boxes) ← G8
```

---

## G1: Layout 型基盤 + 簡単な修正

**目的:** レイアウト関数を最終シグネチャに変更 + 独立した layout-only 修正を同時実施。

### 型定義

```rust
// elidex-layout-block/src/lib.rs に追加

/// Fragmentainer constraints for fragmentation (CSS Fragmentation L3 §3).
#[derive(Clone, Copy, Debug)]
pub struct FragmentainerContext {
    /// Remaining block-size available in current fragmentainer.
    pub available_block_size: f32,
    /// Type of fragmentation (page or column).
    pub fragmentation_type: FragmentationType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FragmentationType {
    Page,
    Column,
}

/// Result of layout, supporting fragmentation.
/// Named `LayoutOutcome` to avoid collision with `elidex_plugin::LayoutResult`.
#[derive(Clone, Debug)]
pub struct LayoutOutcome {
    pub layout_box: LayoutBox,
    /// If Some, more fragments remain — re-invoke layout with this token.
    pub break_token: Option<BreakToken>,
}

/// Opaque token indicating where layout was interrupted.
///
/// For flex: stores interrupted line index + item index within line.
/// For grid: stores interrupted row track index.
/// For table: stores interrupted row index + thead/tfoot entities for repetition.
#[derive(Clone, Debug)]
pub struct BreakToken {
    /// Entity that was being laid out when fragmentation occurred.
    pub entity: Entity,
    /// Block-size consumed before the break.
    pub consumed_block_size: f32,
    /// Nested break token for child that was interrupted.
    pub child_break_token: Option<Box<BreakToken>>,
    /// Layout-mode-specific state for resumption.
    pub mode_data: BreakTokenData,
}

/// Layout-mode-specific break state.
/// No Default — always constructed with explicit child_index / line_index / row_index.
#[derive(Clone, Debug)]
pub enum BreakTokenData {
    /// Block: which child was interrupted (for resumption in `stack_block_children`).
    /// Use `BreakTokenData::Block { child_index: i }` where `i` is the interrupted child.
    Block { child_index: usize },
    /// Flex: which line and item were interrupted.
    Flex { line_index: usize, item_index: usize },
    /// Grid: which row track was interrupted.
    Grid { row_index: usize },
    /// Table: interrupted row + repeated header/footer entities.
    Table { row_index: usize, thead: Option<Entity>, tfoot: Option<Entity> },
}

impl From<LayoutBox> for LayoutOutcome {
    fn from(layout_box: LayoutBox) -> Self {
        Self { layout_box, break_token: None }
    }
}
```

### シグネチャ変更

```rust
// LayoutInput にフィールド追加
pub struct LayoutInput<'a> {
    // ... 既存フィールド ...
    pub fragmentainer: Option<&'a FragmentainerContext>,
    /// Resume token from a previous fragmented layout.
    pub break_token: Option<&'a BreakToken>,
}

// ChildLayoutFn: 戻り値変更
pub type ChildLayoutFn = fn(&mut EcsDom, Entity, &LayoutInput<'_>) -> LayoutOutcome;
```

> **LayoutInput の Copy 維持:** `fragmentainer` と `break_token` は共に参照 (`Option<&'a ...>`)
> なので `LayoutInput` は引き続き `#[derive(Clone, Copy)]` を満たす。

#### 影響する全呼び出しサイト

`ChildLayoutFn` の戻り値を `LayoutBox` → `LayoutOutcome` に変更するため、
以下の全箇所で `.layout_box` 取り出しまたは `LayoutOutcome::from()` ラップが必要:

| ファイル | 関数 / 箇所 | 変更内容 |
|---------|------------|---------|
| `elidex-layout/src/layout.rs` | `dispatch_layout_child()` | 戻り値を `LayoutOutcome` に変更、各 branch の return を `.into()` |
| `elidex-layout/src/layout.rs` | `layout_tree()` | `dispatch_layout_child()` 結果から `.layout_box` を取り出し |
| `elidex-layout-block/src/block/children.rs` | `stack_block_children()` | `layout_child()` 結果から `.layout_box` を取り出し |
| `elidex-layout-block/src/block/mod.rs` | `layout_block_only()` | 戻り値を `LayoutOutcome` に変更 |
| `elidex-layout-block/src/positioned/mod.rs` | `layout_absolutely_positioned()` | `layout_child()` 結果から `.layout_box` を取り出し |
| `elidex-layout-block/src/inline/mod.rs` | `layout_atomic_items()` | `layout_child()` 結果から `.layout_box` を取り出し |
| `elidex-layout-flex/src/lib.rs` | `layout_flex()` item layout | `layout_child()` 結果から `.layout_box` を取り出し |
| `elidex-layout-grid/src/lib.rs` | `layout_grid()` item layout | `layout_child()` 結果から `.layout_box` を取り出し |
| `elidex-layout-table/src/lib.rs` | `layout_table()` cell layout | `layout_child()` 結果から `.layout_box` を取り出し |

全呼び出し元: `fragmentainer: None, break_token: None` を渡し、`.layout_box` で取り出す。

### 同時実施: B-1 + B-4 + D-5 + Flex blockification

| Step | 内容 | 仕様 | 対象 |
|------|------|------|------|
| B-1 | Flex auto margins — free space 吸収 | Flex §8.1 | layout-flex |
| B-4 | Flex visibility:collapse — 主軸 0 + 交差軸貢献 | Flex §4.4 | layout-flex |
| D-5 | Table col span attribute 読取 + caption width fix | §17.5.2.1, §17.4 | layout-table |
| NEW | **Flex item blockification** — inline → block 強制 | Flex §4.2 | layout-flex |

Flex blockification: flex item の display が `inline`/`inline-block`/`inline-table` 等の場合、
block-level equivalent に変換する。`dispatch_layout_child()` 内で flex container の子に対して
display を blockify してからレイアウトする。

**見込み:** +280〜380 lines, 24〜32 tests

---

## G2: CSS プロパティ一括追加

**目的:** 全計画で必要な 18 CSS プロパティを 1 パスで追加。

### ComputedStyle 新フィールド (elidex-plugin)

```rust
// computed_style/flex.rs — AlignmentSafety を全 alignment プロパティに適用
/// CSS Box Alignment Level 3 — safe/unsafe overflow alignment.
/// Applies to: JustifyContent, AlignContent, AlignItems, AlignSelf,
///             JustifyItems, JustifySelf (Grid).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlignmentSafety {
    #[default]
    Unsafe,
    Safe,
}

// computed_style/grid.rs — Grid alignment properties
keyword_enum! { JustifyItems {
    Stretch => "stretch", Start => "start", End => "end",
    Center => "center", Baseline => "baseline",
} }  // default Stretch (CSS Box Alignment L3: initial `normal` → Grid では `stretch` 相当)
keyword_enum! { JustifySelf {
    Auto => "auto", Start => "start", End => "end",
    Center => "center", Stretch => "stretch", Baseline => "baseline",
} }

// computed_style/table.rs — Table property
keyword_enum! { EmptyCells { Show => "show", Hide => "hide" } }  // inherited, default Show

// computed_style/fragmentation.rs — 新ファイル
keyword_enum! {
    /// CSS Fragmentation L3 §3: break-before / break-after values.
    BreakValue {
        Auto => "auto", Avoid => "avoid",
        AvoidPage => "avoid-page", AvoidColumn => "avoid-column",
        Page => "page", Column => "column",
        Left => "left", Right => "right",
        Recto => "recto", Verso => "verso",
    }
}
keyword_enum! {
    /// CSS Fragmentation L3 §3: break-inside values.
    BreakInsideValue {
        Auto => "auto", Avoid => "avoid",
        AvoidPage => "avoid-page", AvoidColumn => "avoid-column",
    }
}
keyword_enum! {
    /// CSS Fragmentation L3 §4.2: box-decoration-break.
    BoxDecorationBreak { Slice => "slice", Cloned => "clone" }
    // variant 名を `Cloned` にして Rust の `Clone` trait との混同を回避
}

// computed_style/columns.rs — 新ファイル
keyword_enum! { ColumnFill { Balance => "balance", Auto => "auto" } }
keyword_enum! { ColumnSpan { None => "none", All => "all" } }

// ComputedStyle (mod.rs) — 18 新フィールド
pub justify_items: JustifyItems,              // Grid §9.1, non-inherited
pub justify_self: JustifySelf,                // Grid §9.2, non-inherited
pub justify_content_safety: AlignmentSafety,  // Box Alignment L3, non-inherited
pub align_content_safety: AlignmentSafety,    // Box Alignment L3, non-inherited
pub empty_cells: EmptyCells,                  // Table §17.5.1, inherited
pub break_before: BreakValue,                 // Frag L3, non-inherited
pub break_after: BreakValue,                  // Frag L3, non-inherited
pub break_inside: BreakInsideValue,           // Frag L3, non-inherited
pub box_decoration_break: BoxDecorationBreak, // Frag L3 §4.2, non-inherited
pub orphans: u32,                             // Frag L3, inherited, default 2
pub widows: u32,                              // Frag L3, inherited, default 2
pub column_count: Option<u32>,                // Multi-col §3, None = auto
pub column_width: Dimension,                  // Multi-col §3, default Auto
pub column_fill: ColumnFill,                  // Multi-col §6, default Balance
pub column_span: ColumnSpan,                  // Multi-col §5, default None
pub column_rule_width: f32,                   // Multi-col §4, default medium (3px)
pub column_rule_style: BorderStyle,           // Multi-col §4, default None
pub column_rule_color: CssColor,              // Multi-col §4, default currentColor
```

#### grid_auto_rows / grid_auto_columns 型変更

CSS Grid L1 §7.6: `grid-auto-rows` / `grid-auto-columns` は track size リストを受け取る
(e.g., `grid-auto-rows: 100px 200px`)。現在の `TrackSize` 単一値を `Vec<TrackSize>` に変更:

```rust
// computed_style/grid.rs — 型変更
pub grid_auto_rows: Vec<TrackSize>,     // was: TrackSize
pub grid_auto_columns: Vec<TrackSize>,  // was: TrackSize
```

- パーサー (`elidex-css-grid`): スペース区切りの track size リストをパース
- Layout (`elidex-layout-grid`): implicit track 生成時にリストを cycling して適用

### CSS パーサー

| Crate | プロパティ |
|-------|-----------|
| elidex-css-flex | `safe`/`unsafe` keyword を全 alignment プロパティのパースに追加 |
| elidex-css-grid | `justify-items`, `justify-self` パース |
| elidex-css-table | `empty-cells` パース |
| elidex-css-multicol (新) | `column-count`, `column-width`, `column-fill`, `column-span`, `column-rule-*`, `columns` shorthand. Note: `column-gap` は既に ComputedStyle に存在 (flex/grid で使用中) し、multicol でもそのまま再利用する |
| elidex-css-box | `break-before`, `break-after`, `break-inside`, `box-decoration-break`, `orphans`, `widows` パース |

### Style Resolve (elidex-style)

各プロパティの resolve + inheritance 設定:
- inherited: `empty_cells`, `orphans`, `widows`
- non-inherited: 残り全て

### Layout での使用

各 layout crate で新プロパティを読み取るコードを追加:
- Grid: `justify_items`/`justify_self` → `position_items()` に水平 alignment
- Grid: `justify_content`/`align_content` → **track 間のスペース分配** (§10.5)
  - `justify-content: center` → track 全体を水平中央に配置
  - `align-content: space-between` → row track 間に均等スペース
  - `space-around`, `space-evenly`, `start`, `end`, `center`, `stretch` の全値を実装
- Table: `empty_cells` → empty cell 判定 + paint skip
- Flex: `AlignmentSafety` → overflow 時の start フォールバック
- Grid: `AlignmentSafety` → Grid alignment にも safe/unsafe 適用
- Flex/Grid: `order` — 現在 layout 順序のみに使用しているが、仕様 (Flexbox §5.4, Grid §5.4)
  では描画順序にも影響する。`order` modified document order が paint order に使われる
  (z-index: auto の場合)。`paint_order.rs` の stacking context 内で `order` 値を考慮した
  描画順に更新が必要

**見込み:** +400〜520 lines, 40〜52 tests (含 elidex-css-multicol 新規作成 + track distribution)

---

## G3: Intrinsic Sizing + 消費者

**目的:** min-content / max-content の統一基盤 + 全消費者を同時実装。

### IntrinsicSizes (elidex-layout)

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct IntrinsicSizes {
    pub min_content: f32,
    pub max_content: f32,
}
```

`compute_intrinsic_sizes(dom, entity, layout_child)` — block/inline/flex/grid/table で分岐:
- Block: min-content = 最長単語幅 (1px containing width で layout), max-content = shrink-wrap
- Inline: min-content = 最長単語, max-content = 改行なし幅
- Flex: min-content = item min-content の最大 (wrap) / 和 (nowrap),
  max-content = item max-content の和 (nowrap/wrap 共通)
- Grid: track min-content / max-content 集約
- Table: §17.5.2 の独自アルゴリズム (cell min/max → column min/max → table min/max)

### 消費者

| Step | 内容 | 仕様 |
|------|------|------|
| B-3 | `flex-basis: content` → `max_content` を flex base size に使用 | Flex §7.3.2 |
| C-4 | `fit-content(200px)` → `min(max_content, max(auto, 200px))` | Grid §7.2.4 |
| NEW | **Flex automatic minimum size** — `min-width/height: auto` の flex item は content-based minimum を使用 (0 ではなく `min_content`) | Flex §4.5 |
| NEW | **Grid full track sizing algorithm** — Initialize → Resolve Intrinsic → Maximize → Stretch の 4 phase 実装 | Grid §12.3-12.6 |

#### Flex §4.5: Automatic Minimum Size

現在 `resolve_min_max()` は `min-width: auto` を 0.0 に解決している。
仕様では flex item の `min-width: auto` (主軸方向) は以下で計算:

```
automatic_minimum_size = min(content_based_minimum, specified_size_suggestion)

content_based_minimum = min_content_size  // from IntrinsicSizes
specified_size_suggestion = width/height if explicit, else infinity
```

`flex-basis: 0` + `overflow: visible` のとき content_based_minimum をそのまま使用。
`overflow: hidden` 等では clamped minimum (0) にフォールバック。

> **inline-flex / inline-grid**: `display: inline-flex` / `display: inline-grid` は
> shrink-to-fit width を使用する (CSS 2.1 §10.3.5)。intrinsic sizing の消費者として
> `compute_intrinsic_sizes()` を呼び出し、max-content を preferred width として使う。

#### Grid §12.3-12.6: Full Track Sizing Algorithm

現在の簡略化された track sizing を 4 phase に拡張:

1. **§12.3 Initialize**: base/limit を初期化
2. **§12.4 Resolve Intrinsic**: spanning item の contribution を distribute
3. **§12.5 Maximize**: 余剰スペースを limit まで拡大
4. **§12.6 Stretch**: `auto` track を stretch (align-content: stretch 時)

`TrackBreadth::FitContent(f32)` variant を追加:
```rust
pub enum TrackBreadth {
    Length(f32),
    Percentage(f32),
    Fr(f32),
    Auto,
    MinContent,
    MaxContent,
    FitContent(f32),  // NEW
}
```

**見込み:** +400〜560 lines, 35〜48 tests

---

## G4: Baseline + 全消費者

**目的:** baseline tracking + flex/grid/table baseline alignment + cross-size definiteness を一括実装。

### Baseline Infrastructure (elidex-plugin + elidex-layout)

```rust
// LayoutBox (elidex-plugin/src/layout_types.rs)
pub struct LayoutBox {
    // ... 既存フィールド ...
    /// First baseline offset from content box top edge.
    pub first_baseline: Option<f32>,
}
```

- `layout_inline()`: 最初の行の baseline を `LayoutBox.first_baseline` に記録
- Block container: 最初の in-flow 子の baseline を伝搬

### 消費者

| Step | 内容 | 仕様 |
|------|------|------|
| B-2 | Flex `align-items: baseline` — per-line で baseline 参加 item を揃える | Flex §9.4 step 9 |
| C-5 | Grid `align-self: baseline` / `justify-self: baseline` | Grid §10.6 |
| D-1 | Table cell `vertical-align: top/middle/bottom/baseline/<length>/<percentage>` | §17.5.1 |
| NEW | **Flex cross-size definiteness propagation** — `align-self: stretch` で cross size が definite になり、子の % 解決に使用可能 | Flex §9.9 |

#### Flex §9.9: Cross-size definiteness

`align-self: stretch` の flex item は definite cross size を持つ。
Item 内の子要素が `height: 50%` のように % 指定の場合、
stretch 後の cross size を containing block として % を解決する。

**実装:** `layout_flex()` の item layout 時に、stretch 後の cross size を
`LayoutInput.containing_height` に設定して子の layout を呼ぶ。

D-1 は baseline 以外の値 (top/middle/bottom) も含むため、baseline infrastructure がなくても
top/middle/bottom は実装可能だが、baseline と同時実装することで cell positioning ロジックの
書き直しを避ける。

**見込み:** +290〜420 lines, 35〜47 tests

---

## G5: Grid Named Features + Shorthand

C-2 (Named Grid Lines) + C-3 (grid-template-areas) + Grid shorthand パースを連続実装。

### Named Grid Lines (C-2)

```rust
// GridTrackList を enum 化 (G7 subgrid で Subgrid variant を追加するため、最初から enum にする)
pub enum GridTrackList {
    Explicit {
        tracks: Vec<TrackSize>,
        line_names: Vec<Vec<String>>,  // tracks.len() + 1 entries
    },
    // Subgrid variant は G7 で追加
}

// GridLine に Named variant 追加 (既存 Span は u32 を維持)
pub enum GridLine {
    Auto,
    Line(i32),
    Span(u32),                        // 既存 — u32 のまま
    Named(String),                    // NEW: [name] 参照
    NamedWithIndex(String, i32),      // NEW: name 2 (2番目の name line)
}
```

- `[name]` 構文パース
- Placement phase: named line → 0-based index 解決

### grid-template-areas (C-3)

- `grid-template-areas` パース: `"header header" "sidebar main"` → 2D Vec
- Area → implicit named lines: `header-start`, `header-end`
- `grid-area: header` shorthand 解決
- Area 形状検証 (矩形チェック)

### Grid shorthand パース (NEW)

| Shorthand | Longhands |
|-----------|-----------|
| `grid-area` | grid-row-start / grid-column-start / grid-row-end / grid-column-end |
| `grid-row` | grid-row-start / grid-row-end |
| `grid-column` | grid-column-start / grid-column-end |

`elidex-css-grid` に shorthand パース + longhand 展開を追加。

### auto-repeat と named lines

`repeat(auto-fill, ...)` / `repeat(auto-fit, ...)` で生成された track の named lines:
- 各反復で line name が重複するため `NamedWithIndex(name, n)` で n 番目を指定
- `auto-fit` の空 track を collapse した場合、line name は残る (collapsed line として)

### Grid placement パイプライン独立関数化 (リファクタ)

G3 で `compute_grid_intrinsic()` (intrinsic.rs) を簡略版 round-robin で実装したが、
実際の grid placement を使わないため明示配置のあるグリッドで shrink-to-fit 幅が不正確。
G5 で named line 解決を placement に追加するタイミングで、placement パイプラインを
独立関数として切り出し、`compute_grid_intrinsic` から再利用する:

1. `layout_grid` の前半 (collect → sort → expand → placement → measure → build_contributions)
   を独立関数 `resolve_grid_contributions()` として切り出し
2. `compute_grid_intrinsic()` から `resolve_grid_contributions()` + `resolve_tracks()` を呼び出し
3. 簡略版 round-robin ロジックを削除

これにより intrinsic.rs と track.rs の 2 経路が 1 本に統合され、
G7 subgrid の双方向 multi-pass にも対応しやすくなる。

### Grid abs-pos containing block (NEW, §11)

Grid area を absolutely positioned 子の containing block として使用:
- `grid-column: 2 / 4; grid-row: 1 / 3` → padding box of that area
- Currently uses grid container padding box; need to resolve grid area rect

**見込み:** +400〜500 lines, 30〜38 tests (placement 切り出し分 +50 lines, +2 tests 含む)

---

## G6: Table 残り + Anonymous Object Generation

D-2 + D-3 + D-6 + D-7 + anonymous table object generation + height redistribution を
elidex-layout-table 1 パスで全修正。

| Step | 内容 | 仕様 |
|------|------|------|
| D-2 | Row/RowGroup borders in collapse — `<tr>`, `<thead>/<tbody>/<tfoot>` border 収集 | §17.6.2 |
| D-3 | Collapse-aware column sizing — collapse border 幅を column width 計算に反映 | §17.6.2.1 |
| D-6 | inline-table — inline layout から table を呼び出し、inline box として配置 | §17.2 |
| D-7 | rowspan=0 → remaining rows, cell % height, anonymous row idempotence | §17.5.3, §17.2.1 |
| NEW | **Anonymous table object generation (full)** — table-row, table-cell, table wrapper の自動生成 | §17.2.1 |
| NEW | **Table height redistribution** — explicit height > content height 時に row 高さを再分配 | §17.5.3 |

#### §17.2.1 Anonymous Table Object Generation (full)

現在は direct cell → anonymous row のみ。以下を追加:

1. **Anonymous table-row wrapper**: `display: table-cell` が `table-row` 外に出現
   → anonymous `table-row` で wrap
2. **Anonymous table wrapper**: `display: table-row` が `table` 外に出現
   → anonymous `table` で wrap
3. **Anonymous table-cell wrapper**: `table-row` 内の non-table content
   → anonymous `table-cell` で wrap

Idempotence: anonymous entity に marker component を付与し、re-layout 時に再利用。

#### §17.5.3 Table Height Redistribution

```
explicit_height = resolve(style.height)
content_height = sum(row_heights) + spacing
if explicit_height > content_height:
    surplus = explicit_height - content_height
    distribute surplus proportionally among rows
```

**見込み:** +350〜480 lines, 35〜45 tests

---

## G7: Subgrid + Writing Mode

C-6 (Subgrid) + B-6 (Writing Mode) — 最も難度が高い 2 step。

### Subgrid (C-6, CSS Grid Level 2 §2)

```rust
// G5 で作成した GridTrackList enum に Subgrid variant を追加
pub enum GridTrackList {
    Explicit { tracks: Vec<TrackSize>, line_names: Vec<Vec<String>> },
    Subgrid { line_names: Vec<Vec<String>> },  // NEW in G7
}
```

- Subgrid item は親 grid の track を参照
- 親 grid の track sizing に subgrid 内容を反映 (双方向 multi-pass)
  - Pass 1: subgrid item の intrinsic size を親 track に contribute
  - Pass 2: 親 track 確定後、subgrid item 内レイアウト
- Named line 継承 + subgrid 側で追加 line name 可能
- **Subgrid 独自の gap**: subgrid は自身の `gap` を持ち、親の gap とは独立 (§2.4)
- **Subgrid の margin/border/padding**: subgrid item の m/b/p は親 track sizing に影響 (§2.5)
- **収束リスク**: subgrid 内容 → 親 track sizing → subgrid layout のフィードバックループは
  理論上収束しない可能性がある。最大反復回数 (3 pass) を設け、収束しなければ最後の結果を使用

### Writing Mode (B-6)

Writing mode は flex だけでなく grid/table にも影響する:

**Flex (CSS Flexbox §5):**
- `FlexContext.writing_mode` → `is_main_horizontal()` を writing-mode aware に
- `vertical-rl/lr` + `flex-direction: row` → 主軸が vertical
- 交差軸計算も同様

**Grid:**
- `writing-mode` により block/inline axis が回転
- track sizing で physical ↔ logical 変換

**Table:**
- `writing-mode` で行/列の方向が変わる
- `caption-side` が logical (block-start/end) に

**前提:** G5 (Named Lines — subgrid が参照)

**見込み:** +320〜450 lines, 28〜38 tests

---

## G8: Block Fragmentation (CSS Fragmentation L3)

**目的:** G1 で用意した型の実体を block layout に実装。

### 対象ファイル

- `elidex-layout-block/src/block/children.rs` — fragmentation ロジック
- `elidex-layout-block/src/block/mod.rs` — BreakToken 処理
- `elidex-layout/src/layout.rs` — fragment loop

### 実装内容

#### §3.1 Forced Breaks

```rust
fn should_force_break(value: &BreakValue) -> bool {
    matches!(value,
        BreakValue::Page | BreakValue::Column |
        BreakValue::Left | BreakValue::Right |
        BreakValue::Recto | BreakValue::Verso
    )
}
```

#### §3.2 Break Propagation

先頭/末尾の子の forced break を親に伝搬:

```rust
// 最初の子の break-before は親の break-before に伝搬
if is_first_child && should_force_break(&child_style.break_before) {
    // 親の break-before として扱う (親自身は break しない)
    propagated_break = Some(child_style.break_before);
}
// 最後の子の break-after は親の break-after に伝搬
if is_last_child && should_force_break(&child_style.break_after) {
    propagated_break_after = Some(child_style.break_after);
}
```

#### §3.3 Unforced Break Classification

Break opportunity のクラス分け:

| Class | 場所 | 条件 |
|-------|------|------|
| A | block-level sibling 間 | 常に break 可能 (avoid でなければ) |
| A | line box 間 | 常に break 可能 |
| B | first child の前 | 親の先頭で break (低優先) |
| C | last child の後 | 親の末尾で break (最低優先) |

**Monolithic elements** (内部 break 不可, CSS Fragmentation L3 §4):
- replaced elements (`<img>`, `<canvas>`, etc.)
- `overflow` != `visible` の要素 (`hidden` / `scroll` / `auto`)

**Out-of-flow elements** (containing block の fragmentainer に固定):
- `position: absolute` — containing block の fragmentainer に留まる。
  内部はフラグメント化しない (containing block 外に溢れた場合は overflow)
- `position: fixed` — viewport に固定、フラグメント化に参加しない

**CSS transform を持つ要素** (UA 裁量で monolithic 扱い):
- CSS Transforms L1 §2 では stacking context + fixed 子孫の containing block を形成するが、
  BFC は確立しない。CSS Fragmentation L3 §4 の monolithic 定義には厳密には該当しない。
- ただし実装上は monolithic として扱う (内部 break を許可すると transform 合成が
  フラグメントをまたぐ必要があり、複雑度が大幅に増加するため)

> CSS Fragmentation L3 §4: "Monolithic elements include... boxes with
> overflow (in the block flow direction of the fragmentation context)
> other than visible."

#### §3.4 Best Break Point Selection

最初の機会で break するのではなく、最適な break 点を選択:

```rust
fn find_best_break(candidates: &[BreakCandidate]) -> Option<usize> {
    // 1. break-inside: avoid が尊重される break 点を優先
    // 2. orphans/widows 制約を満たす break 点を優先
    // 3. 空のフラグメントを避ける
    // 4. Class A > Class B > Class C の優先順
    candidates.iter()
        .filter(|c| !c.violates_avoid)
        .filter(|c| c.satisfies_orphans_widows)
        .min_by_key(|c| c.class)
        .or_else(|| candidates.first())  // fallback: 最初の機会
}
```

#### §4.2 box-decoration-break

```rust
// break 点で border/padding/background の処理
match style.box_decoration_break {
    BoxDecorationBreak::Slice => {
        // デフォルト: border/padding は切断、次フラグメントで続行
        // 最初のフラグメント: border-top + padding-top あり
        // 中間フラグメント: border-top/bottom なし
        // 最後のフラグメント: border-bottom + padding-bottom あり
    }
    BoxDecorationBreak::Cloned => {
        // 各フラグメントで border/padding/background を完全に複製
        // 各フラグメントが独立した box のように描画
    }
}
```

#### Float + Fragmentation Interaction

浮動要素 (`float: left/right`) はフラグメント境界で以下の挙動:

- Float は monolithic — 内部で break しない
- Float が fragmentainer に収まらない場合:
  - Float 全体を次のフラグメントに移動 (forced break ではない)
  - Float を配置しようとした位置が fragmentainer の残りを超える → 次 fragmentainer へ
- Float の clearance (`clear: left/right/both`) は fragmentainer 境界をまたがない
  - 前のフラグメントの float は次フラグメントの clear に影響しない
- Float context (`FloatContext`) は各フラグメントでリセット

### Fragment Loop (layout.rs)

```rust
pub fn layout_fragmented(dom, entity, input, fragmentainer) -> Vec<LayoutOutcome> {
    let mut fragments = Vec::new();
    let mut token: Option<BreakToken> = None;
    loop {
        let mut frag_input = input.clone();
        frag_input.break_token = token.as_ref();
        let result = dispatch_layout_child(dom, entity, &frag_input);
        let done = result.break_token.is_none();
        fragments.push(result);
        // token を更新する前に result を消費済み (push で move) なので
        // break_token の参照は frag_input のスコープ内で有効。
        token = fragments.last().unwrap().break_token.clone();
        if done { break; }
    }
    fragments
}
```

> **実装上の注意:** `frag_input.break_token` は `token` への参照を保持するため、
> `token` の更新は `dispatch_layout_child` 呼び出し後でなければならない。
> 実装時は `token` を loop の末尾で更新し、借用チェッカーを満たす構造にする。

**見込み:** +450〜600 lines, 35〜48 tests

---

## G9: Multi-column Layout (CSS Multi-column L1)

**目的:** Column fragmentainer を生成し、block fragmentation で内容を分割。

### 新クレート: elidex-layout-multicol

```rust
pub fn layout_multicol(
    dom: &mut EcsDom,
    entity: Entity,
    input: &LayoutInput<'_>,
    layout_child: ChildLayoutFn,
) -> LayoutOutcome
```

### アルゴリズム

1. **Column count 決定** (§3.4 pseudo-algorithm):
   ```
   if column_width == Auto && column_count == Auto:
     N = 1  // UA default
   else if column_width != Auto && column_count == Auto:
     N = max(1, floor((available + gap) / (column_width + gap)))
   else if column_width == Auto && column_count != Auto:
     N = column_count
   else:  // both specified
     N = min(column_count, max(1, floor((available + gap) / (column_width + gap))))
   ```
   Note: `column-width` は最小幅であり、実際の幅は `(available - (N-1) * gap) / N`。

2. **Column 幅計算**:
   - `W = (available - (N-1) * gap) / N`

3. **Column fill** (§6):
   - `balance`: 二分探索で全カラムが均等に埋まる最適高さを決定
     - 下限: 最も高い monolithic 要素の高さ (分割不能な最小高さ)
     - 上限: 全内容を 1 column に入れた高さ
     - 収束条件: `|h_new - h_old| < 1px` or 最大 10 iterations
   - `auto`: 順番に埋める (fragmentainer height = container height)

4. **Content fragmentation**:
   - 各 column に `FragmentainerContext { available_block_size: column_height, fragmentation_type: Column }`
   - Block fragmentation (G8) で内容を分割

5. **column-span: all** (§5):
   - spanning 要素の前後でカラム分割をリセット
   - spanning 要素はカラム幅ではなく multicol 全幅でレイアウト
   - **Nested multicol**: `column-span: all` は最も近い multicol 祖先のみ span

6. **Column rule** (§4):
   - カラム間に column-rule を display list に追加
   - column-gap が 0 の場合は rule を中央に描画

7. **Overflow columns** (§8):
   - `column-fill: auto` でカラム高さを超えた場合、追加カラムを生成
   - Overflow column は inline 方向に伸びる

8. **Stacking context** (§3.5):
   - 各 column box が独立した stacking context を確立 (multicol container 自体は BFC を確立)

### ルーティング

`elidex-layout/src/layout.rs` の `dispatch_layout_child()`:
- `column_count.is_some() || column_width != Auto` を検出
- **display: block のみ**: multicol は block container にのみ適用。
  `display: flex` / `display: grid` の要素に `column-count` が指定されても multicol にはならない
  (CSS Multi-column L1 §2: "A multi-column container is a block container")
- multicol container は BFC を確立し、**block layout を置換** (wrapper ではない)
- `layout_multicol()` が内部で column ごとに block fragmentation を実行

### Column 内 break とページ break の相互作用 (§7) — **G11 に延期**

Column 内で `break-before: page` が発生した場合:
- column fragmentation を中断し、page break として BreakToken を返す
- 呼び出し元 (paged context) が新ページで multicol を再開

> **Nested multicol**: multicol 内に multicol がネストした場合、内側の column fragmentation は
> 外側の fragmentainer に制約される。内側の column がオーバーフローしたら、外側の break point
> として扱う。実装は `FragmentainerContext` のネストで自然に対応。
>
> **注記:** Column ↔ Page break interaction、Nested fragmentainer (multicol inside page) は
> Paged Media 基盤が前提のため G11 で対応する。

**見込み:** +550〜750 lines, 35〜48 tests

---

## G10: Flex / Grid / Table Fragmentation

**目的:** G8 の block fragmentation をベースに、各レイアウトモードに fragmentation を追加。

### Flex Fragmentation (CSS Flexbox L1 §12)

```
[主軸方向 (行間)]
  - flex line 間で break opportunity (Class A)
  - break-before/after on flex items
  - break-inside: avoid on flex items
  - flex item の margin は隣接/親子で collapse しない (§3)

[交差軸方向 (wrap 時)]
  - 交差軸が block 方向のとき fragmentainer の影響を受ける
  - wrap された line が fragmentainer を超えたら break

[BreakTokenData::Flex]
  - line_index: 中断された line
  - item_index: line 内で中断された item
  - 次フラグメントで BreakToken から再開

[実装]
  layout_flex() に fragmentainer 対応:
  - line split 後、各 line を fragmentainer remaining に収める
  - line が収まらない場合、BreakToken を返す
  - 次フラグメントで BreakToken から再開
```

### Grid Fragmentation (CSS Grid L1 §10)

```
[Row 間 break]
  - grid row track 間で break opportunity (Class A)
  - break-before/after on grid items
  - spanning item: break する row で item を分割

[BreakTokenData::Grid]
  - row_index: 中断された row track

[実装]
  layout_grid() に fragmentainer 対応:
  - row track positioning で remaining block size をチェック
  - row が収まらない場合、BreakToken を返す
```

### Table Fragmentation (CSS 2.1 §17.5.4)

```
[Row 間 break]
  - table row 間で break opportunity (Class A)
  - thead/tfoot の繰り返し (各フラグメントの先頭/末尾)
  - row group break (thead/tbody/tfoot 境界)

[BreakTokenData::Table]
  - row_index: 中断された row
  - thead/tfoot: 繰り返し対象の entity

[実装]
  layout_table() に fragmentainer 対応:
  - row positioning で remaining block size をチェック
  - thead 高さ + tfoot 高さを remaining から事前差し引き
  - thead/tfoot entity を BreakToken に記録、各フラグメントで再描画
  - **caption は繰り返さない**: `<caption>` は最初のフラグメントにのみ表示 (thead/tfoot と異なる)
  - **counter-increment**: CSS counter は最初のフラグメントでのみ increment する
    (e.g., `content: counter(row)` が各フラグメントで重複 increment しない)
```

**見込み:** +400〜550 lines, 30〜42 tests

---

## G11: Paged Media (CSS Paged Media L3)

**目的:** `@page` ルール + 印刷パイプライン。

### CSS パース

```rust
// @page ルール (elidex-css)
pub struct PageRule {
    pub selectors: Vec<PageSelector>,  // :first, :left, :right, :blank
    pub size: Option<PageSize>,
    pub margins: PageMargins,          // 16 margin box types
    pub properties: Vec<PropertyDeclaration>,  // other properties in @page
}

pub enum PageSize {
    Auto,
    Explicit(f32, f32),          // width, height in px
    Named(NamedPageSize),        // enum with concrete sizes
    LandscapeNamed(NamedPageSize),  // named size with width > height
    LandscapeExplicit(f32, f32),    // explicit size with width > height
    PortraitNamed(NamedPageSize),   // named size with height > width
    PortraitExplicit(f32, f32),     // explicit size with height > width
}

/// CSS Paged Media L3 §5.1: Named page sizes at 96dpi.
pub enum NamedPageSize {
    A5,     // 148mm × 210mm → 559 × 794 px
    A4,     // 210mm × 297mm → 794 × 1123 px
    A3,     // 297mm × 420mm → 1123 × 1587 px
    B5,     // 176mm × 250mm → 665 × 945 px
    B4,     // 250mm × 353mm → 945 × 1334 px
    Letter, // 8.5in × 11in → 816 × 1056 px
    Legal,  // 8.5in × 14in → 816 × 1344 px
    Ledger, // 11in × 17in → 1056 × 1632 px
}
```

### 16 Page-Margin Box Types (§4.2)

```rust
/// CSS Paged Media L3 §4.2: 16 page-margin boxes.
/// Each is a mini formatting context with its own content, width, height.
pub struct PageMargins {
    pub top_left_corner: Option<MarginBoxContent>,
    pub top_left: Option<MarginBoxContent>,
    pub top_center: Option<MarginBoxContent>,
    pub top_right: Option<MarginBoxContent>,
    pub top_right_corner: Option<MarginBoxContent>,
    pub right_top: Option<MarginBoxContent>,
    pub right_middle: Option<MarginBoxContent>,
    pub right_bottom: Option<MarginBoxContent>,
    pub bottom_right_corner: Option<MarginBoxContent>,
    pub bottom_right: Option<MarginBoxContent>,
    pub bottom_center: Option<MarginBoxContent>,
    pub bottom_left: Option<MarginBoxContent>,
    pub bottom_left_corner: Option<MarginBoxContent>,
    pub left_bottom: Option<MarginBoxContent>,
    pub left_middle: Option<MarginBoxContent>,
    pub left_top: Option<MarginBoxContent>,
}

pub struct MarginBoxContent {
    pub content: ContentValue,  // content property value
    pub style: ComputedStyle,   // styling for the margin box
}
```

### Paged Media Context

```rust
pub struct PagedMediaContext {
    pub page_width: f32,
    pub page_height: f32,
    pub page_margins: EdgeSizes,  // main page margins (not margin boxes)
    pub page_rules: Vec<PageRule>,
}
```

### ページ分割

1. Document → `Vec<Page>` に分割
2. 各 Page に `FragmentainerContext { available_block_size: page_content_height, fragmentation_type: Page }`
3. G8 の block fragmentation + G10 の flex/grid/table fragmentation で分割
4. `page-break-before/after` → `break-before/after` にマッピング (legacy 互換)
5. **Viewport → page area**: paged layout 中は initial containing block = page area

### 汎用 CSS Counter 基盤 (CSS Lists L3 §5-7)

`counter(page)` / `counter(pages)` のために汎用 CSS counter を G11 で実装する。

```rust
// counter-reset / counter-increment / counter()
// elidex-css / elidex-style での解決
pub struct CounterState {
    counters: HashMap<String, Vec<i32>>,  // name → stack of values (nested scopes)
}
```

- `counter-reset: name value` — カウンタースコープ作成 (CSS Lists L3 §5.1)
- `counter-increment: name value` — カウンター増減 (CSS Lists L3 §5.2)
- `counter(name)` / `counter(name, list-style-type)` — 値取得 (CSS Lists L3 §6.1)
- `counters(name, string)` — 入れ子カウンター連結 (CSS Lists L3 §6.2)
- **フラグメンテーション制約**: G10 table fragmentation で記録済みのコメントに基づき、
  continuation fragment では `counter-increment` をスキップ (CSS Fragmentation L3 §4 note)
- **list-style-type マーカー**: 現在のハードコード `list_counter` を汎用 counter に移行
  - `<ol>` の `counter-reset: list-item` / `counter-increment: list-item` を暗黙適用
  - `list-style-type` の `counter(list-item)` 評価

### Page Counters

- `counter(page)`: 現在のページ番号 (1-based) — 汎用 counter 基盤の特殊カウンター
- `counter(pages)`: 総ページ数 — **2-pass layout が必要**
  - Pass 1: content → pages に分割、総ページ数を確定
  - Pass 2: `counter(pages)` を含む margin box を再レイアウト
  - 最適化: counter(pages) 未使用なら 1-pass で済む

### Page selectors

```
:first  → 最初のページのみ
:left   → 左ページ (偶数ページ、LTR の場合)
:right  → 右ページ (奇数ページ、LTR の場合)
:blank  → forced break で生じた空白ページ
```

### Marks and Bleed (§7, 低優先)

- `marks: crop cross` — トンボ/十字マーク (印刷用)
- `bleed: 3mm` — 裁ち落とし領域
- これらは display list の最外側に追加

### レンダリング

- ページごとに独立した display list 生成
- margin box の content を各ページのレイアウト時に生成
- ページ番号はカウンターで自動挿入

### エントリポイント

```rust
// elidex-shell or elidex-render
pub fn layout_paged(
    dom: &mut EcsDom,
    page_ctx: &PagedMediaContext,
    font_db: &FontDatabase,
    layout_child: ChildLayoutFn,  // layout dispatch for content fragmentation
) -> Vec<DisplayList>
```

**見込み:** +800〜1,100 lines, 40〜55 tests

---

## 全体サマリー

| Group | 内容 | Lines | Tests |
|-------|------|-------|-------|
| G1 | 型基盤 + blockification + 簡単修正 | +280〜380 | 24〜32 |
| G2 | CSS プロパティ一括 (18 props) + track distribution | +400〜520 | 40〜52 |
| G3 | Intrinsic Sizing + auto min + track sizing 4-phase | +400〜560 | 35〜48 |
| G4 | Baseline + cross-size definiteness | +290〜420 | 35〜47 |
| G5 | Grid Named Features + shorthand + abs-pos CB + placement切り出し | +400〜500 | 30〜38 |
| G6 | Table 残り + anonymous objects + height redistribution | +350〜480 | 35〜45 |
| G7 | Subgrid (gap/mbp) + Writing Mode (flex/grid/table) | +320〜450 | 28〜38 |
| G8 | Block Frag + propagation + best-break + box-decoration | +450〜600 | 35〜48 |
| G9 | Multi-column (stacking ctx, nested span, balance) | +550〜750 | 35〜48 |
| G10 | Flex/Grid/Table Frag (margin non-collapse) | +400〜550 | 30〜42 |
| G11 | Paged Media + 16 margin boxes + 汎用 CSS counter + 2-pass | +900〜1,250 | 45〜62 |
| **合計** | | **+4,590〜6,260** | **365〜490** |

## 最適化効果

| 指標 | ナイーブ (28+ step) | 最適化 (11 group) | 削減 |
|------|-------------------|------------------|------|
| elidex-plugin 変更回数 | 12+ | 3 (G1, G2, G4) | 75% |
| コンパイルサイクル | 28+ | 11 | 61% |
| Layout 関数シグネチャ変更 | 2 | 1 | 50% |
| ComputedStyle フィールド追加回数 | 10+ | 1 (G2) | 90% |

## 実行順序

```
G1 → G2 → G3 → G4 → G5 → G6 → G7 → G8 → G9 → G10 → G11
```

- G1: パーサー/style 無関係で最軽量。fragmentainer 型 + blockification。
- G2: CSS property 追加 + track distribution。以降全 group がこの型を使う。
- G3→G4: 基盤 + 消費者ペア。独立なので逆順でも可。
- G5, G6: 独立。並列可。
- G7: G5 完了が前提 (subgrid は named lines に依存)。
- G8: Fragmentation 実体。G2 の break-* + box-decoration-break を使用。
- G9: G8 完了が前提 (column = fragmentainer)。
- G10: G8 完了が前提。G9 と並列可。
- G11: G8 完了が前提。G9, G10 と並列可。

## 検証

各 Group 完了時:
```sh
cargo test -p elidex-layout -p elidex-layout-block -p elidex-layout-flex \
           -p elidex-layout-grid -p elidex-layout-table
cargo test -p elidex-plugin -p elidex-css-flex -p elidex-css-grid \
           -p elidex-css-table -p elidex-css-box
mise run lint
cargo fmt --all -- --check
```

G2 以降は追加 (G2 で elidex-css-multicol を新規作成):
```sh
cargo test -p elidex-css-multicol
```

G9 以降はさらに追加:
```sh
cargo test -p elidex-layout-multicol
```
