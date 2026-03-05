
# 未決設計事項

本ドキュメントは、現在の設計でギャップまたは不十分と特定されたアーキテクチャ領域を追跡する。他の設計決定への影響（ブロッキング依存関係はより緊急）で優先順位を付ける。

## 優先度定義

- **P0（ブロッキング）**: 他の設計決定がこれに依存。影響を受ける領域の確定前に解決が必要。
- **P1（主要ギャップ）**: 機能的なブラウザに必須だが、ある程度独立して設計可能。
- **P2（不完全）**: 部分的に対処済み、より深い検討が必要。

---

## OPEN-001: マルチプロセスアーキテクチャ [P0] — 解決済

**解決先**: 第5章（プロセスアーキテクチャ & 非同期ランタイム）

**概要**: 段階的緩和モデル — Phase 1–3（SpiderMonkey時代）はサイト単位プロセス分離、全Rust移行後にクラッシュ隔離のみに緩和可能。ProcessModel enum（SiteIsolation / PerTab / Shared / SingleProcess）がビルド時/起動時に設定可能。IPCはトレイト抽象化（ProcessChannel / LocalChannel）で、ゼロコストのプロセス統合を実現。elidex-appのデフォルトはSingleProcess。

---

## OPEN-002: メディアパイプライン（Audio/Video） [P1] — 解決済

**解決先**: 第20章（メディアパイプライン）

**概要**: ロイヤリティフリーコーデック用のRust/CライブラリデコーダとMediaDecoderトレイト（VP8/VP9/AV1はdav1d/libvpx、Opus、Vorbis/lewton、FLAC/claxon、MP3/minimp3）、特許負担コーデック用のプラットフォームデコーダ（H.264/H.265はAVFoundation/MediaFoundation/VA-API、AACはプラットフォーム経由）（ADR #34）。コーデック分離のためのサンドボックス化Decoderプロセス。ゼロコピーGPUテクスチャインポートによるハードウェアアクセラレーションデコード。MediaPlayerがデマクサ→デコーダ→A/V同期→レンダラを協調。A/V同期にオーディオマスタークロック。SourceBuffer経由のMSEが同パイプラインに供給。CDMトレイトとCDMプロセス定義済みのEMEだがv1はCDMなし出荷（Clear Keyをリファレンスとして可能）。専用リアルタイムオーディオスレッド、ロックフリーコマンドキュー、AudioGraph評価、オーディオスレッド上のAudioWorklet、サンプル精度AudioParamオートメーション付きWeb Audio API。パーミッション（第8章）統合のメディアキャプチャ（getUserMedia/getDisplayMedia）。WebRTCインターフェース定義（MediaStreamモデル）、完全実装は延期。プラットフォームオーディオ出力抽象化（CoreAudio/WASAPI/PipeWire）。Core/compat：MPEG-1/AVI=compat、WMV/FLV=非サポート。

---

## OPEN-003: ストレージ＆キャッシュアーキテクチャ [P1] — 解決済

**解決先**: 第22章（ストレージ＆キャッシュアーキテクチャ）

**概要**: 2カテゴリモデル：ブラウザ所有データは集中化されたbrowser.sqlite（Cookie、HSTS、履歴、ブックマーク、設定、パーミッション）、Webコンテンツデータはオリジン単位の分離SQLiteデータベース（elidex.storage、IndexedDB、Cache API、OPFS）。StorageBackendトレイトがSQLite依存を抽象化。OriginStorageManagerがIPC仲介セキュリティ検証でオリジン単位アクセスを協調。ダブルキーパーティショニングによるHTTPキャッシュ。メモリキャッシュ（画像デコード、スタイル共有、フォントグリフ、バイトコード）。即時バック/フォワードナビゲーション用bfcache。LRU退避と永続ストレージ付与によるQuotaManager。

---

## OPEN-004: ナビゲーション＆ページライフサイクル [P2] — 解決済

**解決先**: 第9章（ナビゲーション＆ページライフサイクル）

**概要**: Browserプロセス（NavigationController：URL検証、セキュリティチェック、リダイレクト追跡、サイト分離プロセス選択）とRendererプロセス（DocumentLoader：Document作成、プリロードスキャナー付きHTMLパース、サブリソースロード、DOMContentLoaded/loadイベント）にまたがる統一的エンドツーエンドナビゲーションフロー。ナビゲーション種別：標準、フォーム、履歴、リロード、同一ドキュメント、復元。History API（Core）とNavigation API（Core、SPA向け推奨）。Rendererプロセス凍結によるbfcache、メモリ上限付き退去（ADR #35、デフォルト6エントリ / 512 MB）。完全なページライフサイクルイベントシーケンス（beforeunload → pagehide → freeze → resume → pageshow）。パーサーブロッキングスクリプト中の並列リソースフェッチ用プリロードスキャナー。PerformanceNavigationTiming経由のNavigationTiming公開。サイト分離：同一サイトはRenderer再利用、クロスサイトは新Renderer作成。

---

## OPEN-005: GPUプロセス＆コンポジター詳細 [P2] — 解決済

**解決先**: 第15章拡張（セクション15.4–15.11）、第6章（スレッドモデル）

**概要**: レイヤーツリーをECSコンポーネントではなく独立構造として設計。昇格基準と爆発防止を含む。ディスプレイリスト中間表現（デルタ更新、アリーナアロケーション）。wgpu上のVelloを直接依存（トレイト抽象化なし — ADR #26）、接触面を隔離。段階的テクスチャ管理：個別テクスチャ → 小画像用アトラス → 統一管理。FramePolicy enum（Vsync/Continuous/OnDemand/FixedRate）でブラウザとアプリ両ユースケースに対応するフレームスケジューリング。メインスレッド＋コンポジタのパイプライン化。VRRサポート。コンポジタ駆動のスクロール、アニメーション（transform/opacity/filter）、スクロール連動効果。

---

## OPEN-006: HTTP/HTTPS実装詳細 [P1] — 解決済

**解決先**: 第10章拡張（セクション10.5–10.12）

**概要**: HttpTransportトレイトがhyper依存を抽象化。完全なプロトコルネゴシエーション（HTTP/1.1、HTTP/2、HTTP/3）、接続管理、TLS（rustls + aws-lc-rs）、HTTPS-Onlyデフォルト、セキュリティファーストのデフォルト（DoH、サードパーティCookieブロック）、Network ProcessでのCORS施行、CHIPS付きCookie管理、セキュリティレスポンスヘッダ、PACを含むプロキシサポート。

---

## OPEN-007: 画像デコードパイプライン [P1] — 解決済

**解決先**: 第18章（画像デコードパイプライン）

**概要**: ネットワークバイトからGPUテクスチャまでの完全な画像デコードパイプライン。フォーマットcore/compat分類（PNG/JPEG/WebP/AVIF/GIF/APNGがcore、ICO/BMP/TIFFがcompat）。Rustクレートデフォルトのトレイト抽象化ImageDecoder、プラットフォームネイティブデコーダ差し替え可能（ADR #31）。rayonプール（第6章）でのオフメインスレッドデコード。ヘッダーファーストレイアウト、プログレッシブJPEGレンダリング、ダウンスケールJPEGデコード。レスポンシブイメージ（srcset/pictureソース選択）。IntersectionObserver経由の遅延読み込み。画像デコードキャッシュ（LRU、128MBデフォルト）がHTTPキャッシュ（第22章）とGPUテクスチャ（第15章）と連携。独立ImageAnimationSchedulerによるアニメーション画像スケジューリング（Web Animationsではない）。Blob URLとdata URLサポート。elidex-appで設定可能なデコーダ戦略とキャッシュ予算。

---

## OPEN-008: SVGレンダリング [P1] — 解決済

**解決先**: 第19章（SVGレンダリング）

**概要**: インラインSVG要素をSVG固有コンポーネント（SvgGeometry、SvgTransform、SvgViewport）付きのECSエンティティとして格納。座標ベースジオメトリ用のSvgLayoutSystem（CSSボックスレイアウトとは別）。DisplayItemをSvgPath/SvgTextで拡張しVello経由の統一ペイントパイプライン（ADR #32）。SVG-as-imageは直接Velloパスでレンダリング（ECSなし）、画像デコードキャッシュ（第18章）にビットマップとしてキャッシュ。SVGフィルタエフェクトはGPUレンダーパスDAGとして実装、CSSフィルタ実装と共有。SVGグラデーション/パターンをVelloブラシにマッピング。SVGテキストは共有テキストパイプライン（第16章）をSVG固有配置で使用。SMILはcompatに分類、elidex-compatがWAAPIに変換。クリッピング、マスキング、`<use>`要素再利用。

---

## OPEN-009: アニメーション＆スクロールアーキテクチャ [P1] — 解決済

**解決先**: 第17章（アニメーション＆スクロールアーキテクチャ）、第15章§15.9（コンポジタ駆動操作）

**概要**: FrameProducerがイベントループ（第5章）とレンダリングパイプライン（第15章）間を協調。Web Animations APIを統一内部モデル — CSS TransitionsとCSS Animationsを作成時にWAAPIインスタンスに変換（ADR #29）。AnimationEngineがECS並列クエリ（ActiveAnimationsコンポーネント、DocumentTimelineリソース）でtick — C+Bハイブリッドパターン（ADR #30）。コンポジタ昇格フローと降格処理。PropertyInterpolator（Oklab色、Transform分解/補間/再合成）。アニメーション合成スタック（Replace/Add/Accumulate）。スムーススクロール、スクロールスナップ、スクロールアンカリング。ScrollTimelineによるスクロール連動アニメーション。完全なアニメーションイベントライフサイクル。elidex-appはブラウザイベントループなしでFrameProducerを直接駆動可能。

---

## OPEN-010: パーミッションモデル（ブラウザモード） [P2] — 解決済

**解決先**: 第8章拡張（セクション8.3–8.8）

**概要**: ブラウザモードとアプリモードで共有される統一Permission enum。付与メカニズムのみ異なる（ランタイムユーザープロンプト vs ビルド時マニフェスト）。Browser Process内のPermissionManagerが唯一の権限。3層チェック：Permissions-Policy（ドキュメント）AND iframeのallow（フレーム）AND オリジンレベル決定。オリジン単位の永続ストレージ（browser.sqlite、第22章）。Permissions API（navigator.permissions）とonchangeイベント。プロンプトUIはPermissionPrompterトレイト経由でBrowserShell（第24章）に委譲。アプリモードは拡張ケイパビリティ（FileRead/Write、NetworkUnrestricted、ProcessSpawn等）付きの静的AppCapability。デバッグビルドでケイパビリティ監査ログ。

---

## OPEN-011: 非同期I/Oランタイム＆イベントループ統合 [P0] — 解決済

**解決先**: 第5章（プロセスアーキテクチャ & 非同期ランタイム）

**概要**: プロセスごとのランタイム戦略。Renderer: カスタムelidexイベントループがメインスレッドを所有、tokio current_thread reactorをI/Oバックエンドとして使用。Network/Browser: tokioマルチスレッド。AsyncRuntimeトレイト抽象化で将来の置換オプションを保持。Rendererイベントループが JSイベントループ（第13章）、IPC、I/O、vsyncを単一の統合ループに統合。長期: Rendererで実証済みのイベントループが他プロセスのtokioを置換可能。

---

## OPEN-012: 永続化基盤 [P1] — 解決済

**解決先**: 第22章（ストレージ＆キャッシュアーキテクチャ）

**概要**: 統一2層アーキテクチャ：StorageBackendトレイト（低レベル、SQLiteを抽象化）とドメイントレイト（高レベル、CookiePersistence/HistoryStore/BookmarkPersistence）。WALモード、secure_delete、堅牢化pragmas付きSQLiteを初期バックエンド。ブラウザ所有データはbrowser.sqliteに集中、Webコンテンツデータはオリジン単位の分離データベース。ディレクトリ構造によるプロファイル分離。elidex-appは設定可能なストレージディレクトリとバックエンドを取得。

---

## OPEN-013: File API＆Streams [P1] — 解決済

**解決先**: 第21章（File API＆Streams）

**概要**: Browserプロセス内のBlobStoreにハイブリッドメモリ/ディスクバッキング（インライン≤256 KB、ディスクスピル>256 KB）。オリジン単位のBlob URLレジストリ。FileReaderはcompatに分類（モダンcore：`blob.text()`、`blob.arrayBuffer()`、`blob.stream()`）。Streams APIはRust `ByteStream`（async Streamトレイト）でバッキングし、ScriptSession経由でJS ReadableStream/WritableStreamにブリッジ — pullプロトコルが自然なバックプレッシャーを提供。Rust-to-Rustファストパスによりjs境界を越えないストリーミング（例：fetch → decompress → OPFS write）。ネイティブflate2/miniz_oxideによる圧縮ストリーム。File System Access APIはパーミッションモデル（第8章）とプラットフォームファイルダイアログ（第23章）と統合。共有メモリマップドファイル経由のSyncAccessHandleでゼロIPCのread/write（ADR #33） — SQLite-on-webに不可欠。OPFSストレージはクォータシステム（第22章）下。

---

## OPEN-014: エンベディングAPI [P1] — 解決済

**解決先**: 第26章（エンベディングAPI）

**概要**: elidex-app向けRustネイティブエンベディングAPI。ビルダーパターンのEngine構造体、EngineConfig（ProcessMode SingleProcess/MultiProcess、FeatureFlags、CodecConfig）。ViewConfig付きView構造体（ViewContent: URL/HTML/File/Blank、SurfaceConfig: raw-window-handle経由のCreateWindow/AttachToWindow/Headless、PermissionConfig、NavigationPolicy）。ネイティブ↔Webブリッジ：expose_function（serde経由のRust→JS、JSからの非同期呼び出し）、双方向メッセージチャネル、window.__elidex名前空間。リソースリクエストインターセプト用カスタムResourceLoaderトレイト（app://スキーム、埋め込みアセット）。マルチビューサポート（Engineを共有する独立View群）。SSR/テスト/スクリーンショット用VelloのCPUバックエンドによるヘッドレスモード。CDPサーバーによるDevTools。非Rustエンベッダー向けのcbindgen経由C API。セマンティックバージョニング付きAPI安定性階層（安定/準安定/不安定）。

---

## OPEN-015: プロセス内スレッドモデル [P1] — 解決済

**解決先**: 第6章（プロセス内スレッドモデル）

**概要**: Rendererは4つのスレッドクラス：メインスレッド（ECS DOM所有者、イベントループ、スクリプト）、コンポジタースレッド（独立レイヤー合成、スクロール、GPU送信）、rayonプール（並列スタイル/レイアウト/デコード）、Web Workerスレッド（Worker毎に1:1 OSスレッド）。コンポジターはFrameSourceトレイト抽象化で段階的B→C移行を実現：アプローチB（DisplayListメッセージパッシング、Phase 1–3）からアプローチC（共有ECS読み取り、Phase 4+）へプロセス分離緩和時に移行。ECS並行性モデルはフェーズ分離アクセスに基づく（アプローチBではロックなし、アプローチCではダブルバッファリングレイヤー）。Browser/Networkプロセスはtokioマルチスレッドとspawn_blocking（SQLite用）。GPUプロセス：GPUスレッド1つ + IPCスレッド1つ。

**ブロック**: ~~OPEN-005~~（コンポジタースレッディング）、~~OPEN-009~~（オフメインスレッドスクロール/アニメーション）

---

## 小規模ギャップ（既存章またはOPEN項目に吸収可能）

独立したOPEN項目にするほど大きくないが、親章や関連OPEN項目の作業時に対処すべき事項。

| 領域 | 吸収先 | 備考 |
| --- | --- | --- |
| Web Fonts（@font-face読み込み、FOIT/FOUT、font-display、variable fonts） | Ch16拡張 | 現Ch16はシェーピングのみ。フォント読み込みはレンダリングブロッキングリソースで体感パフォーマンスに影響。ネットワーク取得 → デコード → シェーピングのパイプライン。 |
| Canvas 2D実装アーキテクチャ | Ch12拡張 | P0 Web APIとして列挙されているが1行で記述。即時モードレンダリングモデルが保持モードDOMと根本的に異なる。GPUアクセラレーション（OffscreenCanvas）、ワーカースレッドレンダリング。 |
| メモリ管理戦略 | OPEN-001 / OPEN-003 | タブごとメモリ予算、メモリプレッシャー処理（OSの低メモリ通知）、画像退避、タブ破棄。プロセス横断的な関心事。 |
| Selection / editing / contentEditable | Ch13（将来のOM） | ScriptSessionで「将来のOMプラグイン」として言及済み。複雑（カーソル移動、範囲選択、inputイベント、execCommand）。ブロッキングではないが実装スコープは大きい。 |
| スクロール動作詳細 | ~~OPEN-005~~ / ~~OPEN-009~~ | smooth scrolling、scroll anchoring、overscroll-behavior、scroll-snap。第15章§15.9と第17章§17.7でカバー。 |
| フォーム＆入力ウィジェット | Ch23拡張 | Ch2でクレート列挙済み（elidex-html-forms）。プラットフォームネイティブの日付/色ピッカー（Ch23）、バリデーションAPI、オートフィル統合。 |
| 印刷 / @media print | 後回し | 最終的には必要だが設計への影響は小さい。印刷固有スタイルによる別レイアウトパス。 |
| URLスキームディスパッチ（mailto:, tel:, カスタム） | Ch13 / Ch14拡張 | ブラウザが外部アプリにURLを引き渡す必要がある。プラットフォーム依存（xdg-open、NSWorkspace、ShellExecute）。 |
| PDF表示（インラインまたは外部） | Ch14拡張または小セクション新設 | インラインPDFビューア（pdf.js相当）またはOS委譲。大きな機能だが自己完結的。 |
| クラッシュレポート | OPEN-001拡張 | プロセスクラッシュキャプチャ、ミニダンプ生成、オプショナルアップロード。特にRendererクラッシュ回復に重要。 |
| エンジンテレメトリ/ロギング | Ch27（テスト）拡張 | 構造化ロギング（tracingクレート）、パフォーマンスカウンター、エラーテレメトリパイプライン。Ch5でパーサーパターンのテレメトリに言及があるが汎用フレームワークなし。 |
| Server-Sent Events (EventSource) | Ch12拡張 | WebSocketより単純。HTTPベースのストリーミング。OPEN-011（非同期ランタイム）に依存。 |
| ブラウザ自動化プロトコル（WebDriver / CDP） | Ch24（DevTools）拡張 | Selenium、Playwright、PuppeteerがWebDriverまたはChrome DevTools Protocolに依存。WebDriver BiDiが両方を統合する新興標準。DevTools（第24章 §24.4）とインフラを共有。elidex自体のCIテスト（第27章）にも最低1つの自動化プロトコルが必要。 |
| 自動更新メカニズム | 後回し | 本番デプロイに不可欠だがエンジンアーキテクチャの問題ではない。別プロセスのアップデーター、差分更新、ロールバック。プロダクトレベルのインフラ。 |
| Spectre/Meltdown＆サイドチャネル緩和 | OPEN-001拡張 | Site Isolation（サイト単位プロセス）、高精度タイマー制限、COOP/COEP背後のSharedArrayBufferゲーティング。プロセスモデル設計時に自然に対処。 |
| Performance Observer / Reporting API | Ch12拡張 | PerformanceObserver、Long Tasks API、Reporting API（CSP違反レポート、deprecationレポート）。エンジン全体の計測ポイント。 |

---

## サマリーマトリクス

> **注記**: 全15項目が解決済み。章番号は現在のリファクタリング後のナンバリングを反映。

| ID | タイトル | 優先度 | ステータス | 依存 | 推定スコープ |
| --- | --- | --- | --- | --- | --- |
| OPEN-001 | マルチプロセスアーキテクチャ | P0 | **解決済（第5章）** | — | — |
| OPEN-002 | メディアパイプライン | P1 | **解決済（第20章）** | ~~OPEN-001~~ | — |
| OPEN-003 | ストレージ＆キャッシュ | P1 | **解決済（第22章）** | ~~OPEN-001~~ | — |
| OPEN-004 | ナビゲーション＆ライフサイクル | P2 | **解決済（第9章）** | ~~OPEN-001~~, ~~003~~ | — |
| OPEN-005 | GPU＆コンポジター | P2 | **解決済（第15章）** | ~~OPEN-001~~ | — |
| OPEN-006 | HTTP/HTTPS実装 | P1 | **解決済（第10章）** | ~~OPEN-001~~ | — |
| OPEN-007 | 画像デコードパイプライン | P1 | **解決済（第18章）** | ~~OPEN-001~~, ~~003~~, ~~005~~ | — |
| OPEN-008 | SVGレンダリング | P1 | **解決済（第19章）** | ~~OPEN-005~~ | — |
| OPEN-009 | アニメーション＆スクロール | P1 | **解決済（第17章）** | ~~OPEN-005~~ | — |
| OPEN-010 | パーミッションモデル | P2 | **解決済（第8章）** | Ch. 8, 23, 24 | — |
| OPEN-011 | 非同期I/Oランタイム | P0 | **解決済（第5章）** | ~~OPEN-001~~ | — |
| OPEN-012 | 永続化基盤 | P1 | **解決済（第22章）** | ~~OPEN-001~~, ~~003~~ | — |
| OPEN-013 | File API＆Streams | P1 | **解決済（第21章）** | ~~OPEN-001~~, ~~011~~, ~~012~~ | — |
| OPEN-014 | エンベディングAPI | P1 | **解決済（第26章）** | ~~OPEN-001~~, Ch. 23, 24 | — |
| OPEN-015 | プロセス内スレッドモデル | P1 | **解決済（第6章）** | ~~OPEN-001~~, ~~011~~ | — |
