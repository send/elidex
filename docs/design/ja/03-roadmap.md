
# 3. 開発ロードマップ

## Phase 0：基盤（1〜3ヶ月目）

プロジェクトインフラを確立し、互換性サーベイを実施。

| 成果物 | ステータス | 依存関係 |
| --- | --- | --- |
| クレート構成を持つCargoワークスペース | 完了 | — |
| CIパイプライン（GitHub Actions） | 完了 | — |
| elidex-crawlerサーベイ実施（900サイト） | 完了 | — |
| サーベイ結果分析、compatルール優先度付け | 完了（[第29章](29-survey-analysis.md)） | クローラー結果 |
| プラグイントレイト定義（CssPropertyHandler, HtmlElementHandler, LayoutModel） | 完了 | — |
| ECS DOMストレージプロトタイプ | 完了 | — |
| アーキテクチャ決定記録（ADR）：セキュリティモデル、テキストパイプライン、writing-mode座標系 | 完了（35件） | — |

## Phase 0.5：拡張サーベイ（3〜4ヶ月目、条件付き）

Phase 0の結果で有意なHTMLエラー普及率が示された場合にトリガーされる条件付きフェーズ。LLM支援エラー回復への投資価値を判断するためにクロールを拡大。

> **Phase 0 サーベイ結果（第29章）:** Phase 0サーベイ（900サイト）で回復不能パーサーエラー率 0% が確認され、トリガー条件（有意なHTMLエラー普及率）が未達成のため、本フェーズは**スキップ**。

| 成果物 | ステータス | 依存関係 |
| --- | --- | --- |
| サイトリストを3,000〜5,000サイトに拡大 | スキップ（トリガー条件未達成） | Phase 0分析 |
| サイトごとにトップページ + 5〜10サブページをクロール | スキップ（トリガー条件未達成） | 拡張サイトリスト |
| 全ページをhtml5everで解析、エラーイベント記録・回復可能性分類 | スキップ（トリガー条件未達成） | クロールデータ |
| LLMトレーニング/評価用の壊れたHTMLコーパス構築 | スキップ（トリガー条件未達成） | エラー分類 |
| 判定ゲート：回復不能エラー率に基づくLLMフォールバックのGo/No-Go | **No-Go 確定** — 回復不能エラー率 0%（[第29章](29-survey-analysis.md)） | 分析完了 |

**判定ゲート：** 回復不能エラー率が約2%未満の場合、LLMランタイムフォールバックは延期され、ルールベース回復のみを実装。elidex-app向けLLM開発者診断はいずれにせよ進行。

> **Phase 0 サーベイ結果（第29章）:** 900サイトのサーベイで回復不能エラー率 0%（< 2% 閾値）。**No-Go 確定** — LLMランタイムフォールバックは延期、ルールベース回復のみを実装。elidex-app向けLLM開発者診断（elidex-llm-diag）はPhase 3で予定通り進行。

## Phase 1：最小限レンダリング（4〜8ヶ月目）

最初のピクセルを画面に表示する。目標は\<div\>、\<p\>、\<span\>、\<a\>、\<img\>をブロックレイアウトと基本CSS（color, font, margin, padding, border, display, position）でレンダリングすること。

| 成果物 | 想定期間 | 依存関係 |
| --- | --- | --- |
| HTML5 strictパーサー（parse_strict） | 4週間 | プラグイントレイト |
| CSSパーサー + コアプロパティハンドラ | 4週間 | プラグイントレイト |
| StyleSystem（並列スタイル解決） | 3週間 | ECS DOM、CSSパーサー |
| ブロックレイアウトエンジン | 4週間 | StyleSystem |
| テキストシェイピングパイプライン（rustybuzz + fontdb） | 3週間 | — |
| wgpuレンダリングバックエンド + テキストラスタライゼーション | 4週間 | レイアウト、テキストシェイピング |
| ウィンドウシェル（iced/egui）にレンダリング出力を表示 | 2週間 | wgpuバックエンド |

**マイルストーン：** スタイル付きテキストと画像を含む静的HTML5ドキュメントをレンダリングするウィンドウ。

## Phase 2：インタラクティブエンジン（9〜14ヶ月目）

JavaScript実行、Flexboxレイアウト、ネットワーキングを追加。エンジンがシンプルな動的Webページを表示可能になる。

| 成果物 | 想定期間 | 依存関係 |
| --- | --- | --- |
| wasmtime統合（elidex-wasm-runtime） | 4週間 | ECS DOM |
| DOM APIプラグイン層（elidex-dom-api、Living Standard） | 5週間 | ECS DOM |
| 共有DOMホスト関数（elidex-dom-host、JS + Wasm） | 3週間 | DOM API、wasmtime |
| SpiderMonkey統合（elidex-js-spidermonkey、Phase 1-3 JS） | 5週間 | DOMホスト関数 |
| イベントシステム（click, input, keyboard） | 3週間 | DOMバインディング |
| Flexboxレイアウトプラグイン | 4週間 | ブロックレイアウト |
| ネットワーキングスタック（hyper + rustls） | 3週間 | — |
| Fetch API実装 | 2週間 | ネットワーキング、wasmtime |
| プロセスサンドボックス（Linux先行） | 3週間 | マルチプロセスアーキテクチャ |
| Tolerantパーサー（elidex-parser-tolerant） | 3週間 | クローラーデータ分析 |
| CIでのWPT統合 | 2週間 | レンダリングパイプライン |

**マイルストーン：** URLにナビゲートし、JavaScript駆動ページ（例：シンプルなSPA）をレンダリングしてインタラクション可能に。

## Phase 3：実用レベル（15〜20ヶ月目）

CSS Grid、互換レイヤー、アクセシビリティ、アプリランタイムを追加。elidexがモダンサイトの日常ブラウジングとデスクトップアプリ構築に使用可能になる。

| 成果物 | 想定期間 | 依存関係 |
| --- | --- | --- |
| CSS Gridレイアウトプラグイン | 5週間 | Flexレイアウト |
| Tableレイアウトプラグイン | 3週間 | ブロックレイアウト |
| 互換レイヤー：タグ正規化 + CSSプレフィクス解決 | 3週間 | クローラーデータ |
| 互換レイヤー：文字コード変換（Shift_JIS、EUC-JP） | 2週間 | クローラーデータ |
| BiDiテキストサポート | 3週間 | テキストパイプライン |
| CJK縦書きモード | 4週間 | レイアウトエンジン、テキストパイプライン |
| アクセシビリティツリー + AccessKit統合 | 4週間 | ECS DOM、レイアウト |
| Canvas 2D API | 3週間 | wgpuバックエンド |
| elidex-appランタイムMVP（Wasm + JS、多言語） | 3週間 | コア安定、wasmtime |
| レガシーDOM API互換レイヤー（elidex-dom-compat） | 3週間 | DOM APIコア |
| Shadow DOM + Web Componentsサポート | 4週間 | DOM API、ECSツリースコーピング |
| elidex-jsパーサー + AST（ES2020+ Stage 1） | 6週間 | — |
| ブラウザクローム（タブ、アドレスバー、履歴） | 4週間 | ナビゲーション、UIフレームワーク |
| Chromium/Firefoxとのパフォーマンスベンチマーク | 2週間 | レンダリングパイプライン |
| elidex-app向けLLM駆動開発者診断（elidex-llm-diag） | 3週間 | strictパーサー、candle/llama.cpp |
| 壊れたHTMLコーパス収集 + LLMファインチューニングデータセット | 2週間 | クローラー結果 |

**マイルストーン：** claude.ai、主要ニュースサイト、GitHubをブラウズ。RustまたはWasmターゲット言語でelidex-appサンプルデスクトップアプリを構築。

## Phase 4：プロダクション対応（21〜30ヶ月目）

日常使用に耐えるよう強化。セキュリティ監査、クロスプラットフォームサポート、WebWorkers、Service Workers、PWAサポート。

| 成果物 | 想定期間 | 依存関係 |
| --- | --- | --- |
| サンドボックス強化（macOS、Windows） | 4週間 | Linuxサンドボックス |
| Web Workers（スレッド上のWasmインスタンス） | 4週間 | wasmtime、スレッドモデル |
| WebSocket + Server-Sent Events | 2週間 | ネットワーキング |
| IndexedDB | 3週間 | wasmtime |
| Service Workers + PWAサポート | 5週間 | Web Workers、Fetch、キャッシュ |
| CSSアニメーション + トランジション（コンポジター駆動） | 4週間 | コンポジター |
| フォームコントロール（ネイティブレンダリング） | 4週間 | レイアウト、イベントシステム |
| セキュリティ監査（外部） | 継続 | 全セキュリティコード |
| 最初の非推奨化サイクル（データ駆動） | 2週間 | クローラー再実行 |
| elidex-jsバイトコードコンパイラ + インタプリタ（Stage 2） | 8週間 | elidex-jsパーサー |
| elidex-jsインラインキャッシュ + hidden classes（Stage 3） | 6週間 | バイトコードインタプリタ |
| ESレガシー互換レイヤー（elidex-js-compat：Annex B、var quirks） | 4週間 | elidex-jsコア |
| LLMランタイムフォールバック（elidex-llm-runtime）統合 | 4週間 | ファインチューニング済みモデル、tolerantパーサー |
| オフラインルール生成パイプライン（LLM → ルールベースパーサー） | 3週間 | LLMランタイム、クローラーコーパス |

> **Phase 0 サーベイ結果（第29章 §29.6）:** 上記2項目（LLMランタイムフォールバック、オフラインルール生成パイプライン）はPhase 0 No-Go判定により**暫定延期**。将来のクロールデータで回復不能エラーが有意に検出された場合に再検討可能。

**マイルストーン：** モダンサイト向けデイリードライバーブラウザとしてのelidex。elidex-app 1.0リリース。

## Phase 5：長期（30ヶ月目以降）

**elidex-jsベースラインJIT（Stage 4）：** elidex-jsバイトコード用のCraneliftベースJITコンパイラ。計算集約的JSにおけるSpiderMonkeyとのパフォーマンスギャップを橋渡し。

**SpiderMonkey撤去：** elidex-jsが許容可能な実世界パフォーマンス（ベンチマークで検証）を達成したら、SpiderMonkeyを削除。純粋Rustスタックの目標を達成。

**elidex-js最適化JIT（Stage 5）：** 必要に応じて投機的最適化パスを追加。Stage 4ベースラインJITがターゲットワークロードに不十分な場合のみ追求。

**WebGPU API：** elidexのネイティブwgpuバックエンドを活用し、JavaScriptとWasmにGPUコンピュートを公開。

**DevTools：** elidexのECSアーキテクチャを念頭に設計された組み込みインスペクターとプロファイラー。

**拡張システム：** elidexのデュアルディスパッチプラグインモデルにスコープされた軽量拡張API。

**定期的非推奨化：** 継続的なクローラーサーベイが全三層（HTML、DOM API、ECMAScript）にわたる機能削除決定を定期的なケイデンスで通知。
