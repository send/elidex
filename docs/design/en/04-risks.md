
# 4. Risks and Mitigations

## 4.1 Technical Risks

| Risk | Severity | Mitigation |
| --- | --- | --- |
| Web compatibility too low for practical browser use | High | Compat layer (Ch. 7) is the escape hatch. Crawler data drives prioritization. Bar is "works well enough for modern sites," not "passes all WPT." |
| Self-built JS engine maturity | High | Staged approach (parser → interpreter → IC → JIT). SpiderMonkey remains as fallback throughout Phase 1–3. elidex-app can adopt elidex-js earlier (lighter workloads). ES2020+-only core omits Annex B, simplifying substantially. |
| DOM API completeness for real-world sites | Medium | Modern frameworks (React, Vue, Svelte) use a smaller DOM API surface than raw JS. Legacy APIs implemented on-demand based on breakage data from crawler. |
| Vello immaturity for production browser use | Medium | Vello is under active development by the Linebender project. Contact surface is isolated (Ch. 15 §15.6, ADR #26) — all upstream code has zero Vello dependency. Software fallback (Vello CPU backend) ensures rendering always works. |
| Plugin system performance overhead | Medium | Static dispatch (generics/enums) at compile time, not dynamic dispatch (trait objects) in hot paths. Plugins resolved at build time. |
| ECS memory model mismatch with DOM semantics | Medium | Not all relationships are 1:1 entity-component (e.g., LayerTree is independent, ADR #27). Hybrid approach: ECS for DOM + independent structures where N:M relationships exist. |
| Codec patent and licensing exposure | Medium | Patent-encumbered codecs (H.264, H.265, AAC) delegate to platform decoders only (ADR #34). No patent-encumbered code in elidex distribution. |

## 4.2 Scope and Resource Risks

| Risk | Severity | Mitigation |
| --- | --- | --- |
| Scope creep from web platform complexity | High | Plugin architecture forces each feature to be a discrete, removable unit. Strict phase gates prevent adding features before foundations are solid. |
| Single-developer bottleneck | High | Modular crate structure enables parallel development if contributors join. Focus on unique value (core engine) and leverage existing crates (rustybuzz, cssparser, html5ever, mozjs). |
| WebRTC complexity | Medium | WebRTC full implementation deferred (Ch. 20 §20.11). Interface defined for future integration. Can use existing Rust WebRTC libraries (webrtc-rs) when ready. |
| DRM/Widevine licensing | Medium | EME API defined with CDM plugin slot (Ch. 20 §20.8). v1 ships without CDM. Architectural readiness ensures no redesign when licensing is obtained. |

## 4.3 Ecosystem Risks

| Risk | Severity | Mitigation |
| --- | --- | --- |
| Key Rust crate unmaintained | Medium | Critical dependencies (rustybuzz, wgpu, Vello, AccessKit) are actively maintained by established teams. Trait abstraction pattern (ADR #31, #34) enables swapping implementations. |
| Platform API changes | Low | Platform Abstraction layer (Ch. 23) isolates platform specifics. Changes affect adapters only. |
| Browser engine consolidation pressure | Medium | Elidex's dual-use model (browser + app runtime) provides value even if browser market share is small. elidex-app competes with Electron/Tauri, not just Chrome. |

## 4.4 Risk Monitoring

Each risk has an assigned owner and is reviewed at each phase gate. Severity levels are reassessed as the project progresses. Emerging risks from implementation experience are added to this registry.
