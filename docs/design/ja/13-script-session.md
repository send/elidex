
# 13. ScriptSession：統一的なScript ↔ ECS境界

スクリプティングレイヤーはブラウザエンジンで最も複雑な部分であり、elidexの「レガシーを切る」哲学が最も大きなインパクトを持つ領域である。設計はエンジン全体で使われる同じ三層パターンを適用する：モダンな標準のためのクリーンなコア、レガシーのためのプラガブル互換レイヤー、そして両方を統合するデュアルディスパッチプラグインシステム。

| レイヤー | コア（モダン） | 互換（レガシー） | 切断境界 |
| --- | --- | --- | --- |
| HTML | HTML5 Living Standard | 非推奨タグ → HTML5 | HTML5仕様 |
| DOM API | DOM Living Standard | ライブコレクション、document.write、レガシーイベント | DOM Living Standard vs レガシーquirks |
| CSSOM | CSSOM Living Standard | （現在なし。将来のcompat用にアーキテクチャ準備済み） | CSSOM仕様 |
| ECMAScript | ES2020+（let/const、arrow、async/await、modules、class） | Annex Bセマンティクス、var quirks、with、arguments.callee | ES2020ベースライン + Annex B境界 |

標準Web APIはスクリプトにオブジェクト指向のビューを提示する（Node → Element → HTMLElement継承、CSSStyleDeclaration、CSSStyleSheet階層）が、elidexの内部はデータ指向（ECSエンティティID + 型付きコンポーネント配列）である。このインピーダンスミスマッチはすべてのObject Model APIに現れる：DOM、CSSOM、および将来のOM（Selection、Range、Performance等）。

各OMレイヤーでこのミスマッチをアドホックに解決するのではなく、elidexはすべてのScript ↔ ECSインタラクションを仲介する統一的なScriptSessionを導入する。ORMのUnit of Work / Sessionパターン（例：SQLAlchemy Session、JPA EntityManager）に類似。

## 13.1 アーキテクチャ

```
Script Engine (JS / Wasm)
       │
       ▼
  ScriptSession  ← Identity Map + Mutation Buffer + GC + Live Queries
       │
       ├── DomApiHandler plugins   (DOM操作、第12章)
       ├── CssomApiHandler plugins (CSSOM操作、第12章)
       ├── 将来のOMプラグイン      (Selection, Range, Performance, ...)
       │
       ▼
   ECS (Entity + Components)
```

セッションは全OMレイヤーが共有する5つのサービスを提供する：

| サービス | 解決する問題 | メカニズム |
| --- | --- | --- |
| Identity Map | `el.style === el.style`がtrueでなければならない。同じ(entity, component)は同じJSラッパーオブジェクトを返す必要がある。 | HashMap<(EntityId, ComponentKind), JsObjectRef>。新しいラッパー作成前にチェック。 |
| Mutation Buffer | スクリプト変更がレンダリングとインターリーブしてはならない。単一スクリプトタスク内のDOMとCSSOM変更がアトミックに見える必要がある。 | Vec<Mutation>がスクリプト実行中のすべての変更を収集。スクリプトステップ間にECSにフラッシュ。 |
| Flush | バッファされた変更のECSコンポーネントへの一括適用。MutationObserverレコードを生成。 | `session.flush(dom)`が全バッファ変更を適用、コンポーネント状態を差分、MutationRecordsを発行。 |
| ライブクエリ管理 | `getElementsByClassName`等のライブコレクションがDOM変更を自動的に反映する必要がある。 | 登録されたライブクエリは各flush後に再評価。スナップショットクエリ（querySelectorAll）は追跡しない。 |
| GC協調 | JSがラッパーオブジェクトをGCしたとき、Identity Mapエントリを削除する必要がある。 | 弱参照またはdrop時の呼び出しクリーンアップ。Session.release(ref)がエントリを除去。 |

## 13.2 ScriptSessionトレイト

```rust
pub trait ScriptSession {
    /// Identity Map：同じ(entity, component)は常に同じJSラッパーを返す
    fn get_or_create_wrapper(
        &mut self,
        entity: EntityId,
        component: ComponentKind,
    ) -> JsObjectRef;

    /// 変更（DOMまたはCSSOM）をバッファし後でフラッシュ
    fn record_mutation(&mut self, mutation: Mutation);

    /// フラッシュ：全バッファ変更をECSに適用、MutationRecordsを返す
    fn flush(&mut self, dom: &mut EcsDom) -> Vec<MutationRecord>;

    /// ライブクエリ登録（例：getElementsByClassNameの結果）
    fn register_live_query(&mut self, query: LiveQuery) -> LiveQueryHandle;

    /// GC通知：Identity Mapからラッパーを除去
    fn release(&mut self, js_ref: JsObjectRef);
}

pub enum Mutation {
    // DOM変更
    SetAttribute(EntityId, String, String),
    AppendChild(EntityId, EntityId),
    RemoveChild(EntityId, EntityId),
    SetInnerHtml(EntityId, String),
    // CSSOM変更
    SetInlineStyle(EntityId, String, String),    // entity, property, value
    InsertCssRule(EntityId, usize, String),       // stylesheet entity, index, rule text
    DeleteCssRule(EntityId, usize),               // stylesheet entity, index
    // 将来のOM変更
    // SetSelection(...), etc.
}
```

## 13.3 イベントループ統合

イベントループはスクリプト実行、セッションフラッシュ、レンダリングをインターリーブする中央シーケンサーである。ScriptSessionによりフラッシュポイントが明示的になる：

```rust
loop {
    // 1. 任意のタスクソースから最古のマクロタスクを実行：
    //    - イベントハンドラ（click, input, keyboard）
    //    - setTimeout / setInterval コールバック
    //    - MessagePort / postMessage
    //    - Fetchレスポンスハンドラ
    let task = task_queue.pop();
    script_engine.eval(task);

    // 2. 全マイクロタスクをドレイン（Promise.then、MutationObserverコールバック、
    //    queueMicrotask）
    while let Some(microtask) = microtask_queue.pop() {
        script_engine.eval(microtask);
    }

    // 3. セッションフラッシュ：全バッファDOM/CSSOM変更をECSに適用
    let mutation_records = session.flush(&mut dom);
    // MutationObserverコールバックを配信（さらにマイクロタスクをトリガーする可能性）
    deliver_mutation_observers(mutation_records);
    drain_microtasks();

    // 4. レンダリング機会がある場合：requestAnimationFrameコールバックを実行
    if vsync_ready() {
        for cb in animation_frame_callbacks.drain(..) {
            script_engine.eval(cb);
        }
        drain_microtasks();
        session.flush(&mut dom);  // rAF変更をフラッシュ
    }

    // 5. 必要ならレンダー
    if dom.has_pending_style_invalidations() {
        run_style_system();
        run_layout_system();
        let display_list = run_paint_system();
        compositor_channel.send(CompositorMsg::SubmitDisplayList(display_list));
    }

    // 6. アイドル時間が残っている場合：requestIdleCallbackコールバックを実行
    if has_idle_time() {
        for cb in idle_callbacks.drain(..) {
            script_engine.eval(cb);
        }
    }
}
```

このループはJSイベントループのセマンティクス（ステップ1〜6）を示す。外部イベント収集（IPC、I/O）と待機フェーズを含む完全な統合Rendererイベントループは第5章5.4.2節で定義。この順序はHTML Living Standardで規定されており、正確に実装する必要がある。変更はスクリプト実行中にセッションバッファに蓄積され、明確に定義されたポイント（ステップ3と4）でECSにアトミックに適用されるため、レンダリングパイプラインは常に一貫した状態を見る。変更がDOM APIから発生したかCSSOMから発生したかに関係なく、MutationObserverは一貫した順序付きレコードを受け取る。
