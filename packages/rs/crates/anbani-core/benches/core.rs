//! Throughput of the kernel on a fixed 64 KiB Georgian text (lorem, seed 1): conversion, bicameral capitals,
//! interpret, georgianise (fast and balanced), contractions, tokeniser, inspector, normalisation.
//! `cargo bench -p anbani-core`.

#![expect(missing_docs, reason = "criterion_group! generates an undocumented fn")]

use std::hint::black_box;

use anbani_core::lorem::Lorem;
use anbani_core::nlp::{contractions, georgianisation, preprocessing};
use anbani_core::normalize::{normalize, Profile};
use anbani_core::{convert, inspect, interpret};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};

fn corpus() -> String {
    let mut lorem = Lorem::seeded(1);
    let mut text = String::new();
    while text.len() < 64 * 1024 {
        text.push_str(&lorem.paragraphs(40, 1, "\n\n"));
    }
    text
}

fn bench(c: &mut Criterion) {
    let text = corpus();
    let latin = convert(&text, "mkhedruli", "qwerty").unwrap_or_default();
    let mut g = c.benchmark_group("kernel");
    g.throughput(Throughput::Bytes(text.len() as u64));
    g.bench_function("convert mkhedruli→mtavruli", |b| b.iter(|| convert(black_box(&text), "mkhedruli", "mtavruli")));
    g.bench_function("convert mkhedruli→national", |b| b.iter(|| convert(black_box(&text), "mkhedruli", "national")));
    g.bench_function("convert mkhedruli→khutsuri (capitals)", |b| {
        b.iter(|| convert(black_box(&text), "mkhedruli", "khutsuri"));
    });
    g.bench_function("interpret →asomtavruli", |b| b.iter(|| interpret(black_box(&text), "asomtavruli")));
    g.bench_function("expand_text", |b| b.iter(|| contractions::expand_text(black_box(&text))));
    g.bench_function("word_tokenize", |b| b.iter(|| preprocessing::word_tokenize(black_box(&text))));
    g.bench_function("inspect", |b| b.iter(|| inspect::inspect(black_box(&text))));
    g.bench_function("normalize search", |b| b.iter(|| normalize(black_box(&text), Profile::Search)));
    g.bench_function("georgianise fast", |b| {
        b.iter(|| georgianisation::georgianise(black_box(&latin), georgianisation::Mode::Fast));
    });
    g.finish();
    let small = &latin[..latin.char_indices().nth(2000).map_or(latin.len(), |(i, _)| i)];
    let mut g = c.benchmark_group("georgianise-balanced");
    g.sample_size(10);
    g.throughput(Throughput::Bytes(small.len() as u64));
    g.bench_function("georgianise balanced (2k chars)", |b| {
        b.iter(|| georgianisation::georgianise(black_box(small), georgianisation::Mode::Balanced));
    });
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
