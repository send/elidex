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

`gc/trace.rs` の arm 追加は OWN 圏（payload `f64` ゆえ trace no-op）。

## Follow-up slots (carve、PM へ報告)

- `#11-vm-date-local-timezone` — full local-tz（system tz 取得 + `getTimezoneOffset` 実値 +
  local `get*`/`set*`）。tz-database dep or OS tz 取得の設計判断待ち（`intl-icu-deferral` 傘下）。
- `#11-vm-date-parse-nonstandard-formats` — 非 ISO implementation-defined parse（RFC 2822 風等）。
- `#11-vm-date-tolocalestring-intl` — `toLocaleString`/`toLocaleDateString`/`toLocaleTimeString`
  （Intl 依存、`intl-icu-deferral` 傘下）。

### code-review 由来 (VM 共通機構 = 構造的分離で別 PR、Date が driver)

- `#11-vm-ordinary-to-primitive-seam` (F2/F3) — `Date.prototype[Symbol.toPrimitive]` / `toJSON`
  は現状 `[[DateValue]]` を直接読み `OrdinaryToPrimitive` / `Invoke(toISOString)` を bypass →
  user の `valueOf`/`toString`/`toISOString` override と `toJSON` の generic 性を無視。`ops.rs`
  の `to_primitive`（@@toPrimitive 再チェックで recurse）から `ordinary_to_primitive` helper を
  抽出し、Date/Number/String wrapper 共通で spec-faithful に。VM 共通機構ゆえ別 PR。
- `#11-object-prototype-tostring-builtin-tag` (F4) — §20.1.3.6 builtinTag
  (`[[DateValue]]`→"Date" / NumberData→"Number" 等)。`Object.prototype.toString.call(new Date())`
  が "[object Object]"（lodash `isDate` 等の cross-realm brand-check 破綻）。既存
  Number/String/Boolean wrapper も同 gap = 一貫実装は共通機構ゆえ別 PR。
- `#11-vm-clock-injection` (F11) — `now_epoch_ms` は bare `SystemTime::now()`。VmInner-owned
  clock（test determinism / virtual time / Date.now 分解能低減）の seam 化（`start_instant` 前例）。
- `#11-vm-date-decompose-perf` (F12) — getter/formatter/setter が `year_from_time` の補正ループを
  getDate=4×/toString=7× 再計算。`decompose(t)->{year,month,date,…}` helper で1回に集約。

### external-converge (Codex R1) 由来

- `#11-vm-native-fn-generic-invocation` — **既存 VM bug (Date 無関係、Codex R1 中に発見)**。
  `Object.prototype.toString` (等の native fn) を **generic invoke** (own-property に assign して
  呼ぶ / `.call` / `.apply`) すると **全 receiver** で `"Cannot convert undefined or null to
  object"` を throw する。interpreter の inherited-method fast-path (`({}).toString()`) のみ動く。
  Codex R1 #4 の `Object.prototype.toString` builtin-tag Date arm (§20.1.3.6) は code-fix 済だが、
  この既存 bug が JS observable (`Object.prototype.toString.call(new Date())`) を阻む。call
  dispatch は本 PR 未変更ゆえ既存 regression。VM core (native-fn generic dispatch)、Date scope 外 → 別 PR。

## Test (engine-independent unit)
`tests/tests_date_api.rs`: ctor 4 forms / `get*` / `set*` in-place / `Date.parse` ISO /
format round-trip / `Symbol.toPrimitive` / structuredClone(Date) / invalid→NaN / TimeClip ±8.64e15 境界 /
`Date.UTC` / `Date.now` monotonic。
