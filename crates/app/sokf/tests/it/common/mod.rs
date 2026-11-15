//! This module holds the helpers that the end-to-end test files share. The
//! helpers run the binary under test, and they copy a fixture repository for
//! the binary to run on.

#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use assert_cmd::Command;

/// `sokf` returns a new command that runs the binary under test, with a home directory
/// that no test writes unless it asks for one, so that the user's own agent settings never
/// reach a test, and without git's system configuration, such as the `core.autocrlf=true`
/// that Git for Windows sets there, so that the git that sokf runs reads the bytes that a
/// test committed.
pub fn sokf() -> Command {
    let mut cmd = Command::cargo_bin("sokf").unwrap();
    let home = std::env::temp_dir().join(format!("sokf-cli-no-home-{}", std::process::id()));
    cmd.env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("GIT_CONFIG_NOSYSTEM", "1");
    cmd
}

/// `with_hooks` returns a configuration with a `[harness.NAME]` table for each harness of
/// `harnesses` that opts it in to its hooks part, which is off by default. Each table ends
/// with its `with` line, so a test may append more keys of the last table.
pub fn with_hooks(harnesses: &[&str]) -> String {
    harnesses
        .iter()
        .map(|h| format!("[harness.{h}]\nwith = [\"hooks\"]\n"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `output` runs `cmd` and returns its stdout, its stderr, and its exit code.
pub fn output(cmd: &mut Command) -> (String, String, i32) {
    let out = cmd.output().unwrap();
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        out.status.code().unwrap(),
    )
}

/// `git` runs git in `dir`, with an identity and without signing or hooks,
/// and returns its stdout. It clears each `GIT_` variable that it inherits, such
/// as the `GIT_INDEX_FILE` that git sets for a commit hook, so that it reads
/// and writes only the repository in `dir`.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let mut cmd = std::process::Command::new("git");
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            cmd.env_remove(key);
        }
    }
    cmd.env("GIT_CONFIG_NOSYSTEM", "1");
    let out = cmd
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "tag.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// `run` runs the binary with `args` and returns its stdout, its stderr, and
/// its exit code.
pub fn run(args: &[&str]) -> (String, String, i32) {
    output(sokf().args(args))
}

/// `run_in` runs the binary with `args` in the directory `dir` and returns its
/// stdout, its stderr, and its exit code.
pub fn run_in(dir: &Path, args: &[&str]) -> (String, String, i32) {
    output(sokf().args(args).current_dir(dir))
}

/// A `Fixture` is a copy of a fixture repository in a temporary directory.
/// Dropping the `Fixture` removes the directory.
pub struct Fixture(pub PathBuf);

impl Fixture {
    pub fn new(name: &str) -> Self {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../lib/sokf-core/tests/fixtures")
            .join(name);
        let dir =
            std::env::temp_dir().join(format!("sokf-cli-{name}-{}-{}", std::process::id(), rand()));
        let _ = fs::remove_dir_all(&dir);
        copy_dir(&source, &dir);
        // The fixture's path is canonical, because the tool prints the
        // canonical root, and on macOS the temporary directory is reached
        // through a symbolic link.
        Fixture(dunce::canonicalize(&dir).unwrap())
    }

    pub fn root(&self) -> &str {
        self.0.to_str().unwrap()
    }

    /// `path` returns the path on disk of `rel`, a `/`-separated path in the
    /// copy, joined one component at a time so that it has the platform's
    /// separator, as the tool prints it.
    pub fn path(&self, rel: &str) -> PathBuf {
        rel.split('/').fold(self.0.clone(), |p, c| p.join(c))
    }

    /// `run` runs the binary on the copy, with `--repo-root` and then `args`,
    /// from the test's own directory.
    pub fn run(&self, args: &[&str]) -> (String, String, i32) {
        output(sokf().arg("--repo-root").arg(self.root()).args(args))
    }

    /// `run_here` runs the binary as `run` does, from the copy's directory.
    pub fn run_here(&self, args: &[&str]) -> (String, String, i32) {
        output(
            sokf()
                .arg("--repo-root")
                .arg(self.root())
                .args(args)
                .current_dir(&self.0),
        )
    }

    pub fn write(&self, rel: &str, text: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// `files` returns the path and the bytes of every file under the copy,
    /// sorted by path. It skips the `.sokf` cache directory.
    pub fn files(&self) -> Vec<(String, Vec<u8>)> {
        fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.file_name().is_some_and(|n| n == ".sokf") {
                    continue;
                }
                if path.is_dir() {
                    walk(base, &path, out);
                } else {
                    // The path takes forward slashes on every platform, so that a
                    // test compares it with the same text on Windows.
                    let rel = path
                        .strip_prefix(base)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    out.push((rel, fs::read(&path).unwrap()));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.0, &self.0, &mut out);
        out.sort();
        out
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `project` returns a repository with the store that `sokf init` creates.
pub fn project() -> Fixture {
    let f = Fixture::new("manifest");
    fs::remove_dir_all(f.0.join("knowledge")).unwrap();
    let (out, err, code) = f.run(&["init", "--scope", "test"]);
    assert_eq!(code, 0, "{out}{err}");
    f
}

/// `METHOD` is the folder of the files of the sokf pack, each at the path
/// where it lands in a project.
pub const METHOD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../packs/sokf/files");

/// `take_method` copies each file of the sokf pack into the repository `f`,
/// at its path, as pakf places it.
pub fn take_method(f: &Fixture) {
    copy_dir(Path::new(METHOD), &f.0);
}

/// `tree` returns every file of the repository `f`, `/.sokf/cache` and
/// `/.git` aside, by its path from the root.
pub fn tree(f: &Fixture) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let rel = path
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if rel == ".sokf/cache" || rel == ".git" {
                continue;
            }
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.push((format!("/{rel}"), fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(&f.0, &f.0, &mut out);
    out.sort();
    out
}

pub fn rand() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    std::collections::hash_map::RandomState::new()
        .build_hasher()
        .finish()
}

pub fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
