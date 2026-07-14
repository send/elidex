# VM `Date` builtin — ECMA-262 §21.4 Date Objects

Status: in-progress (branch `vm-date-builtin`)
Slot: `#11-vm-date-builtin` (post-boa-deletion paydown campaign)

## 背景

post-flip (#457) の renderer は native elidex-js VM を使う。`Date` builtin が VM に未実装
（`natives_date` 無し / `Date` prototype 無し / `"Date"` global 無し / `ObjectKind::Date` 無し）で、
`new Date()` / `Date.now()` が **ReferenceError** になり実サイトを破壊する。`Date` は
ES2020 baseline の core surface — compat/deprecated ではなく **clean core** に属する。

## Scope (§21.4 spec-mandated surface, full)

- **Constructor** (§21.4.2): `new Date()`=now / `new Date(ms)` / `new Date(isoString)` /
  `new Date(y,m,d,h,mi,s,ms)`（2〜7 引数）/ `Date()` as-function → 現在時刻の string。
- **Static** (§21.4.3): `Date.now()` / `Date.parse(str)` / `Date.UTC(y,m,...)`。
- **Prototype** (§21.4.4):
  - `getTime` / `valueOf`
  - `get{FullYear,Month,Date,Day,Hours,Minutes,Seconds,Milliseconds}` + `getUTC*` 版 + `getTimezoneOffset`
  - `set{Time,Milliseconds,Seconds,Minutes,Hours,Date,Month,FullYear}` + `setUTC*` 版
  - `toISOString` / `toJSON` / `toString` / `toDateString` / `toTimeString` / `toUTCString`
  - `[Symbol.toPrimitive]`（`"default"`→string, `"string"`→string, `"number"`→number）
- **internal slot** `[[DateValue]]` = ms since epoch（invalid = `NaN`）。`ObjectKind::Date(f64)` で保持
  （NumberWrapper と同型の internal-slot；side-table は CLAUDE.md ECS-native/side-store 判定で反則）。
- **system time** = 既存 `now_epoch_ms`（`SystemTime::now() - UNIX_EPOCH`, integer-ms）を再利用
  （One-issue-one-way、複製 NG）。

## Scoping 判断 (lens で確定)

### timezone → **UTC-baseline** (full local-tz は follow-up)
`CLAUDE.md intl-icu-deferral`（ICU/tz-database dep 禁止）ゆえ、正確な local timezone を持てない。
baseline は **UTC-baseline**: `getTimezoneOffset()` = `+0`、local `get*`/`set*` == UTC `get*`/`set*`
（`LocalTime(t)` / `UTC(t)` AO は identity）。実行環境の tz に依存せず **決定的**。
これは spec の local-time 解釈からの意図的逸脱で、`Date` の観測可能な値が host tz に依らないことを
保証する documented baseline。full local-tz は tz-db 解禁後の follow-up slot。

### Date.parse → **ISO 8601 (§21.4.1.32) full + toString round-trip** (非 ISO は follow-up)
Date Time String Format（`YYYY-MM-DDTHH:mm:ss.sssZ`, date-only / expanded-year `±YYYYYY` 含む）は
**spec-mandated** ゆえ full 実装。加えて自前 `toString` 出力の round-trip を保証。
非 ISO の implementation-defined format（RFC 2822 風 `"Mon Jan 01 2020 ..."` 等）は
implementation-defined ゆえ bounded surface として follow-up slot に carve。

### Annex B → **除外**
`getYear` / `setYear` / `toGMTString` は Annex B（`policy-deprecated-and-annex-b-out-of-scope`）
ゆえ実装しない。`toLocaleString` 系は Intl 依存（`intl-icu-deferral`）ゆえ follow-up。

## 設計

### Module 構成 (`vm/natives_date/`, cohesion seam で分割)
`natives_object/` 前例に倣い dir 化。§21.4 は algorithm / parse / format / natives の明確な seam を持つ:
- `mod.rs` — constructor / statics / prototype native fn + `register_date_prototype` + `now_epoch_ms`
- `algorithms.rs` — §21.4.1 の pure AO（time-value 分解/合成 + `MakeTime`/`MakeDay`/`MakeDate`/`TimeClip`）
- `parse.rs` — Date Time String Format parser
- `format.rs` — `toISOString` / `toString` / `toDateString` 等の文字列生成

### Canonical AO (§21.4.1, webref 確認済)
- `Day` §21.4.1.3 / `TimeWithinDay` §21.4.1.4 / `DayFromYear` §21.4.1.6 /
  `YearFromTime` §21.4.1.8 / `MonthFromTime` §21.4.1.11 / `DateFromTime` §21.4.1.12 /
  `WeekDay` §21.4.1.13 / `HourFromTime` §21.4.1.14 …
- `MakeTime` §21.4.1.27 / `MakeDay` §21.4.1.28 / `MakeDate` §21.4.1.29 / `TimeClip` §21.4.1.31
- `LocalTime` §21.4.1.25 / `UTC` §21.4.1.26 → UTC-baseline では identity

### set* の mutation
`ObjectKind::Date(f64)` を in-place で書き換え（`set*` は既存 time-value を分解 → 該当 field 差替 →
`MakeDate`/`MakeTime`/`TimeClip` で再合成 → kind 書き戻し）。

### 配線
`register_number_prototype` パターン: `create_object_with_methods` で proto、
`create_constructable_function("Date", ...)` で ctor、`wire_constructor_global`、statics install。
`VmInner.date_prototype: Option<ObjectId>` 追加（mod.rs / init.rs default / gc/collect.rs root）。
`ops_property.rs` の `JsValue::Object` proto lookup は `object.prototype` field 経由なので追加不要。

## host/ touch (PM 承認済 2026-07-13, spec-faithful)

design lens で touch の正しさは収束、coordination 承認取得済み。L3(DOM-form)/L4(history) の
触るファイルと disjoint（衝突リスク実質ゼロ）:
1. **`host/structured_clone.rs`** — `ObjectKind::Date` で `classify` の exhaustive match が壊れ
   arm 追加が**コンパイル不可避**。`Date` は [Serializable] ゆえ spec-faithful に clone（新 Date、same `[[DateValue]]`）。
2. **`host/file.rs`** — `now_epoch_ms`（system time source）を `natives_date` へ canonical 移設し
   file.rs はそれを呼ぶ（複製回避 = One-issue-one-way）。
3. **`host/indexeddb/value.rs`** — **1 の帰結として semantic に必然**（初版 plan の列挙漏れ。converge
   中に Date key/value 対応として過剰に膨らみ、design re-gate Axis 1 = CRIT×2 を受けて下記まで最小化）。
   `structured_clone.rs` が `Date` を [Serializable] にした瞬間、IDB の `clone_value` は Date を通す
   ようになり、`reject_non_json_storable` の `_ => Kind::Leaf` に落ちて **JSON backend が黙って ISO
   string 化**する（stored-then-read が String になる silent type change。その `_` arm の comment 自身が
   「ここに来るのは leaf か unclonable だけ」と書いており、cloneable な Date の到達は想定外）。既存の
   cloneable-but-not-JSON kind (Error / RegExp / Blob / ArrayBuffer / TypedArray / DataView) と **同一
   stance** で `Kind::Reject("a Date")` を 1 arm 追加（+ module note 更新）。
   **key route（`IdbKey::Date` の生成 / 読み戻し）は入れない** → carve（下記 "IDB × Date"）。
4. **`natives_json.rs` (VM core) + `host/worker_scope.rs` + `host/structured_serialize.rs`** —
   **3 と同根**（1 の帰結。Codex R4 が指摘、初版 plan の列挙漏れ）。`stringify_to_string` を core に
   する JSON-shortcut serializer は全部で 4 つ — IDB `value_to_json` / worker+SW `serialize_message` /
   history `structured_serialize_for_storage` / `Response.json()`。うち **`Response.json()` は真の JSON
   serialization** ゆえ Date→ISO string が spec 通り正しく（over-fix しない）、IDB は 3 で塞がる。残る
   **worker / history が silent Date→String corruption**: `toJSON` は throw せず ISO string を返すので
   BigInt / cyclic のような **loud failure path に乗らない**（既存 docstring は "BigInt / cyclic / Map /
   Date … `JSON.stringify` throws" と書いていたが Date について **事実誤認** — Date builtin が無く到達不能
   だった頃の推測。R4 で訂正。`ObjectKind` に Map は存在しない）。

   **検出は encoder の walk の *内側* に置く**（R4 の pre-scan walker は R5 で mis-designed と判明 →
   撤去）。`natives_json::stringify_for_structured_shortcut` = `JsonSerializer` に `reject_date` mode を
   足し、`SerializeJSONProperty` が観測する値に対して **step 2 (`toJSON` hook) の直前** に Date を fail
   させる。walk の外の pre-scan は **traversal を複製する**ので必ずズレる — Codex R5 が 3 つとも突いた:
   (a) **accessor 経由の Date を見られない**（getter 呼び出しは observable side effect ゆえ pre-scan は
   skip せざるを得ない → ISO string が素通り）、(b) **depth cap が無い**（encoder の `MAX_JSON_DEPTH` に
   到達する前に Rust stack を溢れさせ得る）、(c) **user 例外の順序を壊す**（user code を1行も走らせずに
   `DataCloneError` を返し、throwing `toJSON` / getter の propagate contract に違反）。in-walk なら
   3 つとも**構造的に**消える（getter の戻り値を見る / 既存 depth cap が先に効く / JSON の walk 順が保たれる）。
   walk は1つ = One-issue-one-way。

   failure は非-`ThrowValue` な `VmError` なので、**各 serializer の既存 error 分岐がそのまま効く**
   (呼び出し側の変更ゼロ): worker → `DataCloneError`（cross-thread ゆえ degrade 先なし / 既存の "cannot
   represent" mapping）、history → `None` に degrade（CR-3 = browsers が受ける `pushState` を throw に
   変えない。§7.4.4 が serialized snapshot から復元するので `history.state` は同期的に `null` = ISO
   string には決してならない）。
   → faithful encoding は既存 slot `#11-worker-structured-serialize` /
   `#11-history-state-structured-serialize-fidelity`（両者が full walker を wholesale で所有）。

`gc/trace.rs` の arm 追加は OWN 圏（payload `f64` ゆえ trace no-op）。

### 1000-line debt (touch-time、**follow-up standalone PR**)

CLAUDE.md 上 split は **単独 PR / 単独 commit**（feature PR に bundle 禁止）ゆえ本 PR に含めない。本 PR
land 直後に standalone split PR で回収する（defer でなく sequencing）。対象 = 本 PR が触った 1000+ file:

- **`tests/tests_indexeddb.rs`** (~1090行) — Codex R3 指摘。本 PR の追加は carve 後 ~28行 (Axis 5 の
  >50 LoC 閾値未満) だが file 自体は debt。
- **`natives_json.rs`** (1030 → 1097行、+67 = Axis 5 該当) — Codex R5 の in-walk 化で touch。**stringify /
  parse が明確な cohesion seam**（`JsonSerializer` 系 vs `JsonParser` 系）→ 2 module に割る。

`tests/tests_worker.rs` は R5 で 1002行に達したため、worker の Date serialization case を
**`tests/tests_json_shortcut_date.rs`** に切り出して 947行に戻した（本 PR 内で解消済）。

## Follow-up slots (carve、PM へ報告)

- `#11-vm-date-local-timezone` — full local-tz（system tz 取得 + `getTimezoneOffset` 実値 +
  local `get*`/`set*`）。tz-database dep or OS tz 取得の設計判断待ち（`intl-icu-deferral` 傘下）。
- `#11-vm-date-parse-nonstandard-formats` — 非 ISO implementation-defined parse（RFC 2822 風等）。
- `#11-vm-date-tolocalestring-intl` — `toLocaleString`/`toLocaleDateString`/`toLocaleTimeString`
  （Intl 依存、`intl-icu-deferral` 傘下）。

### code-review 由来 (VM 共通機構 = 構造的分離で別 PR、Date が driver)

**✅ slot 不要 (converge R1 で in-PR 解消)**: `#11-vm-ordinary-to-primitive-seam` (F2/F3) — Codex R1 が
「defer せず今直せ」と正しく指摘。`ops.rs` の `to_primitive` から `ordinary_to_primitive` (§7.1.1.1) を
helper 抽出し、`Date.prototype[Symbol.toPrimitive]` (§21.4.4.45 step 6) / `toJSON` (§21.4.4.37 steps
1-4) をそこへ委譲済み。user の `valueOf`/`toString`/`toISOString` override が honored。**slot 抹消**。

- `#11-object-prototype-tostring-builtin-tag` (F4) — §20.1.3.6 builtinTag。**Date arm (step 12) は
  converge R1 で in-PR 解消**。残 gap = primitive wrapper (NumberData / StringData / BooleanData =
  steps 9-11) が step-14 の "Object" default に落ちる件。**Why defer**: wrapper 3 種の一貫実装は Date
  と無関係な VM 共通機構で、Date builtin の scope 外。**Re-eval trigger**:
  `#11-vm-native-fn-generic-invocation` 着手時（JS observable `Object.prototype.toString.call(x)` は
  両者が揃って初めて通るので、同じ PR で end-to-end 検証できる）。
- `#11-vm-clock-injection` (F11) — `now_epoch_ms` は bare `SystemTime::now()`。VmInner-owned
  clock（test determinism / virtual time / Date.now 分解能低減）の seam 化（`start_instant` 前例）。
  **Why defer**: clock injection は VM-wide の seam（`Performance.now` / timer / `File.lastModified`
  も同 clock を引く）で、Date builtin 単体より広い。**Re-eval trigger**: timer / event-loop の
  determinism が要る test harness 作業、または `#11-keepalive-event-loop-step1-snapshot` 着手時。
- `#11-vm-date-decompose-perf` (F12) — getter/formatter/setter が `year_from_time` の補正ループを
  getDate=4×/toString=7× 再計算。`decompose(t)->{year,month,date,…}` helper で1回に集約。

### external-converge (Codex R1-R3) 由来

- `#11-vm-native-fn-generic-invocation` — **既存 VM bug (Date 無関係、Codex R1 中に発見)**。
  `Object.prototype.toString` (等の native fn) を **generic invoke** (own-property に assign して
  呼ぶ / `.call` / `.apply`) すると **全 receiver** で `"Cannot convert undefined or null to
  object"` を throw する。interpreter の inherited-method fast-path (`({}).toString()`) のみ動く。
  Codex R1 #4 の `Object.prototype.toString` builtin-tag Date arm (§20.1.3.6 step 12) は code-fix 済だが、
  この既存 bug が JS observable (`Object.prototype.toString.call(new Date())`) を阻む。
  **Why defer**: 壊れているのは native-fn の **call dispatch** (`.call` / `.apply` / own-property
  assign 経由) — 本 PR が一切触らない VM core 機構で、Date builtin と独立に既存 regression。ここで
  直すと blast radius が call dispatch 全体（全 native fn × 全 receiver）に広がり、Date の spec
  surface と混ざる。**Re-eval trigger**: VM の native-fn call dispatch を次に触る PR、または
  `#11-object-prototype-tostring-builtin-tag` 着手時（両者を揃えて初めて
  `Object.prototype.toString.call(x)` が end-to-end で通る）。

### IDB × Date — carve (Codex R2/R3 + design re-gate Axis 1 CRIT×2)

converge 中に `host/indexeddb/value.rs` へ Date **key** 対応 (`IdbKey::Date` の生成 / 読み戻し) を
入れたが、design re-gate (Axis 1 Layering) で **CRIT×2** → **revert して既存 slot に carve**。
value 側の reject 1 arm のみ残す（"host/ touch" §3 参照）。

- **Date key** (`IdbKey::Date`) → 既存 slot **`#11-idb-binary-key`**。**Why defer**: backend の key
  route（inline key-path 抽出 / index key / cursor `update()` の key 再検証 / auto-increment
  injection）は全て `util::json_to_idb_key` / `idb_key_to_json` を通り、**構造的に `IdbKey::Date` を
  生成・保持できない**。VM host 側だけ有効化すると explicit-key route (`add(v,k)` / KeyRange / `cmp`)
  でしか動かない **半端 support** になる（Codex が R2「Date value を reject しろ」→ R3「reject するな、
  inline Date key path が壊れる」と矛盾できたのは、**どちらの経路も VM host からは到達不能**で、
  欠けているピースが backend にあったから）。**Re-eval trigger**: `elidex-indexeddb` の `key.rs` が
  `IdbKey::Date` / `IdbKey::Binary` を全 route で round-trip できるようになった時。それまで Date key は
  `DataError`（test `date_is_not_a_valid_key` が contract を lock）。
- **Date value** → 既存 slot **`#11-idb-structured-clone-storage`**。**Why defer**: JSON backend は
  `toJSON` 経由で Date を ISO string に落とすので stored-then-read が String になる。faithful な
  round-trip には storage format 自体の入れ替えが要る（`value.rs` の `-0` / `undefined` / `NaN` /
  `BigInt` 拒否と同根の制約）。それまでは既存の cloneable-but-not-JSON kind と同一 stance で
  `DataCloneError` を upfront throw（test `add_date_value_throws_data_clone_error`）。
  **Re-eval trigger**: `#11-idb-structured-clone-storage`（JSON → structured-clone storage）着手時。

## Test (engine-independent unit)
`tests/tests_date_api.rs`: ctor 4 forms / `get*` / `set*` in-place / `Date.parse` ISO /
format round-trip / `Symbol.toPrimitive` / structuredClone(Date) / invalid→NaN / TimeClip ±8.64e15 境界 /
`Date.UTC` / `Date.now` monotonic。
