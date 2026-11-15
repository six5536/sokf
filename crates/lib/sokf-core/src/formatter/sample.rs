//! This module holds the sample of each language: a short text that oxfmt,
//! Prettier, Biome, and dprint all format unchanged under sokf's settings,
//! so that a formatter that fails on it fails for a reason of its
//! environment, never of the text (design-loading, "Comparing by layout").

/// `SAMPLES` lists the sample of each extension that oxfmt or Prettier
/// formats. Glimmer, which formats Handlebars, removes the final newline.
const SAMPLES: &[(&str, &str)] = &[
    ("md", "# Sample\n"),
    ("markdown", "# Sample\n"),
    ("mdx", "# Sample\n"),
    ("json", "{ \"a\": 1 }\n"),
    ("jsonc", "{ \"a\": 1 }\n"),
    ("json5", "{ a: 1 }\n"),
    ("yaml", "a: 1\n"),
    ("yml", "a: 1\n"),
    ("toml", "a = 1\n"),
    ("js", "const a = 1;\n"),
    ("jsx", "const a = 1;\n"),
    ("mjs", "const a = 1;\n"),
    ("cjs", "const a = 1;\n"),
    ("ts", "const a = 1;\n"),
    ("tsx", "const a = 1;\n"),
    ("mts", "const a = 1;\n"),
    ("cts", "const a = 1;\n"),
    ("css", "a {\n  color: red;\n}\n"),
    ("scss", "a {\n  color: red;\n}\n"),
    ("less", "a {\n  color: red;\n}\n"),
    ("html", "<p>a</p>\n"),
    ("htm", "<p>a</p>\n"),
    ("vue", "<template><p>a</p></template>\n"),
    ("graphql", "type A {\n  a: Int\n}\n"),
    ("gql", "type A {\n  a: Int\n}\n"),
    ("hbs", "<p>{{a}}</p>"),
    ("handlebars", "<p>{{a}}</p>"),
];

/// `sample` returns the sample of the language `ext`, or `None` when sokf
/// has none, as for a language that only Biome, dprint, or a command of
/// `[format]` formats.
pub fn sample(ext: &str) -> Option<&'static str> {
    SAMPLES.iter().find(|(e, _)| *e == ext).map(|(_, t)| *t)
}

/// `extensions` returns the extensions that have a sample.
#[cfg(test)]
pub fn extensions() -> impl Iterator<Item = &'static str> {
    SAMPLES.iter().map(|(e, _)| *e)
}
