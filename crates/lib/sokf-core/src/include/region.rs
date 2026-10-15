//! This module extracts the named regions of a repository file. Section 9.3
//! of the format specification defines them.

/// A `RegionError` reports a region marker that breaks the pairing of `begin`
/// and `end` markers (SOKF_FORMAT_95, SOKF_FORMAT_112), and gives the name of
/// the region and the number of the line.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegionError {
    /// A `begin` marker opens a region that is already open.
    #[error("region `{name}` begins again at line {line} while it is still open")]
    AlreadyOpen {
        /// The name of the region.
        name: String,
        /// The line of the marker, counted from 1.
        line: usize,
    },
    /// An `end` marker closes a region that isn't open.
    #[error("region `{name}` ends at line {line} without a matching `begin` marker")]
    NotOpen {
        /// The name of the region.
        name: String,
        /// The line of the marker, counted from 1.
        line: usize,
    },
    /// A `begin` marker has no matching `end` marker.
    #[error("region `{name}` begins at line {line} but never ends")]
    NeverClosed {
        /// The name of the region.
        name: String,
        /// The line of the marker, counted from 1.
        line: usize,
    },
}

impl RegionError {
    /// `line` returns the line of the marker that breaks the pairing.
    pub fn line(&self) -> usize {
        match self {
            RegionError::AlreadyOpen { line, .. }
            | RegionError::NotOpen { line, .. }
            | RegionError::NeverClosed { line, .. } => *line,
        }
    }
}

enum Marker<'a> {
    Begin(&'a str),
    End(&'a str),
}

fn is_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// `marker` returns the marker on a line, `sokf:begin <name>` or
/// `sokf:end <name>`, found anywhere in the line. The name is the next word
/// after the prefix, and it must match `[A-Za-z0-9_-]+`.
fn marker(line: &str) -> Option<Marker<'_>> {
    for (prefix, begin) in [("sokf:begin ", true), ("sokf:end ", false)] {
        if let Some(i) = line.find(prefix) {
            let rest = &line[i + prefix.len()..];
            let name = rest.split_whitespace().next().unwrap_or("");
            if is_name(name) {
                return Some(if begin {
                    Marker::Begin(name)
                } else {
                    Marker::End(name)
                });
            }
        }
    }
    None
}

/// `extract` returns the lines of `region` in `text`, or of the whole file
/// when `region` is `None`, with every marker line stripped. It returns
/// `Ok(None)` when no region has that name. It joins regions that share a
/// name in file order.
// sokf-implements: SOKF_FORMAT_85
// sokf-implements: SOKF_FORMAT_95
// sokf-implements: SOKF_FORMAT_128
// sokf-implements: SOKF_FORMAT_136
// sokf-implements: SOKF_FORMAT_137
// sokf-implements: SOKF_FORMAT_112
pub fn extract(text: &str, region: Option<&str>) -> Result<Option<String>, RegionError> {
    let mut open: Vec<(String, usize)> = Vec::new();
    let mut out = String::new();
    let mut found = region.is_none();
    for (i, line) in text.lines().enumerate() {
        let line_no = i + 1;
        match marker(line) {
            Some(Marker::Begin(name)) => {
                if open.iter().any(|(n, _)| n == name) {
                    return Err(RegionError::AlreadyOpen {
                        name: name.to_string(),
                        line: line_no,
                    });
                }
                if Some(name) == region {
                    found = true;
                }
                open.push((name.to_string(), line_no));
            }
            Some(Marker::End(name)) => {
                let Some(pos) = open.iter().position(|(n, _)| n == name) else {
                    return Err(RegionError::NotOpen {
                        name: name.to_string(),
                        line: line_no,
                    });
                };
                open.remove(pos);
            }
            None => {
                let inside = match region {
                    None => true,
                    Some(r) => open.iter().any(|(n, _)| n == r),
                };
                if inside {
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
    }
    if let Some((name, line)) = open.into_iter().next() {
        return Err(RegionError::NeverClosed { name, line });
    }
    Ok(found.then_some(out))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const FILE: &str = "use x;\n// sokf:begin cli\nfn a() {}\n# sokf:begin other\nfn b() {}\n// sokf:end cli\nfn c() {}\n// sokf:end other\n<!-- sokf:begin cli -->\nfn d() {}\n<!-- sokf:end cli -->\n";

    // sokf-tests: SOKF_FORMAT_136
    // sokf-tests: SOKF_FORMAT_137
    #[test]
    fn extracts_regions_and_whole_files() {
        assert_eq!(
            extract(FILE, Some("cli")).unwrap().unwrap(),
            "fn a() {}\nfn b() {}\nfn d() {}\n"
        );
        assert_eq!(
            extract(FILE, Some("other")).unwrap().unwrap(),
            "fn b() {}\nfn c() {}\n"
        );
        assert_eq!(
            extract(FILE, None).unwrap().unwrap(),
            "use x;\nfn a() {}\nfn b() {}\nfn c() {}\nfn d() {}\n"
        );
        assert_eq!(extract(FILE, Some("nope")).unwrap(), None);
        assert_eq!(extract("", None).unwrap().unwrap(), "");
        assert_eq!(
            extract("// sokf:begin e\n// sokf:end e\n", Some("e"))
                .unwrap()
                .unwrap(),
            ""
        );
    }

    #[test]
    fn non_markers_are_content() {
        let text =
            "sokf:begin\nsokf:begin x:y\nsokf:end\n<!-- sokf:begin ok -->\n<!-- sokf:end ok -->\n";
        assert_eq!(
            extract(text, None).unwrap().unwrap(),
            "sokf:begin\nsokf:begin x:y\nsokf:end\n"
        );
        assert_eq!(extract(text, Some("ok")).unwrap(), Some(String::new()));
        assert_eq!(extract(text, Some("x")).unwrap(), None);
    }

    #[test]
    fn region_errors_name_the_line() {
        assert_eq!(
            extract("// sokf:begin a\n// sokf:begin a\n", Some("a")).unwrap_err(),
            RegionError::AlreadyOpen {
                name: "a".into(),
                line: 2
            }
        );
        assert_eq!(
            extract("// sokf:end a\n", None).unwrap_err(),
            RegionError::NotOpen {
                name: "a".into(),
                line: 1
            }
        );
        assert_eq!(
            extract("x\n// sokf:begin a\n", Some("b")).unwrap_err(),
            RegionError::NeverClosed {
                name: "a".into(),
                line: 2
            }
        );
        assert!(
            RegionError::NotOpen {
                name: "a".into(),
                line: 1
            }
            .to_string()
            .contains("line 1")
        );
    }

    fn arb_file() -> impl Strategy<Value = (String, Vec<(String, Vec<String>)>)> {
        // The strategy builds a file of plain lines and regions, some of which
        // share a name. It returns the file with the lines that extraction
        // should return for each region name.
        prop::collection::vec(
            (
                prop::sample::select(vec!["r1", "r2", "r3"]),
                prop::collection::vec("[a-z ]{0,8}", 0..4),
            ),
            0..5,
        )
        .prop_map(|regions| {
            let mut text = String::from("head\n");
            let mut expected: Vec<(String, Vec<String>)> = Vec::new();
            for (name, lines) in regions {
                text.push_str(&format!("// sokf:begin {name}\n"));
                for l in &lines {
                    text.push_str(l);
                    text.push('\n');
                }
                text.push_str(&format!("// sokf:end {name}\n"));
                match expected.iter_mut().find(|(n, _)| *n == name) {
                    Some((_, acc)) => acc.extend(lines),
                    None => expected.push((name.to_string(), lines)),
                }
            }
            text.push_str("tail\n");
            (text, expected)
        })
    }

    // sokf-tests: FORMAT_P-6
    proptest! {
        #![proptest_config(crate::proptest_cases::cases(256))]

        #[test]
        fn extraction_concatenates_same_named_regions_and_strips_markers((text, expected) in arb_file()) {
            for (name, lines) in &expected {
                let want: String = lines.iter().map(|l| format!("{l}\n")).collect();
                prop_assert_eq!(extract(&text, Some(name)).unwrap().unwrap(), want);
            }
            prop_assert_eq!(extract(&text, Some("absent")).unwrap(), None);
            let whole = extract(&text, None).unwrap().unwrap();
            prop_assert!(!whole.contains("sokf:begin") && !whole.contains("sokf:end"));
            let all_lines: usize = expected.iter().map(|(_, l)| l.len()).sum();
            prop_assert_eq!(whole.lines().count(), all_lines + 2);
        }
    }
}
