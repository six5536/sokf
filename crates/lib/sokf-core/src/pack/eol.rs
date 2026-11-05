//! This module reads and writes the line endings of a pack's files in the
//! user's repository. A checkout of the repository can turn each LF of a
//! file into CRLF, so the pack reads each CRLF as LF, as git's `text=auto`
//! does on a commit, and writes a file over a CRLF copy with CRLF (the
//! decision "The committed bytes" of design-pack).

use std::borrow::Cow;

use super::lock::hash_of;
use crate::LineEnding;

/// `normalise` returns `bytes` with each CRLF turned into LF. A carriage
/// return that no line feed follows stays.
// sokf-implements: CLI-13_AC-46
pub fn normalise(bytes: &[u8]) -> Cow<'_, [u8]> {
    if !bytes.windows(2).any(|w| w == b"\r\n") {
        return Cow::Borrowed(bytes);
    }
    let mut out = Vec::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'\r' || bytes.get(i + 1) != Some(&b'\n') {
            out.push(b);
        }
    }
    Cow::Owned(out)
}

/// `same_text` returns whether `a` and `b` hold the same text once each CRLF
/// is read as LF.
pub fn same_text(a: &[u8], b: &[u8]) -> bool {
    normalise(a) == normalise(b)
}

/// `is_base` returns whether `bytes` are the base whose hash is `base`: the
/// hash of the bytes, or of the bytes with each CRLF read as LF, is `base`.
/// The first hash matches a file that a pack committed with CRLF, and the
/// second a file that a checkout converted to CRLF.
// sokf-implements: CLI-13_AC-47
pub fn is_base(bytes: &[u8], base: &str) -> bool {
    hash_of(bytes) == base || hash_of(&normalise(bytes)) == base
}

/// `keep_ending` returns `text`, which sync writes over the copy `existing`,
/// with the line ending of the copy's first line, as `validate --fix` keeps
/// a file's line ending: each line ends with CRLF when the copy's first line
/// does, and `text` is unchanged otherwise.
// sokf-implements: CLI-13_AC-48
pub fn keep_ending(existing: &[u8], text: Vec<u8>) -> Vec<u8> {
    if LineEnding::detect_bytes(existing) == LineEnding::Lf {
        return text;
    }
    let lf = normalise(&text);
    let mut out = Vec::with_capacity(lf.len() + lf.len() / 32);
    for &b in lf.iter() {
        if b == b'\n' {
            out.push(b'\r');
        }
        out.push(b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_crlf_turns_into_lf() {
        assert_eq!(&*normalise(b"a\r\nb\rc\r\r\n\r"), b"a\nb\rc\r\n\r");
        assert!(matches!(normalise(b"a\nb\r"), Cow::Borrowed(_)));
        assert!(same_text(b"a\r\nb\n", b"a\nb\r\n"));
        assert!(!same_text(b"a\rb\n", b"a\nb\n"));
    }

    #[test]
    fn a_file_is_its_base_with_either_line_ending() {
        let lf = hash_of(b"a\nb\n");
        assert!(is_base(b"a\nb\n", &lf));
        assert!(is_base(b"a\r\nb\r\n", &lf));
        assert!(!is_base(b"a\nc\n", &lf));
        let crlf = hash_of(b"a\r\nb\r\n");
        assert!(is_base(b"a\r\nb\r\n", &crlf));
    }

    #[test]
    fn a_write_keeps_the_line_ending_of_the_copy() {
        assert_eq!(
            keep_ending(b"x\r\ny\n", b"a\nb\r\nc".to_vec()),
            b"a\r\nb\r\nc"
        );
        assert_eq!(keep_ending(b"x\ny\r\n", b"a\nb\r\n".to_vec()), b"a\nb\r\n");
        assert_eq!(keep_ending(b"", b"a\n".to_vec()), b"a\n");
    }
}
