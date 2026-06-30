//! Upstream reference: db/adapter/get-default-model-name.ts
//!
//! Resolve a (possibly custom or pluralized) model name back to its default schema key, so
//! `schema[key]` lookups work. Upstream's `initGetDefaultModelName` returns a closure capturing
//! `schema`/`usePlural`; in Rust this is a plain function taking them as parameters.

use crate::db::types::BetterAuthDbSchema;
use crate::error::BetterAuthError;

/// Get the default model key for `model` (which may be a custom `modelName` and/or pluralized).
///
/// # Errors
/// Returns a [`BetterAuthError`] if no matching model is found in the schema.
pub fn get_default_model_name(
    schema: &BetterAuthDbSchema,
    use_plural: bool,
    model: &str,
) -> Result<String, BetterAuthError> {
    // The `model` may have had `usePlural` applied; try again without the trailing `s`.
    if use_plural && model.ends_with('s') {
        let pluraless = &model[..model.len() - 1];
        if schema.contains_key(pluraless) {
            return Ok(pluraless.to_owned());
        }
        if let Some(key) = schema
            .iter()
            .find(|(_, t)| t.model_name == pluraless)
            .map(|(k, _)| k.clone())
        {
            return Ok(key);
        }
    }

    if schema.contains_key(model) {
        return Ok(model.to_owned());
    }
    if let Some(key) = schema
        .iter()
        .find(|(_, t)| t.model_name == model)
        .map(|(k, _)| k.clone())
    {
        return Ok(key);
    }

    Err(BetterAuthError::new(format!(
        "Model \"{model}\" not found in schema"
    )))
}

#[cfg(test)]
#[path = "get_default_model_name.test.rs"]
mod resolver_tests;
