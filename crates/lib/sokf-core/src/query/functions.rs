//! This module implements the built-in functions of the JMESPath
//! specification over `serde_json::Value`.

use jmespath::ast::Ast;
use serde_json::{Map, Value};

use super::eval::{Arg, Failure, article, equal, eval, number, type_name};

/// `Type` is a type that a function's argument can have.
#[derive(Clone, Copy)]
enum Type {
    Any,
    Number,
    String,
    Object,
    Array,
    /// `NumbersOrStrings` is an array whose elements are all numbers or all strings.
    NumbersOrStrings,
    ArrayOfNumbers,
    ArrayOfStrings,
    StringOrArray,
    ArrayOrString,
    ArrayObjectOrString,
    Expr,
}

impl Type {
    fn name(self) -> &'static str {
        match self {
            Type::Any => "any value",
            Type::Number => "a number",
            Type::String => "a string",
            Type::Object => "an object",
            Type::Array => "an array",
            Type::NumbersOrStrings => "an array[number] or an array[string]",
            Type::ArrayOfNumbers => "an array[number]",
            Type::ArrayOfStrings => "an array[string]",
            Type::StringOrArray => "a string or an array",
            Type::ArrayOrString => "an array or a string",
            Type::ArrayObjectOrString => "an array, an object, or a string",
            Type::Expr => "an expression",
        }
    }

    fn accepts(self, arg: &Arg<'_>) -> bool {
        let value = match arg {
            Arg::Expr(_) => return matches!(self, Type::Expr),
            Arg::Value(v) => v,
        };
        let all = |f: fn(&Value) -> bool| value.as_array().is_some_and(|a| a.iter().all(f));
        match self {
            Type::Any => true,
            Type::Number => value.is_number(),
            Type::String => value.is_string(),
            Type::Object => value.is_object(),
            Type::Array => value.is_array(),
            Type::NumbersOrStrings => all(Value::is_number) || all(Value::is_string),
            Type::ArrayOfNumbers => all(Value::is_number),
            Type::ArrayOfStrings => all(Value::is_string),
            Type::StringOrArray | Type::ArrayOrString => value.is_string() || value.is_array(),
            Type::ArrayObjectOrString => value.is_string() || value.is_array() || value.is_object(),
            Type::Expr => false,
        }
    }
}

fn arg_type(arg: &Arg<'_>) -> &'static str {
    match arg {
        Arg::Expr(_) => "expression",
        Arg::Value(v) => type_name(v),
    }
}

/// `check` checks the number of arguments and the type of each one, and
/// returns the first mismatch as a failure. When `variadic` is true, the
/// last type also applies to every extra argument.
fn check(name: &str, args: &[Arg<'_>], types: &[Type], variadic: bool) -> Result<(), Failure> {
    let ok = if variadic {
        args.len() >= types.len()
    } else {
        args.len() == types.len()
    };
    if !ok {
        return Err(Failure::Arity {
            name: name.into(),
            expected: types.len(),
            variadic,
            actual: args.len(),
        });
    }
    for (i, arg) in args.iter().enumerate() {
        let t = types[i.min(types.len() - 1)];
        if !t.accepts(arg) {
            return Err(Failure::Type {
                name: name.into(),
                position: i + 1,
                expected: t.name().into(),
                actual: arg_type(arg).into(),
            });
        }
    }
    Ok(())
}

fn value<'a>(arg: &'a Arg<'_>) -> &'a Value {
    match arg {
        Arg::Value(v) => v,
        Arg::Expr(_) => &Value::Null,
    }
}

fn expr<'a>(arg: &Arg<'a>) -> &'a Ast {
    match arg {
        Arg::Expr(ast) => ast,
        Arg::Value(_) => unreachable!("checked as an expression"),
    }
}

fn array<'a>(arg: &'a Arg<'_>) -> &'a [Value] {
    value(arg).as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn f64s(arg: &Arg<'_>) -> Vec<f64> {
    array(arg).iter().filter_map(Value::as_f64).collect()
}

/// `call` calls the built-in function `name` with `args`, and returns its
/// result or the failure that stopped it.
pub(super) fn call(name: &str, args: &[Arg<'_>]) -> Result<Value, Failure> {
    use Type::*;
    let a = args;
    Ok(match name {
        "abs" => {
            check(name, a, &[Number], false)?;
            match value(&a[0]) {
                Value::Number(n) if n.is_i64() => match n.as_i64().and_then(i64::checked_abs) {
                    Some(i) => Value::from(i),
                    None => Value::from(i64::MIN.unsigned_abs()),
                },
                Value::Number(n) if n.is_u64() => Value::Number(n.clone()),
                other => number(other.as_f64().unwrap_or(0.0).abs()),
            }
        }
        "avg" => {
            check(name, a, &[ArrayOfNumbers], false)?;
            let v = f64s(&a[0]);
            if v.is_empty() {
                Value::Null
            } else {
                number(v.iter().sum::<f64>() / v.len() as f64)
            }
        }
        "ceil" | "floor" => {
            check(name, a, &[Number], false)?;
            match value(&a[0]) {
                Value::Number(n) if !n.is_f64() => Value::Number(n.clone()),
                other => {
                    let f = other.as_f64().unwrap_or(0.0);
                    number(if name == "ceil" { f.ceil() } else { f.floor() })
                }
            }
        }
        "contains" => {
            check(name, a, &[StringOrArray, Any], false)?;
            let needle = value(&a[1]);
            Value::Bool(match value(&a[0]) {
                Value::String(s) => needle.as_str().is_some_and(|n| s.contains(n)),
                Value::Array(items) => items.iter().any(|i| equal(i, needle)),
                _ => false,
            })
        }
        "ends_with" => {
            check(name, a, &[String, String], false)?;
            Value::Bool(text(&a[0]).ends_with(text(&a[1])))
        }
        "starts_with" => {
            check(name, a, &[String, String], false)?;
            Value::Bool(text(&a[0]).starts_with(text(&a[1])))
        }
        "join" => {
            check(name, a, &[String, ArrayOfStrings], false)?;
            let parts: Vec<&str> = array(&a[1]).iter().filter_map(Value::as_str).collect();
            Value::from(parts.join(text(&a[0])))
        }
        "keys" => {
            check(name, a, &[Object], false)?;
            Value::Array(
                object(&a[0])
                    .keys()
                    .map(|k| Value::from(k.as_str()))
                    .collect(),
            )
        }
        "values" => {
            check(name, a, &[Object], false)?;
            Value::Array(object(&a[0]).values().cloned().collect())
        }
        "length" => {
            check(name, a, &[ArrayObjectOrString], false)?;
            Value::from(match value(&a[0]) {
                Value::String(s) => s.chars().count(),
                Value::Array(items) => items.len(),
                Value::Object(o) => o.len(),
                _ => 0,
            })
        }
        "map" => {
            check(name, a, &[Expr, Array], false)?;
            let ast = expr(&a[0]);
            Value::Array(
                array(&a[1])
                    .iter()
                    .map(|v| eval(ast, v))
                    .collect::<Result<_, _>>()?,
            )
        }
        "max" | "min" => {
            check(name, a, &[NumbersOrStrings], false)?;
            pick(array(&a[0]).iter(), |v| v, name == "max")
                .cloned()
                .unwrap_or(Value::Null)
        }
        "max_by" | "min_by" => {
            check(name, a, &[Array, Expr], false)?;
            let keyed = keyed(name, array(&a[0]), expr(&a[1]))?;
            pick(keyed.into_iter(), |(_, k)| k, name == "max_by").map_or(Value::Null, |(v, _)| v)
        }
        "merge" => {
            check(name, a, &[Object], true)?;
            let mut out = Map::new();
            for arg in a {
                for (k, v) in object(arg) {
                    out.insert(k.clone(), v.clone());
                }
            }
            Value::Object(out)
        }
        "not_null" => {
            check(name, a, &[Any], true)?;
            a.iter()
                .map(value)
                .find(|v| !v.is_null())
                .cloned()
                .unwrap_or(Value::Null)
        }
        "reverse" => {
            check(name, a, &[ArrayOrString], false)?;
            match value(&a[0]) {
                Value::String(s) => Value::from(s.chars().rev().collect::<std::string::String>()),
                Value::Array(items) => Value::Array(items.iter().rev().cloned().collect()),
                _ => Value::Null,
            }
        }
        "sort" => {
            check(name, a, &[NumbersOrStrings], false)?;
            let mut items = array(&a[0]).to_vec();
            items.sort_by(cmp);
            Value::Array(items)
        }
        "sort_by" => {
            check(name, a, &[Array, Expr], false)?;
            let mut keyed = keyed(name, array(&a[0]), expr(&a[1]))?;
            keyed.sort_by(|x, y| cmp(&x.1, &y.1));
            Value::Array(keyed.into_iter().map(|(v, _)| v).collect())
        }
        "sum" => {
            check(name, a, &[ArrayOfNumbers], false)?;
            number(f64s(&a[0]).iter().sum())
        }
        "to_array" => {
            check(name, a, &[Any], false)?;
            match value(&a[0]) {
                Value::Array(_) => value(&a[0]).clone(),
                other => Value::Array(vec![other.clone()]),
            }
        }
        "to_number" => {
            check(name, a, &[Any], false)?;
            // A string converts only when it's exactly a JSON number, with no
            // whitespace around it.
            match value(&a[0]) {
                Value::Number(n) => Value::Number(n.clone()),
                Value::String(s) if s.trim() == s => match serde_json::from_str::<Value>(s) {
                    Ok(Value::Number(n)) if n.is_f64() => number(n.as_f64().unwrap_or(0.0)),
                    Ok(Value::Number(n)) => Value::Number(n),
                    _ => Value::Null,
                },
                _ => Value::Null,
            }
        }
        "to_string" => {
            check(name, a, &[Any], false)?;
            match value(&a[0]) {
                Value::String(s) => Value::from(s.as_str()),
                other => Value::from(serde_json::to_string(other).unwrap_or_default()),
            }
        }
        "type" => {
            check(name, a, &[Any], false)?;
            Value::from(arg_type(&a[0]))
        }
        _ => return Err(Failure::UnknownFunction(name.into())),
    })
}

fn text<'a>(arg: &'a Arg<'_>) -> &'a str {
    value(arg).as_str().unwrap_or("")
}

fn object<'a>(arg: &'a Arg<'_>) -> &'a Map<std::string::String, Value> {
    static EMPTY: std::sync::LazyLock<Map<std::string::String, Value>> =
        std::sync::LazyLock::new(Map::new);
    value(arg).as_object().unwrap_or(&EMPTY)
}

/// `cmp` orders two values that are both numbers or both strings, and
/// treats any other pair as equal.
fn cmp(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x
            .as_f64()
            .partial_cmp(&y.as_f64())
            .unwrap_or(std::cmp::Ordering::Equal),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}

/// `pick` returns the item whose key is the greatest, when `max` is true, or
/// the least, by `cmp`. Of items with equal keys, it returns the first.
fn pick<T>(items: impl Iterator<Item = T>, key: impl Fn(&T) -> &Value, max: bool) -> Option<T> {
    // `min_by` replaces the item that it holds when the comparison of that
    // item with the next one is `Greater`. So the next item replaces it
    // when its key is greater, for `max`, or less, for `min`.
    items.min_by(|best, item| {
        let order = cmp(key(item), key(best));
        if max { order } else { order.reverse() }
    })
}

/// `keyed` pairs each item with the key that `ast` returns for it. Every
/// key must be a number, or every key a string, and otherwise `keyed`
/// returns a type failure for the second argument.
fn keyed(name: &str, items: &[Value], ast: &Ast) -> Result<Vec<(Value, Value)>, Failure> {
    let mut out = Vec::with_capacity(items.len());
    let mut first: Option<&'static str> = None;
    for item in items {
        let key = eval(ast, item)?;
        let kind = type_name(&key);
        let ok = match first {
            None => kind == "number" || kind == "string",
            Some(f) => f == kind,
        };
        if !ok {
            return Err(Failure::Type {
                name: name.into(),
                position: 2,
                expected: match first {
                    None => "an expression that returns a number or a string".into(),
                    Some(f) => format!("an expression that returns {}", article(f)),
                },
                actual: kind.into(),
            });
        }
        first.get_or_insert(kind);
        out.push((item.clone(), key));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn run(expr: &str, data: &Value) -> Result<Value, Failure> {
        let compiled = jmespath::compile(expr).unwrap();
        super::super::eval::eval(compiled.as_ast(), data)
    }

    #[test]
    fn every_function_has_its_specification_result() {
        let d = json!({
            "n": [3, 1, 2], "s": ["b", "a"], "e": [], "o": {"k": "v", "j": 1},
            "str": "abc", "neg": -1.5, "mixed": [{"a": 2, "b": "y"}, {"a": 1, "b": "x"}]
        });
        let cases = [
            ("abs(neg)", json!(1.5)),
            ("avg(n)", json!(2)),
            ("avg(e)", Value::Null),
            ("ceil(neg)", json!(-1)),
            ("floor(neg)", json!(-2)),
            ("contains(str, 'bc')", json!(true)),
            ("contains(n, `1.0`)", json!(true)),
            ("contains(str, `1`)", json!(false)),
            ("ends_with(str, 'c')", json!(true)),
            ("starts_with(str, 'b')", json!(false)),
            ("join('-', s)", json!("b-a")),
            ("keys(o)", json!(["k", "j"])),
            ("values(o)", json!(["v", 1])),
            ("length(str)", json!(3)),
            ("length(o)", json!(2)),
            ("map(&a, mixed)", json!([2, 1])),
            ("max(n)", json!(3)),
            ("min(s)", json!("a")),
            ("max(e)", Value::Null),
            ("max_by(mixed, &a).b", json!("y")),
            ("min_by(mixed, &b).a", json!(1)),
            ("min_by(e, &a)", Value::Null),
            ("merge(o, `{\"k\": 2}`)", json!({"k": 2, "j": 1})),
            ("not_null(`null`, e, n)", json!([])),
            ("not_null(`null`)", Value::Null),
            ("reverse(str)", json!("cba")),
            ("reverse(n)", json!([2, 1, 3])),
            ("sort(n)", json!([1, 2, 3])),
            ("sort(s)", json!(["a", "b"])),
            ("sort_by(mixed, &a)[].b", json!(["x", "y"])),
            ("sort_by(e, &a)", json!([])),
            ("sum(n)", json!(6)),
            ("sum(e)", json!(0)),
            ("to_array(o)", json!([{"k": "v", "j": 1}])),
            ("to_array(n)", json!([3, 1, 2])),
            ("to_number('2.5')", json!(2.5)),
            ("to_number('1e3')", json!(1000)),
            ("to_number(' 1')", Value::Null),
            ("to_number('1 ')", Value::Null),
            ("to_number('x')", Value::Null),
            ("abs(`-9007199254740993`)", json!(9007199254740993i64)),
            ("abs(`-9223372036854775808`)", json!(9223372036854775808u64)),
            ("ceil(`9007199254740993`)", json!(9007199254740993i64)),
            ("floor(`-2`)", json!(-2)),
            ("to_number(neg)", json!(-1.5)),
            ("to_string(o)", json!("{\"k\":\"v\",\"j\":1}")),
            ("to_string(str)", json!("abc")),
            ("type(o)", json!("object")),
            ("type(`null`)", json!("null")),
            ("type(`true`)", json!("boolean")),
        ];
        for (expr, expected) in cases {
            assert_eq!(run(expr, &d).unwrap(), expected, "{expr}");
        }
        assert!(matches!(
            run("map(n, &a)", &d),
            Err(Failure::Type { position: 1, .. })
        ));
        assert!(matches!(run("sort(mixed)", &d), Err(Failure::Type { .. })));
        assert!(matches!(
            run("join('-', n)", &d),
            Err(Failure::Type { position: 2, .. })
        ));
        assert!(matches!(
            run("sort_by(mixed, &b, `1`)", &d),
            Err(Failure::Arity { .. })
        ));
        // A function accepts an expression reference only where its signature
        // declares one.
        assert!(matches!(
            run("type(&a)", &d),
            Err(Failure::Type { position: 1, .. })
        ));
        assert!(matches!(
            run("not_null(&a, `1`)", &d),
            Err(Failure::Type { .. })
        ));
    }
}
