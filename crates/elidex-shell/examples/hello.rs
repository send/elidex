//! Render a styled HTML page or load a URL in the elidex browser.
//!
//! Usage:
//!   cargo run --example hello -p elidex-shell                    # built-in demo
//!   cargo run --example hello -p elidex-shell -- <https://send.sh> # load URL

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If a URL is passed as the first argument, load it.
    if let Some(url) = std::env::args().nth(1) {
        return elidex_shell::run_url(&url);
    }

    // Otherwise, render the built-in demo page.
    let html = r#"
        <div class="header">
            <h1>elidex</h1>
            <p>Phase 1 — Minimal Rendering</p>
        </div>
        <div class="content">
            <p>Hello, world! This is rendered by the elidex browser engine.</p>
            <div class="box red">Red box</div>
            <div class="box blue">Blue box</div>
            <div class="box green">Green box</div>
        </div>
    "#;

    let css = r"
        body {
            margin: 0;
            padding: 20px;
            background-color: #f0f0f0;
        }
        .header {
            background-color: #2c3e50;
            color: #ecf0f1;
            padding: 20px;
            margin-bottom: 20px;
        }
        h1 {
            font-size: 32px;
            margin-bottom: 8px;
        }
        p {
            font-size: 16px;
            margin-bottom: 12px;
        }
        .content {
            padding: 20px;
        }
        .box {
            display: block;
            width: 300px;
            height: 60px;
            padding: 10px;
            margin-bottom: 10px;
            color: white;
            font-size: 18px;
        }
        .red { background-color: #e74c3c; }
        .blue { background-color: #3498db; }
        .green { background-color: #27ae60; }
    ";

    elidex_shell::run(html, css)
}
