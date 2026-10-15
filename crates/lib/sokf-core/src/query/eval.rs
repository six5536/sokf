//! This module evaluates a JMESPath expression against a `serde_json::Value`
//! by walking the syntax tree that the `jmespath` crate parses.

use std::fmt;

use jmespath::ast::{Ast, Comparator};
use serde_json::{Map, Value};

use super::functions;

/// `Failure` is an error that evaluating an expression can raise: one of the
/// runtime errors that the JMESPath specification defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Failure {
    /// The expression calls a function that the specification does not
    /// define.
    UnknownFunction(String),
    /// The expression calls a function with the wrong number of arguments.
    Arity {
        /// The name of the function.
        name: String,
        /// The number of arguments that the function takes, or the least
        /// number when it is variadic.
        expected: usize,
        /// Whether the function accepts more than `expected` arguments.
        variadic: bool,
        /// The number of arguments given.
        actual: usize,
    },
    /// The expression passes a function an argument of the wrong type, or an
    /// expression reference returns a value of the wrong type.
    Type {
        /// The name of the function.
        name: String,
        /// The position of the argument, counted from 1.
        position: usize,
        /// The types that the argument accepts.
        expected: String,
        /// The type of the value given.
        actual: String,
    },
    /// The expression takes a slice with a step of 0.
    Slice,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::UnknownFunction(name) => write!(f, "unknown function `{name}`"),
            Failure::Arity {
                name,
                expected,
                variadic,
                actual,
            } => write!(
                f,
                "`{name}` takes {}{expected} argument{}, but {actual} {} given",
                if *variadic { "at least " } else { "" },
                if *expected == 1 { "" } else { "s" },
                if *actual == 1 { "was" } else { "were" }
            ),
            Failure::Type {
                name,
                position,
                expected,
                actual,
            } => write!(
                f,
                "argument {position} of `{name}` must be {expected}, not {}",
                article(actual)
            ),
            Failure::Slice => f.write_str("a slice step must not be 0"),
        }
    }
}

/// `article` puts the indefinite article before a type name, as in "a
/// number" or "an array". It leaves `null` without one, because that type
/// has a single value.
pub(super) fn article(kind: &str) -> String {
    match kind {
        "null" => kind.to_string(),
        _ if kind.starts_with(['a', 'e', 'i', 'o', 'u']) => format!("an {kind}"),
        _ => format!("a {kind}"),
    }
}

/// `Arg` is a function argument: a value, or an expression reference
/// (`&expr`).
pub(super) enum Arg<'a> {
    /// An evaluated value.
    Value(Value),
    /// An expression that the function applies to each element.
    Expr(&'a Ast),
}

/// `truthy` returns the JMESPath truth of a value. A value is false when it
/// is `null`, `false`, an empty string, an empty array, or an empty object,
/// and true otherwise.
pub(super) fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(_) => true,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// `type_name` returns the JMESPath type name of a value.
pub(super) fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// `equal` tests two values for JMESPath equality. It compares two numbers
/// by value, but two integers exactly, so that large integers stay
/// distinct. It compares arrays element by element, and objects regardless
/// of the order of their keys.
pub(super) fn equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            if x.is_f64() || y.is_f64() {
                x.as_f64() == y.as_f64()
            } else {
                x == y
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| equal(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| equal(v, w)))
        }
        _ => a == b,
    }
}

/// `number` turns a numeric result into a value. A whole number that an
/// `f64` holds exactly becomes an integer, so that `1` prints as `1`, not
/// `1.0`. A number that JSON can't represent, such as NaN, becomes `null`.
pub(super) fn number(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < 9_007_199_254_740_992.0 {
        Value::from(f as i64)
    } else {
        serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number)
    }
}

/// `eval` evaluates `ast` against `data`.
pub(super) fn eval(ast: &Ast, data: &Value) -> Result<Value, Failure> {
    match ast {
        Ast::Identity { .. } => Ok(data.clone()),
        Ast::Field { name, .. } => Ok(data.get(name).cloned().unwrap_or(Value::Null)),
        Ast::Subexpr { lhs, rhs, .. } => {
            let left = eval(lhs, data)?;
            eval(rhs, &left)
        }
        Ast::Literal { value, .. } => Ok(serde_json::to_value(&**value).unwrap_or(Value::Null)),
        Ast::Index { idx, .. } => Ok(match data.as_array() {
            Some(a) => {
                let i = if *idx >= 0 {
                    *idx as usize
                } else {
                    match a.len().checked_sub(idx.unsigned_abs() as usize) {
                        Some(i) => i,
                        None => return Ok(Value::Null),
                    }
                };
                a.get(i).cloned().unwrap_or(Value::Null)
            }
            None => Value::Null,
        }),
        Ast::Or { lhs, rhs, .. } => {
            let left = eval(lhs, data)?;
            if truthy(&left) {
                Ok(left)
            } else {
                eval(rhs, data)
            }
        }
        Ast::And { lhs, rhs, .. } => {
            let left = eval(lhs, data)?;
            if truthy(&left) {
                eval(rhs, data)
            } else {
                Ok(left)
            }
        }
        Ast::Not { node, .. } => Ok(Value::Bool(!truthy(&eval(node, data)?))),
        Ast::Condition {
            predicate, then, ..
        } => {
            if truthy(&eval(predicate, data)?) {
                eval(then, data)
            } else {
                Ok(Value::Null)
            }
        }
        Ast::Comparison {
            comparator,
            lhs,
            rhs,
            ..
        } => {
            let left = eval(lhs, data)?;
            let right = eval(rhs, data)?;
            Ok(compare(comparator, &left, &right))
        }
        Ast::ObjectValues { node, .. } => Ok(match eval(node, data)? {
            Value::Object(o) => Value::Array(o.into_iter().map(|(_, v)| v).collect()),
            _ => Value::Null,
        }),
        Ast::Projection { lhs, rhs, .. } => match eval(lhs, data)? {
            Value::Array(items) => {
                let mut out = Vec::new();
                for item in &items {
                    let value = eval(rhs, item)?;
                    if !value.is_null() {
                        out.push(value);
                    }
                }
                Ok(Value::Array(out))
            }
            _ => Ok(Value::Null),
        },
        Ast::Flatten { node, .. } => match eval(node, data)? {
            Value::Array(items) => {
                let mut out = Vec::new();
                for item in items {
                    match item {
                        Value::Array(inner) => out.extend(inner),
                        other => out.push(other),
                    }
                }
                Ok(Value::Array(out))
            }
            _ => Ok(Value::Null),
        },
        Ast::MultiList { elements, .. } => {
            if data.is_null() {
                return Ok(Value::Null);
            }
            elements
                .iter()
                .map(|e| eval(e, data))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        // sokf-implements: CLI-7_AC-59
        Ast::MultiHash { elements, .. } => {
            if data.is_null() {
                return Ok(Value::Null);
            }
            let mut out = Map::new();
            for pair in elements {
                out.insert(pair.key.clone(), eval(&pair.value, data)?);
            }
            Ok(Value::Object(out))
        }
        Ast::Function { name, args, .. } => {
            let mut values = Vec::with_capacity(args.len());
            for arg in args {
                values.push(match arg {
                    Ast::Expref { ast, .. } => Arg::Expr(ast),
                    other => Arg::Value(eval(other, data)?),
                });
            }
            functions::call(name, &values)
        }
        // An expression reference has a meaning only as a function argument,
        // and anywhere else it evaluates to `null`.
        Ast::Expref { .. } => Ok(Value::Null),
        Ast::Slice {
            start, stop, step, ..
        } => {
            if *step == 0 {
                return Err(Failure::Slice);
            }
            Ok(match data.as_array() {
                Some(a) => Value::Array(slice(a, *start, *stop, *step)),
                None => Value::Null,
            })
        }
    }
}

fn compare(comparator: &Comparator, left: &Value, right: &Value) -> Value {
    match comparator {
        Comparator::Equal => Value::Bool(equal(left, right)),
        Comparator::NotEqual => Value::Bool(!equal(left, right)),
        ordering => match (left.as_f64(), right.as_f64()) {
            (Some(a), Some(b)) => Value::Bool(match ordering {
                Comparator::LessThan => a < b,
                Comparator::LessThanEqual => a <= b,
                Comparator::GreaterThan => a > b,
                _ => a >= b,
            }),
            _ => Value::Null,
        },
    }
}

/// `slice` returns a slice of an array as the specification defines it: with
/// the semantics of a Python slice, and with the endpoints clamped to the
/// array. It does the arithmetic in `i64`, so that a step near `i32::MAX`
/// can't overflow.
fn slice(array: &[Value], start: Option<i32>, stop: Option<i32>, step: i32) -> Vec<Value> {
    let len = array.len() as i64;
    let step = i64::from(step);
    if len == 0 {
        return Vec::new();
    }
    let endpoint = |e: i32| -> i64 {
        let e = i64::from(e);
        if e < 0 {
            let e = e + len;
            if e >= 0 {
                e
            } else if step < 0 {
                -1
            } else {
                0
            }
        } else if e < len {
            e
        } else if step < 0 {
            len - 1
        } else {
            len
        }
    };
    let a = start.map_or(if step < 0 { len - 1 } else { 0 }, endpoint);
    let b = stop.map_or(if step < 0 { -1 } else { len }, endpoint);
    let mut out = Vec::new();
    let mut i = a;
    if step > 0 {
        while i < b {
            out.push(array[i as usize].clone());
            i += step;
        }
    } else {
        while i > b {
            out.push(array[i as usize].clone());
            i += step;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(expr: &str, data: &Value) -> Result<Value, Failure> {
        let compiled = jmespath::compile(expr).unwrap();
        eval(compiled.as_ast(), data)
    }

    // sokf-tests: CLI_P-14
    #[test]
    fn objects_keep_the_order_read_and_hashes_the_order_written() {
        let data = json!({"nodes": [{"kind": "document", "path": "/k/a.md", "type": "Note", "id": "a", "status": "stable"}]});
        let out = run("nodes[0]", &data).unwrap();
        assert_eq!(
            serde_json::to_string(&out).unwrap(),
            r#"{"kind":"document","path":"/k/a.md","type":"Note","id":"a","status":"stable"}"#
        );
        let out = run("nodes[].{z: id, a: kind, m: {y: path, b: type}}", &data).unwrap();
        assert_eq!(
            serde_json::to_string(&out).unwrap(),
            r#"[{"z":"a","a":"document","m":{"y":"/k/a.md","b":"Note"}}]"#
        );
        let out = run("merge(nodes[0], `{\"id\": \"b\"}`)", &data).unwrap();
        assert_eq!(
            serde_json::to_string(&out).unwrap(),
            r#"{"kind":"document","path":"/k/a.md","type":"Note","id":"b","status":"stable"}"#
        );
        assert_eq!(
            run("keys(nodes[0])", &data).unwrap(),
            json!(["kind", "path", "type", "id", "status"])
        );
        assert_eq!(
            run("nodes[0].*", &data).unwrap(),
            json!(["document", "/k/a.md", "Note", "a", "stable"])
        );
    }

    #[test]
    fn numbers_print_as_integers_when_they_are() {
        assert_eq!(number(3.0), json!(3));
        assert_eq!(number(-2.0), json!(-2));
        assert_eq!(number(2.5), json!(2.5));
        assert_eq!(number(f64::NAN), Value::Null);
        assert!(equal(&json!(1), &json!(1.0)));
        assert!(!equal(
            &json!(9007199254740993i64),
            &json!(9007199254740992i64)
        ));
        assert!(equal(
            &json!(9007199254740993i64),
            &json!(9007199254740993i64)
        ));
        assert!(equal(
            &json!({"a": 1, "b": [1.0]}),
            &json!({"b": [1], "a": 1.0})
        ));
        assert!(!equal(&json!({"a": 1}), &json!({"a": 1, "b": 2})));
        assert!(!equal(&json!([1]), &json!([1, 2])));
        assert!(!equal(&json!("1"), &json!(1)));
    }

    #[test]
    fn failures_display_their_subject() {
        let data = json!({"a": [3, 1, 2], "s": "x"});
        assert_eq!(
            run("nope(a)", &data).unwrap_err().to_string(),
            "unknown function `nope`"
        );
        assert_eq!(
            run("length(a, a)", &data).unwrap_err().to_string(),
            "`length` takes 1 argument, but 2 were given"
        );
        assert_eq!(
            run("merge()", &data).unwrap_err().to_string(),
            "`merge` takes at least 1 argument, but 0 were given"
        );
        assert_eq!(
            run("abs(s)", &data).unwrap_err().to_string(),
            "argument 1 of `abs` must be a number, not a string"
        );
        assert_eq!(
            run("sort_by(a, &to_array(@))", &data)
                .unwrap_err()
                .to_string(),
            "argument 2 of `sort_by` must be an expression that returns a number or a string, not an array"
        );
        assert_eq!(
            run("a[::0]", &data).unwrap_err().to_string(),
            "a slice step must not be 0"
        );
        assert!(matches!(run("a[::0]", &data), Err(Failure::Slice)));
        // A bare expression reference evaluates to `null`.
        assert_eq!(run("&a", &data).unwrap(), Value::Null);
        assert_eq!(run("a[-4]", &data).unwrap(), Value::Null);
        assert_eq!(run("a[-1]", &data).unwrap(), json!(2));
    }
}
