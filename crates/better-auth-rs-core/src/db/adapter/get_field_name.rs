//! Upstream reference: db/adapter/get-field-name.ts
//!
//! Get the field name to use in the database for `(model, field)` — i.e. the field's customized
//! `fieldName`, or the default field key when none is set.

use crate::db::types::BetterAuthDbSchema;
use crate::error::BetterAuthError;

use super::get_default_field_name::get_default_field_name;
use super::get_default_model_name::get_default_model_name;

/// Get the database column name for `(model, field)`.
///
/// # Errors
/// Returns a [`BetterAuthError`] if the model or field is not found.
pub fn get_field_name(
    schema: &BetterAuthDbSchema,
    use_plural: bool,
    model: &str,
    field: &str,
) -> Result<String, BetterAuthError> {
    let model = get_default_model_name(schema, use_plural, model)?;
    let field = get_default_field_name(schema, use_plural, &model, field)?;

    Ok(schema
        .get(&model)
        .and_then(|t| t.fields.get(&field))
        .and_then(|f| f.config.field_name.clone())
        .unwrap_or(field))
}
