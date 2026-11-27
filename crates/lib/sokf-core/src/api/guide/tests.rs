//! The unit tests of the guide.

use super::*;
use crate::{
    Source,
    config::Texts,
    grammar::{RegexCache, compile, embedded_formats, load_grammar},
    markdown::is_guidance_marker,
    yaml,
};

/// `NOTE` is a schema with guidance on the document, a key, a link, a
/// section, and a block, and a limit on the block.
const NOTE: &str = "line-limit: 0\nguidance: A note records one decision.\nfrontmatter:\n  type:\n    const: Note\n  owner:\n    pattern: '^[a-z]+$'\n    example: someone\n    guidance: The person who answers for the decision.\nlinks:\n  - rel: relates-to\n    min: 0\n    guidance: A decision this one depends on.\nsections:\n  - heading: { from: title }\n    level: 1\n    blocks: []\n  - heading: Reason\n    level: 2\n    guidance: What made the other options worse.\n    blocks:\n      - kind: prose\n        guidance: One reason to each sentence.\n        max: { hard: 2, when-at-limit: 'Split the note.' }\n        example: The cache cannot hold a second copy of the index.\n";

// Each guidance line of the scaffold of a schema holds the words that the
// guide prints for the same part at the same states, and the guide of a
// document gives the example that the scaffold writes for each part.
// sokf-tests: CLI-17_AC-7
// sokf-tests: CLI-17_AC-38
#[test]
fn the_guide_holds_the_words_of_the_scaffolds_guidance_lines() {
    let grammar = load_grammar().unwrap();
    let regexes = RegexCache::new();
    let registry = embedded_formats(&grammar, &regexes).unwrap();
    let path = RepoPath::parse("/k/schemas/note.md").unwrap();
    let schema = compile(
        &yaml::parse(NOTE, 1).unwrap(),
        &grammar,
        &registry,
        &regexes,
        &path,
        "schema-note",
    )
    .unwrap()
    .0;
    let texts = Texts::english();
    let words = scaffold::Words::new(&texts, "en");
    let states = States::default();
    let host = scaffold::SokfScaffold::with_words(&schema, &words);
    let scaffold = derive::derive_one(&schema.core, &states, &host).unwrap();
    let text = "---\ntype: Note\nid: note-a\ntitle: A\n---\n\n# A\n";
    let source = Source::from_text(RepoPath::parse("/k/note-a.md").unwrap(), text);
    let parsed = parse_source(&source, ParseMode::Document);
    let doc = Document::new(source, parsed);
    let guide = document_guide(&doc, "Note", &schema, None, &states, &words);
    let mut said: Vec<String> = guide.text.iter().cloned().collect();
    said.extend(guide.frontmatter.iter().filter_map(|k| k.text.clone()));
    said.extend(guide.links.iter().filter_map(|l| l.text.clone()));
    for s in &guide.sections {
        said.extend(s.text.clone());
        said.extend(s.blocks.iter().filter_map(|b| b.text.clone()));
    }
    let lines: Vec<&str> = scaffold
        .lines()
        .map(str::trim)
        .filter(|l| is_guidance_marker(l))
        .map(|l| {
            l.strip_prefix("# sokf:guidance ")
                .or_else(|| {
                    l.strip_prefix("<!-- sokf:guidance ")
                        .and_then(|l| l.strip_suffix(" -->"))
                })
                .unwrap()
        })
        .collect();
    assert_eq!(lines.len(), 5, "{scaffold}");
    for words in lines {
        assert!(
            said.iter().any(|s| s == words),
            "{words:?} is not in {said:?}"
        );
    }
    let reason = &guide.sections[1];
    assert_eq!(
        reason.example.as_deref(),
        Some("## Reason\n\nThe cache cannot hold a second copy of the index.")
    );
    assert_eq!(
        guide.frontmatter[1].example.as_deref(),
        Some("owner: someone")
    );
    assert_eq!(
        guide.links[0].example,
        "links:\n  - rel: relates-to\n    to: target-id"
    );
}
