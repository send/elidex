#![allow(unused_must_use)]
use std::fmt::Write;

use criterion::{criterion_group, criterion_main, Criterion};
use elidex_css::{parse_stylesheet, Origin};
use elidex_ecs::{Attributes, EcsDom};
use elidex_style::resolve_styles;

/// Build a flat DOM with `n` div children under body.
fn build_flat_dom(n: usize) -> (EcsDom, String) {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    dom.append_child(root, html);
    dom.append_child(html, body);

    let mut css = String::from("body { margin: 8px; }\n");
    for i in 0..n {
        let mut attrs = Attributes::default();
        attrs.set("class", format!("item-{i}"));
        let div = dom.create_element("div", attrs);
        dom.append_child(body, div);
        let _ = writeln!(
            css,
            ".item-{i} {{ color: rgb({}, {}, {}); font-size: {}px; display: block; }}",
            i % 256,
            (i * 7) % 256,
            (i * 13) % 256,
            12 + i % 12,
        );
    }

    (dom, css)
}

/// Build a deeply nested DOM (chain of divs).
fn build_deep_dom(depth: usize) -> (EcsDom, String) {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    dom.append_child(root, html);

    let mut parent = html;
    for _ in 0..depth {
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(parent, div);
        parent = div;
    }

    let css = "div { color: red; font-size: 14px; }".to_string();
    (dom, css)
}

fn bench_style_resolve(c: &mut Criterion) {
    c.bench_function("resolve_100_flat", |b| {
        b.iter_batched(
            || {
                let (dom, css) = build_flat_dom(100);
                let sheet = parse_stylesheet(&css, Origin::Author);
                (dom, sheet)
            },
            |(mut dom, sheet)| {
                resolve_styles(&mut dom, &[&sheet], elidex_plugin::Size::new(1280.0, 720.0));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("resolve_1000_flat", |b| {
        b.iter_batched(
            || {
                let (dom, css) = build_flat_dom(1000);
                let sheet = parse_stylesheet(&css, Origin::Author);
                (dom, sheet)
            },
            |(mut dom, sheet)| {
                resolve_styles(&mut dom, &[&sheet], elidex_plugin::Size::new(1280.0, 720.0));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("resolve_deep_100", |b| {
        b.iter_batched(
            || {
                let (dom, css) = build_deep_dom(100);
                let sheet = parse_stylesheet(&css, Origin::Author);
                (dom, sheet)
            },
            |(mut dom, sheet)| {
                resolve_styles(&mut dom, &[&sheet], elidex_plugin::Size::new(1280.0, 720.0));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_style_resolve);
criterion_main!(benches);
