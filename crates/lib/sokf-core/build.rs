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
//! The script also sets `SOKF_BUILD_FINGERPRINT`, a hash of what decides the
//! results of a check: the crate's sources, the embedded files, and the
//! workspace's `Cargo.lock`, which pins the tramlines engine and every other
//! dependency. The cache records the fingerprint in its version, so a build
//! that checks differently starts from an empty store (CACHE-1_AC-6). A
//! packaged crate has no lock file, and its fingerprint covers its sources.

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
    let lock = workspace.join("Cargo.lock");
    if lock.is_file() {
        println!("cargo:rerun-if-changed={}", lock.display());
        hash.update(&fs::read(&lock).expect("read Cargo.lock"));
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
