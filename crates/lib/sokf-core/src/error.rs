//! The core has one error type, `Error`. The command-line interface (CLI)
//! exits with code 2 for every variant.

use std::path::PathBuf;

use crate::RepoPath;

/// `Result` is the result type of this crate, with `Error` as its error.
pub type Result<T> = std::result::Result<T, Error>;

/// `Error` holds the errors of this crate. Every variant renders a message
/// that the CLI prints unchanged after `error: `.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The tool finds no repository root (CLI-1_AC-5).
    #[error("not in a Git repository; pass --repo-root")]
    NoRepository,

    /// The tool refuses a `config.toml` file (CONF-1_AC-3, CONF-1_AC-4,
    /// CONF-2_AC-6, CONF-2_AC-7, CONF-3_AC-5, CONF-3_AC-6).
    #[error("{path}: {message}")]
    Config {
        /// `path` is the path of the configuration file that the tool refuses.
        path: RepoPath,
        /// The message says what is wrong with the file.
        message: String,
    },

    /// The tool refuses a `formats.yaml` file.
    #[error("{path}: {message}")]
    FormatsFile {
        /// `path` is the path of the formats file that the tool refuses.
        path: RepoPath,
        /// The message says what is wrong with the file.
        message: String,
    },

    /// The tool refuses a `PATHS` argument whose path is not in the
    /// repository (CLI-4_AC-15).
    #[error("{0}: no such path in the repository")]
    Path(String),

    /// `init` refuses to run (contract-cli CLI-2), and the message identifies
    /// the path or the option at fault.
    #[error("init: {0}")]
    Init(String),

    /// `harness` refuses to run (contract-cli CLI-3), and the message
    /// identifies the harness, part, event, or file at fault.
    #[error("harness: {0}")]
    Harness(String),

    /// `new` refuses to run (contract-cli CLI-6).
    #[error("new: {0}")]
    New(String),

    /// `info` refuses to run (contract-cli CLI-5).
    #[error("info: {0}")]
    Info(String),

    /// `graph` refuses a `START` argument, and the message lists each way the
    /// tool can read the argument (CLI-7_AC-19).
    #[error("graph: {0}")]
    Start(String),

    /// `locate` refuses an `id`, and the message gives the `id` and the paths
    /// of the documents that declare it (CLI-8_AC-6).
    #[error("locate: {0}")]
    Locate(String),

    /// A JMESPath expression doesn't compile (CLI-4_AC-17,
    /// CLI-7_AC-21).
    #[error("query: {0}")]
    Query(String),

    /// An input or output (I/O) operation fails at a path.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// `path` is the path at which the operation fails.
        path: PathBuf,
        /// `source` is the I/O error that causes the failure.
        #[source]
        source: std::io::Error,
    },

    /// The code breaks one of its own invariants.
    #[error("internal: {0}")]
    Internal(String),
}

impl From<tramlines_md::Error> for Error {
    fn from(e: tramlines_md::Error) -> Self {
        match e {
            tramlines_md::Error::Internal(m) => Error::Internal(m),
        }
    }
}

/// An error of agent-harness-adapter is an I/O error at its path, or else a
/// refusal of `harness`.
impl From<agent_harness_adapter_core::Error> for Error {
    fn from(e: agent_harness_adapter_core::Error) -> Self {
        match e {
            agent_harness_adapter_core::Error::Io { path, source } => Error::Io { path, source },
            agent_harness_adapter_core::Error::Internal(m) => Error::Internal(m),
            other => Error::Harness(other.to_string()),
        }
    }
}

impl Error {
    /// `io` builds an [`Error::Io`] for `path` from a [`std::io::Error`].
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    /// `config` builds an [`Error::Config`].
    pub fn config(path: RepoPath, message: impl Into<String>) -> Self {
        Error::Config {
            path,
            message: message.into(),
        }
    }

    /// `formats` builds an [`Error::FormatsFile`].
    pub fn formats(path: RepoPath, message: impl Into<String>) -> Self {
        Error::FormatsFile {
            path,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error as _, io, path::Path};

    use super::*;

    #[test]
    fn io_constructor_and_display() {
        let e = Error::io(
            Path::new("/tmp/x"),
            io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
        );
        assert!(matches!(e, Error::Io { .. }));
        let msg = e.to_string();
        assert!(
            msg.contains("/tmp/x"),
            "the message contains the path: {msg}"
        );
        assert!(
            msg.contains("denied"),
            "the message contains the source error: {msg}"
        );
        assert!(e.source().is_some());
    }

    #[test]
    fn a_kit_error_is_an_io_error_or_a_harness_refusal() {
        let e: Error = agent_harness_adapter_core::Error::io(
            "/tmp/x",
            io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
        )
        .into();
        assert!(matches!(e, Error::Io { .. }), "{e}");
        let e: Error = agent_harness_adapter_core::Error::file("/.sokf/config.toml", "bad").into();
        assert_eq!(e.to_string(), "harness: /.sokf/config.toml: bad");
        let e: Error = agent_harness_adapter_core::Error::Internal("bug".into()).into();
        assert_eq!(e.to_string(), "internal: bug");
        let e: Error = agent_harness_adapter_core::Error::UnknownHarness {
            harness: "cline".into(),
        }
        .into();
        assert_eq!(e.to_string(), "harness: no harness named `cline`");
    }

    #[test]
    fn every_message_names_its_subject() {
        let path = RepoPath::parse("/.sokf/config.toml").unwrap();
        assert_eq!(
            Error::config(path.clone(), "unknown key").to_string(),
            "/.sokf/config.toml: unknown key"
        );
        assert_eq!(
            Error::formats(path, "bad").to_string(),
            "/.sokf/config.toml: bad"
        );
        assert_eq!(
            Error::Path("x".into()).to_string(),
            "x: no such path in the repository"
        );
        assert_eq!(Error::New("no schema".into()).to_string(), "new: no schema");
        assert_eq!(Error::Info("x".into()).to_string(), "info: x");
        assert_eq!(Error::Init("x".into()).to_string(), "init: x");
        assert_eq!(Error::Harness("x".into()).to_string(), "harness: x");
        assert_eq!(Error::Start("x".into()).to_string(), "graph: x");
        assert_eq!(Error::Locate("x".into()).to_string(), "locate: x");
        assert_eq!(Error::Query("x".into()).to_string(), "query: x");
        assert_eq!(Error::Internal("x".into()).to_string(), "internal: x");
        assert!(Error::NoRepository.to_string().contains("--repo-root"));
        assert!(Error::NoRepository.source().is_none());
    }
}
