
# 2. アーキテクチャ概要

## 2.1 プロセスアーキテクチャ

Elidexは、Chromiumのセキュリティモデルに着想を得つつLadybirdのクリーンな分離を参考にしたマルチプロセスアーキテクチャを使用する。5つのプロセスタイプがIPC経由で通信する（詳細は第5章）：

| プロセス | 責務 | 主要な依存関係 |
| --- | --- | --- |
| Browserプロセス | Chrome UI（タブ、アドレスバー、設定）、ナビゲーション＆セッション管理、プロファイル＆Cookie保存、パーミッション仲介（第8章） | iced or egui（Rustネイティブ GUI）、ipc-channel |
| Rendererプロセス | HTML/CSS解析、DOM管理（ECS）、スタイル解決（並列）、レイアウト計算（並列）、ディスプレイリスト生成、JavaScript実行 | elidex-core + plugins、Boa（Phase 1–3）→ elidex-js、wasmtime（Wasmランタイム）、rayon（並列処理） |
| Networkプロセス | HTTP/HTTPSスタック、DNS解決、コネクションプーリング、Cookie jar、TLS、WebSocket | hyper + rustls、h3 |
| GPUプロセス | GPUラスタライゼーション、レイヤー合成、コンポジタ駆動スクロール＆アニメーション | wgpu、Vello |
| Utilityプロセス | メディアデコード、オーディオ処理。オンデマンド生成、アイドル時に終了 | dav1d、プラットフォームコーデック |

各タブは独自のRendererプロセスを生成し、Browserプロセスからサンドボックス化される。GPUプロセスがwgpuサーフェス管理を担当し、各Renderer内のコンポジタスレッドがそれと連携する。

## 2.2 クレート構成

プロジェクトは明確な依存関係境界を持つCargoワークスペースとして構成される：

```
elidex/
├── elidex-core/              # フレームワーク（機能固有のロジックは含まない）
│   ├── elidex-ecs/           # Entity Component System
│   ├── elidex-pipeline/      # レンダリングパイプラインオーケストレーション
│   ├── elidex-plugin/        # プラグイントレイト定義＆レジストリ
│   ├── elidex-plugin-macros/ # デュアルディスパッチ生成用procマクロ
│   └── elidex-render/        # GPUレンダリングフレームワーク（wgpu）
├── elidex-plugins/            # 機能プラグイン（個別にトグル可能）
│   ├── elidex-html-base/     # コアHTML5要素（<div>, <span>, <a>, <img>）
│   ├── elidex-html-media/    # <video>, <audio>, <canvas>
│   ├── elidex-html-forms/    # <input>, <form>, <select>
│   ├── elidex-css-box/       # display, position, margin, padding, ボックスモデル
│   ├── elidex-css-flex/      # Flexbox
│   ├── elidex-css-grid/      # CSS Grid
│   ├── elidex-css-text/      # フォント、テキスト装飾、writing modes
│   ├── elidex-css-anim/      # トランジション、アニメーション
│   ├── elidex-layout-block/  # Blockレイアウトアルゴリズム
│   ├── elidex-layout-flex/   # Flexレイアウトアルゴリズム
│   ├── elidex-layout-grid/   # Gridレイアウトアルゴリズム
│   ├── elidex-layout-table/  # Tableレイアウトアルゴリズム
│   ├── elidex-dom-api/       # DOM APIプラグイントレイト＋ハンドラ（Living Standard）
│   ├── elidex-dom-compat/    # レガシーDOM APIシム（ライブコレクション、document.write）
│   └── elidex-a11y/          # アクセシビリティツリー生成
├── elidex-script/             # スクリプティングレイヤー
│   ├── elidex-script-session/ # ScriptSession: 統一的なScript ↔ ECS境界
│   │                          #   Identity Map、Mutation Buffer、GC協調
│   ├── elidex-js/            # 自前JSエンジン（ES2020+コア、Rust製）
│   ├── elidex-js-compat/     # ESレガシーセマンティクス（Annex B、var quirks）
│   ├── elidex-js/ # Boaブリッジ（Phase 1-3フォールバック）
│   ├── elidex-wasm-runtime/  # wasmtime統合
│   └── elidex-dom-host/      # 共有DOMホスト関数（JS + Wasm）
├── elidex-text/               # テキストパイプライン
│   ├── elidex-shaping/       # テキストシェイピング（rustybuzz）
│   ├── elidex-bidi/          # 双方向テキスト（unicode-bidi）
│   └── elidex-linebreak/     # 改行処理（icu4x）
├── elidex-compat/             # ブラウザモード互換性（オプショナル）
│   ├── elidex-parser-tolerant/  # エラー回復HTMLパーサー
│   ├── elidex-compat-tags/      # 非推奨タグ → HTML5変換
│   ├── elidex-compat-css/       # ベンダープレフィクス解決
│   ├── elidex-compat-charset/   # Shift_JIS/EUC-JP → UTF-8
│   └── elidex-compat-dom/       # レガシーJS APIシム（document.all等）
├── elidex-llm-repair/         # LLM支援エラー回復（現在は一時停止）
│   ├── elidex-llm-runtime/   # ローカル推論（candle/llama.cpp、停止中）
│   └── elidex-llm-diag/      # 開発モード診断メッセージ生成（停止中）
├── elidex-net/                # ネットワーキング
│   ├── elidex-http/          # HTTP/1.1, HTTP/2, HTTP/3（hyper + h3）
│   ├── elidex-tls/           # TLS（rustls）
│   ├── elidex-cache/         # ディスク/メモリキャッシュ
│   ├── elidex-net-middleware/ # ミドルウェアトレイト＋パイプライン
│   └── elidex-resource/      # ResourceLoaderトレイト（http://, file://, app://）
├── elidex-security/           # セキュリティモデル
│   ├── elidex-sandbox/       # プロセスサンドボックス
│   ├── elidex-origin/        # 同一オリジンポリシー、CORS
│   └── elidex-csp/           # Content Security Policy
├── elidex-api/                # Web API実装（DOM以外）
│   ├── elidex-api-fetch/     # Fetch API（P0コア）
│   ├── elidex-api-canvas/    # Canvas 2D（P0コア）
│   ├── elidex-api-workers/   # Web Workers（P1コア）
│   ├── elidex-api-ws/        # WebSocket（P1コア）
│   ├── elidex-api-observers/ # Intersection/Resize Observer（P1コア）
│   ├── elidex-api-crypto/    # Web Crypto API（P1コア）
│   ├── elidex-api-cookies/   # CookieStore API（P1コア）
│   ├── elidex-api-storage/   # elidex.storage 非同期KV API（P1コア）
│   ├── elidex-api-idb/       # IndexedDB（P2コア）
│   ├── elidex-api-gpu/       # WebGL/WebGPU（P2コア、wgpuバックエンド）
│   ├── elidex-api-sw/        # Service Workers（P3コア）
│   ├── elidex-api-xhr/       # XMLHttpRequest互換（Fetch経由でシム）
│   ├── elidex-api-storage-compat/ # localStorage/sessionStorage互換（ブロッキングIPCシム）
│   └── elidex-api-cookies-compat/ # document.cookie互換（CookieStore経由でシム）
├── elidex-platform/            # プラットフォーム抽象化レイヤー
│   ├── elidex-platform-api/   # トレイト定義（PlatformProvider、サブシステムトレイト）
│   ├── elidex-platform-linux/  # Linux（X11/Wayland、IBus/Fcitx、AT-SPI2）
│   ├── elidex-platform-macos/  # macOS（Cocoa、Input Method Kit、NSAccessibility）
│   ├── elidex-platform-windows/ # Windows（Win32、TSF、UIA）
│   └── elidex-platform-common/ # 共有ユーティリティ（イベント正規化、キーマッピング）
├── elidex-shell/               # ブラウザシェル
│   ├── elidex-shell-api/      # トレイト定義（TabManager、NavigationManager等）
│   ├── elidex-shell-state/    # デフォルト状態マネージャー実装
│   ├── elidex-chrome-native/  # ネイティブchrome（egui/iced、Phase 1-2）
│   ├── elidex-chrome-selfhost/ # セルフホストchrome（HTML/CSS、Phase 3+）
│   ├── elidex-devtools/       # DevTools実装
│   └── elidex-extension-host/ # 拡張機能マウントポイントとライフサイクル
├── elidex-browser/            # フルブラウザ（コア＋全プラグイン＋互換＋シェル）
├── elidex-app/                # アプリランタイム（コア＋選択プラグインのみ）
└── elidex-crawler/            # Web互換性サーベイツール
```
