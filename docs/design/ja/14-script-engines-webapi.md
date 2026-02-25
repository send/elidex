
# 14. スクリプトエンジン & Web API

本章ではスクリプト実行エンジン（ECMAScriptとWebAssembly）およびDOM/CSSOM以外のWeb API群をカバーする。すべてのスクリプトエンジンはScriptSession（第13章）を通じてのみECSとやり取りする。

## 14.1 ECMAScriptエンジン

ECMAScript実行も同じ「コア + 互換」パターンに従う。エンジンはES2020+をモダンベースラインとし、レガシーセマンティクスはオプショナルプラグインで処理する：

```rust
pub enum EsSpecLevel {
    Modern,          // ES2020+: let/const, arrow fn, class, async/await, modules,
                     //          destructuring, template literals, optional chaining
    LegacySemantics, // var hoisting quirks, ブロック内function hoisting,
                     //          sloppy mode == 型変換
    AnnexB,          // JS内HTMLコメント, __proto__アクセサ,
                     //          RegExpレガシー機能, 文字列HTMLメソッド
}
```

### 14.1.1 何を切るか

| 機能 | コア | 互換 | 根拠 |
| --- | --- | --- | --- |
| let / const / class / arrow | ✓ | — | モダンな変数宣言と構文 |
| async / await / Promise | ✓ | — | 非同期モデル。イベントループと密結合（第13章）。 |
| ES Modules (import/export) | ✓ | — | 主要モジュールシステム。CommonJSは互換レイヤーのみ。 |
| Proxy / Reflect | ✓ | — | メタプログラミング。モダンフレームワーク（Vue 3 reactivity）に必要。 |
| var hoisting（関数スコープ） | ✗ | ✓ | 無数のバグの源泉。varはパースされるがコアではletとして扱われる。互換がquirky hoistingを復元。 |
| with文 | ✗ | ✓ | エンジン最適化を阻害（スコープが予測不能）。strictモードですでに禁止。 |
| arguments.callee / .caller | ✗ | ✓ | strictモードで禁止。末尾呼び出しとインライン最適化を阻害。 |
| __proto__アクセサ | ✗ | ✓ | Annex B。代わりにObject.getPrototypeOf()を使用。 |
| JS内HTMLコメント (\<!-- --\>) | ✗ | ✓ | 1990年代の\<script\>隠蔽のAnnex B遺物。 |
| eval()（直接） | 制限付き | ✓ | コアはstrict-mode evalのみサポート（新スコープ）。sloppy eval（ローカルスコープ注入）は互換。 |

### 14.1.2 実装戦略

JSエンジンの構築はelidexで最大の単一コンポーネントである。SpiderMonkeyが作業中のブラウザを提供し続ける中、実装はステージごとに進行する：

| ステージ | 成果物 | elidex-browser使用 | elidex-app使用 |
| --- | --- | --- | --- |
| 1 | Parser + AST（ES2020+構文） | SpiderMonkey | SpiderMonkeyまたはWasm |
| 2 | バイトコードコンパイラ + インタプリタ | SpiderMonkey | elidex-js（自前） |
| 3 | インラインキャッシュ + hidden classes | 切替可能：SpiderMonkeyまたはelidex-js | elidex-js |
| 4 | ベースラインJIT（Craneliftバックエンド） | elidex-js | elidex-js |
| 5 | 最適化JIT（必要に応じて） | elidex-js | elidex-js |

Boaプロジェクト（Rust製JSエンジン）は参考になるが、ブラウザ用途にはプロダクション品質ではない。elidex-jsエンジンはBoaのアーキテクチャを研究しつつ異なるトレードオフを取れる——特に、elidex-jsはAnnex Bとsloppy modeをコアから省略でき、実装を大幅に簡素化できる。

ScriptEngineトレイト抽象化により、SpiderMonkeyとelidex-jsはいつでも交換可能。エンジンは直接ECSアクセスではなくScriptSessionを受け取る：

```rust
pub trait ScriptEngine: Send + Sync {
    fn name(&self) -> &str;
    fn eval(&self, source: &str, ctx: &mut ScriptContext) -> Result<JsValue>;
    fn call(&self, func: &JsFunction, args: &[JsValue]) -> Result<JsValue>;
    fn bind_session(&mut self, session: &mut dyn ScriptSession);
    fn run_microtasks(&mut self);
}

enum ScriptBackend {
    SpiderMonkey(MozJsEngine),    // Phase 1-3: 成熟、フル互換
    ElidexJs(ElidexJsEngine),     // Phase 2+: 自前、成長中
}
```

## 14.2 Wasmランタイム

WebAssemblyサポートはwasmtime（成熟したRustネイティブWasmランタイム）経由で提供される。WasmはJSの従属ではなく、対等な一級市民である：

| コンテキスト | JSエンジン | Wasmランタイム |
| --- | --- | --- |
| elidex-browser | SpiderMonkey → elidex-js（フェーズ移行）。\<script\>タグを処理。 | wasmtime。JSからのWebAssembly.instantiate()とネイティブ.wasmモジュールを処理。 |
| elidex-app | elidex-js（ES2020+のみ、互換なし）。JS/TSを使用するアプリ向け。 | wasmtime。非JS言語（Rust、Go、C++、Zig等）の主要ランタイム。 |

WasmモジュールはJSエンジンが使用するのと同じScriptSessionを介してDOMおよびCSSOMとやり取りし、呼び出し言語に関係なく一貫した動作を保証する：

```rust
// JSエンジンとWasmランタイムで共有されるホスト関数
// すべての書き込みは共有ScriptSession経由
pub trait SessionHostFunctions {
    fn query_selector(&self, root: EntityId, selector: &str) -> Option<EntityId>;
    fn get_attribute(&self, entity: EntityId, name: &str) -> Option<String>;
    fn set_attribute(&mut self, entity: EntityId, name: &str, value: &str);
    fn add_event_listener(&mut self, entity: EntityId, event: &str, cb: CallbackRef);
    fn set_inline_style(&mut self, entity: EntityId, property: &str, value: &str);
    fn batch_update(&mut self, ops: &[SessionOperation]) -> Vec<OperationResult>;
}
```

batch_update APIはWasmパフォーマンスに不可欠。各Wasm→ホスト境界越えにはオーバーヘッドがあるため、複数操作を単一呼び出しにバッチ化することでこのコストを劇的に削減する。バッチ内のすべての操作はセッションバッファに記録され、一緒にフラッシュされる。

## 14.3 多言語アプリケーションランタイム

Wasmランタイムにより、elidex-appは多言語アプリケーションプラットフォームとなる。開発者が言語を選択：

| 言語 | ツールチェーン | ユースケース例 |
| --- | --- | --- |
| Rust | wasm-bindgen + wasm-pack | 最大パフォーマンス。elidex内部と共有型システム。 |
| TypeScript / JS | elidex-js（ES2020+）直接 | Web開発者の馴染みの言語。Electronからの段階的移行。 |
| Go | TinyGo → Wasm | 社内ツールを構築するバックエンドチーム。 |
| C / C++ | Emscripten → Wasm | 既存ネイティブアプリケーションの移植。 |
| Zig | zig build → Wasm | シンプルなツールチェーンでのシステムプログラミング。 |
| C# / Kotlin | Blazor / Kotlin/Wasm | エンタープライズチーム（.NETまたはJVMエコシステム）。 |

## 14.4 Web APIスコープ

DOM以外のWeb APIも、他のすべてのレイヤーに適用されるのと同じcore/compat/deprecatedパターンに従う。各APIはWebApiSpecLevelで分類され、プラグインとして実装される：

### 14.4.1 コアWeb API

| 優先度 | API | クレート | 備考 |
| --- | --- | --- | --- |
| P0 | Fetch API | elidex-api-fetch | Promiseベースのネットワーキング。XMLHttpRequestを置換。 |
| P0 | Canvas 2D | elidex-api-canvas | Webアプリとフレームワークで広く使用。 |
| P0 | setTimeout / setInterval | （組み込み） | マクロタスクスケジューリング。イベントループの基盤（第13章）。 |
| P0 | requestAnimationFrame | （組み込み） | レンダリング同期コールバック。イベントループのステップ4。 |
| P1 | Web Workers | elidex-api-workers | ワーカースレッドごとに別のJSまたはWasmインスタンス。 |
| P1 | WebSocket | elidex-api-ws | リアルタイム通信。 |
| P1 | requestIdleCallback | （組み込み） | 低優先度タスクスケジューリング。React Schedulerが使用。 |
| P1 | Intersection / Resize Observer | elidex-api-observers | MutationObserver同様、ECS変更追跡に自然にマッピング。 |
| P1 | Web Crypto API | elidex-api-crypto | セキュリティ基盤。Rustのring/aws-lc-rsでバッキング。 |
| P1 | CookieStore API | elidex-api-cookies | 非同期、Promiseベースのcookieアクセス。document.cookieのモダン代替。 |
| P1 | Broadcast Channel | elidex-api-broadcast | タブ間通信。マルチプロセスIPCにマッピング。 |
| P2 | IndexedDB | elidex-api-idb | 非同期クライアントサイドストレージ。 |
| P2 | WebGL / WebGPU | elidex-api-gpu | GPUコンピュート。wgpuバックエンドと自然にフィット。 |
| P2 | navigator.sendBeacon | elidex-api-beacon | 信頼性の高いテレメトリ送信。 |
| P3 | Service Workers | elidex-api-sw | オフラインサポート、PWA。 |

### 14.4.2 互換Web API

これらのAPIは互換レイヤーが有効なelidex-browserで利用可能だが、elidex-appコアからは除外される。それぞれモダンな同等物にシムされる：

| API | 互換シム | コア同等物 | 根拠 |
| --- | --- | --- | --- |
| XMLHttpRequest | elidex-api-xhr | Fetch API | コールバックベース、同期モードがメインスレッドをブロック。内部的にFetch経由でシム。 |
| localStorage / sessionStorage | elidex-api-storage-compat | elidex.storage（非同期） | 同期APIがRendererからBrowserプロセスへのブロッキングIPCを要求。14.4.3節参照。 |
| document.cookie | elidex-api-cookies-compat | CookieStore API | 同期的な文字列パースAPI。CookieStore経由でシム。 |

### 14.4.3 ストレージアーキテクチャ

同期ストレージAPI（localStorage、sessionStorage）はelidexのマルチプロセスアーキテクチャと根本的に非互換である。マルチプロセスブラウザでは、これらのAPIはRendererプロセスがBrowserプロセスにブロッキングIPC呼び出しを発行し、応答が到着するまでメインスレッドを停止させる。

Elidexはelidex-app向けにモダンな非同期代替を導入し、ブラウザモードでも利用可能にする：

```rust
// elidex.storage — 非同期KVストレージ（コア、両モードで利用可能）
pub trait AsyncStorage: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn keys(&self) -> Result<Vec<String>>;
}
```

| モード | 同期ストレージ（localStorage） | 非同期ストレージ（elidex.storage） |
| --- | --- | --- |
| elidex-app | 利用不可（コンパイル時除外） | 主要ストレージAPI |
| elidex-browser（コア） | 利用不可 | 利用可能、推奨 |
| elidex-browser（互換） | ブロッキングIPCシム経由で利用可能 | 利用可能 |

これはdocument.write → innerHTMLやgetElementsByClassName → querySelectorAllと同じ原則に従う：同期的なレガシーAPIは互換に存在し、コアはモダンな非ブロッキング代替を提供する。

### 14.4.4 非推奨Web API

| API | ステータス | 備考 |
| --- | --- | --- |
| WebSQL | 未実装 | すでにブラウザから削除済み。IndexedDBを使用。 |
| Application Cache (AppCache) | 未実装 | Service Workersに置換。 |
| document.domain setter | 未実装 | セキュリティリスク。モダンブラウザで削除済み。 |
