//! `Expr` is a compiled JMESPath expression over `serde_json::Value`. The
//! `jmespath` crate parses the expression, and the `eval` module walks its
//! syntax tree, so an object read from the input keeps its key order.

use std::fmt;

use serde_json::Value;

use super::eval;
use crate::{Error, Result};

/// `Expr` is a compiled JMESPath expression, kept with the source text it
/// was compiled from.
#[derive(Clone)]
pub struct Expr {
    source: String,
    compiled: jmespath::Expression<'static>,
}

impl Expr {
    /// `compile` compiles `source`. A parse failure is an [`Error::Query`]
    /// that carries the `jmespath` crate's message.
    // sokf-implements: CLI-4_AC-17
    // sokf-implements: CLI-7_AC-21
    pub fn compile(source: &str) -> Result<Expr> {
        let compiled = jmespath::compile(source)
            .map_err(|e| Error::Query(e.to_string().trim_end().to_string()))?;
        Ok(Expr {
            source: source.to_string(),
            compiled,
        })
    }

    /// `source` returns the expression as written.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// `eval` evaluates the expression against `value`. An object in the
    /// result keeps the key order that it was read with. An object literal in
    /// the expression is the exception: its keys come out sorted, because the
    /// `jmespath` crate parses it into a sorted map. A runtime
    /// failure is an [`Error::Query`].
    // sokf-implements: CLI-4_AC-14
    // sokf-implements: CLI-4_AC-17
    // sokf-implements: CLI-7_AC-22
    pub fn eval(&self, value: &Value) -> Result<Value> {
        eval::eval(self.compiled.as_ast(), value).map_err(|e| Error::Query(e.to_string()))
    }

    /// `truthy` evaluates the expression against `value` and returns whether
    /// the result is true by JMESPath's rules, as `graph` filters require
    /// (CLI-7_AC-44). It returns `false` when the evaluation fails at
    /// runtime.
    // sokf-implements: CLI-7_AC-43
    // sokf-implements: CLI-7_AC-44
    // sokf-implements: CLI-7_AC-11
    pub fn truthy(&self, value: &Value) -> bool {
        eval::eval(self.compiled.as_ast(), value)
            .map(|v| eval::truthy(&v))
            .unwrap_or(false)
    }
}

impl fmt::Debug for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Expr").field(&self.source).finish()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn compiles_or_refuses_with_the_crate_message() {
        let e = Expr::compile("type=='Decision'").unwrap();
        assert_eq!(e.source(), "type=='Decision'");
        assert!(format!("{e:?}").contains("type=='Decision'"));
        let err = Expr::compile("nodes[?").unwrap_err();
        assert!(matches!(err, Error::Query(_)));
        assert!(err.to_string().starts_with("query: "), "{err}");
    }

    #[test]
    fn eval_projects_and_reports_runtime_failures() {
        let e = Expr::compile("nodes[?kind=='document'].id").unwrap();
        let value = json!({"nodes": [
            {"kind": "document", "id": "a"},
            {"kind": "file"},
            {"kind": "document", "id": "b"}
        ]});
        assert_eq!(e.eval(&value).unwrap(), json!(["a", "b"]));
        assert_eq!(
            Expr::compile("nope").unwrap().eval(&value).unwrap(),
            Value::Null
        );
        let bad = Expr::compile("contains(tags, 'core')").unwrap();
        assert!(matches!(bad.eval(&json!({})), Err(Error::Query(_))));
    }

    // sokf-tests: CLI-7_AC-44
    // sokf-tests: CLI-7_AC-11
    #[test]
    fn truthy_follows_jmespath_and_treats_failure_as_false() {
        let doc = json!({"type": "Decision", "tags": ["core"], "title": "", "n": 0, "list": [], "map": {}});
        let cases = [
            ("type=='Decision'", true),
            ("type=='Issue'", false),
            ("contains(tags, 'core')", true),
            ("title", false),
            ("n", true),
            ("list", false),
            ("map", false),
            ("missing", false),
            ("!missing", true),
            ("tags", true),
        ];
        for (src, expected) in cases {
            assert_eq!(Expr::compile(src).unwrap().truthy(&doc), expected, "{src}");
        }
        // `tags` is absent, so `contains` fails with a type error, which
        // `truthy` treats as false.
        let no_tags = json!({"type": "Decision"});
        assert!(
            !Expr::compile("contains(tags, 'core')")
                .unwrap()
                .truthy(&no_tags)
        );
        assert!(Expr::compile("!resource").unwrap().truthy(&no_tags));
    }
}
