//! Upstream reference: db/adapter/get-default-field-name.ts
//!
//! Resolve a (possibly custom) field name back to its default schema field key. `id`/`_id` always
//! resolve to `id` (every model is auto-given an `id` that plugin schemas can't redefine).

use crate::db::types::BetterAuthDbSchema;
use crate::error::BetterAuthError;

use super::get_default_model_name::get_default_model_name;

/// Get the default field key for `(model, field)`.
///
/// # Errors
/// Returns a [`BetterAuthError`] if the model or field is not found.
pub fn get_default_field_name(
    schema: &BetterAuthDbSchema,
    use_plural: bool,
    model: &str,
    field: &str,
) -> Result<String, BetterAuthError> {
    // `id` is auto-provided to every model and never appears in plugin schema fields.
    if field == "id" || field == "_id" {
        return Ok("id".to_owned());
    }

    let model = get_default_model_name(schema, use_plural, model)?;
    let table = schema
        .get(&model)
        .ok_or_else(|| BetterAuthError::new(format!("Model \"{model}\" not found in schema")))?;

    if table.fields.contains_key(field) {
        return Ok(field.to_owned());
    }
    // Otherwise search by a customized `fieldName`.
    if let Some(key) = table
        .fields
        .iter()
        .find(|(_, f)| f.config.field_name.as_deref() == Some(field))
        .map(|(k, _)| k.clone())
    {
        return Ok(key);
    }

    Err(BetterAuthError::new(format!(
        "Field {field} not found in model {model}"
    )))
}
