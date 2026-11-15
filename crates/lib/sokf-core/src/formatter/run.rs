//! This module runs the formatter: it finds the program, reads its version,
//! and formats a batch of texts in one temporary directory outside the
//! repository (CONF-10_AC-5).

use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use super::{preset::Formatter, sample::sample};
use crate::{RepoRoot, temp_dir::TempDir};

/// An `Outcome` is what a batch settled of one text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The formatter gave the text this form.
    Form(String),
    /// The formatter fails on the text for a reason of the text, and the
    /// store keeps the failure.
    Failed,
    /// The formatter lasted longer than this limit on the text alone, in a
    /// language with a sample, and the store keeps the failure with the
    /// limit.
    Hung(Duration),
    /// The batch didn't run the text alone, because every pair that holds
    /// it holds a text that the formatter fails on for good.
    Skipped,
    /// The batch settled nothing of the text, for this reason, so the next
    /// run formats it again.
    Unsettled(Fault),
}

/// A `Fault` is why a batch left a text unsettled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// The formatter failed on a text in a language that has no sample.
    NoSample,
    /// The formatter lasted longer than the limit on a text alone in a
    /// language with no sample, or the batch tried no text after a text
    /// that lasted longer than the limit alone, in any language.
    Hang,
    /// The formatter failed on a sample, so its environment is at fault.
    Sample,
    /// A run of the formatter after the batch didn't start, or its file
    /// couldn't be written.
    Broken,
}

impl Fault {
    /// `stops` returns whether the fault stops the formatting of the rest
    /// of the command: every fault but a language with no sample, so that
    /// a command runs the formatter past its limit at most twice.
    pub fn stops(self) -> bool {
        self != Fault::NoSample
    }
}

/// `program` returns the path of the formatter's program: the program as
/// the command names it, from the repository root when it holds a `/`, or
/// else the one in `node_modules/.bin` at the root, or else the one on the
/// path. It returns `None` when no such file exists.
pub fn program(root: &RepoRoot, name: &str) -> Option<PathBuf> {
    if name.contains('/') || name.contains(std::path::MAIN_SEPARATOR) {
        let path = Path::new(name);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.as_path().join(path)
        };
        return path.is_file().then_some(path);
    }
    let bin = root.as_path().join("node_modules").join(".bin");
    let dirs = std::iter::once(bin).chain(
        std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
            .unwrap_or_default(),
    );
    for dir in dirs {
        for candidate in candidates(&dir, name) {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// `candidates` returns the files in `dir` that can be the program `name`:
/// on Windows the name with each executable extension, because npm puts a
/// shell script without an extension beside the `.cmd` file, and elsewhere
/// the name itself.
fn candidates(dir: &Path, name: &str) -> Vec<PathBuf> {
    if cfg!(windows) && Path::new(name).extension().is_none() {
        ["exe", "cmd", "bat"]
            .iter()
            .map(|ext| dir.join(format!("{name}.{ext}")))
            .collect()
    } else {
        vec![dir.join(name)]
    }
}

/// `version` returns the version of the formatter whose program is at
/// `path`. For a detected formatter, it reads the `version` of the
/// `package.json` of its npm package, so that it starts no process.
/// Otherwise it asks the program for `--version`, in the system's temporary
/// directory, so that the program touches no file of the repository, for
/// `limit` at most. It returns `None` when neither gives one, and `Err`
/// with the reason when the program lasts longer than `limit`, so that a
/// command spends the limit on the program no more.
// sokf-implements: CACHE-2_AC-7
pub fn version(
    root: &RepoRoot,
    path: &Path,
    package: Option<&str>,
    limit: Duration,
) -> Result<Option<String>, String> {
    if let Some(package) = package
        && let Some(v) = package_version(root, path, package)
    {
        return Ok(Some(format!("{package}@{v}")));
    }
    let mut cmd = command(path);
    cmd.current_dir(std::env::temp_dir()).arg("--version");
    let Ok(out) = run(cmd, Some(limit)) else {
        return Ok(None);
    };
    if out.timed_out {
        return Err(format!(
            "`{} --version` ran for longer than the limit",
            path.display()
        ));
    }
    if !out.success {
        return Ok(None);
    }
    let text = if out.stdout.trim().is_empty() {
        out.stderr
    } else {
        out.stdout
    };
    Ok(Some(text.trim().to_string()))
}

/// `package_version` returns the version in the `package.json` of the npm
/// package `package`: the one in the directories above the program at
/// `path` once its links are followed, as npm installs it, or else the one
/// in `node_modules/<package>` at the root, as pnpm, whose programs are
/// scripts, and npm on Windows install it.
pub fn package_version(root: &RepoRoot, path: &Path, package: &str) -> Option<String> {
    let named = |file: &Path| -> Option<String> {
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(file).ok()?).ok()?;
        (value.get("name").and_then(|n| n.as_str()) == Some(package))
            .then(|| value.get("version")?.as_str().map(str::to_string))
            .flatten()
    };
    let real = fs::canonicalize(path).ok();
    real.iter()
        .flat_map(|r| r.ancestors().skip(1).take(5))
        .find_map(|dir| named(&dir.join("package.json")))
        .or_else(|| {
            named(
                &root
                    .as_path()
                    .join("node_modules")
                    .join(package)
                    .join("package.json"),
            )
        })
}

/// `format` formats `texts`, each an extension and a text, with
/// `formatter`, whose program is at `program`, and returns the outcome of
/// each text. It writes each text into one new temporary directory as
/// `<n>.<extension>`, writes the settings and an empty ignore file there,
/// and runs the formatter's command once over the files, and stops each run
/// that lasts longer than `limit`. When that run fails, it keeps the form
/// of each text that the run changed, runs the sample of each language of
/// the other texts, and then runs the formatter on each of them alone, as a
/// new file, to find the texts that it fails on. `others` gives, for each
/// text, the other texts of its pairs, or nothing where a pair holds a text
/// outside the batch, and a text whose other texts the formatter fails on
/// for good doesn't run alone. It returns `Err` with the reason when the
/// formatter doesn't start.
// sokf-implements: CACHE-2_AC-5
// sokf-implements: CONF-10_AC-5
pub fn format(
    formatter: &Formatter,
    program: &Path,
    texts: &[(&str, &str)],
    others: &[Vec<usize>],
    limit: Duration,
) -> Result<Vec<Outcome>, String> {
    let dir = TempDir::new("format").map_err(|e| format!("no temporary directory: {e}"))?;
    let settings = dir.path().join(&formatter.settings_file);
    let empty = dir.path().join("sokf-empty-ignore");
    write(&settings, &formatter.settings)?;
    write(&empty, "")?;
    let names: Vec<String> = texts
        .iter()
        .enumerate()
        .map(|(i, (ext, _))| format!("{i}.{ext}"))
        .collect();
    for (name, (_, text)) in names.iter().zip(texts) {
        write(&dir.path().join(name), text)?;
    }
    let all: Vec<&str> = names.iter().map(String::as_str).collect();
    let run_on = |files: &[&str]| {
        run_on(
            formatter,
            program,
            dir.path(),
            (&settings, &empty),
            files,
            limit,
        )
    };
    let first = run_on(&all)?;
    let read = |name: &str| fs::read_to_string(dir.path().join(name)).ok();
    if first.success {
        return Ok(names
            .iter()
            .map(|n| read(n).map_or(Outcome::Failed, Outcome::Form))
            .collect());
    }
    // A text that the failed run changed was formatted, whatever else
    // failed. A run that sokf stopped may have written part of a file, so
    // it formatted nothing.
    // sokf-implements: CLI-4_AC-75
    let mut outcomes: Vec<Option<Outcome>> = names
        .iter()
        .zip(texts)
        .map(|(name, (_, text))| {
            (!first.timed_out)
                .then(|| read(name))
                .flatten()
                .filter(|f| f != text)
                .map(Outcome::Form)
        })
        .collect();
    // A run after the batch that doesn't start leaves the texts that it
    // didn't settle unsettled, and the forms of the batch stay.
    let rest = match settle_alone(dir.path(), texts, others, &mut outcomes, limit, run_on) {
        Ok(()) => Fault::Hang,
        Err(_) => Fault::Broken,
    };
    Ok(outcomes
        .into_iter()
        .map(|o| o.unwrap_or(Outcome::Unsettled(rest)))
        .collect())
}

/// `settle_alone` settles each text that a failed batch left in
/// `outcomes`: it runs the samples of their languages, in `dir`, with
/// `run_on`, which stops each run at `limit`, and then each text alone,
/// until one lasts longer than the limit, which it records as a hang with
/// `limit`, and leaves the texts after that one in `outcomes` unsettled.
fn settle_alone(
    dir: &Path,
    texts: &[(&str, &str)],
    others: &[Vec<usize>],
    outcomes: &mut [Option<Outcome>],
    limit: Duration,
    run_on: impl Fn(&[&str]) -> Result<Output, String>,
) -> Result<(), String> {
    // The samples of the languages of the other texts run together, before
    // any text runs alone. A sample that fails shows a fault of the
    // environment, which fails every text, so the run stores no failure.
    // sokf-implements: CLI-4_AC-70
    // sokf-implements: CLI-4_AC-71
    let exts: BTreeSet<&str> = texts
        .iter()
        .zip(outcomes.iter())
        .filter(|(_, o)| o.is_none())
        .map(|((ext, _), _)| *ext)
        .collect();
    if !samples_pass(dir, &exts, &run_on)? {
        for o in outcomes.iter_mut().filter(|o| o.is_none()) {
            *o = Some(Outcome::Unsettled(Fault::Sample));
        }
        return Ok(());
    }
    // Each other text runs alone after the samples, even one that the
    // batch held alone, because a first run can fail or last longer for a
    // cold start, until one lasts longer than the limit. A text runs alone
    // as a new file, which no process that a stopped run left behind
    // writes. A failure in a language with no sample may be the
    // environment's, so it is unsettled.
    // sokf-implements: CLI-4_AC-72
    // sokf-implements: CLI-4_AC-73
    // sokf-implements: CLI-4_AC-74
    // sokf-implements: CLI-4_AC-76
    for i in 0..texts.len() {
        if outcomes[i].is_some() {
            continue;
        }
        if !others[i].is_empty()
            && others[i]
                .iter()
                .all(|&j| outcomes[j] == Some(Outcome::Failed))
        {
            outcomes[i] = Some(Outcome::Skipped);
            continue;
        }
        let (ext, text) = texts[i];
        let alone = format!("{i}-alone.{ext}");
        write(&dir.join(&alone), text)?;
        let one = run_on(&[&alone])?;
        outcomes[i] = Some(if one.success {
            fs::read_to_string(dir.join(&alone)).map_or(Outcome::Failed, Outcome::Form)
        } else if sample(ext).is_some() {
            if one.timed_out {
                Outcome::Hung(limit)
            } else {
                Outcome::Failed
            }
        } else if one.timed_out {
            Outcome::Unsettled(Fault::Hang)
        } else {
            Outcome::Unsettled(Fault::NoSample)
        });
        if one.timed_out {
            break;
        }
    }
    Ok(())
}

/// `samples_pass` writes the sample of each of `exts` that has one into
/// `dir` as `sample.<extension>`, and returns whether `run_on` succeeds on
/// them in one run. With no sample, it starts no run and returns `true`.
fn samples_pass(
    dir: &Path,
    exts: &BTreeSet<&str>,
    run_on: impl Fn(&[&str]) -> Result<Output, String>,
) -> Result<bool, String> {
    let mut files = Vec::new();
    for ext in exts {
        if let Some(text) = sample(ext) {
            let name = format!("sample.{ext}");
            write(&dir.join(&name), text)?;
            files.push(name);
        }
    }
    if files.is_empty() {
        return Ok(true);
    }
    let files: Vec<&str> = files.iter().map(String::as_str).collect();
    Ok(run_on(&files)?.success)
}

/// `write` writes `text` to the file at `path`, and returns the reason when
/// it can't.
fn write(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// `run_on` runs the formatter's command over `files` in `dir`, with the
/// placeholders of its arguments filled in from `paths`, the settings and
/// the empty ignore file, for `limit` at most.
fn run_on(
    formatter: &Formatter,
    program: &Path,
    dir: &Path,
    (settings, empty): (&Path, &Path),
    files: &[&str],
    limit: Duration,
) -> Result<Output, String> {
    // The run reads no output of the formatter, so a process that the
    // formatter leaves behind holds no pipe of sokf's open.
    let mut cmd = command(program);
    cmd.current_dir(dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for arg in &formatter.args {
        if arg == "{files}" {
            cmd.args(files);
        } else {
            cmd.arg(fill(arg, settings, empty));
        }
    }
    run(cmd, Some(limit)).map_err(|e| format!("`{}` doesn't start: {e}", formatter.program))
}

/// `fill` returns `arg` with `{settings}` and `{empty}` replaced by the
/// paths of the settings and of the empty ignore file.
fn fill(arg: &str, settings: &Path, empty: &Path) -> OsString {
    arg.replace("{settings}", &settings.to_string_lossy())
        .replace("{empty}", &empty.to_string_lossy())
        .into()
}

/// `command` returns a command for the program at `path` that reads no
/// input and runs without the variables that point git at a repository,
/// such as the `GIT_DIR` that a commit hook exports (CONF-10_AC-7).
// sokf-implements: CONF-10_AC-7
pub(crate) fn command(path: &Path) -> Command {
    let mut cmd = Command::new(path);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd
}

/// An `Output` is what one run of a program gave.
#[derive(Debug)]
struct Output {
    /// Whether the program exited with success.
    success: bool,
    /// Whether sokf stopped the program at the time limit.
    timed_out: bool,
    /// The program's standard output.
    stdout: String,
    /// The program's standard error.
    stderr: String,
}

/// `run` runs `cmd` and returns its output. With `limit`, it stops the
/// program when it runs for longer.
// sokf-implements: CONF-10_AC-8
fn run(mut cmd: Command, limit: Option<Duration>) -> std::io::Result<Output> {
    let mut child = cmd.spawn()?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if limit.is_some_and(|l| start.elapsed() > l) {
            stop(&mut child);
            // A process that the formatter started can hold the pipes open,
            // so the run reads no more of them.
            return Ok(Output {
                success: false,
                timed_out: true,
                stdout: String::new(),
                stderr: String::new(),
            });
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    // A process that the formatter left behind can hold the pipes open, so
    // the run waits for their end no longer than the rest of the limit.
    let left = limit.map_or(Duration::from_secs(1), |l| {
        l.saturating_sub(start.elapsed())
    });
    // A limit of many years would overflow the clock, and no pipe needs
    // longer than a day.
    let deadline = Instant::now()
        + left
            .max(Duration::from_millis(100))
            .min(Duration::from_hours(24));
    let read = |rx: std::sync::mpsc::Receiver<String>| {
        rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap_or_default()
    };
    Ok(Output {
        success: status.success(),
        timed_out: false,
        stdout: read(stdout),
        stderr: read(stderr),
    })
}

/// `stop` kills `child` and waits for it.
fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// `drain` reads `pipe` to its end on a thread of its own, so that a
/// program that writes much output never blocks on a full pipe, and sends
/// the text when the pipe ends.
fn drain<R: std::io::Read + Send + 'static>(pipe: Option<R>) -> std::sync::mpsc::Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut pipe) = pipe {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            text = String::from_utf8_lossy(&bytes).into_owned();
        }
        let _ = tx.send(text);
    });
    rx
}
