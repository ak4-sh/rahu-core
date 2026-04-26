use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use clap::Parser;
use rahu_core::{
    lexer::{LexDiagKind, Lexer},
    tokens::TokenKind,
};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "rahu-lexbench")]
#[command(about = "Benchmark lexer throughput across a Python repository")]
struct Args {
    /// Path to the Python repository to scan.
    path: PathBuf,

    /// Number of benchmark iterations to run.
    #[arg(long, default_value_t = 1)]
    iterations: usize,

    /// Suppress diagnostics in the printed report.
    #[arg(long)]
    no_diags: bool,

    /// Print individual diagnostics encountered while lexing.
    #[arg(long)]
    show_diags: bool,

    /// Maximum number of diagnostics to print per iteration.
    #[arg(long, default_value_t = 100)]
    diag_limit: usize,

    /// Suppress trivia in the printed report.
    #[arg(long)]
    no_trivia: bool,
}

#[derive(Debug, Clone)]
struct BenchResult {
    elapsed: Duration,
    files: usize,
    bytes: usize,
    tokens: usize,
    diagnostics: usize,
    trivia: usize,
    shown_diagnostics: Vec<PrintedDiagnostic>,
    diagnostics_truncated: bool,
}

struct SourceFile {
    path: PathBuf,
    bytes: usize,
    src: String,
}

#[derive(Debug, Clone)]
struct PrintedDiagnostic {
    path: PathBuf,
    kind: LexDiagKind,
    start: u32,
    end: u32,
    snippet: String,
    context: String,
}

fn is_python_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("py") | Some("pyi")
    )
}

fn collect_python_files(root: &Path) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| is_python_file(path))
        .collect()
}

fn lex_source(src: &str) -> (usize, Vec<rahu_core::lexer::LexDiag>, usize) {
    let mut lexer = Lexer::new(src);
    let mut tokens = 0usize;

    loop {
        let tok = lexer.next_token();
        tokens += 1;
        if tok.kind == TokenKind::Eof {
            break;
        }
    }

    let diagnostics = lexer.take_diagnostics();
    let trivia = lexer.take_trivia().len();
    (tokens, diagnostics, trivia)
}

fn diagnostic_context(src: &str, start: u32, end: u32, radius: usize) -> String {
    let start = start as usize;
    let end = end as usize;

    let mut context_start = start.saturating_sub(radius);
    while context_start > 0 && !src.is_char_boundary(context_start) {
        context_start -= 1;
    }

    let mut context_end = end.saturating_add(radius).min(src.len());
    while context_end < src.len() && !src.is_char_boundary(context_end) {
        context_end += 1;
    }

    src[context_start..context_end].replace(['\n', '\r'], " ")
}

fn load_sources(files: &[PathBuf]) -> Result<Vec<SourceFile>> {
    files.iter()
        .map(|path| {
            let src = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            Ok(SourceFile {
                path: path.clone(),
                bytes: src.len(),
                src,
            })
        })
        .collect()
}

fn run_once(sources: &[SourceFile], diag_limit: Option<usize>) -> BenchResult {
    let total_bytes = sources.iter().map(|source| source.bytes).sum();
    let start = Instant::now();
    let mut total_tokens = 0usize;
    let mut total_diagnostics = 0usize;
    let mut total_trivia = 0usize;
    let mut shown_diagnostics = Vec::new();
    let mut diagnostics_truncated = false;

    for source in sources {
        let (tokens, diagnostics, trivia) = lex_source(&source.src);
        total_tokens += tokens;
        total_diagnostics += diagnostics.len();
        total_trivia += trivia;

        if let Some(limit) = diag_limit {
            let remaining = limit.saturating_sub(shown_diagnostics.len());
            if remaining > 0 {
                shown_diagnostics.extend(diagnostics.iter().take(remaining).map(|diag| {
                    PrintedDiagnostic {
                        path: source.path.clone(),
                        kind: diag.kind,
                        start: diag.span.start,
                        end: diag.span.end,
                        snippet: diag.span.slice(&source.src).to_owned(),
                        context: diagnostic_context(&source.src, diag.span.start, diag.span.end, 40),
                    }
                }));
            }
            if diagnostics.len() > remaining {
                diagnostics_truncated = true;
            }
        }
    }

    BenchResult {
        elapsed: start.elapsed(),
        files: sources.len(),
        bytes: total_bytes,
        tokens: total_tokens,
        diagnostics: total_diagnostics,
        trivia: total_trivia,
        shown_diagnostics,
        diagnostics_truncated,
    }
}

fn print_diagnostics(result: &BenchResult, iteration: usize, diag_limit: usize) {
    println!("iteration:    {}", iteration);
    if result.shown_diagnostics.is_empty() {
        println!("diagnostic entries: none");
        return;
    }

    println!(
        "diagnostic entries: showing {} of {}",
        result.shown_diagnostics.len(),
        result.diagnostics
    );
    for diag in &result.shown_diagnostics {
        println!(
            "{}: {:?} @ {}..{}: {:?}",
            diag.path.display(),
            diag.kind,
            diag.start,
            diag.end,
            diag.snippet
        );
        println!("  context: {:?}", diag.context);
    }
    if result.diagnostics_truncated {
        println!("diagnostic entries: truncated at {}", diag_limit);
    }
}

fn format_count(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);

    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx != 0 && idx % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }

    out.chars().rev().collect()
}

fn print_result(result: &BenchResult, no_diags: bool, no_trivia: bool) {
    let seconds = result.elapsed.as_secs_f64();
    let mb = result.bytes as f64 / 1_000_000.0;
    println!("files:        {}", format_count(result.files));
    println!("bytes:        {:.2} MB", mb);
    println!("tokens:       {}", format_count(result.tokens));
    if !no_diags {
        println!("diagnostics:  {}", format_count(result.diagnostics));
    }
    if !no_trivia {
        println!("trivia:       {}", format_count(result.trivia));
    }
    println!("time:         {:.3} ms", seconds * 1000.0);
    if seconds > 0.0 {
        println!("throughput:   {:.2} MB/s", mb / seconds);
        println!(
            "token rate:   {:.2}M tok/s",
            result.tokens as f64 / seconds / 1_000_000.0
        );
        println!("files/sec:    {:.2}", result.files as f64 / seconds);
    } else {
        println!("throughput:   inf MB/s");
        println!("token rate:   inf tok/s");
        println!("files/sec:    inf");
    }
}

fn percentile_duration(runs: &[BenchResult], percentile: f64) -> Duration {
    let last = runs.len().saturating_sub(1);
    let idx = ((last as f64) * percentile).ceil() as usize;
    runs[idx.min(last)].elapsed
}

fn print_summary(runs: &[BenchResult], no_diags: bool, no_trivia: bool) {
    let median = &runs[runs.len() / 2];
    print_result(median, no_diags, no_trivia);
    println!("iterations:   {}", runs.len());
    println!(
        "median:       {:.3} ms",
        median.elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "p95:          {:.3} ms",
        percentile_duration(runs, 0.95).as_secs_f64() * 1000.0
    );
    println!(
        "best:         {:.3} ms",
        runs.first().unwrap().elapsed.as_secs_f64() * 1000.0
    );
    println!(
        "worst:        {:.3} ms",
        runs.last().unwrap().elapsed.as_secs_f64() * 1000.0
    );
}

fn main() -> Result<()> {
    let args = Args::parse();
    anyhow::ensure!(args.iterations > 0, "--iterations must be at least 1");
    anyhow::ensure!(
        args.path.is_dir(),
        "{} is not a directory",
        args.path.display()
    );

    let files = collect_python_files(&args.path);
    anyhow::ensure!(
        !files.is_empty(),
        "no Python files found under {}",
        args.path.display()
    );
    let sources = load_sources(&files)?;

    let mut runs = Vec::with_capacity(args.iterations);
    let diag_limit = args.show_diags.then_some(args.diag_limit);
    for iteration in 0..args.iterations {
        let result = run_once(&sources, diag_limit);
        if args.show_diags {
            print_diagnostics(&result, iteration + 1, args.diag_limit);
        }
        runs.push(result);
    }
    runs.sort_by_key(|result| result.elapsed);

    if runs.len() == 1 {
        print_result(&runs[0], args.no_diags, args.no_trivia);
    } else {
        print_summary(&runs, args.no_diags, args.no_trivia);
    }

    Ok(())
}
