use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use clap::Parser;
use rahu_core::{lexer::Lexer, tokens::TokenKind};
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
}

struct SourceFile {
    bytes: usize,
    src: String,
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

fn lex_source(src: &str) -> (usize, usize, usize) {
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

fn load_sources(files: &[PathBuf]) -> Result<Vec<SourceFile>> {
    files.iter()
        .map(|path| {
            let src = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            Ok(SourceFile {
                bytes: src.len(),
                src,
            })
        })
        .collect()
}

fn run_once(sources: &[SourceFile]) -> BenchResult {
    let total_bytes = sources.iter().map(|source| source.bytes).sum();
    let start = Instant::now();
    let mut total_tokens = 0usize;
    let mut total_diagnostics = 0usize;
    let mut total_trivia = 0usize;

    for source in sources {
        let (tokens, diagnostics, trivia) = lex_source(&source.src);
        total_tokens += tokens;
        total_diagnostics += diagnostics;
        total_trivia += trivia;
    }

    BenchResult {
        elapsed: start.elapsed(),
        files: sources.len(),
        bytes: total_bytes,
        tokens: total_tokens,
        diagnostics: total_diagnostics,
        trivia: total_trivia,
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
    for _ in 0..args.iterations {
        runs.push(run_once(&sources));
    }
    runs.sort_by_key(|result| result.elapsed);

    if runs.len() == 1 {
        print_result(&runs[0], args.no_diags, args.no_trivia);
    } else {
        print_summary(&runs, args.no_diags, args.no_trivia);
    }

    Ok(())
}
