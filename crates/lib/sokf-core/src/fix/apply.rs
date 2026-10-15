//! `apply` carries out a fix plan on disk. It renames files, writes each
//! file with that file's line ending, rewrites a file only when the new
//! bytes differ from those on disk, and returns the fix operations it
//! performed.

use std::fs;

use super::FixPlan;
use crate::{Error, Finding, FixOp, RepoRoot, Result, source::create_parent};

/// `apply` carries out a plan in order. It moves files first, then writes
/// the new files, and last rewrites the files that changed, at their paths
/// after the move. It returns the operations it performed and the findings
/// that the plan leaves unfixed.
// sokf-implements: SOKF_VALIDATION_17
// sokf-implements: SOKF_VALIDATION_18
// sokf-implements: SOKF_VALIDATION_33
// sokf-implements: SOKF_VALIDATION_35
// sokf-implements: SOKF_VALIDATION_36
// sokf-implements: SOKF_VALIDATION_37
// sokf-implements: SOKF_VALIDATION_38
pub fn apply(root: &RepoRoot, plan: FixPlan) -> Result<(Vec<FixOp>, Vec<Finding>)> {
    let mut ops = Vec::new();
    // sokf-implements: CLI-4_AC-24
    for m in &plan.moves {
        let from = m.from.to_fs(root);
        let to = m.to.to_fs(root);
        create_parent(&to)?;
        fs::rename(&from, &to).map_err(|e| Error::io(&from, e))?;
        ops.push(FixOp::Moved {
            from: m.from.clone(),
            to: m.to.clone(),
        });
    }
    for c in &plan.creates {
        let fs_path = c.path.to_fs(root);
        create_parent(&fs_path)?;
        fs::write(&fs_path, c.eol.apply(&c.text).as_bytes()).map_err(|e| Error::io(&fs_path, e))?;
        ops.push(FixOp::Created {
            path: c.path.clone(),
        });
    }
    for r in &plan.rewrites {
        let fs_path = r.path.to_fs(root);
        // sokf-implements: CLI-12_AC-2
        let bytes = r.eol.apply(&r.text).into_bytes();
        let current = fs::read(&fs_path).ok();
        if current.as_deref() == Some(bytes.as_slice()) {
            continue;
        }
        fs::write(&fs_path, &bytes).map_err(|e| Error::io(&fs_path, e))?;
        ops.push(FixOp::Rewrote {
            path: r.path.clone(),
        });
    }
    Ok((ops, plan.refused))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        LineEnding, RepoPath,
        fix::{Move, Rewrite},
    };

    fn temp(name: &str) -> PathBuf {
        let dir = crate::test_support::fresh_dir(&format!("apply-{name}"));
        fs::create_dir_all(dir.join("k/open")).unwrap();
        fs::write(dir.join("k/open/i.md"), "old\r\n").unwrap();
        fs::write(dir.join("k/same.md"), "same\n").unwrap();
        dir
    }

    #[test]
    fn moves_then_writes_only_what_differs() {
        let dir = temp("apply");
        let root = RepoRoot::new(dir.clone());
        let p = |s: &str| RepoPath::parse(s).unwrap();
        let plan = FixPlan {
            rewrites: vec![
                Rewrite {
                    path: p("/k/closed/i.md"),
                    text: "new\n".into(),
                    eol: LineEnding::CrLf,
                },
                Rewrite {
                    path: p("/k/same.md"),
                    text: "same\n".into(),
                    eol: LineEnding::Lf,
                },
            ],
            creates: vec![Rewrite {
                path: p("/k/closed/index.md"),
                text: "# closed\n".into(),
                eol: LineEnding::Lf,
            }],
            moves: vec![Move {
                from: p("/k/open/i.md"),
                to: p("/k/closed/i.md"),
            }],
            refused: vec![],
        };
        let (ops, refused) = apply(&root, plan).unwrap();
        assert!(refused.is_empty());
        assert_eq!(
            ops,
            vec![
                FixOp::Moved {
                    from: p("/k/open/i.md"),
                    to: p("/k/closed/i.md")
                },
                FixOp::Created {
                    path: p("/k/closed/index.md")
                },
                FixOp::Rewrote {
                    path: p("/k/closed/i.md")
                }
            ]
        );
        assert_eq!(fs::read(dir.join("k/closed/i.md")).unwrap(), b"new\r\n");
        assert_eq!(
            fs::read(dir.join("k/closed/index.md")).unwrap(),
            b"# closed\n"
        );
        assert!(!dir.join("k/open/i.md").exists());
        let bad = FixPlan {
            rewrites: vec![],
            creates: vec![],
            moves: vec![Move {
                from: p("/k/missing.md"),
                to: p("/k/x.md"),
            }],
            refused: vec![],
        };
        assert!(apply(&root, bad).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
