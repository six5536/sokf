//! This module builds each `git` command that sokf runs, so that every one
//! reads the repository of the directory where it runs, whatever repository
//! the environment names (the concern "Running git" of
//! design-core-library). It also holds the runner that reads objects of the
//! repository, which the scan of `new` and the check of a `verified` entry
//! share.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
};

/// `LOCAL` holds the variables that point git at a repository, once in each
/// process.
static LOCAL: OnceLock<Vec<String>> = OnceLock::new();

/// `KNOWN` holds the variables that point git at a repository as git 2.51
/// lists them, without those of `CONFIG`, so that a git that can't list
/// them, or a wrapper on the path that refuses the option, still has them
/// cleared. It also holds two variables that the list leaves out and that
/// name the state of the user's repository: `GIT_QUARANTINE_PATH`, which
/// git sets beside `GIT_OBJECT_DIRECTORY` for a hook that receives a push,
/// and in which git refuses to update a ref, such as the branch of a clone,
/// and `GIT_ATTR_SOURCE`, the tree of a `git --attr-source`, which a clone
/// lacks, so that its checkout fails.
const KNOWN: [&str; 15] = [
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
    "GIT_QUARANTINE_PATH",
    "GIT_ATTR_SOURCE",
];

/// `CONFIG` holds the variables of `git rev-parse --local-env-vars` that
/// carry the configuration of the command line, the user's `git -c` and
/// `GIT_CONFIG_KEY_<N>`, which name no repository. git itself keeps them
/// when it runs a command in another repository, such as a submodule.
const CONFIG: [&str; 2] = ["GIT_CONFIG_PARAMETERS", "GIT_CONFIG_COUNT"];

/// `local` returns the variables that point git at a repository, such as
/// `GIT_DIR` and `GIT_INDEX_FILE`: those of `KNOWN`, and each other one
/// that `git rev-parse --local-env-vars` lists, without those of `CONFIG`,
/// so that the list follows the git that runs. The probe itself runs
/// without any variable of git's, so that it reads no repository either.
fn local() -> &'static [String] {
    LOCAL.get_or_init(|| {
        let mut probe = Command::new("git");
        probe
            .args(["rev-parse", "--local-env-vars"])
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("GIT_") {
                probe.env_remove(key);
            }
        }
        let listed = probe
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .unwrap_or_default();
        merged(&listed)
    })
}

/// `merged` returns the variables of `KNOWN`, and each other one of
/// `listed`, the output of `git rev-parse --local-env-vars`, that isn't one
/// of `CONFIG`.
fn merged(listed: &str) -> Vec<String> {
    let mut local: Vec<String> = KNOWN.iter().map(|key| (*key).to_string()).collect();
    for key in listed.lines().map(str::trim) {
        if key.starts_with("GIT_") && !CONFIG.contains(&key) && !local.iter().any(|k| k == key) {
            local.push(key.to_string());
        }
    }
    local
}

/// `command` returns a `git` command without the variables that point git
/// at a repository, such as the `GIT_DIR` that a commit hook exports, but
/// with the user's configuration of the command line. git asks for no user
/// name or password on the terminal, so a command that needs one fails, and
/// the user's credential helpers still answer. Each caller sets the
/// directory and adds what only its own commands need.
// sokf-implements: CLI-1_AC-19
pub(crate) fn command() -> Command {
    let mut cmd = Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    for key in local() {
        cmd.env_remove(key);
    }
    cmd
}

/// `PATHSPECS` holds the settings of the pathspecs that git refuses beside
/// `GIT_LITERAL_PATHSPECS`.
pub(crate) const PATHSPECS: [&str; 3] = [
    "GIT_GLOB_PATHSPECS",
    "GIT_NOGLOB_PATHSPECS",
    "GIT_ICASE_PATHSPECS",
];

/// `literal` makes `cmd` read each pathspec literally, with
/// `GIT_LITERAL_PATHSPECS`, and clears the other settings of the
/// pathspecs, such as the `GIT_GLOB_PATHSPECS` of a `git --glob-pathspecs`,
/// which git refuses beside it.
pub(crate) fn literal(cmd: &mut Command) -> &mut Command {
    cmd.env("GIT_LITERAL_PATHSPECS", "1");
    for key in PATHSPECS {
        cmd.env_remove(key);
    }
    cmd
}

/// `MIN_GIT` is the oldest git, as `(major, minor)`, that has every option
/// that the runner's callers use, such as `config --type=bool`.
pub(crate) const MIN_GIT: (u32, u32) = (2, 18);

/// `NO_LAZY_FETCH` is the oldest git that honours `GIT_NO_LAZY_FETCH`.
pub(crate) const NO_LAZY_FETCH: (u32, u32) = (2, 44);

/// A `Partial` is the kind of a partial clone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Partial {
    /// A clone that may lack blobs.
    Blobless,
    /// A clone that may lack trees too.
    Treeless,
}

/// A `Git` runs `git` in one directory on the repository there, to read
/// its objects.
#[derive(Debug, Clone)]
pub(crate) struct Git {
    /// The directory.
    pub(crate) dir: PathBuf,
}

/// `VERSION` holds the version of the `git` on the path, once in each
/// process.
static VERSION: OnceLock<Option<(u32, u32)>> = OnceLock::new();

impl Git {
    /// `new` returns a runner for `dir`. It runs no `git`.
    pub(crate) fn new(dir: &Path) -> Self {
        Git {
            dir: dir.to_path_buf(),
        }
    }

    /// `head` returns the commit that `HEAD` names, when it names one.
    pub(crate) fn head(&self) -> Option<String> {
        self.text(&["rev-parse", "--verify", "-q", "HEAD^{commit}"])
    }

    /// `is_shallow` returns whether the repository is a shallow clone,
    /// whose history stops at a commit that git shows as adding every
    /// file.
    pub(crate) fn is_shallow(&self) -> bool {
        self.text(&["rev-parse", "--is-shallow-repository"])
            .is_some_and(|out| out == "true")
    }

    /// `command` returns a `git` command in the runner's directory, as
    /// `command` builds it, so that it reads the repository there whatever
    /// `GIT_DIR` the environment holds and never asks on the terminal, and
    /// keeps the configuration of the command line, such as the
    /// `safe.directory` of a CI job. It also reads each pathspec literally,
    /// so a directory named with `*` or `[` means itself, and clears the
    /// other settings of the pathspecs that a `git --glob-pathspecs`
    /// exports to a hook, which git refuses beside the literal one. It
    /// never fetches an object that a partial clone lacks, where git can
    /// tell it not to.
    pub(crate) fn command(&self) -> Command {
        let mut cmd = command();
        literal(&mut cmd)
            .current_dir(&self.dir)
            .env("GIT_NO_LAZY_FETCH", "1")
            .stderr(Stdio::null());
        cmd
    }

    /// `run` runs `git` with `args`, and returns its standard output when
    /// it succeeds.
    pub(crate) fn run(&self, args: &[&str]) -> Option<Vec<u8>> {
        let out = self
            .command()
            .args(args)
            .stdin(Stdio::null())
            .output()
            .ok()?;
        out.status.success().then_some(out.stdout)
    }

    /// `text` runs `git` with `args`, and returns its output trimmed, when
    /// it succeeds and prints something.
    pub(crate) fn text(&self, args: &[&str]) -> Option<String> {
        let out = self.run(args)?;
        let text = String::from_utf8_lossy(&out).trim().to_string();
        (!text.is_empty()).then_some(text)
    }

    /// `is_top` returns whether the directory is the top of a git
    /// repository's worktree, so that a path from the repository root means
    /// the same in git as on disk.
    pub(crate) fn is_top(&self) -> bool {
        let Some(top) = self.text(&["rev-parse", "--show-toplevel"]) else {
            return false;
        };
        match (fs::canonicalize(top), fs::canonicalize(&self.dir)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }

    /// `partial` returns the kind of a partial clone, when the repository's
    /// configuration names a promisor remote or sets
    /// `extensions.partialclone`. A clone is treeless when any filter of
    /// its remotes leaves out trees: one that names trees, alone or combined
    /// with another filter, or one that keeps only objects of one type.
    pub(crate) fn partial(&self) -> Option<Partial> {
        let promisor = self
            .text(&[
                "config",
                "--type=bool",
                "--get-regexp",
                r"^remote\..*\.promisor$",
            ])
            .is_some_and(|out| out.lines().any(|l| l.ends_with(" true")));
        if !promisor
            && self
                .text(&["config", "--get", "extensions.partialclone"])
                .is_none()
        {
            return None;
        }
        let filters = self
            .text(&[
                "config",
                "--get-regexp",
                r"^remote\..*\.partialclonefilter$",
            ])
            .unwrap_or_default();
        Some(
            if filters.contains("tree:") || filters.contains("object:type=") {
                Partial::Treeless
            } else {
                Partial::Blobless
            },
        )
    }

    /// `merge_base` returns the commit where `head` and `commit` parted,
    /// when they have one.
    pub(crate) fn merge_base(&self, head: Option<&str>, commit: &str) -> Option<String> {
        self.text(&["merge-base", head?, commit])
    }

    /// `blobs` reads the text of each blob of `oids` through one `git
    /// cat-file --batch`, and returns it by its id. A blob that the
    /// repository lacks, whose text isn't UTF-8, or that `cat-file` didn't
    /// give, because it failed to start or stopped early, has no text.
    pub(crate) fn blobs<'a>(
        &self,
        oids: impl IntoIterator<Item = &'a str>,
    ) -> BTreeMap<String, Option<String>> {
        // Each distinct blob is read once, in the order of the map's keys,
        // which is the order of the batch.
        let mut texts: BTreeMap<String, Option<String>> = oids
            .into_iter()
            .map(|oid| (oid.to_string(), None))
            .collect();
        if texts.is_empty() {
            return texts;
        }
        let mut names = String::new();
        for oid in texts.keys() {
            let _ = writeln!(names, "{oid}");
        }
        if let Ok(mut child) = self
            .command()
            .args(["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            let mut input = child.stdin.take();
            // A writer thread keeps a large batch from filling both pipes.
            let writer = std::thread::spawn(move || {
                if let Some(stdin) = input.as_mut() {
                    let _ = stdin.write_all(names.as_bytes());
                }
                drop(input);
            });
            if let Some(stdout) = child.stdout.take() {
                let mut reader = BufReader::new(stdout);
                for text in texts.values_mut() {
                    let mut header = String::new();
                    if reader.read_line(&mut header).unwrap_or(0) == 0 {
                        break;
                    }
                    // `<oid> blob <size>`, or `<oid> missing`.
                    let Some(size) = header
                        .split_whitespace()
                        .nth(2)
                        .and_then(|s| s.parse::<usize>().ok())
                    else {
                        continue;
                    };
                    let mut body = vec![0; size + 1];
                    if reader.read_exact(&mut body).is_err() {
                        break;
                    }
                    body.pop();
                    *text = String::from_utf8(body).ok();
                }
            }
            let _ = writer.join();
            let _ = child.wait();
        }
        texts
    }
}

/// A `Raw` is one entry of the output of a plumbing diff or of `git log`
/// with `--raw -z`: the blobs of the file before and after, which are
/// `None` for the id of zeros that stands for an absent file, the status,
/// such as `M` or `R100`, the file's paths, two for a rename or a copy and
/// one otherwise, and the last line of `git log`'s format before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Raw {
    pub(crate) before: Option<String>,
    pub(crate) after: Option<String>,
    pub(crate) status: String,
    pub(crate) paths: Vec<String>,
    pub(crate) header: Option<String>,
}

/// `raw` parses the output of a diff or of `git log` with `--raw -z`, in
/// which each entry is `:<modes> <old> <new> <status>` and then its paths,
/// and each other field is a line of `git log`'s format, each field ended
/// by a NUL.
pub(crate) fn raw(out: &[u8]) -> Vec<Raw> {
    let blob = |oid: &str| (!oid.bytes().all(|b| b == b'0')).then(|| oid.to_string());
    let mut found = Vec::new();
    let mut header = None;
    let mut fields = out.split(|b| *b == 0).map(String::from_utf8_lossy);
    while let Some(field) = fields.next() {
        // `git log` ends the format's line with a line break before the
        // first entry of its commit.
        let field = field.trim_start_matches('\n');
        let Some(meta) = field.strip_prefix(':') else {
            if !field.is_empty() {
                header = Some(field.to_string());
            }
            continue;
        };
        let parts: Vec<&str> = meta.split(' ').collect();
        let status = parts.get(4).copied().unwrap_or_default().to_string();
        let count = if status.starts_with(['R', 'C']) { 2 } else { 1 };
        let paths: Vec<String> = fields
            .by_ref()
            .take(count)
            .map(std::borrow::Cow::into_owned)
            .collect();
        if paths.len() < count {
            break;
        }
        found.push(Raw {
            before: parts.get(2).and_then(|o| blob(o)),
            after: parts.get(3).and_then(|o| blob(o)),
            status,
            paths,
            header: header.clone(),
        });
    }
    found
}

/// `version` returns the version of the `git` on the path, as `(major,
/// minor)`, when `git --version` gives one. It asks git once in each
/// process.
pub(crate) fn version() -> Option<(u32, u32)> {
    *VERSION.get_or_init(ask_version)
}

/// `ask_version` asks `git --version` for the version of the `git` on the
/// path, such as `git version 2.51.1` or `git version 2.39.3 (Apple
/// Git-145)`.
fn ask_version() -> Option<(u32, u32)> {
    let out = command()
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let number = text.split_whitespace().nth(2)?;
    let mut parts = number.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts
        .next()?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok()?;
    Some((major, minor))
}

/// `diff` returns the unified diff of the text `found` against the text
/// `expected` of the file `rel`, with the paths `a/<rel>` and `b/<rel>`,
/// through `git diff --no-index` in a temporary directory, or an empty text
/// when the two are equal. git reads no attributes of the user's or of the
/// system, so that no textconv or conversion changes either side, and finds
/// no repository above the temporary directory. It returns why it failed
/// when it can't write the texts, git can't start, or git fails.
pub(crate) fn diff(rel: &str, expected: &str, found: &str) -> Result<String, String> {
    let dir = crate::temp_dir::TempDir::new("diff")
        .map_err(|e| format!("no temporary directory: {e}"))?;
    let write = |p: &Path, text: &str| -> std::io::Result<()> {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(p, text)
    };
    let attributes = dir.path().join("attributes");
    let files = [
        (dir.path().join("a").join(rel), expected),
        (dir.path().join("b").join(rel), found),
        (attributes.clone(), ""),
    ];
    for (p, text) in &files {
        write(p, text).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    let ceiling = dir.path().parent().unwrap_or(dir.path());
    let out = command()
        .current_dir(dir.path())
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_CEILING_DIRECTORIES", ceiling)
        .args(["-c", "core.autocrlf=false", "-c", "core.eol=lf"])
        .args(["-c", "core.quotePath=false"])
        .arg("-c")
        .arg(format!("core.attributesFile={}", attributes.display()))
        .args([
            "diff",
            "--no-index",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--no-prefix",
            "--",
        ])
        .arg(format!("a/{rel}"))
        .arg(format!("b/{rel}"))
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run `git`: {e}"))?;
    // `git diff --no-index` exits with 1 when the files differ.
    match out.status.code() {
        Some(0 | 1) => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        _ => Err(format!(
            "`git diff` failed on {rel}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_diff_names_both_sides_of_the_file() {
        let d = diff(".claude/settings.json", "x\n", "y\n").unwrap();
        assert!(d.contains("--- a/.claude/settings.json\n"), "{d}");
        assert!(d.contains("+++ b/.claude/settings.json\n"), "{d}");
        assert!(d.contains("-x\n+y\n"), "{d}");
        assert_eq!(diff("AGENTS.md", "x\n", "x\n").unwrap(), "");
    }

    // The list of the git on the path begins with `KNOWN` and keeps the
    // configuration of the command line.
    #[test]
    fn the_list_of_the_git_on_the_path_holds_the_known_variables() {
        assert!(local().starts_with(&merged("")), "{:?}", local());
        for key in ["GIT_CONFIG_PARAMETERS", "GIT_CONFIG_COUNT"] {
            assert!(!local().iter().any(|k| k == key), "{:?}", local());
        }
    }

    // A git that lists no variable still clears each of `KNOWN`, and one that
    // lists a new variable clears it too, once, while the configuration of
    // the command line stays.
    #[test]
    fn the_list_holds_each_known_variable_and_each_new_one() {
        assert_eq!(merged(""), KNOWN.map(str::to_string).to_vec());
        for key in ["GIT_DIR", "GIT_QUARANTINE_PATH", "GIT_ATTR_SOURCE"] {
            assert!(merged("").iter().any(|k| k == key), "{key}");
        }
        let local =
            merged("GIT_DIR\nGIT_CONFIG_COUNT\nGIT_NEW_REPOSITORY\nGIT_CONFIG_PARAMETERS\n");
        let mut expected = KNOWN.map(str::to_string).to_vec();
        expected.push("GIT_NEW_REPOSITORY".to_string());
        assert_eq!(local, expected);
    }

    #[test]
    fn a_command_clears_each_variable_and_never_prompts() {
        let cmd = command();
        let envs: Vec<_> = cmd.get_envs().collect();
        for key in local() {
            assert!(
                envs.contains(&(std::ffi::OsStr::new(key), None)),
                "{key} in {envs:?}"
            );
        }
        assert!(envs.contains(&(
            std::ffi::OsStr::new("GIT_TERMINAL_PROMPT"),
            Some(std::ffi::OsStr::new("0"))
        )));
    }

    #[test]
    fn a_command_of_the_runner_reads_pathspecs_literally_and_fetches_nothing() {
        let git = Git::new(Path::new("."));
        let cmd = git.command();
        let envs: Vec<_> = cmd.get_envs().collect();
        let os = std::ffi::OsStr::new;
        for key in ["GIT_LITERAL_PATHSPECS", "GIT_NO_LAZY_FETCH"] {
            assert!(
                envs.contains(&(os(key), Some(os("1")))),
                "{key} in {envs:?}"
            );
        }
        for key in PATHSPECS {
            assert!(envs.contains(&(os(key), None)), "{key} in {envs:?}");
        }
    }

    #[test]
    fn a_raw_output_gives_each_entry_with_its_paths_and_header() {
        let out = b"commit aaa 200\0\n:100644 100644 1111 2222 M\0b/x.md\0\0commit bbb 100\0\n:100644 100644 1111 1111 R100\0a/x.md\0b/x.md\0\0commit ccc 50\0\n:000000 100644 0000 1111 A\0a/x.md\0";
        let found = raw(out);
        assert_eq!(found.len(), 3, "{found:?}");
        assert_eq!(found[0].header.as_deref(), Some("commit aaa 200"));
        assert_eq!(found[0].before.as_deref(), Some("1111"));
        assert_eq!(found[0].after.as_deref(), Some("2222"));
        assert_eq!(found[1].paths, ["a/x.md", "b/x.md"]);
        assert_eq!(found[1].status, "R100");
        assert_eq!(found[2].before, None);
        assert_eq!(found[2].header.as_deref(), Some("commit ccc 50"));
        let cut = raw(b":100644 100644 1 2 R90\0only");
        assert!(cut.is_empty(), "{cut:?}");
        let none = raw(b"");
        assert!(none.is_empty(), "{none:?}");
    }
}
