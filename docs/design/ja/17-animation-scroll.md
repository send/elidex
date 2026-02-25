
# 17. アニメーション＆スクロールアーキテクチャ

## 17.1 概要

アニメーションはブラウザエンジンで最もパフォーマンスに敏感な領域の一つ。ユーザーはアニメーションジャンク（フレーム落ち）を1フレームの欠落でも知覚する。本章では以下を定義：

- 統一アニメーションモデル（Web Animationsを内部表現として使用）
- イベントループ（第5章）とレンダリングパイプライン（第15章）を橋渡しするFrameProducerコーディネータ
- ECSクエリでアニメーションをtickするAnimationEngine
- コンポジタアニメーション昇格（いつ、どのようにアニメーションがメインスレッドを離れるか）
- スムーススクロールとスクロールアンカリング

コンポジタ側のアニメーションとスクロール（レイヤー変換、スクロールオフセット更新、スクロール物理）は第15章§15.9で定義済み。本章はメインスレッド側をカバー：アニメーションタイミング、補間、ライフサイクル、コンポジタに情報を渡す協調レイヤー。

## 17.2 FrameProducer

### 17.2.1 役割

FrameProducerはイベントループとレンダリングパイプラインの間に位置する。イベントループ（第5章）がフレーム生成の**タイミング**を決定し、FrameProducerがフレーム内で**何が起こるか**を決定する。

```
EventLoop（第5章）                    FrameProducer（第17章）
┌─────────────────────┐              ┌─────────────────────────┐
│ vsync / dirty signal │              │ AnimationEngine.tick()  │
│ drain microtasks     │──produce()──▶│ ObserverRunner.run()    │
│ run rAF callbacks    │              │ RenderingPipeline.run() │
│ (script-side work)   │              │ CompositorChannel.submit│
└─────────────────────┘              └─────────────────────────┘
```

この分離は3つの利点を提供：

1. **責務の明確化**：イベントループはスクリプト実行とタイミングを所有し、FrameProducerはレンダリングオーケストレーションを所有。第5章と第17章の境界がクリーン。
2. **テスタビリティ**：FrameProducerはモックECS Worldと注入タイムスタンプで単体テスト可能。イベントループ不要。
3. **elidex-app柔軟性**：アプリモードアプリケーションはフルブラウザイベントループなしでFrameProducerを直接駆動可能。

### 17.2.2 フレームシーケンス

各フレームはHTML仕様の「update the rendering」ステップに従う。イベントループがスクリプト側のステップを処理し、その後FrameProducerにレンダリングを引き渡す：

```
vsyncシグナル到着
  │
  │  [Event Loop — スクリプト側]
  ├─ 1. マイクロタスクキューのドレイン
  ├─ 2. requestAnimationFrameコールバック実行
  │     └─ JSがDOM/CSSOMを変更する可能性（ScriptSessionにバッファ）
  ├─ 3. ScriptSession flush（変更 → ECS）
  │
  │  [FrameProducer — レンダリング側]
  ├─ 4. AnimationEngine.tick(frame_time)
  │     └─ すべてのアニメーションをサンプリング、ComputedStyle書込またはコンポジタ更新をキュー
  ├─ 5. ResizeObserver通知
  ├─ 6. IntersectionObserver通知
  ├─ 7. StyleSystem.resolve()       ─ 並列（rayon）
  ├─ 8. LayoutSystem.compute()      ─ 並列（rayon）
  ├─ 9. PaintSystem.paint()         ─ レイヤー単位で並列
  ├─10. LayerTree構築/更新
  ├─11. DisplayListをコンポジタに送信（第15章§15.8、第6章）
  │
  └─ フレーム完了。イベントループが次のイベントバッチを再開。
```

ステップ5–6（Observer）はさらなるDOM変更をトリガーしうるため、再フラッシュとアニメーション再tickが必要になりうる。FrameProducerはこの再入ループを上限設定（最大4回、Chromiumと同一）して無限Observerサイクルを防止。

### 17.2.3 インターフェース

```rust
pub struct FrameProducer {
    animation_engine: AnimationEngine,
    observer_runner: ObserverRunner,
    rendering_pipeline: RenderingPipeline,
    compositor_channel: CompositorChannel,
    frame_policy: FramePolicy,           // 第15章§15.8
    frame_timing: FrameTimingRecorder,
}

impl FrameProducer {
    /// スクリプト側ステップ完了後にイベントループから呼び出される。
    pub fn produce(&mut self, world: &mut World, frame_time: FrameTime) {
        // ステップ4: アニメーションtick
        let compositor_updates = self.animation_engine.tick(world, frame_time);

        // ステップ5–6: Observer（最大4回まで再入）
        let mut observer_rounds = 0;
        while self.observer_runner.has_pending(world) && observer_rounds < 4 {
            self.observer_runner.run(world);
            observer_rounds += 1;
        }

        // ステップ7–10: レンダリングパイプライン
        self.rendering_pipeline.run(world);

        // ステップ11: コンポジタへ送信
        self.compositor_channel.submit(world, compositor_updates);

        // タイミング記録
        self.frame_timing.record(world);
    }
}
```

### 17.2.4 elidex-appでの使用

elidex-appでは、ブラウザイベントループなしでFrameProducerを駆動可能：

```rust
// elidex-app: 最小フレームループ
let mut world = World::new();
let mut frame_producer = FrameProducer::new(config);

// OnDemandモード: dirty時のみフレーム生成
loop {
    let event = platform.wait_event();  // 入力またはdirtyまでブロック
    handle_event(&mut world, event);

    if world.resource::<DirtyFlag>().is_set() {
        let frame_time = FrameTime::now();
        frame_producer.produce(&mut world, frame_time);
        world.resource_mut::<DirtyFlag>().clear();
    }
}
```

## 17.3 統一アニメーションモデル

### 17.3.1 内部表現としてのWeb Animations

ElidexはWeb Animationsモデル（WAAPI）を唯一の内部アニメーション表現として使用。CSS TransitionsとCSS Animationsは別エンジンではなく、作成時にWeb Animationsインスタンスに変換される。

```
CSS Transitions                    Web Animations API
  プロパティ変更 ──────────┐           element.animate()
                          ▼                 │
                   ┌───────────────┐         │
                   │  Animation    │◄────────┘
                   │  Instance     │
                   │  (統一)       │
                   ├───────────────┤
                   │  Timeline     │
                   │  Keyframes    │
                   │  Timing       │
                   │  Play state   │
                   └───────┬───────┘
                           │
CSS Animations             │
  @keyframes ──────────────┘
  animation-* プロパティ
```

この統一モデルの意味：

- 1つのアニメーションサンプリングパス、1つのタイミングモデル、1つの補間エンジン。
- `element.getAnimations()`がオリジン（CSSまたはJS）に関係なくすべてのアクティブアニメーションを返す。
- アニメーション合成（同一プロパティを対象とする複数アニメーション）が単一の優先度/合成システムで処理。
- コンポジタ昇格ロジックが1つの入力フォーマットを持つ。

### 17.3.2 ECSコンポーネント

アニメーションデータはECSコンポーネントに格納され、AnimationEngineがECS並列クエリを使用可能：

```rust
/// 1つ以上のアクティブアニメーションを持つエンティティにアタッチ。
pub struct ActiveAnimations {
    pub animations: SmallVec<[AnimationInstance; 4]>,
}

/// 単一アニメーションインスタンス（統一：CSS Transition、CSS Animation、またはWAAPI）。
pub struct AnimationInstance {
    pub id: AnimationId,
    pub origin: AnimationOrigin,
    pub timeline: TimelineRef,
    pub keyframes: KeyframeEffect,
    pub timing: AnimationTiming,
    pub play_state: PlayState,
    pub composite: CompositeOperation,
    /// このアニメーションが対象とするプロパティ
    pub target_property: AnimatableProperty,
    /// trueの場合、コンポジタに昇格済み。
    /// メインスレッドはサンプリングしない。コンポジタが補間を処理。
    pub compositor_promoted: bool,
}

pub enum AnimationOrigin {
    CssTransition { property: CssPropertyId, transition_event_pending: bool },
    CssAnimation { name: AnimationName, iteration_event_pending: bool },
    WebAnimationsApi,
}

pub enum PlayState {
    Idle,
    Running,
    Paused,
    Finished,
}
```

`DocumentTimeline`はECSリソース（ドキュメント/World単位で1つ）：

```rust
/// ECSリソース：ドキュメントのデフォルトタイムライン。
pub struct DocumentTimeline {
    pub current_time: f64,         // ミリ秒
    pub origin_time: Instant,      // ドキュメント作成時
    pub playback_rate: f64,        // 通常1.0
}

impl DocumentTimeline {
    pub fn advance(&mut self, frame_time: Instant) {
        let elapsed = frame_time.duration_since(self.origin_time);
        self.current_time = elapsed.as_secs_f64() * 1000.0 * self.playback_rate;
    }
}
```

### 17.3.3 CSS Transition → Web Animation変換

StyleSystemが`transition`宣言を持つ要素のプロパティ変更を検出した場合、`origin: CssTransition`の`AnimationInstance`を作成：

```rust
impl TransitionHandler {
    pub fn on_style_change(
        &self,
        entity: EntityId,
        property: CssPropertyId,
        old_value: &CssValue,
        new_value: &CssValue,
        transition_def: &TransitionDefinition,
        world: &mut World,
    ) {
        let keyframes = KeyframeEffect {
            frames: vec![
                Keyframe { offset: 0.0, value: old_value.clone() },
                Keyframe { offset: 1.0, value: new_value.clone() },
            ],
        };

        let timing = AnimationTiming {
            duration: transition_def.duration,
            delay: transition_def.delay,
            easing: transition_def.timing_function.clone(),
            iterations: 1.0,
            direction: PlaybackDirection::Normal,
            fill: FillMode::Backwards,
        };

        let instance = AnimationInstance {
            id: AnimationId::new(),
            origin: AnimationOrigin::CssTransition {
                property,
                transition_event_pending: true,
            },
            timeline: TimelineRef::Document,
            keyframes,
            timing,
            play_state: PlayState::Running,
            composite: CompositeOperation::Replace,
            target_property: AnimatableProperty::from(property),
            compositor_promoted: false,
        };

        // ECSに追加 — AnimationEngineが次のtickでピックアップ
        world.get_or_insert_default::<ActiveAnimations>(entity)
            .animations.push(instance);
    }
}
```

### 17.3.4 CSS Animation → Web Animation変換

StyleSystemが要素の`animation-name`を解決した場合、`@keyframes`ルールを検索し`origin: CssAnimation`の`AnimationInstance`を作成：

```rust
impl CssAnimationHandler {
    pub fn on_animation_property(
        &self,
        entity: EntityId,
        animation_props: &CssAnimationProperties,
        keyframes_registry: &KeyframesRegistry,
        world: &mut World,
    ) {
        let Some(keyframes_rule) = keyframes_registry.get(&animation_props.name) else {
            return; // 不明な@keyframes名 — アニメーションなし
        };

        let keyframes = KeyframeEffect {
            frames: keyframes_rule.frames.iter().map(|kf| {
                Keyframe {
                    offset: kf.offset,
                    value: kf.value.clone(),
                }
            }).collect(),
        };

        let timing = AnimationTiming {
            duration: animation_props.duration,
            delay: animation_props.delay,
            easing: animation_props.timing_function.clone(),
            iterations: animation_props.iteration_count,
            direction: animation_props.direction,
            fill: animation_props.fill_mode,
        };

        let instance = AnimationInstance {
            id: AnimationId::new(),
            origin: AnimationOrigin::CssAnimation {
                name: animation_props.name.clone(),
                iteration_event_pending: false,
            },
            timeline: TimelineRef::Document,
            keyframes,
            timing,
            play_state: if animation_props.play_state == CssPlayState::Paused {
                PlayState::Paused
            } else {
                PlayState::Running
            },
            composite: CompositeOperation::Replace,
            target_property: keyframes_rule.target_property(),
            compositor_promoted: false,
        };

        world.get_or_insert_default::<ActiveAnimations>(entity)
            .animations.push(instance);
    }
}
```

### 17.3.5 Web Animations API（スクリプト）

`element.animate()`呼び出しはScriptSessionが処理し、ECSに直接書き込む：

```rust
// ScriptSession内 — JSから呼び出し: element.animate(keyframes, options)
pub fn create_animation(
    &mut self,
    entity: EntityId,
    keyframes: Vec<Keyframe>,
    options: AnimationOptions,
) -> AnimationId {
    let instance = AnimationInstance {
        id: AnimationId::new(),
        origin: AnimationOrigin::WebAnimationsApi,
        // ... keyframesとoptionsから設定
        compositor_promoted: false,
    };

    let id = instance.id;
    self.mutation_buffer.add_animation(entity, instance);
    id
}
```

## 17.4 AnimationEngine

### 17.4.1 Tick

AnimationEngineはECS並列クエリを使用してすべてのアクティブアニメーションをサンプリング：

```rust
pub struct AnimationEngine {
    interpolator: PropertyInterpolator,
    compositor_updates: Vec<CompositorAnimationUpdate>,
}

impl AnimationEngine {
    pub fn tick(
        &mut self,
        world: &mut World,
        frame_time: FrameTime,
    ) -> Vec<CompositorAnimationUpdate> {
        // ドキュメントタイムラインを進める
        world.resource_mut::<DocumentTimeline>().advance(frame_time.instant);

        let timeline_time = world.resource::<DocumentTimeline>().current_time;
        self.compositor_updates.clear();

        // 並列クエリ：各エンティティが独立して処理。
        // 並列クロージャ内での&mut self借用競合を回避するためスレッドローカルアキュムレータを使用。
        let interpolator = &self.interpolator;
        let finish_events: Mutex<Vec<(Entity, AnimationId)>> = Mutex::new(Vec::new());

        world.par_query::<(Entity, &mut ActiveAnimations, &mut ComputedStyle)>()
            .for_each(|(entity, anims, style)| {
                anims.animations.retain_mut(|anim| {
                    if anim.play_state != PlayState::Running {
                        return anim.play_state != PlayState::Idle;
                    }

                    // コンポジタ昇格済みアニメーションはスキップ（コンポジタがサンプリング処理）
                    if anim.compositor_promoted {
                        return true;
                    }

                    // アニメーションをサンプリング
                    let local_time = anim.timing.local_time(timeline_time);
                    match anim.timing.phase(local_time) {
                        AnimationPhase::Before => {
                            if anim.timing.fill == FillMode::Backwards
                                || anim.timing.fill == FillMode::Both
                            {
                                let value = anim.keyframes.sample(0.0, interpolator);
                                style.set(anim.target_property, value);
                            }
                        }
                        AnimationPhase::Active(progress) => {
                            let eased = anim.timing.easing.ease(progress);
                            let value = anim.keyframes.sample(eased, interpolator);
                            style.set(anim.target_property, value);
                        }
                        AnimationPhase::After => {
                            if anim.timing.fill == FillMode::Forwards
                                || anim.timing.fill == FillMode::Both
                            {
                                let value = anim.keyframes.sample(1.0, interpolator);
                                style.set(anim.target_property, value);
                            }
                            anim.play_state = PlayState::Finished;
                            finish_events.lock().unwrap().push((entity, anim.id));
                        }
                        AnimationPhase::Idle => {
                            // アクティブセットから除去
                            return false;
                        }
                    }
                    true
                });
            });

        // 並列クエリ完了後にfinishイベントをディスパッチ
        for (entity, anim_id) in finish_events.into_inner().unwrap() {
            self.queue_finish_event(entity, anim_id);
        }

        std::mem::take(&mut self.compositor_updates)
    }
}
```

### 17.4.2 プロパティ補間

PropertyInterpolatorは各アニメーション可能CSSプロパティの型固有補間を処理：

```rust
pub struct PropertyInterpolator;

impl PropertyInterpolator {
    pub fn interpolate(
        &self,
        property: AnimatableProperty,
        from: &CssValue,
        to: &CssValue,
        progress: f64,
    ) -> CssValue {
        match property {
            // 数値プロパティ：線形補間
            AnimatableProperty::Opacity => {
                let a = from.as_f32();
                let b = to.as_f32();
                CssValue::Float(a + (b - a) * progress as f32)
            }
            // 長さプロパティ
            AnimatableProperty::Width
            | AnimatableProperty::Height
            | AnimatableProperty::MarginTop /* ... */ => {
                let a = from.as_length();
                let b = to.as_length();
                CssValue::Length(a.lerp(&b, progress))
            }
            // 色：Oklabで補間
            AnimatableProperty::Color
            | AnimatableProperty::BackgroundColor => {
                let a = from.as_color().to_oklab();
                let b = to.as_color().to_oklab();
                CssValue::Color(Color::from_oklab(a.lerp(&b, progress)))
            }
            // Transform：分解 → 補間 → 再合成
            AnimatableProperty::Transform => {
                let a = from.as_transform().decompose();
                let b = to.as_transform().decompose();
                CssValue::Transform(a.interpolate(&b, progress).recompose())
            }
            // 離散プロパティ：50%で切り替え
            AnimatableProperty::Display
            | AnimatableProperty::Visibility => {
                if progress < 0.5 { from.clone() } else { to.clone() }
            }
            // ...
        }
    }
}
```

色補間はデフォルトでOklab色空間を使用（CSS Color Level 4）、知覚的に均一な遷移を提供。Transform補間はCSS Transforms Level 2仕様で規定された分解-補間-再合成アプローチを使用。

### 17.4.3 アニメーション合成

複数のアニメーションが同一要素の同一プロパティを対象とする場合、Web Animations合成順序に従って合成：

```rust
pub enum CompositeOperation {
    /// 出力が基礎値を置換
    Replace,
    /// 出力が基礎値に加算
    Add,
    /// 出力が基礎値に蓄積
    Accumulate,
}
```

AnimationEngineは合成順序（CSS Transitions < CSS Animations < WAAPI、次に作成時間順）でアニメーションを処理し合成スタックを適用：

```rust
fn compose_animations(
    anims: &[AnimationInstance],
    base_value: &CssValue,
    property: AnimatableProperty,
    interpolator: &PropertyInterpolator,
) -> CssValue {
    let mut result = base_value.clone();

    // 合成優先度順にソート：transitions、CSS animations、WAAPI
    for anim in anims.iter().filter(|a| a.target_property == property) {
        let sampled = anim.sample(interpolator);
        match anim.composite {
            CompositeOperation::Replace => result = sampled,
            CompositeOperation::Add => result = property.add(&result, &sampled),
            CompositeOperation::Accumulate => result = property.accumulate(&result, &sampled),
        }
    }

    result
}
```

### 17.4.4 タイミング関数

```rust
pub enum TimingFunction {
    Linear,
    CubicBezier(f64, f64, f64, f64),  // 制御点
    Steps(u32, StepPosition),
    // CSS Easing Level 2
    LinearFunction(Vec<LinearStop>),  // linear()とストップ
}
```

組み込み名前付きイージング（`ease`、`ease-in`、`ease-out`、`ease-in-out`）は事前定義の`CubicBezier`値。3次ベジエソルバーはNewton-Raphson反復を使用し、許容誤差は1e-7。

## 17.5 コンポジタ昇格

### 17.5.1 昇格判定

すべてのアニメーションがコンポジタで実行できるわけではない。AnimationEngineは各アニメーションを作成時および再生中に継続的に評価：

```rust
pub fn can_promote_to_compositor(anim: &AnimationInstance) -> PromotionEligibility {
    // コンポジタ合成可能なプロパティのみ（第15章§15.9.1）
    if !anim.target_property.is_compositor_animatable() {
        return PromotionEligibility::Ineligible(Reason::NonCompositableProperty);
    }

    // JSから監視可能な副作用を持つアニメーションはメインスレッドに留まる
    if anim.has_event_listeners_requiring_main_thread() {
        return PromotionEligibility::Eligible { priority: Priority::Low };
    }

    // 複雑なキーフレーム（2超、またはcomposite: add/accumulate）は
    // コンポジタの複雑性制限により昇格の価値が低い可能性
    if anim.keyframes.frames.len() > 8 {
        return PromotionEligibility::Eligible { priority: Priority::Low };
    }

    PromotionEligibility::Eligible { priority: Priority::High }
}
```

コンポジタアニメーション可能プロパティ（第15章§15.9.1より）：`transform`、`opacity`、`filter`（サブセット）。

### 17.5.2 昇格フロー

```
AnimationEngineが昇格可能なアニメーションを検出
  │
  ├─ 対象要素が独自レイヤーを持つことを保証（第15章§15.4.2）
  │   └─ 未昇格の場合：PaintSystemにレイヤー昇格をリクエスト
  │
  ├─ CompositorAnimationUpdateを構築
  │   ├─ 対象レイヤーID
  │   ├─ キーフレーム（コンポジタフレンドリーな形式に事前補間）
  │   ├─ タイミング（duration、delay、easing、iterations）
  │   └─ 現在時刻オフセット
  │
  ├─ CompositorChannel経由でコンポジタに送信
  │
  ├─ アニメーションをcompositor_promoted = trueにマーク
  │   └─ メインスレッドはこのアニメーションのサンプリングを停止
  │
  └─ コンポジタが独立してアニメーション実行（第15章§15.9.1）
```

### 17.5.3 降格（メインスレッドへのフォールバック）

以下の場合にアニメーションがメインスレッドに戻る：

| トリガー | アクション |
| --- | --- |
| JSがアニメーション対象プロパティを変更（`element.style.transform = ...`） | コンポジタアニメーションをキャンセル、メインスレッドが引継ぎ |
| JS経由でアニメーションが一時停止（`animation.pause()`） | 現在状態を転送、メインスレッドで一時停止 |
| 要素が同一プロパティに非合成可能アニメーションを取得 | 正しい合成のため両アニメーションがメインスレッドにフォールバック |
| レイヤー予算超過（第15章§15.4.3） | レイヤー降格、アニメーションがフォールバック |

```rust
pub struct CompositorDemotion {
    pub layer_id: LayerId,
    pub animation_id: AnimationId,
    /// コンポジタが現在の補間値を報告し、
    /// メインスレッドが視覚的なジャンプなく再開できるようにする。
    pub current_value: CssValue,
    pub current_time: f64,
}
```

## 17.6 アニメーションイベント

Web Animations仕様は特定のアニメーションライフサイクルポイントでイベント発火を要求：

| オリジン | イベント | タイミング |
| --- | --- | --- |
| CSS Transition | `transitionrun` | Transition作成（delay開始） |
| CSS Transition | `transitionstart` | アクティブフェーズ開始（delay終了） |
| CSS Transition | `transitionend` | アクティブフェーズ終了 |
| CSS Transition | `transitioncancel` | Transitionキャンセル（要素削除、プロパティ上書き） |
| CSS Animation | `animationstart` | 最初のイテレーション開始 |
| CSS Animation | `animationiteration` | 新イテレーション開始（最初以外） |
| CSS Animation | `animationend` | 全イテレーション完了 |
| CSS Animation | `animationcancel` | Animationキャンセル |
| WAAPI | `finish` | Promiseが解決 / `onfinish`コールバック |
| WAAPI | `cancel` | Promiseが拒否 / `oncancel`コールバック |

イベントはAnimationEngine tick中にキューされ、tick完了後にディスパッチ（サンプリング中ではない）。これにより一貫した状態を保証：イベントハンドラが実行されてDOMを変更する可能性がある前に、すべてのアニメーションがサンプリング済み。

```rust
pub struct AnimationEventQueue {
    events: Vec<PendingAnimationEvent>,
}

pub struct PendingAnimationEvent {
    pub target: EntityId,
    pub event_type: AnimationEventType,
    pub elapsed_time: f64,
    pub animation_name: Option<AnimationName>,  // CSS Animationイベント用
    pub property_name: Option<CssPropertyId>,   // CSS Transitionイベント用
}
```

## 17.7 スクロールアニメーション

### 17.7.1 スムーススクロール

スムーススクロールはコンポジタのスクロールオフセットに対するアニメーションとして実装：

```rust
pub enum ScrollBehavior {
    /// ターゲット位置への即時ジャンプ
    Instant,
    /// イージング付きアニメーションスクロール
    Smooth {
        duration: Duration,
        easing: TimingFunction,
    },
}
```

スムーススクロールリクエストのソース：

| ソース | トリガー |
| --- | --- |
| CSS `scroll-behavior: smooth` | ユーザーがアンカーリンクをクリック、明示的`behavior`なしの`scrollTo()` |
| JS `element.scrollTo({ behavior: 'smooth' })` | 明示的プログラマティックスムーススクロール |
| JS `element.scrollIntoView({ behavior: 'smooth' })` | 要素をビューポートにスクロール |
| ユーザー入力 | `scroll-behavior: smooth`設定時のキーボードスクロール（Page Up/Down、Space） |

スムーススクロールアニメーションはコンポジタ駆動（第15章§15.9.2）。メインスレッドが`SmoothScrollRequest`をコンポジタに送信：

```rust
pub struct SmoothScrollRequest {
    pub target_layer: LayerId,
    pub from_offset: ScrollOffset,
    pub to_offset: ScrollOffset,
    pub duration: Duration,
    pub easing: TimingFunction,
}
```

コンポジタがフレームごとにスクロールオフセットを補間。スムーススクロール中にユーザーが新しいスクロール入力を行った場合、アニメーションが置換（現在位置から新ターゲット）。

### 17.7.2 スクロールスナップ

スクロールスナップポイント（CSS `scroll-snap-type`、`scroll-snap-align`）はスクロール減速中に適用：

```rust
pub struct ScrollSnapConfig {
    pub snap_type: ScrollSnapType,  // none, x, y, both, block, inline
    pub strictness: ScrollSnapStrictness,  // mandatory, proximity
    pub snap_points: Vec<SnapPoint>,
}

pub struct SnapPoint {
    pub position: f64,
    pub alignment: SnapAlignment,  // start, center, end
}
```

コンポジタは以下の時にスナップポイントを評価：
- ユーザースクロール入力終了（指リフト、マウスホイール停止）
- スムーススクロールアニメーション完了
- `mandatory`スナップでプログラマティックスクロールが非スナップ位置をターゲット

`proximity`では、自然な停止位置がスナップポイントのしきい値内にある場合のみスナップ。`mandatory`では常に最近接スナップポイントにスナップ。

### 17.7.3 スクロールアンカリング

スクロールアンカリング（`overflow-anchor`）は、ビューポート上方の要素がサイズ変更した際の視覚的コンテンツジャンプを防止（例：遅延読み込み画像、動的コンテンツ挿入）：

```
レイアウト前：
  ┌─────────────┐ ← ビューポート上端
  │  記事A       │
  │  (100px)     │ ← アンカーノード
  │  記事B       │
  └─────────────┘

ビューポート上方で画像読み込み、コンテンツが200px下方へ押される：

アンカリングなし：           アンカリングあり：
  ┌─────────────┐            ┌─────────────┐
  │  (新しい画像) │           │  (新しい画像) │
  │  記事A       │            │              │ ← スクロールオフセット+200px調整
  │  (100px)     │            ├─────────────┤ ← ビューポート上端
  ├─────────────┤            │  記事A       │
  │  記事B       │ ← ずれた  │  (100px)     │ ← アンカーノードが同じ位置
  └─────────────┘            │  記事B       │
  ユーザーにジャンプ見える!    └─────────────┘
                              視覚的変化なし。
```

実装：

```rust
pub struct ScrollAnchor {
    /// アンカーノード：スクロールコンテナ内の最初の表示要素
    pub anchor_entity: EntityId,
    /// レイアウト前のアンカーノードのスクロールコンテナ上端からのオフセット
    pub anchor_offset_before: f64,
}

impl ScrollAnchoringSystem {
    /// レイアウト後、ペイント前に呼び出される。補償のためスクロールオフセットを調整。
    pub fn adjust(&self, world: &mut World) {
        for (container, anchor, scroll) in
            world.query::<(Entity, &ScrollAnchor, &mut ScrollOffset)>()
        {
            let layout = world.get::<LayoutBox>(anchor.anchor_entity);
            let offset_after = layout.position.y;
            let delta = offset_after - anchor.anchor_offset_before;

            if delta.abs() > 0.5 {  // サブピクセルしきい値
                scroll.y += delta;
            }
        }
    }
}
```

スクロールアンカリングはスクロールコンテナまたはアンカー候補に`overflow-anchor: none`が設定されている場合に抑制。

## 17.8 スクロール連動アニメーション（ScrollTimeline）

CSS Scroll-Linked Animationsはアニメーション進行を時間ではなくスクロール位置で駆動：

```rust
pub enum TimelineRef {
    /// デフォルト：時間ベースのDocumentTimeline
    Document,
    /// スクロール位置で進行駆動
    Scroll(ScrollTimelineConfig),
    /// スクローラ内の要素のビューで進行駆動
    View(ViewTimelineConfig),
}

pub struct ScrollTimelineConfig {
    pub source: ScrollSource,       // nearest, root, または特定要素
    pub axis: ScrollAxis,           // block, inline, x, y
    pub range_start: ScrollOffset,
    pub range_end: ScrollOffset,
}
```

アニメーションのタイムラインが`Scroll`の場合、AnimationEngineはDocumentTimelineではなくスクロール位置から進行を計算：

```rust
fn scroll_progress(config: &ScrollTimelineConfig, world: &World) -> f64 {
    let scroll_offset = get_scroll_offset(config.source, config.axis, world);
    let range = config.range_end - config.range_start;
    if range == 0.0 { return 0.0; }
    ((scroll_offset - config.range_start) / range).clamp(0.0, 1.0)
}
```

スクロール連動アニメーションは対象プロパティがコンポジタアニメーション可能な場合、コンポジタで実行。ジャンクフリーのパララックス、リビールエフェクト、プログレスインジケータを提供。

## 17.9 AnimationEngineライフサイクル管理

### 17.9.1 エンティティ削除

DOM要素削除（`despawn`）時、`ActiveAnimations`コンポーネントはエンティティと共に破棄される。ただしWeb Animations仕様は削除前のcancelイベント発火を要求：

```rust
impl AnimationEngine {
    /// エンティティdespawn前に呼び出し。アクティブアニメーションのcancelイベントをキュー。
    pub fn on_entity_removing(&mut self, entity: EntityId, world: &World) {
        if let Some(anims) = world.get::<ActiveAnimations>(entity) {
            for anim in &anims.animations {
                if anim.play_state == PlayState::Running
                    || anim.play_state == PlayState::Paused
                {
                    self.event_queue.push(PendingAnimationEvent {
                        target: entity,
                        event_type: match &anim.origin {
                            AnimationOrigin::CssTransition { .. } =>
                                AnimationEventType::TransitionCancel,
                            AnimationOrigin::CssAnimation { .. } =>
                                AnimationEventType::AnimationCancel,
                            AnimationOrigin::WebAnimationsApi =>
                                AnimationEventType::Cancel,
                        },
                        elapsed_time: anim.elapsed_time(),
                        animation_name: anim.animation_name(),
                        property_name: anim.property_name(),
                    });
                }

                // コンポジタ昇格済みの場合、コンポジタにキャンセルを送信
                if anim.compositor_promoted {
                    self.compositor_updates.push(
                        CompositorAnimationUpdate::Cancel {
                            animation_id: anim.id,
                        }
                    );
                }
            }
        }
    }
}
```

### 17.9.2 スタイル再計算との相互作用

AnimationEngineがtickして`ComputedStyle`に書き込んだ後、StyleSystemの後続resolve passはアニメーション値を考慮する必要がある。相互作用は以下の順序：

```
AnimationEngine.tick()
  → アニメーション値をComputedStyleに書込
  → 影響エンティティにAnimationDirtyFlagをセット

StyleSystem.resolve()
  → AnimationDirtyFlagありのエンティティ：
      基本スタイル + アニメーション出力 = 最終ComputedStyle
  → フラグなしのエンティティ：
      通常カスケード（アニメーション関与なし）
```

これによりStyleSystemがカスケード中にアニメーション値を上書きすることを回避しつつ、非アニメーションプロパティは通常通り解決。

## 17.10 elidex-appアニメーション

| 側面 | elidex-browser | elidex-app |
| --- | --- | --- |
| FrameProducerドライバ | イベントループ（第5章） | アプリ独自のメインループまたはイベントループ |
| デフォルトFramePolicy | Vsync | OnDemand（第15章§15.8） |
| DocumentTimeline | ドキュメントごとに自動作成 | World初期化時に作成 |
| コンポジタ昇格 | 自動 | 自動（同一ロジック） |
| rAF利用可能性 | あり（HTML仕様） | あり（elidex-app API経由で公開） |
| スクロール連動アニメーション | フルサポート | スクロールコンテナが存在すれば利用可能 |

```rust
// elidex-app: requestAnimationFrame
let app = elidex_app::App::new()
    .frame_policy(FramePolicy::Continuous)
    .build();

// rAF相当
app.request_animation_frame(|frame_time| {
    // ゲーム状態更新
    update_game(frame_time);
});
```

Continuousモードアプリ（ゲーム）では、FrameProducerが毎vsync実行。AnimationEngineが毎フレームtickし、アプリが定義するECSベースのアニメーションにスムーズな補間を提供。
