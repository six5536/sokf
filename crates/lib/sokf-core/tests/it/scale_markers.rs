//! This benchmark measures the cost of markers in a large repository
//! (contract-cache CACHE-3): a million files outside the knowledge roots,
//! one in a hundred of them with a marker, and one marker file changed.
//! The test is ignored by default. Run it by hand on a release build with
//! `cargo test --release -p sokf-core --test it scale_markers --
//! --ignored --nocapture`, and set `SOKF_SCALE_FILES` to change the number
//! of files.

use std::{fs, time::Instant};

use common::{
    TempTree, age, age_file,
    incremental::{run, run_hinted},
};

use crate::common;

/// `write_files` writes the files of the benchmark, a thousand to a
/// directory, and returns the paths of the files that hold a marker.
fn write_files(tree: &TempTree, files: usize) -> Vec<String> {
    let mut marked = Vec::new();
    for i in 0..files {
        let dir = tree.dir().join(format!("src/d{}", i / 1000));
        if i % 1000 == 0 {
            fs::create_dir_all(&dir).unwrap();
        }
        let rel = format!("src/d{}/f{i}.rs", i / 1000);
        let text = if i % 100 == 0 {
            marked.push(rel.clone());
            format!("// sokf-implements: CLI-1_AC-2\nfn f{i}() {{}}\n")
        } else {
            format!("fn f{i}() {{}}\n")
        };
        fs::write(tree.dir().join(&rel), text).unwrap();
    }
    marked
}

/// `touch` adds a marker to one marker file and ages the file, so that the
/// next run reads it once.
fn touch(tree: &TempTree, rel: &str) {
    let text = tree.read(rel);
    tree.write(rel, &format!("{text}// sokf-tests: CLI-1_AC-1\n"));
    age_file(&tree.dir().join(rel), 10);
}

// The benchmark stands in the local tier of tests, which a developer runs
// with `mise run test:local` and every CI run skips, because it measures
// nothing that a run on a developer's machine doesn't, and in CI it ran
// for over half an hour on each system.
mod local_only {
    use super::*;

    // sokf-tests: CACHE-3_AC-8
    // sokf-tests: CACHE-3_AC-9
    // sokf-tests: CACHE-1_AC-36
    #[test]
    #[ignore = "local: it writes a million files: run it by hand on a release build, and set SOKF_SCALE_FILES to change the number of files."]
    fn a_million_files_with_one_changed_marker_file() {
        let files: usize = std::env::var("SOKF_SCALE_FILES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1_000_000);
        let tree = TempTree::fixture("clean");
        let t = Instant::now();
        let marked = write_files(&tree, files);
        eprintln!(
            "wrote {files} files, {} with a marker, in {:?}",
            marked.len(),
            t.elapsed()
        );
        age(tree.dir(), 10);
        let t = Instant::now();
        let first = run(&tree);
        eprintln!(
            "the first run, which reads every file, took {:?}",
            t.elapsed()
        );
        assert!(
            first.2.iter().all(|f| !f.starts_with("/src/")),
            "{:?}",
            first.2
        );
        let t = Instant::now();
        let quiet = run(&tree);
        eprintln!(
            "a run with no change took {:?} and read {} files",
            t.elapsed(),
            quiet.3
        );
        // Change one marker file, and let the full walk find it.
        touch(&tree, &marked[marked.len() / 2]);
        let t = Instant::now();
        let walked = run(&tree);
        let took = t.elapsed();
        eprintln!(
            "the full walk found one changed marker file in {took:?} and read {} files",
            walked.3
        );
        assert_eq!(walked.0.len(), 1, "{:?}", walked.0);
        if files >= 1_000_000 && !cfg!(debug_assertions) {
            assert!(took.as_secs_f64() < 3.0, "the full walk took {took:?}");
        }
        // Change one marker file, and pass its path to the run as a hint.
        let rel = &marked[marked.len() / 3];
        touch(&tree, rel);
        let t = Instant::now();
        let hinted = run_hinted(&tree, &[&format!("/{rel}")]);
        let took = t.elapsed();
        eprintln!(
            "the hinted run found one changed marker file in {took:?} and read {} files",
            hinted.2
        );
        assert_eq!(hinted.0.len(), 1, "{:?}", hinted.0);
        if files >= 1_000_000 && !cfg!(debug_assertions) {
            assert!(took.as_secs_f64() < 1.0, "the hinted run took {took:?}");
        }
        // Change one marker file, then look up a contract. `locate` reads the
        // changed file and caches its markers, so the validate run that follows
        // has nothing to do.
        touch(&tree, &marked[marked.len() / 4]);
        let t = Instant::now();
        let found = sokf_core::locate(
            &tree.root(),
            &["req-cli".to_string()],
            false,
            sokf_core::CacheMode::On,
        );
        let took = t.elapsed();
        eprintln!("locate took {took:?} after one marker file changed");
        assert!(found.is_ok(), "{found:?}");
        if files >= 1_000_000 && !cfg!(debug_assertions) {
            assert!(took.as_secs_f64() < 3.0, "the lookup took {took:?}");
        }
        let t = Instant::now();
        let after = run(&tree);
        eprintln!(
            "the validate run after locate took {:?} and read {} files",
            t.elapsed(),
            after.3
        );
        assert!(after.0.is_empty(), "{:?}", after.0);
    }
}
