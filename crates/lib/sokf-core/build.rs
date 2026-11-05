//! This build script copies `sokf-spec/sokf.tramlines.yaml`,
//! `knowledge/formats/sokf.formats.yaml` and the harness integration's files
//! under `harness/` into `OUT_DIR`, so that the crate can embed them.
//!
//! In the workspace the files live at the repository root, outside this
//! crate, so the script copies them at build time. For a packaged crate,
//! the script falls back to the same paths under the crate's own
//! directory, where symbolic links point at the files at the root, and
//! `cargo package` packages the files that the links point at. When both
//! copies exist, the script uses the workspace copy, so the files at the
//! repository root remain the only ones anyone edits.
//!
//! The script also embeds every directory under `/packs/` that holds a
//! `pack.sokf.yaml`, the packs that `sokf pack sync builtin:<NAME>` copies
//! (D-23 of design-pack). It copies each file of each pack into `OUT_DIR`
//! and writes `OUT_DIR/packs.rs`, the list of the packs and their files. A
//! build without `/packs/` embeds no pack.
//!
//! The script also sets `SOKF_BUILD_FINGERPRINT`, a hash of what decides the
//! results of a check: the crate's sources, the embedded files, and the
//! workspace's `Cargo.lock`, which pins the tramlines engine and every other
//! dependency. The cache records the fingerprint in its version, so a build
//! that checks differently starts from an empty store (CACHE-1_AC-6). A
//! packaged crate has no lock file, and its fingerprint covers its sources.
//!
//! The lock pins a dependency that a `[patch]` table replaces with a `path`
//! by no hash of its sources, so a change to the patched crate leaves the
//! lock as it was. The fingerprint therefore also covers the files of each
//! crate that a `path` entry of a `[patch]` table names, in the workspace's
//! `Cargo.toml` or `.cargo/config.toml`, and the script runs again when one
//! of them changes.

use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// `FILES` lists the paths of the embedded files, relative to the
/// repository root.
const FILES: &[&str] = &[
    "sokf-spec/sokf.tramlines.yaml",
    "knowledge/formats/sokf.formats.yaml",
    "harness/skills/sokf/SKILL.md",
    "harness/instructions.md",
];

fn main() {
    let manifest =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"));
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let workspace = manifest.join("../../..");
    let mut hash = Fnv(0xcbf2_9ce4_8422_2325);
    for rel in FILES {
        let source = [workspace.join(rel), manifest.join(rel)]
            .into_iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| {
                panic!(
                    "{rel} is under neither {} nor {}",
                    workspace.display(),
                    manifest.display()
                )
            });
        println!("cargo:rerun-if-changed={}", source.display());
        // The embedded text has LF line endings even when a checkout on
        // Windows gives the file CRLF, so every build prints the same bytes.
        let text = fs::read_to_string(&source)
            .expect("read embedded file")
            .replace("\r\n", "\n");
        let target = out.join(rel);
        fs::create_dir_all(
            target
                .parent()
                .expect("embedded file path has a parent directory"),
        )
        .expect("create OUT_DIR subdirectory");
        fs::write(target, &text).expect("write embedded file");
        hash.update(rel.as_bytes());
        hash.update(text.as_bytes());
    }
    embed_packs(&workspace, &manifest, &out);
    let lock = workspace.join("Cargo.lock");
    if lock.is_file() {
        println!("cargo:rerun-if-changed={}", lock.display());
        hash.update(&fs::read(&lock).expect("read Cargo.lock"));
    }
    for dir in patched_crates(&workspace) {
        println!("cargo:rerun-if-changed={}", dir.display());
        let mut files = Vec::new();
        crate_files(&dir, &mut files);
        files.sort();
        for file in files {
            println!("cargo:rerun-if-changed={}", file.display());
            hash.update(
                file.strip_prefix(&dir)
                    .unwrap_or(&file)
                    .to_string_lossy()
                    .as_bytes(),
            );
            hash.update(&fs::read(&file).expect("read patched crate file"));
        }
    }
    let src = manifest.join("src");
    println!("cargo:rerun-if-changed={}", src.display());
    let mut files = Vec::new();
    sources(&src, &mut files);
    files.sort();
    for file in files {
        hash.update(
            file.strip_prefix(&src)
                .unwrap_or(&file)
                .to_string_lossy()
                .as_bytes(),
        );
        hash.update(&fs::read(&file).expect("read source file"));
    }
    println!("cargo:rustc-env=SOKF_BUILD_FINGERPRINT={:016x}", hash.0);
}

/// `embed_packs` copies each pack under `/packs/` into `OUT_DIR/packs/` and
/// writes `OUT_DIR/packs.rs`, an array of `EmbeddedPack` values. The packs
/// come from the workspace, or else from the crate's own `packs/`.
fn embed_packs(workspace: &Path, manifest: &Path, out: &Path) {
    let packs = [workspace.join("packs"), manifest.join("packs")]
        .into_iter()
        .find(|p| p.is_dir());
    let mut code = String::from("&[\n");
    if let Some(packs) = packs {
        println!("cargo:rerun-if-changed={}", packs.display());
        let mut dirs: Vec<PathBuf> = fs::read_dir(&packs)
            .expect("read packs directory")
            .map(|e| e.expect("read packs directory entry").path())
            .filter(|p| p.join("pack.sokf.yaml").is_file())
            .collect();
        dirs.sort();
        for dir in dirs {
            let name = dir
                .file_name()
                .expect("a pack directory has a name")
                .to_string_lossy()
                .into_owned();
            let mut files = Vec::new();
            sources(&dir, &mut files);
            files.sort();
            code.push_str(&format!(
                "    crate::pack::EmbeddedPack {{ dir: {name:?}, files: &[\n"
            ));
            for file in files {
                let rel = file
                    .strip_prefix(&dir)
                    .expect("a pack file lies under its pack")
                    .to_string_lossy()
                    .replace('\\', "/");
                let text = fs::read_to_string(&file)
                    .expect("read pack file")
                    .replace("\r\n", "\n");
                let target = out.join("packs").join(&name).join(&rel);
                fs::create_dir_all(target.parent().expect("a pack file has a parent"))
                    .expect("create OUT_DIR subdirectory");
                fs::write(&target, text).expect("write pack file");
                code.push_str(&format!(
                    "        ({rel:?}, include_str!({:?})),\n",
                    target.display().to_string()
                ));
            }
            code.push_str("    ] },\n");
        }
    }
    code.push(']');
    fs::write(out.join("packs.rs"), code).expect("write packs.rs");
}

/// `patched_crates` returns the directory of each crate that a `path` entry
/// of a `[patch]` table names, in the workspace's `Cargo.toml` and in its
/// `.cargo/config.toml`. Cargo resolves both from the workspace root, the
/// directory of the manifest and the parent of `.cargo`. The script reads
/// the lines of each table, as `name = { path = "..." }` or as a `path`
/// key of a `[patch.<SOURCE>.<NAME>]` table, and needs no TOML parser.
fn patched_crates(workspace: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for file in [
        workspace.join("Cargo.toml"),
        workspace.join(".cargo/config.toml"),
    ] {
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        println!("cargo:rerun-if-changed={}", file.display());
        let mut in_patch = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_patch = line.starts_with("[patch.") || line.starts_with("[patch]");
                continue;
            }
            if !in_patch || line.starts_with('#') {
                continue;
            }
            if let Some(path) = path_value(line) {
                let dir = workspace.join(path);
                if dir.is_dir() && !dirs.contains(&dir) {
                    dirs.push(dir);
                }
            }
        }
    }
    dirs
}

/// `path_value` returns the string of the `path` key on `line`, as in
/// `path = "../x"` or `x = { path = "../x" }`.
fn path_value(line: &str) -> Option<&str> {
    let mut rest = line;
    while let Some(at) = rest.find("path") {
        let before = rest[..at].chars().next_back();
        let after = rest[at + 4..].trim_start();
        rest = &rest[at + 4..];
        if before.is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            continue;
        }
        let Some(value) = after.strip_prefix('=') else {
            continue;
        };
        let value = value.trim_start().strip_prefix('"')?;
        return value.find('"').map(|end| &value[..end]);
    }
    None
}

/// `crate_files` adds each file of the crate under `dir` to `out`, leaving
/// out its tests, a `target` directory, and hidden entries, which change
/// no behaviour of the crate. A symbolic link counts as the file that it
/// points to, as it does for the compiler.
fn crate_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.expect("read patched crate entry").path();
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
        if name
            .as_deref()
            .is_some_and(|n| n.starts_with('.') || n == "target" || n == "tests")
        {
            continue;
        }
        if path.is_dir() {
            crate_files(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// `sources` adds each file under `dir` to `out`.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read source directory") {
        let path = entry.expect("read source directory entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// `Fnv` is the 64-bit FNV-1a hash of `source::hash`, with a separator after
/// each input. It needs no dependency, and it gives the same fingerprint on
/// every toolchain.
struct Fnv(u64);

impl Fnv {
    fn update(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
        // A separator keeps two inputs from reading as one.
        self.0 ^= 0xff;
        self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
    }
}
