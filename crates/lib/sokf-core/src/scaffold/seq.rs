//! For `sokf new`, this module finds the schema of a type, renders an id
//! from the schema's id template, and computes the next sequence number or
//! checks a given one.

use super::IdTemplate;
use crate::{
    Compiled, Error, Model, RepoPath, Result,
    grammar::{SokfSchema, States},
    markdown::is_slug,
};

/// `schema_for` returns the knowledge and the compiled schema for
/// `type_name`. When `chosen` gives a store, the schema comes from that
/// store. Otherwise exactly one root must declare the type. Either way, the
/// store must declare the type in exactly one schema, and that schema must
/// pass the grammar. For `Schema`, `schema_for` returns the grammar's own
/// schema, and the knowledge is `None`.
// sokf-implements: CLI-6_AC-17
// sokf-implements: CLI-6_AC-27
// sokf-implements: CLI-6_AC-37
// sokf-implements: CLI-6_AC-60
pub fn schema_for<'a>(
    model: &Model,
    compiled: &'a Compiled,
    type_name: &str,
    chosen: Option<usize>,
) -> Result<(Option<usize>, &'a SokfSchema)> {
    if type_name == "Schema" {
        return Ok((None, &compiled.schema_document));
    }
    let refuse = |msg: String| Error::New(msg);
    let roots: Vec<usize> = model
        .knowledges
        .iter()
        .enumerate()
        .filter(|(i, k)| chosen.is_none_or(|c| c == *i) && !k.schemas_for(type_name).is_empty())
        .map(|(i, _)| i)
        .collect();
    let knowledge = match (roots.as_slice(), chosen) {
        ([], Some(c)) => {
            return Err(refuse(format!(
                "`{type_name}` has no schema in {}",
                model.knowledges[c].root
            )));
        }
        ([], None) => return Err(refuse(format!("`{type_name}` has no schema"))),
        ([one], _) => *one,
        _ => return Err(refuse(ambiguous(model, type_name, &roots))),
    };
    if model.knowledges[knowledge].schemas_for(type_name).len() > 1 {
        return Err(refuse(format!(
            "more than one schema declares `{type_name}`"
        )));
    }
    let Some((_, schema)) = compiled.for_type(model, knowledge, type_name) else {
        return Err(refuse(format!(
            "the schema of `{type_name}` does not pass the grammar"
        )));
    };
    Ok((Some(knowledge), schema))
}

/// `ambiguous` says that more than one root declares `type_name`, names the
/// roots, and lists the qualified type that chooses each one, as `graph` and
/// `locate` list the qualified forms of an ambiguous reference.
// sokf-implements: CLI-6_AC-63
fn ambiguous(model: &Model, type_name: &str, roots: &[usize]) -> String {
    let names: Vec<String> = roots
        .iter()
        .map(|&i| model.knowledges[i].root.to_string())
        .collect();
    let forms: Vec<String> = roots
        .iter()
        .filter_map(|&i| model.store_name(i))
        .map(|store| format!("`{store}:{type_name}`"))
        .collect();
    let advice = if forms.len() == roots.len() {
        format!(". Write {}", forms.join(" or "))
    } else {
        String::new()
    };
    format!(
        "more than one root declares `{type_name}`: {}{advice}",
        names.join(", ")
    )
}

/// `next_seq` returns the next sequence number for `type_name`, which is
/// the highest number in use plus one. It refuses what `numbers` refuses,
/// and a next number that needs more digits than the `seq` placeholder
/// allows.
// sokf-implements: SOKF_SCHEMA_46
// sokf-implements: CLI-6_AC-4
// sokf-implements: CLI-6_AC-21
pub fn next_seq(
    model: &Model,
    knowledge: usize,
    type_name: &str,
    template: &IdTemplate,
) -> Result<u64> {
    let (width, numbers) = numbers(model, knowledge, type_name, template)?;
    let next = numbers.iter().map(|d| d.seq).max().unwrap_or(0) + 1;
    if next.to_string().len() > width {
        return Err(Error::New(format!(
            "the next number {next} needs more than {width} digits"
        )));
    }
    Ok(next)
}

/// `given_seq` returns `seq`, the number that `--seq` gives, for
/// `type_name`. It refuses what `numbers` refuses, a number that needs more
/// digits than the `seq` placeholder allows, and a number that a document
/// of the type already has, whatever its other placeholders, because the
/// sequence number is unique within the type.
// sokf-implements: CLI-6_AC-69
// sokf-implements: CLI-6_AC-70
// sokf-implements: CLI-6_AC-71
pub fn given_seq(
    model: &Model,
    knowledge: usize,
    type_name: &str,
    template: &IdTemplate,
    seq: u64,
) -> Result<u64> {
    let (width, numbers) = numbers(model, knowledge, type_name, template)?;
    if seq.to_string().len() > width {
        return Err(Error::New(format!(
            "the number {seq} needs more than {width} digits"
        )));
    }
    if let Some(d) = numbers.iter().find(|d| d.seq == seq) {
        return Err(Error::New(format!(
            "`{}` in {} already has the number {seq}",
            d.id, d.path
        )));
    }
    Ok(seq)
}

/// `Numbered` is a document of a type with a `seq` placeholder: its path,
/// its id, and its sequence number.
struct Numbered {
    path: RepoPath,
    id: String,
    seq: u64,
}

/// `numbers` returns the width of the `seq` placeholder, and each document
/// of `type_name` with its number. It refuses
/// a template without a `seq` placeholder, a document whose frontmatter
/// does not parse, and a document whose id does not match the id template.
/// The number of such a document is unknown, so a number chosen without
/// the refusal could repeat it.
// sokf-implements: CLI-6_AC-47
// sokf-implements: CLI-6_AC-48
fn numbers(
    model: &Model,
    knowledge: usize,
    type_name: &str,
    template: &IdTemplate,
) -> Result<(usize, Vec<Numbered>)> {
    let refuse = |msg: String| Error::New(msg);
    let Some(width) = template.seq_width() else {
        return Err(refuse(format!(
            "the id template of `{type_name}` has no `seq` placeholder"
        )));
    };
    // A document whose frontmatter does not parse has no readable type, so
    // it might be of this type. `broken_in` checks both the documents of
    // this run and the rows of the store (CACHE-1_AC-21).
    if let Some(path) = model.broken_in(knowledge).first() {
        return Err(refuse(format!(
            "{path} has frontmatter that does not parse"
        )));
    }
    let mut numbers = Vec::new();
    for (path, id) in model.ids_of_type(knowledge, type_name) {
        let Some(seq) = template.seq_of(&id) else {
            return Err(refuse(format!(
                "{path} has an id that does not match the id template"
            )));
        };
        numbers.push(Numbered { path, id, seq });
    }
    Ok((width, numbers))
}

/// `render_id` fills in the id template with the number and the slug. It refuses a slug that is not lowercase letters and digits
/// in words joined by hyphens, a slug for a template without a `slug`
/// placeholder, and a missing slug for a template with a `slug` placeholder.
// sokf-implements: CLI-6_AC-5
// sokf-implements: CLI-6_AC-18
// sokf-implements: CLI-6_AC-23
// sokf-implements: CLI-6_AC-49
pub fn render_id(
    template: &IdTemplate,
    seq: u64,
    slug: Option<&str>,
    states: &States,
) -> Result<String> {
    let refuse = |msg: String| Error::New(msg);
    match slug {
        Some(s) if !template.has_slug() => {
            return Err(refuse(format!(
                "you gave `--slug {s}`, but the id template has no `slug` placeholder"
            )));
        }
        Some(s) if !is_slug(s) => {
            return Err(refuse(format!(
                "`{s}` is not a valid slug: use lowercase letters and digits in words joined by hyphens"
            )));
        }
        None if template.has_slug() => {
            return Err(refuse(
                "the id template has a `slug` placeholder, so `--slug` is required".to_string(),
            ));
        }
        _ => {}
    }
    template.render_with(seq, slug, states).map_err(refuse)
}
