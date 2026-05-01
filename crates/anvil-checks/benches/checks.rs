use std::time::Duration;

use anvil_checks::antipattern::scanner::scan_file;
use anvil_checks::command_safety::matcher::{MatcherContext, find_matching_rule};
use anvil_checks::command_safety::parser::CommandParser;
use anvil_checks::command_safety::rules::{default_filesystem_rules, default_git_rules};
use anvil_checks::secret::entropy::calculate_entropy;
use anvil_checks::secret::{SecretCheckConfig, scan_content};
use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

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

fn generate_typescript_with_antipatterns(lines: usize, antipattern_count: usize) -> String {
    let mut output = String::new();
    let mut line_count = 0;
    let snippets = [
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

    let spacing = lines
        .checked_div(antipattern_count)
        .map_or_else(|| lines.saturating_add(1), |bucket| bucket.max(1));

    let mut inserted = 0;
    while line_count < lines {
        let current_index = line_count;
        push_line(
            &mut output,
            &mut line_count,
            &base_typescript_line(current_index),
        );

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

fn antipattern_benchmarks(c: &mut Criterion) {
    let ts_small = generate_typescript_with_antipatterns(50, 3);
    let ts_medium = generate_typescript_with_antipatterns(500, 10);
    let ts_large = generate_typescript_with_antipatterns(5000, 20);
    let html_file = generate_html_file(200);
    let ts_clean = generate_typescript_with_antipatterns(500, 0);

    let html_options = anvil_checks::antipattern::scanner::ScanOptions {
        patterns: None,
        include_opt_in: true,
    };

    let mut group = c.benchmark_group("antipattern");
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(100);

    group.bench_function("typescript_small", |b| {
        b.iter(|| {
            black_box(scan_file(
                "src/small.ts",
                black_box(ts_small.as_str()),
                None,
            ))
        });
    });

    group.bench_function("typescript_medium", |b| {
        b.iter(|| {
            black_box(scan_file(
                "src/medium.ts",
                black_box(ts_medium.as_str()),
                None,
            ))
        });
    });

    group.sample_size(50);
    group.bench_function("typescript_large", |b| {
        b.iter(|| {
            black_box(scan_file(
                "src/large.ts",
                black_box(ts_large.as_str()),
                None,
            ))
        });
    });

    group.sample_size(100);
    group.bench_function("html_file", |b| {
        b.iter(|| {
            black_box(scan_file(
                "templates/report.html",
                black_box(html_file.as_str()),
                Some(&html_options),
            ))
        });
    });

    group.bench_function("clean_file", |b| {
        b.iter(|| {
            black_box(scan_file(
                "src/clean.ts",
                black_box(ts_clean.as_str()),
                None,
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
