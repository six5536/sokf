//! This module holds the exit codes that CLI-1_AC-9 to CLI-1_AC-11 define and
//! the stdout helpers that every command shares. The codes and the writes are
//! agent-harness-adapter's CLI conventions.

use std::{
    io,
    path::{Path, PathBuf},
};

use agent_harness_adapter_core::cli;
use serde::Serialize;
use sokf_core::{Error, RepoRoot, discover_root};

// sokf:begin output
/// `EXIT_OK` (0) is the exit code when the command finds no error,
/// `EXIT_ERRORS` (1) the exit code when the command finds errors, and
/// `EXIT_FAILURE` (2) the exit code for a usage or internal error, which the
/// command reports on stderr.
pub use agent_harness_adapter_core::cli::{EXIT_ERRORS, EXIT_FAILURE, EXIT_OK};

/// `write_stdout` writes a fully rendered buffer to stdout and flushes it. The
/// function flushes explicitly because, when the program exits, the runtime
/// flushes stdout and ignores any error. Without the explicit flush, the
/// command would see a broken pipe only if the buffer filled while it was still
/// rendering.
pub fn write_stdout(bytes: &[u8]) -> sokf_core::Result<()> {
    cli::write_stdout(bytes).map_err(stdout_err)
}
// sokf:end output

/// `stdout_err` turns a failure to write to stdout into a `sokf` error, so that
/// the helpers that render output can use `?`.
pub fn stdout_err(e: io::Error) -> Error {
    Error::io("<stdout>", e)
}

/// `print_schema` writes the JSON Schema of `output` to stdout, as the file in
/// `/knowledge/json/` holds it, and returns the exit code 0. It reads no
/// repository.
// sokf-implements: CLI-1_AC-14
// sokf-implements: CLI-1_AC-15
pub fn print_schema(output: sokf_core::Output) -> sokf_core::Result<u8> {
    write_stdout(output.schema_text().as_bytes())?;
    Ok(EXIT_OK)
}

/// `json_line` serialises `value` as JSON on one line, ended with a newline.
pub fn json_line(value: &impl Serialize) -> sokf_core::Result<Vec<u8>> {
    cli::json_line(value).map_err(|e| Error::Internal(e.to_string()))
}

/// `cwd` returns the current directory.
pub fn cwd() -> sokf_core::Result<PathBuf> {
    std::env::current_dir().map_err(|e| Error::io(".", e))
}

/// `root` returns the current directory and the repository root that it,
/// or `--repo-root`, gives.
pub fn root(repo_root: Option<&Path>) -> sokf_core::Result<(PathBuf, RepoRoot)> {
    let cwd = cwd()?;
    let root = discover_root(&cwd, repo_root)?;
    Ok((cwd, root))
}

/// `required` returns the value of an argument that clap requires unless
/// `--json-schema` is present, which clap has then already checked.
pub fn required(value: Option<String>, name: &str) -> sokf_core::Result<String> {
    value.ok_or_else(|| Error::Internal(format!("clap requires {name}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    use agent_harness_adapter_core::cli::is_broken_pipe;

    // `main` exits through `cli::finish`, which treats a broken pipe as a clean
    // exit. The test checks that a `sokf` error is a broken pipe only when it
    // is one.
    #[test]
    fn broken_pipe_is_the_only_error_treated_as_clean() {
        let epipe = stdout_err(io::Error::new(io::ErrorKind::BrokenPipe, "Broken pipe"));
        assert!(is_broken_pipe(&epipe));
        assert!(epipe.to_string().contains("<stdout>"), "{epipe}");

        // Any other I/O failure is a real one, however similar it looks.
        for kind in [
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::NotFound,
            io::ErrorKind::WriteZero,
        ] {
            let e = stdout_err(io::Error::new(kind, "nope"));
            assert!(!is_broken_pipe(&e), "{kind:?} must not be swallowed");
        }
        // A non-I/O error is not a broken pipe either, because it has no `kind`
        // to inspect.
        assert!(!is_broken_pipe(&Error::NoRepository));
    }

    #[test]
    fn write_stdout_round_trips_bytes() {
        // The test harness captures the output. The test checks that
        // `write_stdout` returns `Ok` without panicking, both for a non-empty
        // buffer and for an empty one.
        assert!(write_stdout(b"sokf\n").is_ok());
        assert!(write_stdout(b"").is_ok());
    }
}
