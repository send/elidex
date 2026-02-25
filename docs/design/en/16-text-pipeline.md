
# 16. Text & Font Pipeline

## 16.1 Overview

Text is the most complex rendering primitive in a browser engine. It crosses every pipeline stage: parsing (character encoding), DOM (text nodes), styling (font selection, color), layout (line breaking, bidi), and paint (glyph rasterization). Elidex's text pipeline is designed for first-class CJK and bidirectional support from day one.

```
Text node in ECS
  │
  ├── Font resolution (fontdb: system fonts + web fonts)
  ├── Itemization (split into runs by script, language, font)
  ├── Shaping (rustybuzz: codepoints → positioned glyphs)
  ├── BiDi reordering (unicode-bidi: logical → visual order)
  ├── Line breaking (ICU4X: Unicode line break + dictionary CJK)
  ├── Layout (inline formatting context: runs → lines → boxes)
  └── Paint (Vello: glyph outlines or glyph atlas → GPU)
```

## 16.2 Font Resolution

### 16.2.1 Font Matching

Font matching follows the CSS Fonts specification cascade:

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

Matching algorithm (CSS Fonts Level 4): filter by family name → style (italic > oblique > normal) → weight (closest match) → stretch. If no match, try next family in stack. Ultimate fallback: platform default (San Francisco / Segoe UI / Noto Sans).

### 16.2.2 Generic Font Families

| Generic | macOS | Windows | Linux |
| --- | --- | --- | --- |
| `serif` | Times New Roman | Times New Roman | Noto Serif |
| `sans-serif` | Helvetica Neue | Segoe UI | Noto Sans |
| `monospace` | Menlo | Cascadia Mono | Noto Sans Mono |
| `system-ui` | San Francisco | Segoe UI | System default |

CJK fallback chain for `sans-serif`: Hiragino Sans (macOS) → Yu Gothic (Windows) → Noto Sans CJK (Linux).

### 16.2.3 Web Fonts (@font-face)

Loading behavior controlled by `font-display`:
- **swap** (default): Render with fallback immediately, swap when loaded (FOUT).
- **block**: Invisible text up to 3s, then fallback.
- **fallback**: 100ms block, then fallback. Swap if loaded within 3s.
- **optional**: Short block, then fallback permanently. Best for performance.

Core formats: WOFF2 (primary, Brotli-based), WOFF, OpenType/TrueType.

## 16.3 Text Itemization

Text is split into "runs" — segments sharing the same font, script, language, and direction:

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

Itemization steps: script detection (ICU4X ScriptExtensions) → font fallback (split run if primary font lacks coverage) → BiDi analysis (unicode-bidi embedding levels) → language tagging (`lang` attribute inheritance).

## 16.4 Shaping

Shaping converts Unicode codepoints into positioned glyphs via **rustybuzz** (pure-Rust HarfBuzz port):

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

Handles: ligatures (fi, ffl, Arabic connected forms), kerning, complex scripts (Arabic contextual forms, Devanagari conjuncts, Thai mark positioning), OpenType features (`kern`, `liga`, `calt`, `smcp` etc. via CSS `font-feature-settings` and `font-variant-*`).

Shaping runs on the rayon pool for parallelism across independent text runs.

## 16.5 Bidirectional Text (BiDi)

The Unicode Bidirectional Algorithm handles mixed LTR/RTL text via the `unicode-bidi` crate:

1. **Analysis** (during itemization): Compute embedding levels from characters + explicit overrides (`dir`, `unicode-bidi` CSS).
2. **Line breaking** (during layout): Lines broken in logical order.
3. **Reordering** (between layout and paint): Visual order computed per line. Odd-level runs reversed.

CSS: `direction: rtl`, `unicode-bidi: embed | bidi-override | isolate | isolate-override | plaintext`.

## 16.6 Line Breaking

### 16.6.1 Algorithm

Line breaking uses ICU4X `LineSegmenter` with Unicode UAX #14 rules plus language-specific overrides.

### 16.6.2 CJK Line Breaking

CJK-specific rules: generally breakable between CJK characters (no spaces needed); punctuation placement constraints (opening brackets can't end a line, closing brackets can't start); kinsoku shori (禁則処理) for Japanese; dictionary-based segmentation for Japanese word boundaries.

CSS: `word-break: normal | break-all | keep-all`, `overflow-wrap: normal | break-word | anywhere`, `line-break: auto | loose | normal | strict`.

### 16.6.3 Hyphenation

Auto-hyphenation (`hyphens: auto`) via Liang's algorithm (`hyphenation` crate). Language-specific patterns. Soft hyphens (`&shy;`) always respected.

## 16.7 Vertical Writing

Supported from day one for Japanese and Chinese content:

```rust
pub enum WritingMode {
    HorizontalTb,
    VerticalRl,      // Japanese default
    VerticalLr,      // Mongolian
}
```

The layout engine uses abstract **inline/block** coordinates, not physical x/y. Writing mode determines the mapping at paint time:

| | Horizontal-TB | Vertical-RL | Vertical-LR |
| --- | --- | --- | --- |
| Inline axis | x | y | y |
| Block axis | y | x | x |

CSS: `writing-mode`, `text-orientation: mixed | upright | sideways`, `text-combine-upright` (tate-chū-yoko).

## 16.8 Ruby Annotation

Ruby (`<ruby>`) provides phonetic annotations (furigana, pinyin). Annotation positioned above (horizontal) or right (vertical) of base text. `ruby-position` controls placement. Overhang rules allow annotation to extend over adjacent whitespace.

## 16.9 Glyph Rasterization

Two paths: small text (≤48px) uses a **glyph atlas** (CPU rasterize → texture atlas → GPU blit) for cache efficiency; large text (>48px) uses **Vello vector outlines** directly on the GPU.

Subpixel positioning: glyphs rasterized at up to 4 horizontal subpixel positions for better readability. LCD subpixel rendering: Windows (ClearType), Linux (configurable), macOS (disabled since Mojave).

Text decoration (`underline`, `overline`, `line-through`): positioned using font metrics. Descender skipping (`text-decoration-skip-ink: auto`) uses glyph outlines to compute gaps.

## 16.10 Character Encoding

Core: UTF-8 only. Compat layer (`elidex-compat-charset`) handles Shift_JIS, EUC-JP, ISO-2022-JP, GB2312/GBK/GB18030, EUC-KR, ISO-8859-*, Windows-1252. Detection via `<meta charset>`, HTTP `Content-Type`, BOM, or encoding sniffing heuristics.

## 16.11 elidex-app Text

| Aspect | elidex-browser | elidex-app |
| --- | --- | --- |
| Web fonts | Full support | Full support |
| Legacy encodings | Compat layer | Excluded (UTF-8 only) |
| Vertical writing | Full support | Full support |
| Ruby | Full support | Full support |
| Hyphenation data | Bundled common languages | Configurable |
