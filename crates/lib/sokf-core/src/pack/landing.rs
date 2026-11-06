//! This module checks where the files of a pack land, before the planner
//! plans any of them: every file of a pack is text, and a file of `files/`
//! lands outside git's and sokf's own files, outside every knowledge root,
//! and on no file that a harness writes (the decision "Where a file lands
//! outside a store" of design-pack).

use std::collections::BTreeMap;

use super::source::{FILES, PackFiles};
use crate::RepoPath;

/// `PROTECTED` is the map of paths that sync refuses to `check`: each file,
/// or each directory with a trailing `/`, with what writes it.
pub type Protected = BTreeMap<String, String>;

/// `INSTRUCTION_FILES` lists the files that hold the instructions block of
/// a harness. A pack's agent text goes in its `instructions.md`, which the
/// harness puts in the block, so no pack writes these files.
pub const INSTRUCTION_FILES: &[&str] = &["/AGENTS.md", "/CLAUDE.md"];

/// `covers` returns whether `written`, a file or a directory with a
/// trailing `/`, holds the file `path`, with the case of letters ignored,
/// because macOS and Windows open one file under both cases.
pub fn covers(written: &str, path: &str) -> bool {
    let (written, path) = (written.to_lowercase(), path.to_lowercase());
    if written.ends_with('/') {
        path.starts_with(&written)
    } else {
        path == written
    }
}

/// `check` returns a line for each file of `pack` that sync must refuse: a
/// file that isn't UTF-8, and a file of `files/` whose path holds a segment
/// that leaves its place, that lands in `/.git/` or `/.sokf/` or in any
/// `.git` directory, inside one of `roots`, or on a path of `protected`,
/// which maps a file, or a directory with a trailing `/`, to what writes it. It compares paths without regard to
/// the case of letters, because macOS and Windows open `.GIT/` as `.git/`.
// sokf-implements: CLI-13_AC-52
// sokf-implements: CLI-13_AC-53
// sokf-implements: CLI-13_AC-54
// sokf-implements: CLI-13_AC-55
pub fn check(pack: &PackFiles, roots: &[RepoPath], protected: &Protected) -> Vec<String> {
    let mut out = Vec::new();
    let protected: Vec<(&str, &str)> = INSTRUCTION_FILES
        .iter()
        .map(|p| (*p, "the instructions block of a harness"))
        .chain(protected.iter().map(|(p, who)| (p.as_str(), who.as_str())))
        .collect();
    for (rel, bytes) in &pack.files {
        if std::str::from_utf8(bytes).is_err() {
            out.push(format!(
                "{rel} is not UTF-8 text, and every file of a pack is text"
            ));
            continue;
        }
        let Some(rest) = rel.strip_prefix(FILES) else {
            continue;
        };
        let segments: Vec<&str> = rest.split('/').collect();
        if segments
            .iter()
            .any(|s| s.is_empty() || *s == "." || *s == "..")
        {
            out.push(format!(
                "{rel}: the path leaves its place in the repository"
            ));
            continue;
        }
        let dest = format!("/{rest}");
        let lower = dest.to_lowercase();
        let first = segments[0].to_lowercase();
        if first == ".sokf" || segments.iter().any(|s| s.eq_ignore_ascii_case(".git")) {
            out.push(format!(
                "{rel} lands at {dest}, among git's or sokf's own files, which no pack writes"
            ));
            continue;
        }
        if let Some(root) = roots.iter().find(|r| {
            let r = r.as_str().to_lowercase();
            lower == r || lower.starts_with(&format!("{}/", r.trim_end_matches('/')))
        }) {
            out.push(format!(
                "{rel} lands at {dest}, inside the knowledge root {root}; a file of a store belongs under schemas/ or documents/"
            ));
            continue;
        }
        if let Some((_, who)) = protected.iter().find(|(p, _)| covers(p, &dest)) {
            out.push(format!("{rel} lands at {dest}, which {who} writes"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::pack::PackManifest;

    fn pack(files: &[(&str, &[u8])]) -> PackFiles {
        PackFiles {
            manifest: PackManifest {
                name: "m".into(),
                version: "1.0.0".into(),
                requires_sokf: "0.1.0".into(),
                description: "d".into(),
                depends: BTreeMap::new(),
            },
            files: files
                .iter()
                .map(|(r, b)| (r.to_string(), b.to_vec()))
                .collect(),
            executable: BTreeSet::new(),
        }
    }

    fn refused(files: &[(&str, &[u8])]) -> Vec<String> {
        let roots = [RepoPath::parse("/knowledge").unwrap()];
        let protected = BTreeMap::from([
            ("/.claude/settings.json".into(), "the claude harness".into()),
            ("/.claude/skills/sokf/".into(), "the claude harness".into()),
        ]);
        check(&pack(files), &roots, &protected)
    }

    // sokf-tests: CLI-13_AC-52
    // sokf-tests: CLI-13_AC-53
    // sokf-tests: CLI-13_AC-54
    // sokf-tests: CLI-13_AC-55
    #[test]
    fn a_file_lands_only_where_a_pack_may_write() {
        assert!(
            refused(&[
                ("files/rustfmt.toml", b"x"),
                ("files/.github/workflows/ci.yml", b"x"),
                ("files/knowledge.toml", b"x"),
                ("files/.claude/skills/house/SKILL.md", b"x"),
                ("schemas/note.md", b"x"),
            ])
            .is_empty()
        );
        for rel in [
            "files/.git/config",
            "files/.GIT/hooks/pre-commit",
            "files/sub/.git/config",
            "files/.sokf/packs.lock",
            "files/.Sokf/config.toml",
            "files/knowledge/notes/a.md",
            "files/Knowledge/a.md",
            "files/AGENTS.md",
            "files/claude.md",
            "files/.claude/settings.json",
            "files/.claude/skills/sokf/SKILL.md",
            "files/a/../b",
        ] {
            let found = refused(&[(rel, b"x")]);
            assert_eq!(found.len(), 1, "{rel}: {found:?}");
            assert!(found[0].starts_with(rel), "{found:?}");
        }
        let found = refused(&[("schemas/logo.md", &[0xff, 0xfe])]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("is not UTF-8"), "{found:?}");
    }
}
