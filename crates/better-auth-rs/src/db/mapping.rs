//! Upstream source: NONE — this is Rust-only glue with no `.ts` counterpart.
//!
//! In better-auth (TypeScript) the adapter passes plain `Record<string, any>` objects, so no
//! entity↔row conversion is needed. Rust has typed entities, so this bridge is required; there is
//! no `mapping.ts` to diff against.
//!
//! Typed-entity ↔ dynamic-[`Row`] mapping.
//!
//! The [`DatabaseAdapter`](better_auth_rs_core::db::DatabaseAdapter) is schema-agnostic: it speaks
//! `Row`/`DbValue`. The internal (domain) adapter works with typed entities (`User`, `Session`, …).
//! This module bridges the two via serde, using the table schema to recover types that JSON cannot
//! represent natively — notably dates, which an entity serializes as RFC 3339 strings but which must
//! become [`DbValue::DateTime`] so SQL backends bind them as `timestamptz` (and so date comparisons
//! work).

use better_auth_rs_core::db::{DbFieldType, DbValue, Row, TableSchema};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value as Json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Errors converting between a typed entity and a [`Row`].
#[derive(Debug, thiserror::Error)]
pub enum MappingError {
    /// The entity failed to serialize to JSON.
    #[error("serialize entity to row: {0}")]
    Serialize(String),
    /// A row failed to deserialize into the target entity.
    #[error("deserialize row to entity: {0}")]
    Deserialize(String),
    /// The entity serialized to something other than a JSON object.
    #[error("entity did not serialize to a JSON object")]
    NotObject,
}

/// Convert a JSON value to a [`DbValue`], using the field's declared type to recover dates
/// (serialized as RFC 3339 strings) as [`DbValue::DateTime`]. Other types map by JSON shape.
fn json_to_db_value(v: Json, field_type: Option<&DbFieldType>) -> DbValue {
    if matches!(field_type, Some(DbFieldType::Date))
        && let Json::String(s) = &v
        && let Ok(dt) = OffsetDateTime::parse(s, &Rfc3339)
    {
        return DbValue::DateTime(dt);
    }
    match v {
        Json::Null => DbValue::Null,
        Json::Bool(b) => DbValue::Bool(b),
        Json::Number(n) => n
            .as_i64()
            .map(DbValue::Int)
            .or_else(|| n.as_f64().map(DbValue::Float))
            .unwrap_or(DbValue::Null),
        Json::String(s) => DbValue::String(s),
        Json::Array(arr) if !arr.is_empty() && arr.iter().all(Json::is_string) => {
            DbValue::StringArray(
                arr.into_iter()
                    .filter_map(|x| match x {
                        Json::String(s) => Some(s),
                        _ => None,
                    })
                    .collect(),
            )
        }
        Json::Array(arr) if !arr.is_empty() && arr.iter().all(|x| x.is_i64()) => {
            DbValue::IntArray(arr.into_iter().filter_map(|x| x.as_i64()).collect())
        }
        other @ (Json::Array(_) | Json::Object(_)) => DbValue::Json(other),
    }
}

/// Serialize a typed entity into a [`Row`], typing each column via `schema`. Fields absent from the
/// schema (e.g. the implicit `id`, or additional plugin fields) map by JSON shape.
pub fn entity_to_row<T: Serialize>(entity: &T, schema: &TableSchema) -> Result<Row, MappingError> {
    let json = serde_json::to_value(entity).map_err(|e| MappingError::Serialize(e.to_string()))?;
    let Json::Object(map) = json else {
        return Err(MappingError::NotObject);
    };
    Ok(map
        .into_iter()
        .map(|(k, v)| {
            let field_type = schema.field(&k).map(|f| &f.r#type);
            let value = json_to_db_value(v, field_type);
            (k, value)
        })
        .collect())
}

/// Deserialize a [`Row`] into a typed entity. [`DbValue::DateTime`] renders back to an RFC 3339
/// string, which the entity's `time::serde::rfc3339` fields parse.
pub fn row_to_entity<T: DeserializeOwned>(row: &Row) -> Result<T, MappingError> {
    let map: Map<String, Json> = row.iter().map(|(k, v)| (k.clone(), v.to_json())).collect();
    serde_json::from_value(Json::Object(map)).map_err(|e| MappingError::Deserialize(e.to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use better_auth_rs_core::db::{User, core_tables};

    fn ts(secs: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(secs).unwrap()
    }

    fn user() -> User {
        User {
            id: "u1".into(),
            email: "a@b.com".into(),
            email_verified: true,
            name: "Ann".into(),
            image: None,
            created_at: ts(1_780_000_000),
            updated_at: ts(1_780_001_800),
        }
    }

    #[test]
    fn user_round_trips_through_row() {
        let tables = core_tables();
        let schema = tables.get("user").unwrap();
        let row = entity_to_row(&user(), schema).unwrap();

        // dates become DbValue::DateTime (so SQL binds timestamptz), not strings
        assert!(matches!(row.get("createdAt"), Some(DbValue::DateTime(_))));
        assert!(matches!(row.get("updatedAt"), Some(DbValue::DateTime(_))));
        assert_eq!(row.get("emailVerified"), Some(&DbValue::Bool(true)));
        assert_eq!(row.get("email"), Some(&DbValue::from("a@b.com")));
        // `image: None` serializes away (skip_serializing_if) — absent, not Null
        assert!(!row.contains_key("image"));

        let back: User = row_to_entity(&row).unwrap();
        assert_eq!(back, user());
    }

    #[test]
    fn unknown_fields_map_by_shape() {
        let tables = core_tables();
        let schema = tables.get("user").unwrap();
        let row = entity_to_row(&user(), schema).unwrap();
        // `id` is not in the schema field list (implicit PK) yet still maps as a string
        assert_eq!(row.get("id"), Some(&DbValue::from("u1")));
    }
}
