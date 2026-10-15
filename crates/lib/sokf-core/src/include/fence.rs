//! This module builds the code fence around the content of a file-form
//! include (SOKF_FORMAT_92 to SOKF_FORMAT_94).

use crate::RepoPath;

/// `fence_len` returns the length of the fence for `content`: one backtick
/// more than the longest run of backticks in it, and never fewer than three.
// sokf-implements: SOKF_FORMAT_94
fn fence_len(content: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for c in content.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    (longest + 1).max(3)
}

// sokf:begin fence-tag
/// `fence_tag` returns the fence tag for the file at `path`, from the table of
/// extensions in the format specification, or an empty string when the file
/// has no extension.
// sokf-implements: SOKF_FORMAT_93
fn fence_tag(path: &RepoPath) -> String {
    match path.extension() {
        None => String::new(),
        Some("rs") => "rust".into(),
        Some("yml") => "yaml".into(),
        Some("ts") => "typescript".into(),
        Some("py") => "python".into(),
        Some("js") => "javascript".into(),
        Some("sh") => "bash".into(),
        Some("md") => "markdown".into(),
        Some(other) => other.to_string(),
    }
}
// sokf:end fence-tag

/// `fenced` wraps `extraction`, the text taken from the file at `path`, in a
/// fenced code block. The block's tag is the tag that `fence_tag` returns for
/// the file.
// sokf-implements: SOKF_FORMAT_92
// sokf-implements: SOKF_FORMAT_138
pub fn fenced(extraction: &str, path: &RepoPath) -> String {
    let fence = "`".repeat(fence_len(extraction));
    format!("{fence}{}\n{extraction}{fence}", fence_tag(path))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn fence_is_longer_than_any_run_and_at_least_three() {
        assert_eq!(fence_len(""), 3);
        assert_eq!(fence_len("no ticks"), 3);
        assert_eq!(fence_len("a `b` c"), 3);
        assert_eq!(fence_len("``` code ```"), 4);
        assert_eq!(fence_len("`````"), 6);
    }

    #[test]
    fn tags_follow_the_table() {
        let p = |s: &str| RepoPath::parse(s).unwrap();
        assert_eq!(fence_tag(&p("/a/b.rs")), "rust");
        assert_eq!(fence_tag(&p("/a.yml")), "yaml");
        assert_eq!(fence_tag(&p("/a.ts")), "typescript");
        assert_eq!(fence_tag(&p("/a.py")), "python");
        assert_eq!(fence_tag(&p("/a.js")), "javascript");
        assert_eq!(fence_tag(&p("/a.sh")), "bash");
        assert_eq!(fence_tag(&p("/a.md")), "markdown");
        assert_eq!(fence_tag(&p("/a.toml")), "toml");
        assert_eq!(fence_tag(&p("/Makefile")), "");
        assert_eq!(fenced("x\n", &p("/a.rs")), "```rust\nx\n```");
        assert_eq!(fenced("", &p("/Makefile")), "```\n```");
    }

    // sokf-tests: FORMAT_P-5
    proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        #[test]
        fn the_fenced_content_parses_back_unchanged(lines in prop::collection::vec("[a-z` ]{0,12}", 0..6)) {
            let content: String = lines.iter().map(|l| format!("{l}\n")).collect();
            let n = fence_len(&content);
            prop_assert!(n >= 3);
            prop_assert!(!content.contains(&"`".repeat(n)));
            let path = RepoPath::parse("/x.rs").unwrap();
            let block = fenced(&content, &path);
            let segments = crate::markdown::segments(&block, 1);
            let body = crate::markdown::parse(&block, 1, &segments);
            prop_assert_eq!(body.blocks.len(), 1);
            prop_assert_eq!(body.blocks[0].kind, crate::markdown::BlockKind::Code);
            prop_assert_eq!(&body.blocks[0].units[0].text, &content);
            prop_assert_eq!(&body.blocks[0].detail, &crate::markdown::BlockDetail::Code { lang: "rust".into(), fence: n });
        }
    }
}
