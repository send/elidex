# Elidex アーキテクチャ設計ドキュメント

Elidexはレガシー後方互換性を排除して最大パフォーマンスを達成するRust製実験的ブラウザエンジン。Webブラウザと軽量アプリケーションランタイム（elidex-app、Electron/Tauri競合）のデュアルパーパス。

## ドキュメント構成

### Part I — 概要
| 章 | タイトル | 説明 |
| --- | --- | --- |
| 1 | [エグゼクティブサマリー](01-executive-summary.md) | プロジェクトビジョン、コア原則、アーキテクチャ概観 |
| 2 | [アーキテクチャ概要](02-architecture-overview.md) | ハイレベルシステムアーキテクチャとコンポーネント関係 |
| 3 | [ロードマップ](03-roadmap.md) | 開発フェーズとマイルストーン |
| 4 | [リスク＆緩和策](04-risks.md) | 技術、スコープ、エコシステムリスク |

### Part II — コアアーキテクチャ
| 章 | タイトル | 説明 |
| --- | --- | --- |
| 5 | [プロセスアーキテクチャ＆非同期ランタイム](05-process-async.md) | マルチプロセスモデル、Browser/Renderer分離、tokioランタイム |
| 6 | [スレッドモデル](06-thread-model.md) | プロセス内スレッディング：メイン、コンポジター、rayonプール、Worker |
| 7 | [プラグインシステム](07-plugin-system.md) | Core/compat/deprecatedパターン、static dispatch、フィーチャーフラグ |
| 8 | [セキュリティモデル](08-security-model.md) | サンドボックス、パーミッション、CSP、CORS、サイト分離 |

### Part III — コンテンツパイプライン
| 章 | タイトル | 説明 |
| --- | --- | --- |
| 9 | [ナビゲーション＆ページライフサイクル](09-navigation-lifecycle.md) | URL → レンダリングページフロー、履歴、bfcache、プリロードスキャナー |
| 10 | [ネットワークアーキテクチャ](10-network-architecture.md) | HTTPスタック、fetch、キャッシング、Service Worker |
| 11 | [HTMLパーサー](11-parser-design.md) | 厳格コアパーサー、compatエラー回復（LLM関連は停止中） |
| 12 | [DOM＆CSSOM](12-dom-cssom.md) | ECS DOM、CSSカスケード、セレクタマッチング、計算済みスタイル |
| 13 | [ScriptSession](13-script-session.md) | Script ↔ ECS境界、アイデンティティマッピング、変更バッファリング |
| 14 | [スクリプトエンジン＆Web API](14-script-engines-webapi.md) | Boa、elidex-js、wasmtime、Web APIバインディング |

### Part IV — レンダリング
| 章 | タイトル | 説明 |
| --- | --- | --- |
| 15 | [レンダリングパイプライン](15-rendering-pipeline.md) | レイアウト、ペイント、ディスプレイリスト、コンポジター、GPU (Vello/wgpu) |
| 16 | [テキスト＆フォントパイプライン](16-text-pipeline.md) | フォントマッチング、シェーピング、BiDi、行分割、CJK、縦書き |
| 17 | [アニメーション＆スクロール](17-animation-scroll.md) | WAAPI、コンポジターアニメーション、スクロール物理、IntersectionObserver |
| 18 | [画像デコード](18-image-decode.md) | フォーマットサポート、プログレッシブデコード、遅延読込、GPUアップロード |
| 19 | [SVGレンダリング](19-svg-rendering.md) | インラインSVG (ECS)、SVG-as-image (Vello直接)、フィルター |

### Part V — メディア＆データ
| 章 | タイトル | 説明 |
| --- | --- | --- |
| 20 | [メディアパイプライン](20-media-pipeline.md) | 音声/映像、コーデック、MSE、EME/DRM、Web Audio、WebRTCインターフェース |
| 21 | [File API＆ストリーム](21-file-api-streams.md) | Blob、ReadableStream、OPFS、File System Access、圧縮 |
| 22 | [ストレージ＆キャッシュ](22-storage-cache.md) | IndexedDB、Cache API、クォータ、メモリ圧力 |

### Part VI — プラットフォーム＆UI
| 章 | タイトル | 説明 |
| --- | --- | --- |
| 23 | [プラットフォーム抽象化](23-platform-abstraction.md) | OSトレイト、ウィンドウイング、入力、クリップボード、ファイルダイアログ |
| 24 | [ブラウザシェル](24-browser-shell.md) | タブバー、アドレスバー、DevTools、設定、UIフレームワーク |
| 25 | [アクセシビリティ](25-accessibility.md) | A11yツリー、AccessKit、ARIA、フォーカス、ライブリージョン |

### Part VII — elidex-app
| 章 | タイトル | 説明 |
| --- | --- | --- |
| 26 | [エンベディングAPI](26-embedding-api.md) | Engine/View API、ネイティブ↔Webブリッジ、ヘッドレス、Cバインディング |

### Part VIII — 品質＆付録
| 章 | タイトル | 説明 |
| --- | --- | --- |
| 27 | [テスト戦略](27-testing-strategy.md) | WPT、ベンチマーク、ファズテスト、ビジュアルリグレッション |
| 28 | [アーキテクチャ決定記録](28-adr.md) | 35件のADR — 主要設計選択と根拠 |
| 29 | [サーベイ分析](29-survey-analysis.md) | JA/EN 900サイトの互換性サーベイ結果とcompatルール優先度 |

## 言語

本ドキュメントは英語（`en/`）と日本語（`ja/`）の並行版でメンテナンス。
