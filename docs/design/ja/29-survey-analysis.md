
# 29. サーベイ分析

## 29.1 サーベイ概要

elidex-crawlerを使用して、日本語（JA）と英語（EN）の主要Webサイトに対する互換性サーベイを実施した。

| 指標 | JA | EN |
| --- | --- | --- |
| 対象サイト数 | 451 | 449 |
| 成功（HTML取得） | 387 (85.8%) | 412 (91.8%) |
| 失敗 | 64 (14.2%) | 37 (8.2%) |

### 失敗内訳

| 失敗理由 | JA | EN |
| --- | --- | --- |
| robots.txtブロック | 5 | 16 |
| タイムアウト | 26 | 5 |
| 非HTMLコンテンツ | 3 | 1 |
| その他接続エラー | 30 | 15 |

JA側のタイムアウトが多いのは、日本国内ホスティングへの海外からのアクセス制限や、低帯域接続が原因と推測される。EN側ではrobots.txtブロックが多く、大手プラットフォーム（Facebook, Instagram, Twitter/X, LinkedIn, Reddit, Netflix等）がクローラーを拒否している。

## 29.2 HTML分析

### 非推奨タグ

| 指標 | JA | EN |
| --- | --- | --- |
| 使用サイト数 | 17 (4.4%) | 10 (2.4%) |

**JA 非推奨タグ:**

| タグ | 出現総数 | サイト数 | サイト率 |
| --- | --- | --- | --- |
| `<font>` | 94 | 9 | 2.0% |
| `<big>` | 25 | 1 | 0.2% |
| `<center>` | 9 | 7 | 1.6% |
| `<nobr>` | 2 | 1 | 0.2% |

**EN 非推奨タグ:**

| タグ | 出現総数 | サイト数 | サイト率 |
| --- | --- | --- | --- |
| `<center>` | 8 | 5 | 1.1% |
| `<font>` | 6 | 4 | 0.9% |
| `<nobr>` | 2 | 1 | 0.2% |
| `<blink>` | 1 | 1 | 0.2% |

非推奨タグの使用率は低い（JA 4.4% / EN 2.4%）。`<font>`と`<center>`が最も一般的。

### 非推奨属性

| 指標 | JA | EN |
| --- | --- | --- |
| 使用サイト数 | 278 (71.8%) | 297 (72.1%) |

`width`と`height`属性が圧倒的に多い（JA: 59.0%/56.8%、EN: 62.6%/62.8%のサイトで使用）。これらは主に`<img>`タグのサイズヒントとして使われており、レイアウトシフト防止のためにモダンブラウザでも推奨されるパターン。

**JA 上位5属性:**

| 属性 | 出現総数 | サイト数 | サイト率 |
| --- | --- | --- | --- |
| `width` | 13,973 | 266 | 59.0% |
| `height` | 12,588 | 256 | 56.8% |
| `size` | 112 | 38 | 8.4% |
| `border` | 85 | 20 | 4.4% |
| `align` | 53 | 14 | 3.1% |

**EN 上位5属性:**

| 属性 | 出現総数 | サイト数 | サイト率 |
| --- | --- | --- | --- |
| `width` | 17,290 | 281 | 62.6% |
| `height` | 16,662 | 282 | 62.8% |
| `size` | 894 | 31 | 6.9% |
| `color` | 591 | 74 | 16.5% |
| `text` | 88 | 5 | 1.1% |

### パーサーエラー

| 指標 | JA | EN |
| --- | --- | --- |
| エラーありサイト数 | 186 (48.1%) | 181 (43.9%) |
| エラー総数 | 2,309 | 1,463 |

**JA 上位5エラー:**

| エラー | 件数 |
| --- | --- |
| Found special tag while closing generic tag | 724 |
| Duplicate attribute | 405 |
| Unexpected token | 316 |
| No `<p>` tag to close | 121 |
| Unexpected open element | 105 |

**EN 上位5エラー:**

| エラー | 件数 |
| --- | --- |
| Duplicate attribute | 310 |
| Unexpected token | 262 |
| Saw ? in state TagOpen | 197 |
| Found special tag while closing generic tag | 186 |
| Saw = in state BeforeAttributeValue | 85 |

約半数のサイトでパーサーエラーが検出されるが、これらはhtml5everの自動エラー回復により正常に処理される。回復不能エラーは確認されなかった。

## 29.3 CSS分析

### ベンダープレフィックス

| 指標 | JA | EN |
| --- | --- | --- |
| 使用サイト数 | 70 (18.1%) | 177 (43.0%) |

**プレフィックス別:**

| プレフィックス | JA サイト数 (率) | EN サイト数 (率) |
| --- | --- | --- |
| `-webkit-` | 66 (14.6%) | 174 (38.8%) |
| `-ms-` | 40 (8.9%) | 109 (24.3%) |
| `-moz-` | 42 (9.3%) | 127 (28.3%) |
| `-o-` | 12 (2.7%) | 51 (11.4%) |

EN側でプレフィックス使用率が顕著に高い。`-webkit-`が最も普及しており、elidexのcompat層で優先的にサポートすべき。

### 非標準プロパティ

| プロパティ | JA サイト率 | EN サイト率 |
| --- | --- | --- |
| `-webkit-appearance` | 4.7% | 17.6% |
| `-webkit-font-smoothing` | 2.0% | 17.4% |
| `-moz-osx-font-smoothing` | 1.8% | 12.9% |
| `-moz-appearance` | 2.4% | 12.5% |
| `-webkit-tap-highlight-color` | 2.2% | 10.2% |
| `-webkit-overflow-scrolling` | 1.8% | 9.4% |
| `-webkit-text-size-adjust` | 2.2% | 13.6% |
| `-ms-overflow-style` | 2.2% | 7.6% |
| `zoom` | 0.7% | 2.9% |

### エイリアス（旧構文）

| エイリアス | JA サイト率 | EN サイト率 |
| --- | --- | --- |
| `-webkit-box-align` | 2.7% | 14.0% |
| `-webkit-box-pack` | 3.5% | 13.6% |
| `word-wrap` | 3.5% | 13.8% |
| `-webkit-box-orient` | 5.5% | 12.7% |

flexboxの旧構文（`-webkit-box-*`）は依然として使われている。`word-wrap`は`overflow-wrap`のレガシーエイリアス。

## 29.4 JavaScript分析

| 指標 | JA | EN |
| --- | --- | --- |
| `document.write`使用 | 48 (12.4%) | 22 (5.3%) |
| `document.all`使用 | 0 (0.0%) | 0 (0.0%) |

`document.all`の使用はゼロ。`document.write`はJAで12.4%と比較的高いが、多くは広告スクリプトやアナリティクスタグによるもの。elidexはstrict-onlyアプローチのためこれらのAPI互換は不要。

## 29.5 Compatルール優先度

サーベイ結果に基づき、互換性ルールを以下の優先度で実装する。

### P0（必須）

- **width/height presentational hints:** `<img>`の`width`/`height`属性をCSS初期値として適用。サイトの60%以上が使用しており、レイアウトシフト防止に不可欠。
- **`-webkit-`エイリアス:** `-webkit-appearance`→`appearance`、`-webkit-box-*`→`flex`等。EN側で40%近いサイトが使用。

### P1（推奨）

- **`appearance`プロパティ:** `-webkit-appearance`と`-moz-appearance`の標準化対応。
- **font-smoothing:** `-webkit-font-smoothing`と`-moz-osx-font-smoothing`。EN側で17%が使用。
- **`-webkit-text-size-adjust`:** モバイル表示制御。13.6%のENサイトが使用。

### P2（低優先）

- **`<font>`/`<center>`タグ:** 使用率5%未満。elidexのstrict方針に基づき非サポートでも影響は小さい。
- **`document.write`:** strict-onlyアプローチで非対応。
- **`-ms-`/`-o-`プレフィックス:** レガシーブラウザ固有。

## 29.6 Phase 0.5 判定ゲート

### パーサーエラー回復

サーベイ結果により、パーサーエラーは約半数のサイトで検出されるが、エラーの性質はhtml5everの仕様準拠エラー回復アルゴリズムにより自動的に処理可能なものばかりである。

主なエラーカテゴリ:
- **構造エラー**（special tag closing, unexpected open element）: html5everの再構成アルゴリズムが処理
- **属性エラー**（duplicate attribute, 引用符問題）: 最初の値を採用、以降を無視
- **文字参照エラー**: ベストエフォートデコード

回復不能エラー（ページが完全に壊れるケース）は確認されなかった。

### LLMフォールバック判定

**暫定判定: No-Go**

理由:
1. 回復不能パーサーエラーの実証データがゼロ
2. html5everの自動回復で実用上十分
3. LLMランタイムフォールバックのコスト（レイテンシ、メモリ、複雑性）に見合うリターンがない

ただし、elidex-app向けのLLM開発者診断（elidex-llm-diag）はPhase 3で予定通り進行する。これは壊れたHTMLの修復ではなく、開発者のコード品質改善を支援するもの。

## 29.7 Phase 1への示唆

1. **Presentational hints対応が必須:** `width`/`height`属性は60%以上のサイトで使用。Phase 1のCSSパーサーでpresentational hintsをサポートする必要がある。

2. **ベンダープレフィックスの段階的対応:** `-webkit-`を最優先でcompat層に実装。Phase 3の互換レイヤーで本格対応。

3. **flexbox旧構文:** `-webkit-box-*`→`flex`のマッピングはPhase 2のFlexboxレイアウト実装時に組み込む。

4. **パーサー設計への影響:** 約半数のサイトでエラーがあるが、html5everの回復で十分。elidex-parser-tolerant（Phase 2）はhtml5everのエラー回復をそのまま活用し、カスタム回復ロジックは最小限に留める。
