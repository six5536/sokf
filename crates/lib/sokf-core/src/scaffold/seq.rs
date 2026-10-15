//! For `sokf new`, this module finds the schema of a type, renders an id
//! from the schema's id template, and computes the next sequence number.

use super::IdTemplate;
use crate::{
    Compiled, Error, Model, Result,
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
/// the highest number in use plus one. It refuses when a document's
/// frontmatter does not parse or a document's id does not match the id
/// template. Without the refusal, the next number could repeat a number
/// that such a document already uses. It also refuses when the next number
/// needs more digits than the `seq` placeholder allows.
// sokf-implements: SOKF_SCHEMA_46
// sokf-implements: CLI-6_AC-4
// sokf-implements: CLI-6_AC-21
// sokf-implements: CLI-6_AC-47
// sokf-implements: CLI-6_AC-48
pub fn next_seq(
    model: &Model,
    knowledge: usize,
    type_name: &str,
    template: &IdTemplate,
) -> Result<u64> {
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
    let mut highest = 0u64;
    for (path, id) in model.ids_of_type(knowledge, type_name) {
        match template.seq_of(&id) {
            Some(n) => highest = highest.max(n),
            None => {
                return Err(refuse(format!(
                    "{path} has an id that does not match the id template"
                )));
            }
        }
    }
    let next = highest + 1;
    if next.to_string().len() > width {
        return Err(refuse(format!(
            "the next number {next} needs more than {width} digits"
        )));
    }
    Ok(next)
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
