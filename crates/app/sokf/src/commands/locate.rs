//! This module implements `sokf locate`, which prints the paths of documents by id.

use std::path::Path;

use sokf_core::CacheMode;

use crate::{cli::LocateArgs, output};

/// `locate` prints the path of the document that each id identifies, one path
/// to a line. With `--asset`, it prints the path of each id's asset instead,
/// and with `--json`, it prints one array of `{id, path, asset}` objects. When
/// an id is unknown or the command fails, `locate` prints nothing.
// sokf-implements: CLI-8_AC-1
// sokf-implements: CLI-8_AC-4
// sokf-implements: CLI-8_AC-5
// sokf-implements: CLI-8_AC-8
pub fn locate(
    repo_root: Option<&Path>,
    cache: CacheMode,
    args: LocateArgs,
) -> sokf_core::Result<u8> {
    let (_, root) = output::root(repo_root)?;
    let located = sokf_core::locate(&root, &args.ids, args.asset, cache)?;
    let mut buf = Vec::new();
    if args.json {
        buf = output::json_line(&sokf_core::LocateReport::from(located.as_slice()))?;
    } else {
        for l in &located {
            let path = if args.asset {
                l.asset.as_ref().unwrap_or(&l.path)
            } else {
                &l.path
            };
            buf.extend_from_slice(path.display().to_string().as_bytes());
            buf.push(b'\n');
        }
    }
    output::write_stdout(&buf)?;
    Ok(output::EXIT_OK)
}
