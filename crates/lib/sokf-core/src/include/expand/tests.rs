//! The tests of the expected content of the generated blocks.

use proptest::prelude::*;

use super::*;

#[test]
fn interiors_and_definition_blocks_render() {
    assert_eq!(interior(""), "\n");
    assert_eq!(interior("a\nb"), "\na\nb\n\n");
    assert_eq!(render_definitions(&[]), "");
    let entries = vec![
        DefinitionEntry {
            label: "sokf:#K".into(),
            path: RepoPath::parse("/k/a.md").unwrap(),
            slug: None,
        },
        DefinitionEntry {
            label: "sokf:b#h".into(),
            path: RepoPath::parse("/k/b.md").unwrap(),
            slug: Some("h".into()),
        },
    ];
    assert_eq!(
        render_definitions(&entries),
        "<!-- sokf:links -->\n\n[sokf:#K]: /k/a.md\n[sokf:b#h]: /k/b.md#h\n"
    );
}

#[test]
fn expand_removes_markers_and_the_definition_block() {
    let body = "\n# T\n\n<!-- sokf:include x -->\n\nold\n\n<!-- /sokf:include -->\n\ntail\n\n<!-- sokf:links -->\n\n[sokf:x]: /k/x.md\n";
    let segments = markdown::segments(body, 5);
    let path = RepoPath::parse("/k/a.md").unwrap();
    let mut out = Expected::default();
    out.includes
        .insert((path.clone(), 8), Some("\nnew\n\n".into()));
    assert_eq!(
        expand(body, 5, &segments, &Body::default(), &path, &out),
        "# T\n\n\nnew\n\n\ntail"
    );
    let none = Expected::default();
    assert_eq!(
        expand(body, 5, &segments, &Body::default(), &path, &none),
        "# T\n\n\nold\n\n\ntail"
    );
    assert_eq!(
        expand(
            "\n\n",
            1,
            &Segments::default(),
            &Body::default(),
            &path,
            &none
        ),
        ""
    );
}

// sokf-tests: FORMAT_P-9
proptest! {
    #![proptest_config(crate::proptest_cases::cases(256))]

    #[test]
    fn the_definition_block_lists_each_label_once_in_ascending_order(labels in prop::collection::vec("[a-z]{1,4}(#[a-z]{1,3})?", 0..6)) {
        let entries: Vec<DefinitionEntry> = {
            let mut v: Vec<DefinitionEntry> = labels.iter().map(|l| DefinitionEntry {
                label: format!("sokf:{l}"),
                path: RepoPath::parse("/k/t.md").unwrap(),
                slug: l.split_once('#').map(|(_, s)| s.to_string()),
            }).collect();
            v.sort();
            v.dedup();
            v
        };
        let text = render_definitions(&entries);
        if entries.is_empty() {
            prop_assert_eq!(text, "");
        } else {
            let segments = markdown::segments(&text, 1);
            let def = segments.definition.unwrap();
            prop_assert!(!def.trailing);
            let got: Vec<String> = def.entries.iter().map(|(l, _)| l.clone()).collect();
            let want: Vec<String> = entries.iter().map(|e| e.label.clone()).collect();
            prop_assert_eq!(got.clone(), want);
            let mut sorted = got.clone();
            sorted.sort();
            sorted.dedup();
            prop_assert_eq!(got, sorted);
            for ((_, dest), e) in def.entries.iter().zip(&entries) {
                let want = match &e.slug { Some(s) => format!("/k/t.md#{s}"), None => "/k/t.md".into() };
                prop_assert_eq!(dest, &want);
            }
        }
    }
}

// sokf-tests: FORMAT_P-8
proptest! {
    #![proptest_config(crate::proptest_cases::cases(256))]

    #[test]
    fn an_expanded_body_has_no_marker_and_is_a_fixed_point(inner in "[a-z ]{0,20}", outer in "[a-z ]{0,20}") {
        let body = format!("{outer}\n\n<!-- sokf:include x -->\n\n{inner}\n\n<!-- /sokf:include -->\n\n<!-- sokf:links -->\n\n[sokf:x]: /k/x.md\n");
        let segments = markdown::segments(&body, 1);
        let path = RepoPath::parse("/k/a.md").unwrap();
        let mut out = Expected::default();
        out.includes.insert((path.clone(), 3), Some(interior(inner.trim())));
        let once = expand(&body, 1, &segments, &Body::default(), &path, &out);
        prop_assert!(!once.contains("sokf:include") && !once.contains("sokf:links"));
        let again = expand(&once, 1, &markdown::segments(&once, 1), &Body::default(), &path, &Expected::default());
        prop_assert_eq!(again, once);
    }
}
