//! This module reads one file as UTF-8, records the line ending of its first
//! line, normalises the text to line feed (LF), computes the file key, and splits the
//! frontmatter from the body.

use std::{fs, path::Path, time::UNIX_EPOCH};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use tramlines_md::markdown::{Split, split_frontmatter};

use super::fnv1a64;
use crate::{Error, RepoPath, RepoRoot, Result};

/// `LineEnding` is the line ending of a file's first line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineEnding {
    /// A line feed (`\n`).
    Lf,
    /// A carriage return and line feed (`\r\n`).
    CrLf,
}

impl LineEnding {
    /// `apply` converts LF text to this line ending.
    pub fn apply(self, text: &str) -> String {
        match self {
            LineEnding::Lf => text.to_string(),
            LineEnding::CrLf => text.replace('\n', "\r\n"),
        }
    }

    /// `detect` returns the line ending of the first line of `text`, or LF when
    /// `text` has no line break.
    pub fn detect(text: &str) -> Self {
        match text.find('\n') {
            Some(i) if i > 0 && text.as_bytes()[i - 1] == b'\r' => LineEnding::CrLf,
            _ => LineEnding::Lf,
        }
    }
}

// sokf:begin file-key
/// `Stat` holds the metadata that the walk reads without opening the file
/// (CACHE-1_AC-1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Stat {
    /// The byte length.
    pub len: u64,
    /// The modification time in nanoseconds since the epoch, when known.
    pub mtime: Option<i64>,
}

impl Stat {
    /// `of` builds the stat of a file from its metadata.
    pub fn of(meta: &fs::Metadata) -> Stat {
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .and_then(|d| i64::try_from(d.as_nanos()).ok());
        Stat {
            len: meta.len(),
            mtime,
        }
    }
}

/// `FileKey` identifies a file's content for the cache (CACHE-1_AC-1,
/// CACHE-1_AC-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FileKey {
    /// The byte length.
    pub len: u64,
    /// The modification time in nanoseconds since the epoch, when known.
    pub mtime: Option<i64>,
    /// The FNV-1a 64 hash of the bytes.
    pub hash: u64,
}

impl FileKey {
    /// `stat` returns the length and modification time from the key.
    pub fn stat(&self) -> Stat {
        Stat {
            len: self.len,
            mtime: self.mtime,
        }
    }
}
// sokf:end file-key

/// `Source` is one loaded file, with its text normalised to LF.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// The file's path in the repository.
    pub path: RepoPath,
    /// The text, with LF line endings.
    pub text: String,
    /// The line ending of the first line.
    pub eol: LineEnding,
    /// The file key.
    pub key: FileKey,
}

impl Source {
    /// `from_text` builds a source from text, for tests and scaffolds, and
    /// derives the key from the text alone.
    pub fn from_text(path: RepoPath, raw: &str) -> Self {
        let eol = LineEnding::detect(raw);
        Source {
            path,
            text: raw.replace("\r\n", "\n"),
            eol,
            key: FileKey {
                len: raw.len() as u64,
                mtime: None,
                hash: fnv1a64(raw.as_bytes()),
            },
        }
    }

    /// `line_count` returns the number of lines. A last line without a line
    /// break counts as a line.
    pub fn line_count(&self) -> usize {
        if self.text.is_empty() {
            0
        } else {
            self.text.matches('\n').count() + usize::from(!self.text.ends_with('\n'))
        }
    }
}

/// `load` reads a file under `root` as UTF-8 and returns it as a source, with
/// its modification time in the key. It returns an I/O error when the file
/// isn't valid UTF-8.
// sokf-implements: SOKF_VALIDATION_2
// sokf-implements: SOKF_VALIDATION_35
pub fn load(root: &RepoRoot, path: &RepoPath) -> Result<Source> {
    let fs_path = path.to_fs(root);
    let bytes = fs::read(&fs_path).map_err(|e| Error::io(&fs_path, e))?;
    let raw = String::from_utf8(bytes).map_err(|e| {
        Error::io(
            &fs_path,
            std::io::Error::new(std::io::ErrorKind::InvalidData, e.utf8_error()),
        )
    })?;
    let mtime = fs::metadata(&fs_path).ok().and_then(|m| Stat::of(&m).mtime);
    let mut source = Source::from_text(path.clone(), &raw);
    source.key.mtime = mtime;
    Ok(source)
}

/// `load_text` reads a file under `root` and returns only its text, for a file
/// that a document includes rather than one that sokf parses.
pub fn load_text(root: &RepoRoot, path: &RepoPath) -> Result<String> {
    Ok(load(root, path)?.text)
}

/// `create_parent` creates the directory that holds `fs_path`, and each
/// directory above it that is missing.
pub(crate) fn create_parent(fs_path: &Path) -> Result<()> {
    if let Some(parent) = fs_path.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    Ok(())
}

/// `read_optional` reads a file on disk as text, or returns `None` when the
/// file is absent.
pub(crate) fn read_optional(fs_path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(fs_path) {
        Ok(t) => Ok(Some(t)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::io(fs_path, e)),
    }
}

/// `split` splits a source's frontmatter from its body.
// sokf-implements: SOKF_FORMAT_17
pub fn split(source: &Source) -> Split<'_> {
    split_frontmatter(&source.text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src(text: &str) -> Source {
        Source::from_text(RepoPath::parse("/a.md").unwrap(), text)
    }

    #[test]
    fn detects_and_restores_line_endings() {
        assert_eq!(LineEnding::detect("a\r\nb\n"), LineEnding::CrLf);
        assert_eq!(LineEnding::detect("a\nb\r\n"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("no newline"), LineEnding::Lf);
        assert_eq!(LineEnding::detect(""), LineEnding::Lf);
        assert_eq!(LineEnding::CrLf.apply("a\nb\n"), "a\r\nb\r\n");
        assert_eq!(LineEnding::Lf.apply("a\nb\n"), "a\nb\n");
        let s = src("a\r\nb\r\n");
        assert_eq!(s.text, "a\nb\n");
        assert_eq!(s.eol, LineEnding::CrLf);
        assert_eq!(s.key.len, 6);
    }

    #[test]
    fn counts_lines() {
        assert_eq!(src("").line_count(), 0);
        assert_eq!(src("a").line_count(), 1);
        assert_eq!(src("a\n").line_count(), 1);
        assert_eq!(src("a\nb").line_count(), 2);
        assert_eq!(src("a\nb\n").line_count(), 2);
    }

    #[test]
    fn splits_frontmatter_from_body() {
        let s = src("---\ntype: A\nid: a\n---\n\n# Title\n");
        let sp = split(&s);
        assert_eq!(sp.frontmatter, Some(("type: A\nid: a\n", 2)));
        assert_eq!(sp.body, "\n# Title\n");
        assert_eq!(sp.body_line, 5);
    }

    #[test]
    fn no_frontmatter_is_all_body() {
        let s = src("# Title\n---\n");
        let sp = split(&s);
        assert_eq!(sp.frontmatter, None);
        assert_eq!(sp.body, "# Title\n---\n");
        assert_eq!(sp.body_line, 1);
    }

    #[test]
    fn unclosed_frontmatter_runs_to_the_end() {
        let s = src("---\ntype: A\n");
        let sp = split(&s);
        assert_eq!(sp.frontmatter, Some(("type: A\n", 2)));
        assert_eq!(sp.body, "");
        assert_eq!(sp.body_line, 3);
        let s = src("---");
        let sp = split(&s);
        assert_eq!(sp.frontmatter, Some(("", 2)));
        let s = src("---\n---\nbody");
        let sp = split(&s);
        assert_eq!(sp.frontmatter, Some(("", 2)));
        assert_eq!(sp.body, "body");
        assert_eq!(sp.body_line, 3);
    }

    #[test]
    fn loads_from_disk_with_a_key() {
        let dir = crate::test_support::fresh_dir("src");
        fs::write(dir.join("f.md"), "x\r\ny\r\n").unwrap();
        fs::write(dir.join("bad.md"), [0xFF, 0xFE]).unwrap();
        let root = RepoRoot::new(dir.clone());
        let path = RepoPath::parse("/f.md").unwrap();
        let s = load(&root, &path).unwrap();
        assert_eq!(s.text, "x\ny\n");
        assert_eq!(s.eol, LineEnding::CrLf);
        assert!(s.key.mtime.is_some());
        assert_eq!(load_text(&root, &path).unwrap(), "x\ny\n");
        assert!(load(&root, &RepoPath::parse("/missing.md").unwrap()).is_err());
        assert!(load(&root, &RepoPath::parse("/bad.md").unwrap()).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
