use std::time::Duration;

use anvil_checks::antipattern::scanner::scan_file;
use anvil_checks::command_safety::matcher::{MatcherContext, find_matching_rule};
use anvil_checks::command_safety::parser::CommandParser;
use anvil_checks::command_safety::rules::{default_filesystem_rules, default_git_rules};
use anvil_checks::secret::entropy::calculate_entropy;
use anvil_checks::secret::{SecretCheckConfig, scan_content};
use std::hint::black_box;

use criterion::{
    BenchmarkGroup, BenchmarkId, Criterion, criterion_group, criterion_main, measurement::WallTime,
};

const SHORT_ENTROPY_STRING: &str = "A7f9K2mN4pQ8xR3s";
const LONG_ENTROPY_STRING: &str =
    "QWxhZGRpbjpPcGVuU2VzYW1lVG9rZW4xMjM0NTY3ODkwQUJDREVGR0hJSktMTU4=";

const SIMPLE_COMMAND: &str = "git status";
const DANGEROUS_COMMAND: &str = "git push --force";
const COMPOUND_COMMAND: &str = "git add . && git commit -m 'fix' && git push";
const WRAPPED_COMMAND: &str = "sudo bash -c 'rm -rf /tmp/build'";
const COMPLEX_PIPELINE_COMMAND: &str = "git log --oneline | head -20 && npm install && npm test";

fn push_line(output: &mut String, line_count: &mut usize, line: &str) {
    output.push_str(line);
    output.push('\n');
    *line_count += 1;
}

fn base_typescript_line(index: usize) -> String {
    match index % 10 {
        0 => format!("import {{ readFileSync }} from 'node:fs'; // module {index}"),
        1 => format!("type User{index} = {{ id: string; active: boolean }};"),
        2 => format!("const featureFlag{index} = {index} % 2 === 0;"),
        3 => format!(
            "function buildPayload{index}(id: string): string {{ return `${{id}}-{index}`; }}"
        ),
        4 => format!(
            "class Service{index} {{ run(value: number): number {{ return value + {index}; }} }}"
        ),
        5 => format!(
            "const user{index}: User{index} = {{ id: `u-${index}`, active: featureFlag{index} }};"
        ),
        6 => format!(
            "export const result{index} = buildPayload{index}(user{index}.id).toUpperCase();"
        ),
        7 => format!("if (featureFlag{index}) {{ void readFileSync('package.json'); }}"),
        8 => format!("const route{index} = `/api/v1/resource/{index}`;"),
        _ => format!(
            "export function compute{index}(left: number, right: number): number {{ return left + right + {index}; }}"
        ),
    }
}

fn base_rust_line(index: usize) -> String {
    match index % 6 {
        0 => format!("pub struct Record{index} {{ pub id: usize, pub active: bool }}"),
        1 => format!(
            "pub const FEATURE_FLAG_{index}: bool = {};",
            index.is_multiple_of(2)
        ),
        2 => format!(
            "pub fn build_payload_{index}(id: usize) -> String {{ format!(\"{{id}}-{index}\") }}"
        ),
        3 => format!(
            "pub fn compute_{index}(left: usize, right: usize) -> usize {{ left + right + {index} }}"
        ),
        4 => {
            format!("pub fn is_active_{index}(record: &Record{index}) -> bool {{ record.active }}")
        }
        _ => format!("pub const ROUTE_{index}: &str = \"/api/v1/resource/{index}\";"),
    }
}

fn base_python_line(index: usize) -> String {
    match index % 6 {
        0 => format!(
            "FEATURE_FLAG_{index} = {}",
            if index.is_multiple_of(2) {
                "True"
            } else {
                "False"
            }
        ),
        1 => format!(
            "def build_payload_{index}(identifier: str) -> str: return f\"{{identifier}}-{index}\""
        ),
        2 => format!("route_{index} = \"/api/v1/resource/{index}\""),
        3 => format!("record_{index} = {{\"id\": {index}, \"active\": True}}"),
        4 => format!(
            "def compute_{index}(left: int, right: int) -> int: return left + right + {index}"
        ),
        _ => format!("result_{index} = compute_{index}({index}, {index} + 1)"),
    }
}

fn generate_typescript_file(lines: usize, secrets_count: usize) -> String {
    let mut output = String::new();
    let mut line_count = 0;

    let secret_snippets: Vec<String> = vec![
        format!(
            "const api_key = '{}{}';",
            "sk_test_", "1234567890abcdefghijABCD"
        ),
        "const sessionToken = '9xY7qW2vK8mN4pR6';".into(),
        "const githubToken = 'ghp_abcdefghijklmnopqrstuvwxyz1234567890abc';".into(),
        "const backupPassword = 'Tr0ub4dor&3SecureValue';".into(),
        "const npmToken = 'npm_abcdefghijklmnopqrstuvwxyz1234567890';".into(),
        "const awsKey = 'AKIAABCDEFGHIJKLMNOP';".into(),
        "const sendGrid = 'SG.1234567890123456789012.1234567890123456789012345678901234567890123';"
            .into(),
        format!(
            "const stripeLive = '{}{}';",
            "sk_live_", "1234567890abcdefghijABCD"
        ),
        "const dbUrl = 'postgres://admin:supersafepassword@localhost:5432/appdb';".into(),
        "const genericSecret = 'super-secret-value-1234';".into(),
    ];

    let spacing = lines
        .checked_div(secrets_count)
        .map_or_else(|| lines.saturating_add(1), |bucket| bucket.max(1));

    let mut inserted = 0;
    while line_count < lines {
        let current_index = line_count;
        push_line(
            &mut output,
            &mut line_count,
            &base_typescript_line(current_index),
        );

        if inserted < secrets_count
            && line_count < lines
            && line_count % spacing == spacing.saturating_sub(1)
        {
            let snippet = &secret_snippets[inserted % secret_snippets.len()];
            push_line(&mut output, &mut line_count, snippet);
            inserted += 1;
        }
    }

    output
}

fn generate_source_with_antipatterns(
    lines: usize,
    antipattern_count: usize,
    base_line: fn(usize) -> String,
    snippets: &[&str],
) -> String {
    let mut output = String::new();
    let mut line_count = 0;

    let spacing = lines
        .checked_div(antipattern_count)
        .map_or_else(|| lines.saturating_add(1), |bucket| bucket.max(1));

    let mut inserted = 0;
    while line_count < lines {
        let current_index = line_count;
        push_line(&mut output, &mut line_count, &base_line(current_index));

        if inserted < antipattern_count
            && line_count < lines
            && line_count % spacing == spacing.saturating_sub(1)
        {
            let snippet = snippets[inserted % snippets.len()];
            push_line(&mut output, &mut line_count, snippet);
            inserted += 1;
        }
    }

    output
}

fn generate_typescript_with_antipatterns(lines: usize, antipattern_count: usize) -> String {
    const SNIPPETS: &[&str] = &[
        "const payload: any = input;",
        "/* eslint-disable */",
        "// @ts-ignore legacy type mismatch",
        "try { doWork(); } catch (e) {}",
        "const fallback: any = {};",
        "// eslint-disable",
        "const result = value as any;",
        "// @ts-ignore upstream typings",
        "const parser = <any>factory();",
        "/* eslint-disable */",
    ];
    generate_source_with_antipatterns(lines, antipattern_count, base_typescript_line, SNIPPETS)
}

fn generate_rust_with_antipatterns(lines: usize, antipattern_count: usize) -> String {
    const SNIPPETS: &[&str] = &[
        "// TODO refactor soon",
        "// HACK bypass validation",
        "// temporary compatibility shim",
    ];
    generate_source_with_antipatterns(lines, antipattern_count, base_rust_line, SNIPPETS)
}

fn generate_python_with_antipatterns(lines: usize, antipattern_count: usize) -> String {
    const SNIPPETS: &[&str] = &[
        "value = payload  # type: ignore",
        "import os  # noqa",
        "# pylint: disable=all",
        "from service.config import *",
        "result = eval(user_input)",
        "os.system(command)",
        "digest = hashlib.md5(payload)",
    ];
    generate_source_with_antipatterns(lines, antipattern_count, base_python_line, SNIPPETS)
}

fn generate_html_file(lines: usize) -> String {
    let mut output = String::new();
    let mut line_count = 0;

    while line_count < lines {
        push_line(&mut output, &mut line_count, "<section class=\"panel\">");
        if line_count < lines {
            push_line(
                &mut output,
                &mut line_count,
                "  <h2 style=\"color: #cc3300; margin-bottom: 12px;\">Build Dashboard</h2>",
            );
        }
        if line_count < lines {
            push_line(
                &mut output,
                &mut line_count,
                "  <p data-kind=\"status\">Pipeline checks are running across all modules.</p>",
            );
        }
        if line_count < lines {
            push_line(
                &mut output,
                &mut line_count,
                "  <button style=\"padding: 8px 16px; border-radius: 6px;\">Refresh</button>",
            );
        }
        if line_count < lines {
            push_line(&mut output, &mut line_count, "  <script>");
        }
        if line_count < lines {
            push_line(
                &mut output,
                &mut line_count,
                "    window.__buildMeta = { branch: 'main', run: Date.now() };",
            );
        }
        if line_count < lines {
            push_line(
                &mut output,
                &mut line_count,
                "    console.log('inline telemetry enabled');",
            );
        }
        if line_count < lines {
            push_line(&mut output, &mut line_count, "  </script>");
        }
        if line_count < lines {
            push_line(&mut output, &mut line_count, "</section>");
        }
    }

    output
}

fn secret_benchmarks(c: &mut Criterion) {
    let config = SecretCheckConfig::default();
    let small_file = generate_typescript_file(50, 2);
    let medium_file = generate_typescript_file(500, 5);
    let large_file = generate_typescript_file(5000, 10);
    let clean_file = generate_typescript_file(500, 0);

    let mut secret_group = c.benchmark_group("secret_scan");
    secret_group.measurement_time(Duration::from_secs(5));
    secret_group.sample_size(100);

    secret_group.bench_function("small_file", |b| {
        b.iter(|| {
            black_box(scan_content(
                black_box(small_file.as_str()),
                black_box("src/small.ts"),
                black_box(&config),
            ))
        });
    });

    secret_group.bench_function("medium_file", |b| {
        b.iter(|| {
            black_box(scan_content(
                black_box(medium_file.as_str()),
                black_box("src/medium.ts"),
                black_box(&config),
            ))
        });
    });

    secret_group.sample_size(50);
    secret_group.bench_function("large_file", |b| {
        b.iter(|| {
            black_box(scan_content(
                black_box(large_file.as_str()),
                black_box("src/large.ts"),
                black_box(&config),
            ))
        });
    });

    secret_group.sample_size(100);
    secret_group.bench_function("clean_file", |b| {
        b.iter(|| {
            black_box(scan_content(
                black_box(clean_file.as_str()),
                black_box("src/clean.ts"),
                black_box(&config),
            ))
        });
    });
    secret_group.finish();

    let mut entropy_group = c.benchmark_group("entropy");
    entropy_group.sample_size(100);
    entropy_group.bench_function("short_string", |b| {
        b.iter(|| black_box(calculate_entropy(black_box(SHORT_ENTROPY_STRING))));
    });
    entropy_group.bench_function("long_string", |b| {
        b.iter(|| black_box(calculate_entropy(black_box(LONG_ENTROPY_STRING))));
    });
    entropy_group.finish();
}

fn bench_language_inputs(
    group: &mut BenchmarkGroup<'_, WallTime>,
    language: &str,
    extension: &str,
    matched_small: &str,
    matched_medium: &str,
    matched_large: &str,
    clean_medium: &str,
) {
    let small_path = format!("src/{language}_small.{extension}");
    let medium_path = format!("src/{language}_medium.{extension}");
    let large_path = format!("src/{language}_large.{extension}");
    let clean_path = format!("src/{language}_clean.{extension}");

    group.sample_size(100);
    group.bench_function(BenchmarkId::new(language, "matched_50_lines"), |b| {
        b.iter(|| {
            black_box(scan_file(
                black_box(small_path.as_str()),
                black_box(matched_small),
                None,
            ))
        });
    });
    group.bench_function(BenchmarkId::new(language, "matched_500_lines"), |b| {
        b.iter(|| {
            black_box(scan_file(
                black_box(medium_path.as_str()),
                black_box(matched_medium),
                None,
            ))
        });
    });
    group.sample_size(50);
    group.bench_function(BenchmarkId::new(language, "matched_5000_lines"), |b| {
        b.iter(|| {
            black_box(scan_file(
                black_box(large_path.as_str()),
                black_box(matched_large),
                None,
            ))
        });
    });
    group.sample_size(100);
    group.bench_function(BenchmarkId::new(language, "clean_500_lines"), |b| {
        b.iter(|| {
            black_box(scan_file(
                black_box(clean_path.as_str()),
                black_box(clean_medium),
                None,
            ))
        });
    });
}

/// Guard the benchmark inputs on the *same* paths `bench_language_inputs`
/// scans. `scan_file` applies per-pattern allowlist globs to the file path, so
/// asserting on a path the benchmarks never use could pass while a benchmark
/// path is silently allowlisted into an empty workload.
fn assert_language_fixture(
    language: &str,
    extension: &str,
    matched_small: &str,
    matched_medium: &str,
    matched_large: &str,
    clean: &str,
) {
    for (size, matched) in [
        ("small", matched_small),
        ("medium", matched_medium),
        ("large", matched_large),
    ] {
        let matched_path = format!("src/{language}_{size}.{extension}");
        let matched_result = scan_file(&matched_path, matched, None);
        assert!(
            !matched_result.warnings.is_empty(),
            "{matched_path} must exercise at least one default rule"
        );
    }

    let clean_path = format!("src/{language}_clean.{extension}");
    let clean_result = scan_file(&clean_path, clean, None);

    assert!(
        clean_result.warnings.is_empty(),
        "{language} clean fixture unexpectedly emitted: {:?}",
        clean_result
            .warnings
            .iter()
            .map(|warning| warning.id.as_str())
            .collect::<Vec<_>>()
    );
}

fn antipattern_benchmarks(c: &mut Criterion) {
    let ts_small = generate_typescript_with_antipatterns(50, 3);
    let ts_medium = generate_typescript_with_antipatterns(500, 10);
    let ts_large = generate_typescript_with_antipatterns(5000, 20);
    let ts_clean = generate_typescript_with_antipatterns(500, 0);
    let rust_small = generate_rust_with_antipatterns(50, 3);
    let rust_medium = generate_rust_with_antipatterns(500, 10);
    let rust_large = generate_rust_with_antipatterns(5000, 20);
    let rust_clean = generate_rust_with_antipatterns(500, 0);
    let python_small = generate_python_with_antipatterns(50, 3);
    let python_medium = generate_python_with_antipatterns(500, 10);
    let python_large = generate_python_with_antipatterns(5000, 20);
    let python_clean = generate_python_with_antipatterns(500, 0);
    let html_file = generate_html_file(200);

    assert_language_fixture(
        "typescript",
        "ts",
        &ts_small,
        &ts_medium,
        &ts_large,
        &ts_clean,
    );
    assert_language_fixture(
        "rust",
        "rs",
        &rust_small,
        &rust_medium,
        &rust_large,
        &rust_clean,
    );
    assert_language_fixture(
        "python",
        "py",
        &python_small,
        &python_medium,
        &python_large,
        &python_clean,
    );

    let html_options = anvil_checks::antipattern::scanner::ScanOptions {
        patterns: None,
        include_opt_in: true,
    };

    // This group name names the implementation under test. Language labels
    // below describe input artefacts, not separate scanner engines.
    let mut group = c.benchmark_group("antipattern_rust_scanner");
    group.measurement_time(Duration::from_secs(5));

    bench_language_inputs(
        &mut group,
        "typescript",
        "ts",
        &ts_small,
        &ts_medium,
        &ts_large,
        &ts_clean,
    );
    bench_language_inputs(
        &mut group,
        "rust",
        "rs",
        &rust_small,
        &rust_medium,
        &rust_large,
        &rust_clean,
    );
    bench_language_inputs(
        &mut group,
        "python",
        "py",
        &python_small,
        &python_medium,
        &python_large,
        &python_clean,
    );

    group.sample_size(100);
    group.bench_function(BenchmarkId::new("html_opt_in", "200_lines"), |b| {
        b.iter(|| {
            black_box(scan_file(
                "templates/report.html",
                black_box(html_file.as_str()),
                Some(&html_options),
            ))
        });
    });

    group.finish();
}

fn command_safety_benchmarks(c: &mut Criterion) {
    let parser = CommandParser;

    let mut command_group = c.benchmark_group("command_safety");
    command_group.sample_size(100);

    command_group.bench_function("simple_command", |b| {
        b.iter(|| black_box(parser.parse(black_box(SIMPLE_COMMAND))));
    });

    command_group.bench_function("dangerous_command", |b| {
        b.iter(|| black_box(parser.parse(black_box(DANGEROUS_COMMAND))));
    });

    command_group.bench_function("compound_command", |b| {
        b.iter(|| black_box(parser.parse_compound(black_box(COMPOUND_COMMAND))));
    });

    command_group.bench_function("wrapped_command", |b| {
        b.iter(|| black_box(parser.parse(black_box(WRAPPED_COMMAND))));
    });

    command_group.bench_function("complex_pipeline", |b| {
        b.iter(|| black_box(parser.parse_compound(black_box(COMPLEX_PIPELINE_COMMAND))));
    });

    let mut all_rules = default_git_rules();
    all_rules.extend(default_filesystem_rules());
    let context = MatcherContext::default();

    command_group.bench_function("rule_matching", |b| {
        b.iter(|| {
            let parsed = parser.parse(black_box(DANGEROUS_COMMAND));
            let matched = find_matching_rule(
                black_box(&parsed),
                black_box(all_rules.as_slice()),
                Some(&context),
            );
            black_box(matched)
        });
    });

    command_group.finish();
}

criterion_group!(
    benches,
    secret_benchmarks,
    antipattern_benchmarks,
    command_safety_benchmarks
);
criterion_main!(benches);
