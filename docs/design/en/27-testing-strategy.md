
# 27. Testing Strategy

## 27.1 Overview

Testing elidex requires a multi-layered approach: standards conformance (WPT), performance benchmarks, fuzz testing for parser/codec security, integration tests for cross-process coordination, and visual regression tests for rendering correctness.

## 27.2 Web Platform Tests (WPT)

WPT is the industry-standard conformance suite with tens of thousands of test cases.

### 27.2.1 Per-Plugin WPT Mapping

Each plugin crate declares the WPT test IDs it is responsible for. Adding a plugin automatically adds its tests to CI. Removing a plugin shows exactly which WPT tests will fail.

### 27.2.2 Subset Tracking

Elidex does not aim for 100% WPT pass rate. The target subset is explicitly defined and tracked:

| Category | Target | Notes |
| --- | --- | --- |
| HTML parsing | >95% | Core + compat parser combined |
| CSS (supported properties) | >90% | Only properties in elidex's CSS property registry |
| DOM API (core) | >85% | querySelector, mutation, events |
| Fetch API | >90% | Standard HTTP, streaming |
| Web Animations | >80% | WAAPI unified model |
| Canvas 2D | >80% | Vello-backed |
| Compat-layer APIs | >70% | Best-effort for legacy APIs |

### 27.2.3 CI Integration

WPT runs on every PR. Any newly failing test in the tracked subset blocks merge unless the failure is in a deliberately unsupported area. A dashboard tracks pass rate trends over time.

## 27.3 Performance Benchmarks

Automated benchmarks compare against Chromium and Firefox:

| Metric | Benchmark | Target |
| --- | --- | --- |
| Parsing throughput | HTML/CSS parse time (standardized docs) | Within 80% of Chromium |
| Style resolution | Large DOM tree style computation | Faster than Chromium (parallel) |
| Layout | Representative pages (news, app, table) | Competitive with Chromium |
| First paint | URL → first pixel | <200ms warm, <500ms cold |
| Memory footprint | Per-tab peak and steady-state | <50% of Chromium (ECS advantage) |
| Binary size | Browser and app configurations | <30MB (browser), <15MB (app minimal) |

### 27.3.1 Benchmark Infrastructure

```rust
#[bench]
fn style_resolution_1000_nodes(b: &mut Bencher) {
    let world = create_test_dom(1000);
    b.iter(|| {
        StyleSystem::compute(&world);
    });
}
```

Benchmarks use `criterion` for statistical rigor (warm-up, iterations, confidence intervals). Results are tracked in CI with regression detection (>5% regression blocks merge).

## 27.4 Fuzz Testing

Security-critical parsers are fuzz-tested continuously:

| Target | Fuzzer | Corpus |
| --- | --- | --- |
| HTML parser | cargo-fuzz (libFuzzer) | Crawled web pages, WPT fixtures |
| CSS parser | cargo-fuzz | CSS from top 10k sites |
| SVG parser | cargo-fuzz | SVG files from web + crafted edge cases |
| Image decoders | cargo-fuzz | Malformed images (PNG, JPEG, WebP, AVIF) |
| Media demuxers | cargo-fuzz | Truncated/corrupted media files |
| URL parser | cargo-fuzz | RFC edge cases, IDN, punycode |
| HTTP header parser | cargo-fuzz | Malformed headers |

Fuzzers run on CI infrastructure 24/7. Crashes are automatically triaged and filed.

## 27.5 Visual Regression Testing

Rendering correctness is verified by screenshot comparison:

```
Test case (HTML + CSS)
  → elidex renders to headless bitmap (Vello CPU backend)
  → Compare pixel-by-pixel against reference image
  → Diff exceeding threshold → flag for review
```

Reference images are committed to the repository. Platform-specific rendering differences (font hinting, subpixel rendering) are handled with per-platform reference sets and a tolerance threshold.

## 27.6 Integration Tests

Cross-process coordination tests:

| Area | Test Approach |
| --- | --- |
| Navigation (Ch. 9) | Headless browser loads pages, verifies document state transitions |
| IPC (Ch. 5) | Spawn Renderer + Browser processes, verify message protocol |
| bfcache | Navigate away and back, verify DOM state preserved |
| Permission prompts | Simulate prompt responses via test harness |
| Media playback | Load test media files, verify A/V sync within tolerance |
| OPFS | Write/read via SyncAccessHandle, verify data integrity |
| Web fonts | Load WOFF2, verify glyph rendering matches reference |

## 27.7 Unit Tests

Every crate has unit tests for its internal logic. The ECS architecture makes unit testing straightforward: create a minimal World with the components needed, run the system, and assert the output.

```rust
#[test]
fn css_cascade_specificity() {
    let mut world = World::new();
    // Create a <div class="foo"> with two matching rules
    let entity = world.spawn((TagType::Div, Attributes::new(), ...));
    // Add style rules with known specificity
    StyleSystem::compute(&world);
    let style = world.get::<ComputedStyle>(entity).unwrap();
    assert_eq!(style.color, Color::RED); // higher-specificity rule wins
}
```

## 27.8 elidex-app Testing

The Embedding API (Ch. 26) provides a headless mode specifically for testing:

```rust
#[test]
fn app_loads_content() {
    let engine = Engine::builder()
        .with_config(EngineConfig { process_mode: ProcessMode::SingleProcess, .. })
        .build().unwrap();

    let view = engine.create_view(ViewConfig {
        content: ViewContent::Html("<h1>Hello</h1>".into()),
        surface: SurfaceConfig::Headless { width: 800, height: 600 },
        ..Default::default()
    });

    // Wait for load
    let result = view.evaluate_script("document.querySelector('h1').textContent").await;
    assert_eq!(result.unwrap().as_str(), "Hello");
}
```
