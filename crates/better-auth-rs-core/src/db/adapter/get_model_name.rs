//! Upstream reference: db/adapter/get-model-name.ts
//!
//! Get the table name to use in the database for `model` — honoring a customized `modelName` and
//! the adapter's `usePlural` setting.

use crate::db::types::BetterAuthDbSchema;
use crate::error::BetterAuthError;

use super::get_default_model_name::get_default_model_name;

/// Get the database table name for `model`.
///
/// # Errors
/// Returns a [`BetterAuthError`] if the model is not found.
pub fn get_model_name(
    schema: &BetterAuthDbSchema,
    use_plural: bool,
    model: &str,
) -> Result<String, BetterAuthError> {
    let default_key = get_default_model_name(schema, use_plural, model)?;

    // A customized `modelName` (different from the requested `model`) is used as the table name.
    if let Some(table) = schema.get(&default_key)
        && table.model_name != model
    {
        return Ok(if use_plural {
            format!("{}s", table.model_name)
        } else {
            table.model_name.clone()
        });
    }

    Ok(if use_plural {
        format!("{model}s")
    } else {
        model.to_owned()
    })
}
