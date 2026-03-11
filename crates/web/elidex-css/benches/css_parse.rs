#![allow(unused_must_use)]
use std::fmt::Write;

use criterion::{criterion_group, criterion_main, Criterion};
use elidex_css::{parse_stylesheet, Origin};

fn generate_css(rule_count: usize) -> String {
    let mut css = String::new();
    for i in 0..rule_count {
        let _ = writeln!(
            css,
            ".class-{i} {{ color: red; font-size: 14px; display: block; }}"
        );
    }
    css
}

fn bench_css_parse(c: &mut Criterion) {
    let small = generate_css(10);
    let medium = generate_css(100);
    let large = generate_css(1000);

    c.bench_function("css_parse_10_rules", |b| {
        b.iter(|| parse_stylesheet(&small, Origin::Author));
    });

    c.bench_function("css_parse_100_rules", |b| {
        b.iter(|| parse_stylesheet(&medium, Origin::Author));
    });

    c.bench_function("css_parse_1000_rules", |b| {
        b.iter(|| parse_stylesheet(&large, Origin::Author));
    });
}

criterion_group!(benches, bench_css_parse);
criterion_main!(benches);
