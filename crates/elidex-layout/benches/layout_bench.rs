#![allow(unused_must_use)]
use criterion::{criterion_group, criterion_main, Criterion};
use elidex_ecs::{Attributes, EcsDom};
use elidex_layout::layout_tree;
use elidex_plugin::{ComputedStyle, Dimension, Display};
use elidex_text::FontDatabase;

/// Build a flat DOM with `n` block children, all pre-styled.
fn build_block_dom(n: usize) -> EcsDom {
    let mut dom = EcsDom::new();
    let root = dom.create_document_root();
    let html = dom.create_element("html", Attributes::default());
    let body = dom.create_element("body", Attributes::default());
    dom.append_child(root, html);
    dom.append_child(html, body);

    let _ = dom.world_mut().insert_one(
        html,
        ComputedStyle {
            display: Display::Block,
            ..Default::default()
        },
    );
    let _ = dom.world_mut().insert_one(
        body,
        ComputedStyle {
            display: Display::Block,
            margin_top: Dimension::Length(8.0),
            margin_right: Dimension::Length(8.0),
            margin_bottom: Dimension::Length(8.0),
            margin_left: Dimension::Length(8.0),
            ..Default::default()
        },
    );

    for _ in 0..n {
        let div = dom.create_element("div", Attributes::default());
        dom.append_child(body, div);
        let _ = dom.world_mut().insert_one(
            div,
            ComputedStyle {
                display: Display::Block,
                height: Dimension::Length(20.0),
                ..Default::default()
            },
        );
    }

    dom
}

/// Build a flex container with `n` children, all pre-styled.
fn build_flex_dom(n: usize) -> EcsDom {
    let mut dom = EcsDom::new();
    let container = dom.create_element("div", Attributes::default());
    let _ = dom.world_mut().insert_one(
        container,
        ComputedStyle {
            display: Display::Flex,
            ..Default::default()
        },
    );

    for _ in 0..n {
        let child = dom.create_element("div", Attributes::default());
        dom.append_child(container, child);
        let _ = dom.world_mut().insert_one(
            child,
            ComputedStyle {
                display: Display::Block,
                width: Dimension::Length(50.0),
                height: Dimension::Length(30.0),
                ..Default::default()
            },
        );
    }

    dom
}

fn bench_layout(c: &mut Criterion) {
    let font_db = FontDatabase::new();

    c.bench_function("block_100", |b| {
        b.iter_batched(
            || build_block_dom(100),
            |mut dom| {
                layout_tree(&mut dom, 800.0, 600.0, &font_db);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("flex_20", |b| {
        b.iter_batched(
            || build_flex_dom(20),
            |mut dom| {
                layout_tree(&mut dom, 800.0, 600.0, &font_db);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_layout);
criterion_main!(benches);
