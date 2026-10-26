//! EH-attrs-at-creation slice a: end-to-end pins E1–E8 (umbrella §6.2; plan
//! §4.4). E5b is not written (plan §0.5 Q5: no hermetic same-origin URL
//! loader; `load.rs:221` and `:327` call the same builder, which E5 pins).
//!
//! Every pin runs through a shell pipeline builder, so every bracket builds a
//! fresh dispatcher and consumer (umbrella H13). A red pin counts only if its
//! non-vacuity control — a value the script itself writes (`data-ran == "1"`
//! or the `"ran"` post) — is green on the pre-fix base.
//!
//! `PipelineResult::eval_script` returns no value, so a bracket that must
//! report a string writes it into a `data-*` attribute on `<body>`, read back
//! with `probe_body_attr` (a Rust-side read, no new bracket). Elements are
//! resolved with `document.getElementById`.
//!
//! Each pin also carries the shared HEAD control (PM decision 48): an
//! in-script `addEventListener` on the pin's own target and event type, and a
//! `data-ctlend` marker written last. Neither depends on handler content
//! attributes being processed, so on the pre-fix base it proves that the pin's
//! script runs to the end and that the dispatch path and element lookup the pin
//! observes through actually work — a red pin is red at its oracle, not at a
//! fixture premise.

use elidex_script_session::DispatchEvent;

use super::iframe::make_out_of_process_entry;
use super::test_support::{
    build_main_document, build_test_content_state, in_process_entry, probe_body_attr,
    run_oop_and_collect_posts, synth_loaded,
};

/// HEAD control, part 1 (PM decision 48): register, in script, a listener on
/// `$target` for `$ty` that runs `$record` (default: count into `data-ctl`).
macro_rules! ctl_listen {
    ($target:literal, $ty:literal) => {
        ctl_listen!(
            $target,
            $ty,
            "document.body.dataset.ctl=(+document.body.dataset.ctl||0)+1"
        )
    };
    ($target:literal, $ty:literal, $record:literal) => {
        concat!(
            $target,
            ".addEventListener('",
            $ty,
            "',()=>{",
            $record,
            "});"
        )
    };
}

/// HEAD control, part 2: the marker a script writes as its last statement.
macro_rules! ctl_end {
    () => {
        ";document.body.dataset.ctlend=1"
    };
}

/// Assert the HEAD control: the pin's script reached its end, and the
/// in-script listener saw `hits` dispatches through the pin's own path.
fn assert_head_control(pipeline: &crate::PipelineResult, hits: &str) {
    assert_eq!(
        probe_body_attr(pipeline, "data-ctlend").as_deref(),
        Some("1"),
        "HEAD control: the pin's script ran to the end"
    );
    assert_eq!(
        probe_body_attr(pipeline, "data-ctl").as_deref(),
        Some(hits),
        "HEAD control: the pin's lookup + dispatch path reaches an in-script listener"
    );
}

/// Shared markup `M` (umbrella §6.2): an initial-HTML `<button onclick>` and
/// an initial script that dispatches a click at it (plus the HEAD control).
const M: &str = concat!(
    "<body><button id=b onclick=\"document.body.dataset.hit=(+document.body.dataset.hit||0)+1\"></button>",
    "<script>document.body.dataset.ran=1;",
    ctl_listen!("document.getElementById('b')", "click"),
    "document.getElementById('b').dispatchEvent(new MouseEvent('click'))",
    ctl_end!(),
    "</script></body>"
);

fn assert_ran(pipeline: &crate::PipelineResult) {
    assert_eq!(
        probe_body_attr(pipeline, "data-ran").as_deref(),
        Some("1"),
        "control: the initial script ran"
    );
}

/// Dispatch a `click` at the element with `id` through
/// `PipelineResult::dispatch_event` (a real bracket).
fn click(pipeline: &mut crate::PipelineResult, id: &str) {
    let root = pipeline.document;
    let target = pipeline.dom.find_by_id(root, id).expect("target element");
    let mut ev = DispatchEvent::new("click", target);
    pipeline.dispatch_event(&mut ev);
}

/// `eval_script`, asserting the bracket's script completed.
fn eval_ok(pipeline: &mut crate::PipelineResult, src: &str) {
    let r = pipeline.eval_script(src);
    assert!(r.success, "eval failed: {:?}", r.error);
}

// ---- E1 (red, K-4 / K-6): initial-HTML handler delivered at the first bind ----
#[test]
fn e1_initial_html_handler_runs_in_initial_script() {
    let (pipeline, _np) = build_main_document(M);
    assert_ran(&pipeline);
    assert_head_control(&pipeline, "1");
    assert_eq!(
        probe_body_attr(&pipeline, "data-hit").as_deref(),
        Some("1"),
        "E1: data-hit == 1"
    );
}

// ---- E2 (red): the delivered handler runs on a later dispatch ----
#[test]
fn e2_initial_html_handler_runs_on_later_dispatch() {
    let (mut pipeline, _np) = build_main_document(M);
    assert_ran(&pipeline);
    click(&mut pipeline, "b");
    assert_head_control(&pipeline, "2");
    assert_eq!(
        probe_body_attr(&pipeline, "data-hit").as_deref(),
        Some("2"),
        "E2: data-hit == 2"
    );
}

// ---- E3 (red, K-1 across rebinds): the compiled callable keeps identity ----
#[test]
fn e3_callable_identity_across_rebinds() {
    let (mut pipeline, _np) = build_main_document(M);
    assert_ran(&pipeline);
    eval_ok(
        &mut pipeline,
        "window.f=document.getElementById('b').onclick; document.body.dataset.t1=typeof window.f",
    );
    click(&mut pipeline, "b");
    eval_ok(
        &mut pipeline,
        "document.body.dataset.t2=String(document.getElementById('b').onclick===window.f)",
    );
    assert_head_control(&pipeline, "2");
    assert_eq!(
        probe_body_attr(&pipeline, "data-t1").as_deref(),
        Some("function"),
        "E3: first read is a function (HEAD: \"object\")"
    );
    assert_eq!(
        probe_body_attr(&pipeline, "data-t2").as_deref(),
        Some("true"),
        "E3: identity kept"
    );
}

// ---- E4 (red): an initial <body> WindowEventHandlers attribute reaches the Window ----
#[test]
fn e4_initial_body_weh_attribute_delivered_to_window() {
    let (pipeline, _np) = build_main_document(concat!(
        "<body onhashchange=\"document.body.dataset.h=1\">",
        "<script>document.body.dataset.ran=1;",
        ctl_listen!("window", "hashchange"),
        "window.dispatchEvent(new Event('hashchange'))",
        ctl_end!(),
        "</script>"
    ));
    assert_ran(&pipeline);
    assert_head_control(&pipeline, "1");
    assert_eq!(
        probe_body_attr(&pipeline, "data-h").as_deref(),
        Some("1"),
        "E4: data-h == 1"
    );
}

// ---- E5 (red): URL-loaded iframe, out-of-process entry (`load.rs:327`) ----
#[test]
fn e5_oop_iframe_initial_html_handler_runs() {
    let url = "https://other.example/";
    // HEAD control over `postMessage` (no `probe_body_attr` across processes).
    let loaded = synth_loaded(
        concat!(
            "<button id=b onclick=\"postMessage('eh-hit','*')\"></button>",
            "<script>postMessage('ran','*');",
            ctl_listen!(
                "document.getElementById('b')",
                "click",
                "postMessage('ctl-hit','*')"
            ),
            "document.getElementById('b').dispatchEvent(new MouseEvent('click'))",
            ";postMessage('ctl-end','*')</script>"
        ),
        url,
    );
    let entry = make_out_of_process_entry(
        loaded,
        crate::PreEvalFrameState {
            origin: elidex_plugin::SecurityOrigin::from_url(&url::Url::parse(url).unwrap()),
            sandbox_flags: elidex_plugin::SandboxingFlagSet::unsandboxed(),
            iframe_depth: 1,
            credentialless: false,
            referrer: None,
        },
        crate::ipc::DeviceFacts::default(),
    );
    let posts: Vec<String> = run_oop_and_collect_posts(entry)
        .into_iter()
        .map(|(data, _origin)| data)
        .collect();
    assert!(
        posts.iter().any(|d| d == "ran"),
        "control: the initial script ran: {posts:?}"
    );
    assert!(
        posts.iter().any(|d| d == "ctl-end"),
        "HEAD control: the pin's script ran to the end: {posts:?}"
    );
    assert!(
        posts.iter().any(|d| d == "ctl-hit"),
        "HEAD control: the pin's lookup + dispatch path reaches an in-script listener: {posts:?}"
    );
    assert!(
        posts.iter().any(|d| d == "eh-hit"),
        "E5: the handler posted: {posts:?}"
    );
}

// ---- E6 (red): srcdoc iframe (in-process child pipeline) ----
#[test]
fn e6_srcdoc_iframe_initial_html_handler_runs() {
    // `M` inside `srcdoc='…'`: its single quotes become `&#39;`.
    let srcdoc = M.replace('\'', "&#39;");
    let (state, _browser) =
        build_test_content_state(&format!("<iframe srcdoc='{srcdoc}'></iframe>"), "");
    let child = &in_process_entry(&state).pipeline;
    assert_ran(child);
    assert_head_control(child, "1");
    assert_eq!(
        probe_body_attr(child, "data-hit").as_deref(),
        Some("1"),
        "E6: child data-hit == 1"
    );
}

/// E7's markup: a declarative shadow root holding `M`'s button.
const E7: &str = concat!(
    "<!DOCTYPE html><div id=h><template shadowrootmode=open>",
    "<button id=b onclick=\"document.body.dataset.hit=(+document.body.dataset.hit||0)+1\"></button>",
    "</template></div>",
    "<script>document.body.dataset.ran=1;",
    ctl_listen!(
        "document.getElementById('h').shadowRoot.querySelector('#b')",
        "click"
    ),
    "document.getElementById('h').shadowRoot.querySelector('#b').dispatchEvent(new MouseEvent('click'))",
    ctl_end!(),
    "</script>"
);

// ---- E7 control: the markup yields a declarative shadow root before any bind ----
// (the strict tier attaches it; a tolerant fallback would not, and would make
// E7's red vacuous — plan §8 Q-S1).
#[test]
fn e7_control_markup_attaches_declarative_shadow_root_pre_bind() {
    let parsed = elidex_html_parser::parse_progressive_str(E7);
    let h = parsed.dom.find_by_id(parsed.document, "h").expect("#h");
    let sr = parsed
        .dom
        .get_shadow_root(h)
        .expect("declarative shadow root attached by the parse");
    assert!(
        parsed.dom.find_by_id(sr, "b").is_some() || parsed.dom.get_first_child(sr).is_some(),
        "the button is parsed into the shadow root"
    );
}

// ---- E7 (red, K-6): the bind pass walks shadow trees ----
#[test]
fn e7_initial_html_handler_in_declarative_shadow_root_runs() {
    let (pipeline, _np) = build_main_document(E7);
    assert_ran(&pipeline);
    let h = pipeline.dom.find_by_id(pipeline.document, "h").expect("#h");
    assert!(
        pipeline.dom.get_shadow_root(h).is_some(),
        "control: shadow root present"
    );
    assert_head_control(&pipeline, "1");
    assert_eq!(
        probe_body_attr(&pipeline, "data-hit").as_deref(),
        Some("1"),
        "E7: data-hit == 1"
    );
}

// ---- E8 (guard, K-0 T-a1 later binds, T-a2 shadow): three brackets ----
#[test]
fn e8_shadow_direct_clone_keeps_idl_handler_across_binds() {
    let (mut pipeline, _np) = build_main_document(concat!(
        "<div id=src onclick=\"document.body.dataset.attr=1\" onmouseover=\"0\"></div><div id=h></div>\
         <script>document.body.dataset.ran=1;\
         const sr=document.getElementById('h').attachShadow({mode:'open'});\
         window.x=document.getElementById('src').cloneNode();\
         sr.appendChild(x);",
        ctl_listen!("x", "click"),
        "x.onclick=()=>{document.body.dataset.fn=1};\
         document.body.dataset.b1='ok'",
        ctl_end!(),
        "</script>"
    ));
    assert_ran(&pipeline);
    assert_eq!(
        probe_body_attr(&pipeline, "data-b1").as_deref(),
        Some("ok"),
        "control: bracket 1 obtained sr and set x.onclick (decided shape, plan §0.5 Q7)"
    );
    eval_ok(
        &mut pipeline,
        "const h=document.getElementById('h'); document.body.append(h); \
         document.body.dataset.b2=String(h.parentNode===document.body)",
    );
    eval_ok(
        &mut pipeline,
        "x.dispatchEvent(new MouseEvent('click')); \
         document.body.dataset.b3=[document.body.dataset.fn, document.body.dataset.attr].join()",
    );
    assert_head_control(&pipeline, "1");
    assert_eq!(
        probe_body_attr(&pipeline, "data-b2").as_deref(),
        Some("true"),
        "E8: bracket 2"
    );
    assert_eq!(
        probe_body_attr(&pipeline, "data-b3").as_deref(),
        Some("1,"),
        "E8: bracket 3"
    );
}
