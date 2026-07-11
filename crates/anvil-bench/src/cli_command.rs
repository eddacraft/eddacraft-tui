//! Per-command Anvil CLI benchmark runner (RLB-009).
//!
//! This module intentionally benchmarks the resolved `anvil` binary directly —
//! never an arbitrary shell command. Arguments after `--` are passed verbatim to
//! `anvil`, the benchmark runs in isolated state by default, and reports redact
//! raw argument values unless the caller explicitly opts in.

use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use crate::fixture::{RepoSpec, SyntheticRepo, generate_repo};
use crate::spawn::{in_new_process_group, resolve_anvil_binary};

#[cfg(target_os = "linux")]
use crate::proc_sampler::TreeSampler;

pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

const DEFAULT_REPEAT: usize = 10;
const DEFAULT_WARMUP: usize = 2;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_SAMPLE_INTERVAL: Duration = Duration::from_millis(50);
const SCHEMA_VERSION: u32 = 1;
const SAFE_ENV_VARS: [(&str, &str); 4] = [
    ("ANVIL_DISABLE_UPDATE_HINT", "1"),
    ("ANVIL_USAGE_DISABLE", "1"),
    ("ANVIL_INTERCEPT_DISABLE_OBSERVATION", "1"),
    ("DO_NOT_TRACK", "1"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureSpec {
    Empty,
    Small,
    Default,
    Path(PathBuf),
}

impl FixtureSpec {
    fn label(&self) -> String {
        match self {
            Self::Empty => "empty".to_owned(),
            Self::Small => "small".to_owned(),
            Self::Default => "default".to_owned(),
            Self::Path(path) => format!("path:{}", path.display()),
        }
    }

    fn parse(raw: &str) -> Result<Self> {
        match raw {
            "empty" => Ok(Self::Empty),
            "small" => Ok(Self::Small),
            "default" => Ok(Self::Default),
            path if path.starts_with("path:") => {
                let path = path.strip_prefix("path:").expect("prefix checked");
                if path.is_empty() {
                    return Err("path: fixture requires a non-empty path".into());
                }
                Ok(Self::Path(PathBuf::from(path)))
            }
            other => Err(format!(
                "unknown fixture {other:?}; expected empty, small, default, or path:<dir>"
            )
            .into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandBenchmarkConfig {
    pub name: String,
    pub anvil_bin: PathBuf,
    pub anvil_args: Vec<String>,
    pub repeat: usize,
    pub warmup: usize,
    pub fixture: FixtureSpec,
    pub timeout: Duration,
    pub sample_interval: Duration,
    pub output: Option<PathBuf>,
    pub include_raw_argv: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgShape {
    pub position: usize,
    pub kind: ArgKind,
    pub name: Option<String>,
    pub has_value: bool,
    pub sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArgKind {
    Command,
    Flag,
    Positional,
    Separator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandIteration {
    pub index: usize,
    pub duration_ms: f64,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub cpu_pct: Option<f64>,
    pub peak_rss_mib: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAggregate {
    pub samples: usize,
    pub failures: usize,
    pub timeouts: usize,
    pub duration_min_ms: f64,
    pub duration_mean_ms: f64,
    pub duration_median_ms: f64,
    pub duration_p95_ms: f64,
    pub duration_p99_ms: f64,
    pub stdout_max_bytes: u64,
    pub stderr_max_bytes: u64,
    pub cpu_p95_pct: Option<f64>,
    pub cpu_max_pct: Option<f64>,
    pub peak_rss_p95_mib: Option<f64>,
    pub peak_rss_max_mib: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandBenchmarkReport {
    pub schema_version: u32,
    pub benchmark: String,
    pub name: String,
    pub command_family: Option<String>,
    pub generated_at_epoch: u64,
    pub anvil_bin: String,
    pub fixture: String,
    pub safe_env: Vec<String>,
    pub argv: Vec<ArgShape>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_argv: Option<Vec<String>>,
    pub warmup: usize,
    pub iterations: Vec<CommandIteration>,
    pub aggregate: CommandAggregate,
}

impl CommandBenchmarkReport {
    pub fn to_json(&self) -> std::result::Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

pub struct ParsedCommandLine {
    pub config: CommandBenchmarkConfig,
}

pub fn parse_cli_args<I, T>(itr: I) -> Result<ParsedCommandLine>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut args = itr.into_iter().map(Into::into);
    let _program = args.next();
    let mut name = None;
    let mut repeat = DEFAULT_REPEAT;
    let mut warmup = DEFAULT_WARMUP;
    let mut fixture = FixtureSpec::Empty;
    let mut timeout = DEFAULT_TIMEOUT;
    let mut sample_interval = DEFAULT_SAMPLE_INTERVAL;
    let mut output = None;
    let mut include_raw_argv = false;
    let mut anvil_args = Vec::new();

    while let Some(arg) = args.next() {
        if arg == OsStr::new("--") {
            anvil_args = args.map(os_to_string).collect::<Result<Vec<_>>>()?;
            break;
        }
        let arg = os_to_string(arg)?;
        match arg.as_str() {
            "--help" | "-h" => return Err(help_text().into()),
            "--name" => name = Some(next_value(&mut args, "--name")?),
            "--repeat" => repeat = parse_usize(&next_value(&mut args, "--repeat")?, "--repeat")?,
            "--warmup" => warmup = parse_usize(&next_value(&mut args, "--warmup")?, "--warmup")?,
            "--fixture" => fixture = FixtureSpec::parse(&next_value(&mut args, "--fixture")?)?,
            "--timeout-ms" => {
                timeout = Duration::from_millis(parse_u64(
                    &next_value(&mut args, "--timeout-ms")?,
                    "--timeout-ms",
                )?);
            }
            "--sample-interval-ms" => {
                sample_interval = Duration::from_millis(parse_u64(
                    &next_value(&mut args, "--sample-interval-ms")?,
                    "--sample-interval-ms",
                )?);
            }
            "--output" => output = Some(PathBuf::from(next_value(&mut args, "--output")?)),
            "--include-raw-argv" => include_raw_argv = true,
            other => return Err(format!("unknown option {other:?}\n\n{}", help_text()).into()),
        }
    }

    let name = name.ok_or("--name is required")?;
    if repeat == 0 {
        return Err("--repeat must be greater than zero".into());
    }
    if timeout.is_zero() {
        return Err("--timeout-ms must be greater than zero".into());
    }
    if sample_interval.is_zero() {
        return Err("--sample-interval-ms must be greater than zero".into());
    }
    if anvil_args.is_empty() {
        return Err("provide anvil arguments after --".into());
    }

    Ok(ParsedCommandLine {
        config: CommandBenchmarkConfig {
            name,
            anvil_bin: resolve_anvil_binary()?,
            anvil_args,
            repeat,
            warmup,
            fixture,
            timeout,
            sample_interval,
            output,
            include_raw_argv,
        },
    })
}

pub fn help_text() -> &'static str {
    "Usage: anvil-bench-command --name <label> [options] -- <anvil args...>\n\n\
Options:\n  --repeat <n>              Measured iterations (default 10)\n  --warmup <n>              Warmup iterations excluded from report (default 2)\n  --fixture <kind>          empty | small | default | path:<dir> (default empty)\n  --timeout-ms <n>          Per-iteration timeout (default 30000)\n  --sample-interval-ms <n>  Resource sampling interval (default 50)\n  --output <path>           Write JSON report to path\n  --include-raw-argv        Include raw Anvil argv in report (off by default)\n\n\
The runner executes only the resolved anvil binary; args after -- are passed directly."
}

pub fn run(config: &CommandBenchmarkConfig) -> Result<CommandBenchmarkReport> {
    let workspace = BenchmarkWorkspace::new(&config.fixture)?;
    let mut iterations = Vec::with_capacity(config.repeat);

    for _ in 0..config.warmup {
        let _ = run_one(config, workspace.cwd(), None)?;
    }

    for index in 0..config.repeat {
        iterations.push(run_one(config, workspace.cwd(), Some(index))?);
    }

    let report = CommandBenchmarkReport {
        schema_version: SCHEMA_VERSION,
        benchmark: "cli_command".to_owned(),
        name: config.name.clone(),
        command_family: config.anvil_args.first().cloned(),
        generated_at_epoch: epoch_secs(),
        anvil_bin: config.anvil_bin.display().to_string(),
        fixture: config.fixture.label(),
        safe_env: SAFE_ENV_VARS
            .iter()
            .map(|(key, _)| (*key).to_owned())
            .chain(std::iter::once("ANVIL_HOME".to_owned()))
            .collect(),
        argv: redact_argv(&config.anvil_args),
        raw_argv: config.include_raw_argv.then(|| config.anvil_args.clone()),
        warmup: config.warmup,
        aggregate: aggregate(&iterations),
        iterations,
    };

    if let Some(path) = &config.output {
        write_report(&report, path)?;
    }

    Ok(report)
}

fn run_one(
    config: &CommandBenchmarkConfig,
    cwd: &Path,
    measured_index: Option<usize>,
) -> Result<CommandIteration> {
    let anvil_home = TempDir::new()?;
    let mut command = Command::new(&config.anvil_bin);
    command
        .args(&config.anvil_args)
        .current_dir(cwd)
        .env("ANVIL_HOME", anvil_home.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in SAFE_ENV_VARS {
        command.env(key, value);
    }
    in_new_process_group(&mut command);

    let start = Instant::now();
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().map(read_pipe_in_background);
    let stderr = child.stderr.take().map(read_pipe_in_background);

    // `sampler` only exists on Linux — every read is behind the same cfg, so a
    // non-Linux binding would just be dead code under `-D warnings`.
    #[cfg(target_os = "linux")]
    let mut sampler = TreeSampler::start(child.id()).ok();

    let deadline = start + config.timeout;
    let mut timed_out = false;
    let mut polled_status = None;
    loop {
        #[cfg(target_os = "linux")]
        if let Some(sampler) = sampler.as_mut() {
            sampler.tick_rss();
        }

        #[cfg(target_os = "linux")]
        if process_is_zombie(child.id()) {
            break;
        }
        if let Some(status) = child.try_wait()? {
            polled_status = Some(status);
            break;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            kill_child_group(&mut child);
            break;
        }
        thread::sleep(config.sample_interval);
    }

    #[cfg(target_os = "linux")]
    let sample = sampler.and_then(|sampler| sampler.finish().ok());
    #[cfg(not(target_os = "linux"))]
    let sample = None::<crate::budget::MeasurementSample>;

    let status = if timed_out {
        child.wait().ok()
    } else if polled_status.is_some() {
        polled_status
    } else {
        child.wait().ok()
    };
    let duration = start.elapsed();
    let stdout_bytes = join_pipe(stdout)?;
    let stderr_bytes = join_pipe(stderr)?;

    Ok(CommandIteration {
        index: measured_index.unwrap_or(0),
        duration_ms: duration.as_secs_f64() * 1000.0,
        exit_code: if timed_out {
            None
        } else {
            status.and_then(|status| status.code())
        },
        timed_out,
        stdout_bytes,
        stderr_bytes,
        cpu_pct: sample
            .as_ref()
            .map(|sample| normalise_non_negative(sample.steady_state_cpu_pct)),
        peak_rss_mib: sample
            .as_ref()
            .map(|sample| normalise_non_negative(sample.peak_rss_mib)),
    })
}

struct BenchmarkWorkspace {
    tempdir: Option<TempDir>,
    _repo: Option<SyntheticRepo>,
    cwd: PathBuf,
}

impl BenchmarkWorkspace {
    fn new(fixture: &FixtureSpec) -> Result<Self> {
        match fixture {
            FixtureSpec::Empty => {
                let tempdir = TempDir::new()?;
                let cwd = tempdir.path().to_path_buf();
                Ok(Self {
                    tempdir: Some(tempdir),
                    _repo: None,
                    cwd,
                })
            }
            FixtureSpec::Small | FixtureSpec::Default => {
                let tempdir = TempDir::new()?;
                let spec = if matches!(fixture, FixtureSpec::Small) {
                    RepoSpec::small()
                } else {
                    RepoSpec::default()
                };
                let repo = generate_repo(&spec, tempdir.path())?;
                let cwd = repo.root().to_path_buf();
                Ok(Self {
                    tempdir: Some(tempdir),
                    _repo: Some(repo),
                    cwd,
                })
            }
            FixtureSpec::Path(path) => Ok(Self {
                tempdir: None,
                _repo: None,
                cwd: path.clone(),
            }),
        }
    }

    fn cwd(&self) -> &Path {
        // Keep the tempdir considered used: its Drop owns the empty fixture root.
        let _ = self.tempdir.as_ref().map(TempDir::path);
        &self.cwd
    }
}

fn redact_argv(argv: &[String]) -> Vec<ArgShape> {
    let mut shapes = Vec::with_capacity(argv.len());
    let mut command_seen = false;
    let mut previous_wants_value = false;
    for (position, arg) in argv.iter().enumerate() {
        if arg == "--" {
            previous_wants_value = false;
            shapes.push(ArgShape {
                position,
                kind: ArgKind::Separator,
                name: None,
                has_value: false,
                sensitive: false,
            });
            continue;
        }
        if !command_seen && !arg.starts_with('-') {
            command_seen = true;
            previous_wants_value = false;
            shapes.push(ArgShape {
                position,
                kind: ArgKind::Command,
                name: Some(arg.clone()),
                has_value: false,
                sensitive: false,
            });
        } else if let Some(flag) = arg.strip_prefix("--") {
            let (name, has_value) = flag
                .split_once('=')
                .map_or((flag, false), |(name, _)| (name, true));
            let sensitive = is_sensitive_name(name);
            previous_wants_value = !has_value && sensitive;
            shapes.push(ArgShape {
                position,
                kind: ArgKind::Flag,
                name: Some(name.to_owned()),
                has_value,
                sensitive,
            });
        } else if let Some(flag) = arg.strip_prefix('-') {
            let sensitive = is_sensitive_name(flag);
            previous_wants_value = sensitive;
            shapes.push(ArgShape {
                position,
                kind: ArgKind::Flag,
                name: Some(flag.to_owned()),
                has_value: false,
                sensitive,
            });
        } else {
            let sensitive = previous_wants_value;
            previous_wants_value = false;
            shapes.push(ArgShape {
                position,
                kind: ArgKind::Positional,
                name: None,
                has_value: true,
                sensitive,
            });
        }
    }
    shapes
}

fn is_sensitive_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ["token", "secret", "password", "key", "credential"]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn normalise_non_negative(value: f64) -> f64 {
    if value.abs() < f64::EPSILON {
        0.0
    } else {
        value.max(0.0)
    }
}

fn aggregate(iterations: &[CommandIteration]) -> CommandAggregate {
    let mut durations = iterations
        .iter()
        .map(|iteration| iteration.duration_ms)
        .collect::<Vec<_>>();
    durations.sort_by(f64::total_cmp);
    let cpu = sorted_some(iterations.iter().filter_map(|iteration| iteration.cpu_pct));
    let rss = sorted_some(
        iterations
            .iter()
            .filter_map(|iteration| iteration.peak_rss_mib),
    );
    CommandAggregate {
        samples: iterations.len(),
        failures: iterations
            .iter()
            .filter(|iteration| iteration.timed_out || iteration.exit_code != Some(0))
            .count(),
        timeouts: iterations
            .iter()
            .filter(|iteration| iteration.timed_out)
            .count(),
        duration_min_ms: durations.first().copied().unwrap_or(0.0),
        duration_mean_ms: mean(&durations),
        duration_median_ms: percentile(&durations, 50),
        duration_p95_ms: percentile(&durations, 95),
        duration_p99_ms: percentile(&durations, 99),
        stdout_max_bytes: iterations
            .iter()
            .map(|iteration| iteration.stdout_bytes)
            .max()
            .unwrap_or(0),
        stderr_max_bytes: iterations
            .iter()
            .map(|iteration| iteration.stderr_bytes)
            .max()
            .unwrap_or(0),
        cpu_p95_pct: (!cpu.is_empty()).then(|| percentile(&cpu, 95)),
        cpu_max_pct: cpu.last().copied(),
        peak_rss_p95_mib: (!rss.is_empty()).then(|| percentile(&rss, 95)),
        peak_rss_max_mib: rss.last().copied(),
    }
}

fn sorted_some(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut values = values.collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    values
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn percentile(sorted_values: &[f64], percentile: usize) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let rank = (percentile * sorted_values.len()).div_ceil(100);
    sorted_values[rank.saturating_sub(1).min(sorted_values.len() - 1)]
}

fn write_report(report: &CommandBenchmarkReport, path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = report
        .to_json()
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(json.as_bytes())
}

fn read_pipe_in_background<R>(mut reader: R) -> thread::JoinHandle<io::Result<u64>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buf = [0_u8; 8192];
        let mut total = 0_u64;
        loop {
            let read = reader.read(&mut buf)?;
            if read == 0 {
                return Ok(total);
            }
            total = total.saturating_add(read as u64);
        }
    })
}

fn join_pipe(handle: Option<thread::JoinHandle<io::Result<u64>>>) -> Result<u64> {
    match handle {
        Some(handle) => handle
            .join()
            .map_err(|_| "pipe reader panicked")?
            .map_err(Into::into),
        None => Ok(0),
    }
}

// Linux-only: the sole caller (the zombie check in the sampling loop) and the
// unit test are both `#[cfg(target_os = "linux")]`.
#[cfg(target_os = "linux")]
fn process_is_zombie(pid: u32) -> bool {
    let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) else {
        return false;
    };
    stat.rfind(") ")
        .and_then(|idx| stat.get(idx + 2..))
        .and_then(|rest| rest.split_whitespace().next())
        .is_some_and(|state| state == "Z")
}

fn kill_child_group(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, killpg};
        use nix::unistd::Pid;
        if let Ok(pgid) = i32::try_from(child.id()) {
            let _ = killpg(Pid::from_raw(pgid), Signal::SIGKILL);
        }
    }
    let _ = child.kill();
}

fn next_value(args: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<String> {
    let value = args
        .next()
        .ok_or_else(|| format!("{flag} requires a value"))?;
    os_to_string(value)
}

fn os_to_string(value: OsString) -> Result<String> {
    value
        .into_string()
        .map_err(|_| "argument is not valid UTF-8".into())
}

fn parse_usize(value: &str, flag: &str) -> Result<usize> {
    value
        .parse()
        .map_err(|err| format!("{flag} expects a positive integer: {err}").into())
}

fn parse_u64(value: &str, flag: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|err| format!("{flag} expects a positive integer: {err}").into())
}

fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_command_report_round_trips() {
        let iterations = vec![CommandIteration {
            index: 0,
            duration_ms: 10.0,
            exit_code: Some(0),
            timed_out: false,
            stdout_bytes: 3,
            stderr_bytes: 0,
            cpu_pct: Some(1.0),
            peak_rss_mib: Some(2.0),
        }];
        let report = CommandBenchmarkReport {
            schema_version: SCHEMA_VERSION,
            benchmark: "cli_command".to_owned(),
            name: "status".to_owned(),
            command_family: Some("status".to_owned()),
            generated_at_epoch: 0,
            anvil_bin: "target/release/anvil".to_owned(),
            fixture: "empty".to_owned(),
            safe_env: vec!["ANVIL_HOME".to_owned()],
            argv: redact_argv(&["status".to_owned(), "--token=secret".to_owned()]),
            raw_argv: None,
            warmup: 1,
            aggregate: aggregate(&iterations),
            iterations,
        };

        let parsed: CommandBenchmarkReport =
            serde_json::from_str(&report.to_json().unwrap()).unwrap();
        assert_eq!(parsed.schema_version, SCHEMA_VERSION);
        assert_eq!(parsed.aggregate.samples, 1);
        assert!(parsed.raw_argv.is_none());
    }

    #[test]
    fn sensitive_flag_values_are_marked_sensitive() {
        let shapes = redact_argv(&[
            "status".to_owned(),
            "--token".to_owned(),
            "secret".to_owned(),
            "-password".to_owned(),
            "also-secret".to_owned(),
            "--plain".to_owned(),
            "safe".to_owned(),
        ]);

        assert!(
            shapes[2].sensitive,
            "long sensitive flag value is sensitive"
        );
        assert!(
            shapes[4].sensitive,
            "short sensitive flag value is sensitive"
        );
        assert!(!shapes[6].sensitive, "plain flag value is not sensitive");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn unreadable_proc_stat_is_not_treated_as_zombie() {
        assert!(!process_is_zombie(u32::MAX));
    }
}
