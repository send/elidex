//! Tests for Web Workers (WHATWG HTML §10).

use super::*;

// ---------------------------------------------------------------------------
// Helper: create a worker runtime (worker side)
// ---------------------------------------------------------------------------

fn setup_worker(name: &str, script_url: &str) -> (JsRuntime, SessionCore, EcsDom, Entity) {
    let url = ::url::Url::parse(script_url).unwrap();
    let runtime = JsRuntime::for_worker(None, name.to_string(), url);
    let session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();
    (runtime, session, dom, doc)
}

// ---------------------------------------------------------------------------
// Worker global scope tests
// ---------------------------------------------------------------------------

#[test]
fn worker_self_is_global() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(typeof self !== 'undefined')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_name_property() {
    let (mut rt, mut session, mut dom, doc) =
        setup_worker("my-worker", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(self.name === 'my-worker')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_name_empty_default() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval("console.log(self.name === '')", &mut session, &mut dom, doc);
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_no_dom_access() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(typeof document === 'undefined')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_no_window_access() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(typeof window === 'undefined')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_console() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval("console.log('worker says hi')", &mut session, &mut dom, doc);
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs
        .last()
        .is_some_and(|(_, text)| text == "worker says hi"));
}

#[test]
fn worker_timers() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "var id = setTimeout(function() { console.log('timer fired'); }, 1); console.log(typeof id === 'number')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_location_href() {
    let (mut rt, mut session, mut dom, doc) =
        setup_worker("", "https://example.com/scripts/worker.js?v=1#hash");
    let result = rt.eval(
        "console.log(self.location.href === 'https://example.com/scripts/worker.js?v=1#hash')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_location_origin() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(self.location.origin === 'https://example.com')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_location_protocol() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(self.location.protocol === 'https:')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_location_pathname() {
    let (mut rt, mut session, mut dom, doc) =
        setup_worker("", "https://example.com/scripts/worker.js");
    let result = rt.eval(
        "console.log(self.location.pathname === '/scripts/worker.js')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_location_tostring() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(self.location.toString() === 'https://example.com/worker.js')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_navigator_user_agent() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(self.navigator.userAgent === 'elidex/0.1')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_navigator_hardware_concurrency() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(typeof self.navigator.hardwareConcurrency === 'number' && self.navigator.hardwareConcurrency >= 1)",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_navigator_language() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(typeof self.navigator.language === 'string')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_is_secure_context_https() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(self.isSecureContext === true)",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_is_secure_context_http() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "http://example.com/worker.js");
    let result = rt.eval(
        "console.log(self.isSecureContext === false)",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_post_message_queues() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        r#"postMessage({hello: "world"}); console.log("sent");"#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);

    // Verify outgoing messages were queued.
    let outgoing = rt.drain_worker_outgoing();
    assert_eq!(outgoing.len(), 1);
    match &outgoing[0] {
        elidex_api_workers::WorkerToParent::PostMessage { data, .. } => {
            assert!(data.contains("hello"));
            assert!(data.contains("world"));
        }
        other => panic!("Expected PostMessage, got {other:?}"),
    }
}

#[test]
fn worker_close_sets_flag() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    assert!(!rt.bridge().worker_close_requested());
    let result = rt.eval("close();", &mut session, &mut dom, doc);
    assert!(result.success, "JS error: {:?}", result.error);
    assert!(rt.bridge().worker_close_requested());
}

#[test]
fn worker_onmessage_setter_getter() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        r#"
        console.log(self.onmessage === null);
        self.onmessage = function(e) { console.log(e.data); };
        console.log(typeof self.onmessage === 'function');
        "#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs[0].1 == "true", "onmessage should be null initially");
    assert!(
        msgs[1].1 == "true",
        "onmessage should be function after set"
    );
}

#[test]
fn worker_add_event_listener() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        r#"
        addEventListener("message", function(e) {});
        console.log("registered");
        "#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "registered"));
}

#[test]
fn worker_constructor_exists_in_parent() {
    let (mut rt, mut session, mut dom, doc) = setup();
    let result = rt.eval(
        "console.log(typeof Worker === 'function')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_constructor_module_type_rejected() {
    let (mut rt, mut session, mut dom, doc) = setup();
    rt.set_current_url(Some(::url::Url::parse("https://example.com/").unwrap()));
    let result = rt.eval(
        r#"
        try {
            new Worker("worker.js", { type: "module" });
            console.log(false);
        } catch(e) {
            console.log(e instanceof TypeError);
        }
        "#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(
        msgs.last().is_some_and(|(_, text)| text == "true"),
        "Expected TypeError for type:module, got: {msgs:?}"
    );
}

#[test]
fn worker_atob_btoa_available() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "console.log(btoa('hello') === 'aGVsbG8=')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_url_constructor_available() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        "var u = new URL('https://example.com/path'); console.log(u.pathname === '/path')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_structured_clone_available() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        r#"var obj = {a: 1, b: [2, 3]}; var c = structuredClone(obj); console.log(c.a === 1 && c.b.length === 2)"#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

// ---------------------------------------------------------------------------
// Thread-level tests (worker_thread_main via crossbeam channel)
// ---------------------------------------------------------------------------

#[test]
fn worker_terminate() {
    use elidex_api_workers::ParentToWorker;

    let (parent_ch, worker_ch) = elidex_plugin::channel_pair();

    // Spawn a worker that loops indefinitely via setInterval.
    let handle = std::thread::spawn(move || {
        crate::worker_thread::worker_thread_main_with_source(
            "setInterval(function() {}, 100);".to_string(),
            ::url::Url::parse("https://example.com/worker.js").unwrap(),
            String::new(),
            worker_ch,
        );
    });

    // Give the worker time to start.
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Send Shutdown.
    let _ = parent_ch.send(ParentToWorker::Shutdown);

    // The thread should exit within 2 seconds.
    let (done_tx, done_rx) = crossbeam_channel::bounded(1);
    std::thread::spawn(move || {
        let _ = handle.join();
        let _ = done_tx.send(());
    });
    let result = done_rx.recv_timeout(std::time::Duration::from_secs(2));
    assert!(result.is_ok(), "Worker thread did not exit after terminate");
}

#[test]
fn worker_close_thread_exit() {
    use elidex_api_workers::WorkerToParent;

    let (_parent_ch, worker_ch) = elidex_plugin::channel_pair();

    let handle = std::thread::spawn(move || {
        crate::worker_thread::worker_thread_main_with_source(
            "close();".to_string(),
            ::url::Url::parse("https://example.com/worker.js").unwrap(),
            String::new(),
            worker_ch,
        );
    });

    // Drain messages — should get Closed.
    let mut got_closed = false;
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        while let Ok(msg) = _parent_ch.try_recv() {
            if matches!(msg, WorkerToParent::Closed) {
                got_closed = true;
            }
        }
        if got_closed {
            break;
        }
    }
    assert!(got_closed, "Worker should send Closed after close()");

    let (done_tx, done_rx) = crossbeam_channel::bounded(1);
    std::thread::spawn(move || {
        let _ = handle.join();
        let _ = done_tx.send(());
    });
    let result = done_rx.recv_timeout(std::time::Duration::from_secs(2));
    assert!(result.is_ok(), "Worker thread did not exit after close()");
}

#[test]
fn worker_error_propagation() {
    use elidex_api_workers::{ParentToWorker, WorkerToParent};

    let (parent_ch, worker_ch) = elidex_plugin::channel_pair();

    let handle = std::thread::spawn(move || {
        crate::worker_thread::worker_thread_main_with_source(
            "this is not valid javascript !!!".to_string(),
            ::url::Url::parse("https://example.com/worker.js").unwrap(),
            String::new(),
            worker_ch,
        );
    });

    // Drain messages — should get Error.
    let mut got_error = false;
    let mut error_message = String::new();
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        while let Ok(msg) = parent_ch.try_recv() {
            if let WorkerToParent::Error { message, .. } = msg {
                got_error = true;
                error_message = message;
            }
        }
        if got_error {
            break;
        }
    }

    // Shutdown the worker (it's still running its event loop after eval error).
    let _ = parent_ch.send(ParentToWorker::Shutdown);
    let _ = handle.join();

    assert!(got_error, "Worker should report Error on syntax error");
    assert!(
        !error_message.is_empty(),
        "Error message should not be empty"
    );
}

#[test]
fn worker_multiple_concurrent() {
    use elidex_api_workers::{ParentToWorker, WorkerToParent};

    let mut handles = Vec::new();
    let mut parent_channels = Vec::new();

    // Spawn 3 workers that each postMessage their name.
    for i in 0..3 {
        let (parent_ch, worker_ch) = elidex_plugin::channel_pair();
        let name = format!("worker-{i}");
        let script = format!(r#"postMessage(self.name);"#);

        let handle = std::thread::spawn(move || {
            crate::worker_thread::worker_thread_main_with_source(
                script,
                ::url::Url::parse("https://example.com/worker.js").unwrap(),
                name,
                worker_ch,
            );
        });

        handles.push(handle);
        parent_channels.push(parent_ch);
    }

    // Collect messages from all workers.
    let mut names = Vec::new();
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(25));
        for ch in &parent_channels {
            while let Ok(msg) = ch.try_recv() {
                if let WorkerToParent::PostMessage { data, .. } = msg {
                    names.push(data);
                }
            }
        }
        if names.len() >= 3 {
            break;
        }
    }

    // Shutdown all.
    for ch in &parent_channels {
        let _ = ch.send(ParentToWorker::Shutdown);
    }
    for h in handles {
        let _ = h.join();
    }

    assert_eq!(names.len(), 3, "Expected 3 messages, got {}", names.len());
    // Each message is a JSON-stringified worker name.
    for i in 0..3 {
        let expected = format!("\"worker-{i}\"");
        assert!(
            names.contains(&expected),
            "Missing message from worker-{i}, got: {names:?}"
        );
    }
}

#[test]
fn worker_postmessage_roundtrip() {
    use elidex_api_workers::{ParentToWorker, WorkerToParent};

    let (parent_ch, worker_ch) = elidex_plugin::channel_pair();

    // Worker echoes received messages back.
    let handle = std::thread::spawn(move || {
        crate::worker_thread::worker_thread_main_with_source(
            r#"self.onmessage = function(e) { postMessage(e.data); };"#.to_string(),
            ::url::Url::parse("https://example.com/worker.js").unwrap(),
            String::new(),
            worker_ch,
        );
    });

    // Give worker time to initialize.
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Send a message.
    let _ = parent_ch.send(ParentToWorker::PostMessage {
        data: r#"{"ping":42}"#.to_string(),
        origin: "https://example.com".to_string(),
    });

    // Collect echo.
    let mut echo = None;
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(25));
        while let Ok(msg) = parent_ch.try_recv() {
            if let WorkerToParent::PostMessage { data, .. } = msg {
                echo = Some(data);
            }
        }
        if echo.is_some() {
            break;
        }
    }

    let _ = parent_ch.send(ParentToWorker::Shutdown);
    let _ = handle.join();

    let echo = echo.expect("Worker should echo back the message");
    assert!(echo.contains("ping"), "Echo should contain 'ping': {echo}");
    assert!(echo.contains("42"), "Echo should contain '42': {echo}");
}

// ---------------------------------------------------------------------------
// Worker global scope tests (continued)
// ---------------------------------------------------------------------------

#[test]
fn worker_fetch_available() {
    // Create worker with FetchHandle to verify fetch registration.
    let url = ::url::Url::parse("https://example.com/worker.js").unwrap();
    let fetch_handle = std::rc::Rc::new(elidex_net::FetchHandle::with_default_client());
    let mut rt = JsRuntime::for_worker(Some(fetch_handle), String::new(), url);
    let mut session = SessionCore::new();
    let mut dom = EcsDom::new();
    let doc = dom.create_document_root();

    let result = rt.eval(
        "console.log(typeof fetch === 'function')",
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(msgs.last().is_some_and(|(_, text)| text == "true"));
}

#[test]
fn worker_message_event_properties() {
    // Verify MessageEvent object shape when dispatched to worker.
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        r#"
        var received = null;
        self.onmessage = function(e) { received = e; };
        "#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);

    // Dispatch a message event via the runtime method.
    rt.dispatch_worker_message(
        &mut session,
        &mut dom,
        doc,
        r#"{"key":"value"}"#,
        "https://example.com",
    );

    // Check the received event properties.
    let result = rt.eval(
        r#"
        console.log(received !== null);
        console.log(received.data.key === 'value');
        console.log(received.origin === 'https://example.com');
        console.log(received.lastEventId === '');
        console.log(received.source === null);
        console.log(Array.isArray(received.ports) && received.ports.length === 0);
        console.log(received.type === 'message');
        console.log(received.isTrusted === true);
        "#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    // All 8 assertions should be "true".
    for (i, (_, text)) in msgs.iter().enumerate() {
        assert!(
            text == "true",
            "MessageEvent property check {i} failed: got '{text}'"
        );
    }
}

#[test]
fn worker_messageerror_on_clone_failure() {
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    // Create a circular reference and try to postMessage it.
    let result = rt.eval(
        r#"
        var obj = {};
        obj.self = obj;
        postMessage(obj);
        console.log("sent");
        "#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);

    // Check that a SerializationError was queued (becomes MessageError).
    let outgoing = rt.drain_worker_outgoing();
    assert!(
        outgoing
            .iter()
            .any(|msg| matches!(msg, elidex_api_workers::WorkerToParent::MessageError)),
        "Expected MessageError for circular reference, got: {outgoing:?}"
    );
}

#[test]
fn worker_invalid_url_rejected() {
    let (mut rt, mut session, mut dom, doc) = setup();
    rt.set_current_url(Some(::url::Url::parse("https://example.com/").unwrap()));
    // Cross-origin URL should be rejected.
    let result = rt.eval(
        r#"
        try {
            new Worker("https://evil.com/worker.js");
            console.log("no error");
        } catch(e) {
            console.log(e instanceof TypeError);
        }
        "#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(
        msgs.last().is_some_and(|(_, text)| text == "true"),
        "Cross-origin Worker should throw TypeError, got: {msgs:?}"
    );
}

#[test]
fn worker_credentials_option_parsed() {
    // Verify that credentials option is accepted without error.
    let (mut rt, mut session, mut dom, doc) = setup();
    rt.set_current_url(Some(::url::Url::parse("https://example.com/").unwrap()));
    let result = rt.eval(
        r#"
        try {
            // This will fail to fetch (no server), but should NOT throw
            // a TypeError for the credentials option itself.
            new Worker("worker.js", { credentials: "same-origin" });
            console.log("accepted");
        } catch(e) {
            // TypeError from fetch failure is OK, but not from credentials parsing.
            console.log(e instanceof TypeError ? "type-error" : "other-error");
        }
        "#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    // Should get "accepted" (Worker object returned even on fetch failure).
    assert!(
        msgs.last().is_some_and(|(_, text)| text == "accepted"),
        "credentials option should be parsed without error, got: {msgs:?}"
    );
}

#[test]
fn worker_blob_url() {
    // Verify blob: URLs pass the same-origin check in Worker constructor.
    // URL.createObjectURL is not available in this test environment, so we
    // construct a blob: URL string directly and verify it's accepted by the
    // Worker constructor's same-origin check (not rejected as cross-origin).
    let (mut rt, mut session, mut dom, doc) = setup();
    rt.set_current_url(Some(::url::Url::parse("https://example.com/").unwrap()));
    let result = rt.eval(
        r#"
        // blob: URLs should pass the same-origin check.
        // The fetch will fail (no real blob store), but the constructor
        // should NOT throw a SecurityError — it should return a Worker object.
        try {
            var w = new Worker("blob:https://example.com/some-uuid");
            console.log(typeof w === 'object');
        } catch(e) {
            // SecurityError would mean blob: same-origin check failed.
            console.log("error: " + e.message);
        }
        "#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(
        msgs.last().is_some_and(|(_, text)| text == "true"),
        "blob: URL Worker should be accepted, got: {msgs:?}"
    );
}

#[test]
fn worker_dispatch_event() {
    // Verify dispatchEvent is available on the worker global scope.
    let (mut rt, mut session, mut dom, doc) = setup_worker("", "https://example.com/worker.js");
    let result = rt.eval(
        r#"
        var received = false;
        addEventListener("custom", function(e) { received = true; });
        dispatchEvent({ type: "custom" });
        console.log(received);
        "#,
        &mut session,
        &mut dom,
        doc,
    );
    assert!(result.success, "JS error: {:?}", result.error);
    let msgs = rt.console_output().messages();
    assert!(
        msgs.last().is_some_and(|(_, text)| text == "true"),
        "dispatchEvent should fire listener, got: {msgs:?}"
    );
}
