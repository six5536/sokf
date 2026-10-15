//! This module implements `sokf locate` (CLI-8).

use std::collections::BTreeSet;

use crate::{Cache, CacheMode, RepoRoot, Result, config, graph, graph::Location, markdown::slug};

/// `locate` finds the file of each id in `ids`. Before it looks up the
/// ids, it brings the cache up to date with the files that changed. It
/// finds an id among the store's ids and a key among the store's keys,
/// and it finds an asset by the sidecar that pairs with it. When `asset`
/// is true and an id doesn't identify a sidecar, `locate` returns an
/// error. `locate` writes no file except the cache.
// sokf-implements: CLI-9_AC-3
// sokf-implements: CACHE-1_AC-27
// sokf-implements: CACHE-1_AC-21
pub fn locate(
    root: &RepoRoot,
    ids: &[String],
    asset: bool,
    cache: CacheMode,
) -> Result<Vec<Location>> {
    let config = config::load_config(root)?;
    let cache = Cache::open(root, cache);
    // `locate` needs the keys that the schemas capture only to resolve a
    // key reference, so it loads them only when one of the ids is a key
    // reference.
    if ids.iter().any(|id| graph::is_key_reference(id)) {
        let slugs = key_slugs(ids);
        let (model, keys) = super::load_keys_for(root, &config, &cache, &slugs)?;
        return graph::locate(&model, Some(&keys), ids, asset);
    }
    let model = super::load_for_lookup(root, &config, &cache)?;
    graph::locate(&model, None, ids, asset)
}

/// `key_slugs` returns the key of each key reference in `ids`, both as
/// written and as its slug.
pub fn key_slugs(ids: &[String]) -> BTreeSet<String> {
    let mut slugs = BTreeSet::new();
    for id in ids.iter().filter(|id| graph::is_key_reference(id)) {
        if let Some((_, key)) = id.split_once('#')
            && !key.is_empty()
        {
            slugs.insert(key.to_string());
            slugs.insert(slug(key));
        }
    }
    slugs
}
