use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rahu_core::{lexer::Lexer, tokens::TokenKind};
use std::hint::black_box;

const LEX_ONE_FILE: &str = include_str!("fixtures/lex_one_file.py");

fn lex_all(src: &str) -> (usize, usize, usize) {
    let mut lexer = Lexer::new(src);
    let mut tokens = 0usize;

    loop {
        let tok = lexer.next_token();
        tokens += 1;
        if tok.kind == TokenKind::Eof {
            break;
        }
    }

    let diagnostics = lexer.take_diagnostics().len();
    let trivia = lexer.take_trivia().len();
    (tokens, diagnostics, trivia)
}

fn make_large_string() -> String {
    LEX_ONE_FILE.repeat(64)
}

fn make_fstring_heavy_source() -> String {
    let mut src = String::new();
    for i in 0..2_000 {
        src.push_str(&format!(
            "value_{i} = f\"item={{{i}}} hex={{{i}:#06x}} repr={{{i}!r}} nested={{{i} + 1}}\"\n"
        ));
    }
    src
}

fn make_comment_heavy_source() -> String {
    let mut src = String::new();
    for i in 0..4_000 {
        src.push_str(&format!("# comment line {i}: lorem ipsum dolor sit amet\n"));
        src.push_str(&format!("value_{i} = {i}  # trailing comment {i}\n"));
    }
    src
}

fn make_unicode_identifier_source() -> String {
    let mut src = String::new();
    for i in 0..4_000 {
        src.push_str(&format!(
            "\u{03b1}\u{03c1}\u{03b9}\u{03b8}\u{03bc}\u{03cc}\u{03c2}_{i} = \u{03b4}\u{03b5}\u{03b4}\u{03bf}\u{03bc}\u{03ad}\u{03bd}\u{03b1}_{i} + \u{03c0}\u{03c1}\u{03bf}\u{03c3}\u{03b8}\u{03ae}\u{03ba}\u{03b7}_{i}\n"
        ));
    }
    src
}

fn bench_sources(c: &mut Criterion) {
    let large = make_large_string();
    let fstrings = make_fstring_heavy_source();
    let comments = make_comment_heavy_source();
    let unicode = make_unicode_identifier_source();

    let mut group = c.benchmark_group("lexer");
    group.bench_function("lex_one_file", |b| {
        b.iter(|| black_box(lex_all(black_box(LEX_ONE_FILE))));
    });
    group.bench_function("lex_large_string", |b| {
        b.iter(|| black_box(lex_all(black_box(&large))));
    });
    group.bench_function("lex_fstring_heavy", |b| {
        b.iter(|| black_box(lex_all(black_box(&fstrings))));
    });
    group.bench_function("lex_comment_heavy", |b| {
        b.iter(|| black_box(lex_all(black_box(&comments))));
    });
    group.bench_function("lex_unicode_identifiers", |b| {
        b.iter(|| black_box(lex_all(black_box(&unicode))));
    });
    group.finish();
}

fn bench_large_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer_large_string_sizes");
    for repeat in [1usize, 8, 32, 64] {
        let src = LEX_ONE_FILE.repeat(repeat);
        group.bench_with_input(BenchmarkId::from_parameter(repeat), &src, |b, src| {
            b.iter(|| black_box(lex_all(black_box(src))));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_sources, bench_large_sizes);
criterion_main!(benches);
