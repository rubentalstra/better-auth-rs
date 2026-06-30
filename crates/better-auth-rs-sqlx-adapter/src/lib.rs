//! PostgreSQL [`DatabaseAdapter`] backed by SQLx.
//!
//! Dynamic SQL is built with sqlx's own [`QueryBuilder`] (stable, sqlx-0.9-only) rather than a
//! query-builder crate, so the generated SQL matches better-auth's Postgres path exactly:
//!
//! - operators map to `= <> < <= > >= IN NOT IN LIKE`; `eq/ne null` use `IS [NOT] NULL`;
//! - case-insensitive matching uses native `ILIKE` for `contains/starts_with/ends_with` and
//!   `LOWER(col) = LOWER($n)` for equality/membership;
//! - `AND`-connector conditions form one group, `OR`-connector conditions another, combined as
//!   `(AND group) AND (OR group)`;
//! - `create`/`update`/`consume_one`/`increment_one` use `RETURNING *`; `count` uses `count("id")`;
//! - `consume_one`/`increment_one` scope to a single row via `id IN (SELECT id … LIMIT 1)`.
//!
//! Values bind through `push_bind` (never string-interpolated): booleans, timestamps (`timestamptz`)
//! and JSON (`jsonb`) bind natively; arrays bind as JSON text (parity with upstream `supportsArrays = false`).

use async_trait::async_trait;
use better_auth_rs_core::db::{
    AdapterError, BetterAuthDbSchema, Connector, CountArgs, CreateArgs, DatabaseAdapter,
    DbFieldType, DbValue, DeleteArgs, FindManyArgs, FindOneArgs, IncrementArgs, MatchMode, Row,
    SortDirection, TableSchema, UpdateArgs, Where, WhereOperator,
};
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::{Column, Postgres, QueryBuilder, Row as _, TypeInfo};
use time::OffsetDateTime;

const ADAPTER_ID: &str = "sqlx-postgres";

/// PostgreSQL adapter over a [`PgPool`]. Cheap to clone (the pool is `Arc`-backed).
#[derive(Clone, Debug)]
pub struct SqlxPostgresAdapter {
    pool: PgPool,
    use_plural: bool,
}

impl SqlxPostgresAdapter {
    /// Connect using a `DATABASE_URL`, creating a pool.
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self::from_pool(pool))
    }

    /// Wrap an existing pool.
    pub fn from_pool(pool: PgPool) -> Self {
        Self {
            pool,
            use_plural: false,
        }
    }

    /// Use plural table names (`user` → `users`).
    pub fn with_plural(mut self, plural: bool) -> Self {
        self.use_plural = plural;
        self
    }

    /// The underlying pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// The `CREATE TABLE IF NOT EXISTS` statements for a schema, ordered so foreign-key targets
    /// are created first.
    pub fn schema_sql(schema: &BetterAuthDbSchema) -> Vec<String> {
        let mut tables: Vec<(&String, &TableSchema)> = schema.iter().collect();
        tables.sort_by_key(|(name, t)| (t.order.unwrap_or(u32::MAX), (*name).clone()));
        tables.iter().map(|(_, t)| create_table_sql(t)).collect()
    }

    /// Create the schema's tables (idempotent).
    pub async fn run_migrations(&self, schema: &BetterAuthDbSchema) -> Result<(), AdapterError> {
        for stmt in Self::schema_sql(schema) {
            // SQL is generated from our own schema registry (not user input).
            sqlx::query(sqlx::AssertSqlSafe(stmt))
                .execute(&self.pool)
                .await
                .map_err(backend_err)?;
        }
        Ok(())
    }

    fn table(&self, model: &str) -> String {
        if self.use_plural {
            format!("{model}s")
        } else {
            model.to_string()
        }
    }
}

fn backend_err(e: sqlx::Error) -> AdapterError {
    AdapterError::Backend {
        adapter: ADAPTER_ID.to_string(),
        message: e.to_string(),
    }
}

/// `"ident"` with embedded quotes doubled.
fn quote_ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn pg_type(field_ty: &DbFieldType, bigint: bool) -> &'static str {
    match field_ty {
        DbFieldType::String | DbFieldType::Enum(_) => "text",
        DbFieldType::Number => {
            if bigint {
                "bigint"
            } else {
                "integer"
            }
        }
        DbFieldType::Boolean => "boolean",
        DbFieldType::Date => "timestamptz",
        DbFieldType::Json => "jsonb",
        // Arrays are stored as JSON text (parity with upstream supportsArrays = false).
        DbFieldType::StringArray | DbFieldType::NumberArray => "text",
    }
}

fn create_table_sql(t: &TableSchema) -> String {
    let mut s = format!(
        "CREATE TABLE IF NOT EXISTS {} (\n  {} text PRIMARY KEY",
        quote_ident(&t.model_name),
        quote_ident("id")
    );
    let mut fks = Vec::new();
    for (key, f) in &t.fields {
        let col = f.field_name.clone().unwrap_or_else(|| key.clone());
        s.push_str(&format!(
            ",\n  {} {}",
            quote_ident(&col),
            pg_type(&f.r#type, f.bigint)
        ));
        if f.required {
            s.push_str(" NOT NULL");
        }
        if f.unique {
            s.push_str(" UNIQUE");
        }
        if let Some(r) = &f.references {
            fks.push(format!(
                ",\n  FOREIGN KEY ({}) REFERENCES {} ({}) ON DELETE CASCADE",
                quote_ident(&col),
                quote_ident(&r.model),
                quote_ident(&r.field)
            ));
        }
    }
    for fk in fks {
        s.push_str(&fk);
    }
    s.push_str("\n)");
    s
}

/// Bind a value with `push_bind` (owned, injection-safe).
fn bind_value(qb: &mut QueryBuilder<Postgres>, v: &DbValue) {
    match v {
        DbValue::Null => {
            qb.push_bind(Option::<String>::None);
        }
        DbValue::Bool(b) => {
            qb.push_bind(*b);
        }
        DbValue::Int(i) => {
            qb.push_bind(*i);
        }
        DbValue::Float(f) => {
            qb.push_bind(*f);
        }
        DbValue::String(s) => {
            qb.push_bind(s.clone());
        }
        DbValue::DateTime(dt) => {
            qb.push_bind(*dt);
        }
        DbValue::Json(j) => {
            qb.push_bind(j.clone());
        }
        DbValue::StringArray(_) | DbValue::IntArray(_) => {
            qb.push_bind(serde_json::to_string(&v.to_json()).unwrap_or_else(|_| "[]".to_string()));
        }
    }
}

fn is_string(v: &DbValue) -> bool {
    matches!(v, DbValue::String(_))
}

/// Append a single predicate to `qb`.
fn push_condition(qb: &mut QueryBuilder<Postgres>, c: &Where) {
    let col = quote_ident(&c.field);
    let insensitive = c.mode == MatchMode::Insensitive;
    match c.operator {
        WhereOperator::Eq => {
            if c.value.is_null() {
                qb.push(&col).push(" IS NULL");
            } else if insensitive && is_string(&c.value) {
                qb.push("LOWER(").push(&col).push(") = LOWER(");
                bind_value(qb, &c.value);
                qb.push(")");
            } else {
                qb.push(&col).push(" = ");
                bind_value(qb, &c.value);
            }
        }
        WhereOperator::Ne => {
            if c.value.is_null() {
                qb.push(&col).push(" IS NOT NULL");
            } else if insensitive && is_string(&c.value) {
                qb.push("LOWER(").push(&col).push(") <> LOWER(");
                bind_value(qb, &c.value);
                qb.push(")");
            } else {
                qb.push(&col).push(" <> ");
                bind_value(qb, &c.value);
            }
        }
        WhereOperator::Lt => {
            qb.push(&col).push(" < ");
            bind_value(qb, &c.value);
        }
        WhereOperator::Lte => {
            qb.push(&col).push(" <= ");
            bind_value(qb, &c.value);
        }
        WhereOperator::Gt => {
            qb.push(&col).push(" > ");
            bind_value(qb, &c.value);
        }
        WhereOperator::Gte => {
            qb.push(&col).push(" >= ");
            bind_value(qb, &c.value);
        }
        WhereOperator::In => push_in(qb, &col, &c.value, false, insensitive),
        WhereOperator::NotIn => push_in(qb, &col, &c.value, true, insensitive),
        WhereOperator::Contains => push_like(qb, &col, &c.value, c.mode, "%{v}%"),
        WhereOperator::StartsWith => push_like(qb, &col, &c.value, c.mode, "{v}%"),
        WhereOperator::EndsWith => push_like(qb, &col, &c.value, c.mode, "%{v}"),
    }
}

fn push_in(
    qb: &mut QueryBuilder<Postgres>,
    col: &str,
    value: &DbValue,
    negate: bool,
    insensitive: bool,
) {
    let items: Vec<DbValue> = match value {
        DbValue::StringArray(a) => a.iter().cloned().map(DbValue::String).collect(),
        DbValue::IntArray(a) => a.iter().copied().map(DbValue::Int).collect(),
        other => vec![other.clone()], // scalar wrapped to a 1-element list (upstream behavior)
    };
    if items.is_empty() {
        // Empty IN is always-false; empty NOT IN is always-true.
        qb.push(if negate { "TRUE" } else { "FALSE" });
        return;
    }
    let lower = insensitive && items.iter().all(is_string);
    if lower {
        qb.push("LOWER(").push(col).push(")");
    } else {
        qb.push(col);
    }
    qb.push(if negate { " NOT IN (" } else { " IN (" });
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        if lower {
            qb.push("LOWER(");
            bind_value(qb, item);
            qb.push(")");
        } else {
            bind_value(qb, item);
        }
    }
    qb.push(")");
}

fn push_like(
    qb: &mut QueryBuilder<Postgres>,
    col: &str,
    value: &DbValue,
    mode: MatchMode,
    pattern: &str,
) {
    let Some(s) = value.as_str() else {
        qb.push("FALSE");
        return;
    };
    let bound = pattern.replace("{v}", s);
    qb.push(col).push(if mode == MatchMode::Insensitive {
        " ILIKE "
    } else {
        " LIKE "
    });
    qb.push_bind(bound);
}

/// Append the `WHERE` clause: `(AND-conn group) AND (OR-conn group)`, matching upstream.
fn push_where(qb: &mut QueryBuilder<Postgres>, wheres: &[Where]) {
    if wheres.is_empty() {
        return;
    }
    let and: Vec<&Where> = wheres
        .iter()
        .filter(|w| w.connector == Connector::And)
        .collect();
    let or: Vec<&Where> = wheres
        .iter()
        .filter(|w| w.connector == Connector::Or)
        .collect();
    let mut started = false;
    if !and.is_empty() {
        qb.push(" WHERE (");
        for (i, c) in and.iter().enumerate() {
            if i > 0 {
                qb.push(" AND ");
            }
            push_condition(qb, c);
        }
        qb.push(")");
        started = true;
    }
    if !or.is_empty() {
        qb.push(if started { " AND (" } else { " WHERE (" });
        for (i, c) in or.iter().enumerate() {
            if i > 0 {
                qb.push(" OR ");
            }
            push_condition(qb, c);
        }
        qb.push(")");
    }
}

fn push_select_cols(qb: &mut QueryBuilder<Postgres>, select: &Option<Vec<String>>) {
    match select {
        Some(cols) if !cols.is_empty() => {
            for (i, c) in cols.iter().enumerate() {
                if i > 0 {
                    qb.push(", ");
                }
                qb.push(quote_ident(c));
            }
        }
        _ => {
            qb.push("*");
        }
    }
}

fn get_opt<'r, T>(row: &'r PgRow, ord: usize) -> Option<T>
where
    T: sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres>,
{
    row.try_get::<Option<T>, _>(ord).ok().flatten()
}

/// Decode a whole row into a [`Row`], typing each column by its Postgres type.
fn row_to_row(row: &PgRow) -> Row {
    let mut out = Row::new();
    for col in row.columns() {
        let ord = col.ordinal();
        let value = match col.type_info().name() {
            "BOOL" => get_opt::<bool>(row, ord).map(DbValue::Bool),
            "INT8" => get_opt::<i64>(row, ord).map(DbValue::Int),
            "INT4" => get_opt::<i32>(row, ord).map(|i| DbValue::Int(i as i64)),
            "INT2" => get_opt::<i16>(row, ord).map(|i| DbValue::Int(i as i64)),
            "FLOAT8" => get_opt::<f64>(row, ord).map(DbValue::Float),
            "FLOAT4" => get_opt::<f32>(row, ord).map(|f| DbValue::Float(f as f64)),
            "TIMESTAMPTZ" => get_opt::<OffsetDateTime>(row, ord).map(DbValue::DateTime),
            "JSON" | "JSONB" => get_opt::<serde_json::Value>(row, ord).map(DbValue::Json),
            "UUID" => get_opt::<uuid::Uuid>(row, ord).map(|u| DbValue::String(u.to_string())),
            _ => get_opt::<String>(row, ord).map(DbValue::String),
        };
        out.insert(col.name().to_string(), value.unwrap_or(DbValue::Null));
    }
    out
}

fn project(mut row: Row, select: Option<&[String]>) -> Row {
    if let Some(sel) = select
        && !sel.is_empty()
    {
        row.retain(|k, _| sel.iter().any(|s| s == k));
    }
    row
}

/// Generate an opaque text id, used when `create` is called without one (better-auth ids are
/// opaque strings).
fn generate_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

#[async_trait]
impl DatabaseAdapter for SqlxPostgresAdapter {
    fn id(&self) -> &str {
        ADAPTER_ID
    }

    async fn create(&self, args: CreateArgs) -> Result<Row, AdapterError> {
        let CreateArgs {
            model,
            mut data,
            select,
            force_allow_id,
        } = args;
        if !force_allow_id {
            data.remove("id");
        }
        data.entry("id".into())
            .or_insert_with(|| DbValue::String(generate_id()));
        // Skip null columns so they default to NULL (and avoid binding a typed NULL).
        let entries: Vec<(&String, &DbValue)> = data
            .iter()
            .filter(|(k, v)| k.as_str() == "id" || !v.is_null())
            .collect();

        let mut qb = QueryBuilder::<Postgres>::new("INSERT INTO ");
        qb.push(quote_ident(&self.table(&model))).push(" (");
        for (i, (k, _)) in entries.iter().enumerate() {
            if i > 0 {
                qb.push(", ");
            }
            qb.push(quote_ident(k));
        }
        qb.push(") VALUES (");
        for (i, (_, v)) in entries.iter().enumerate() {
            if i > 0 {
                qb.push(", ");
            }
            bind_value(&mut qb, v);
        }
        qb.push(") RETURNING *");

        let row = qb
            .build()
            .fetch_one(&self.pool)
            .await
            .map_err(backend_err)?;
        Ok(project(row_to_row(&row), select.as_deref()))
    }

    async fn find_one(&self, args: FindOneArgs) -> Result<Option<Row>, AdapterError> {
        let FindOneArgs {
            model,
            r#where,
            select,
            join: _,
        } = args;
        let mut qb = QueryBuilder::<Postgres>::new("SELECT ");
        push_select_cols(&mut qb, &select);
        qb.push(" FROM ").push(quote_ident(&self.table(&model)));
        push_where(&mut qb, &r#where);
        qb.push(" LIMIT 1");
        let row = qb
            .build()
            .fetch_optional(&self.pool)
            .await
            .map_err(backend_err)?;
        Ok(row.as_ref().map(row_to_row))
    }

    async fn find_many(&self, args: FindManyArgs) -> Result<Vec<Row>, AdapterError> {
        let FindManyArgs {
            model,
            r#where,
            limit,
            offset,
            sort_by,
            select,
            join: _,
        } = args;
        let mut qb = QueryBuilder::<Postgres>::new("SELECT ");
        push_select_cols(&mut qb, &select);
        qb.push(" FROM ").push(quote_ident(&self.table(&model)));
        push_where(&mut qb, &r#where);
        if let Some(sort) = &sort_by {
            qb.push(" ORDER BY ").push(quote_ident(&sort.field));
            qb.push(match sort.direction {
                SortDirection::Asc => " ASC",
                SortDirection::Desc => " DESC",
            });
        }
        if let Some(l) = limit {
            qb.push(" LIMIT ").push_bind(l as i64);
        }
        if let Some(o) = offset {
            qb.push(" OFFSET ").push_bind(o as i64);
        }
        let rows = qb
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(backend_err)?;
        Ok(rows.iter().map(row_to_row).collect())
    }

    async fn count(&self, args: CountArgs) -> Result<u64, AdapterError> {
        let CountArgs { model, r#where } = args;
        let mut qb = QueryBuilder::<Postgres>::new("SELECT count(\"id\") FROM ");
        qb.push(quote_ident(&self.table(&model)));
        push_where(&mut qb, &r#where);
        let row = qb
            .build()
            .fetch_one(&self.pool)
            .await
            .map_err(backend_err)?;
        let n: i64 = row.try_get(0).map_err(backend_err)?;
        Ok(n.max(0) as u64)
    }

    async fn update(&self, args: UpdateArgs) -> Result<Option<Row>, AdapterError> {
        let UpdateArgs {
            model,
            r#where,
            update,
        } = args;
        // Singular mutation with an empty predicate (or nothing to set) is a no-op.
        if r#where.is_empty() || update.is_empty() {
            return Ok(None);
        }
        let mut qb = QueryBuilder::<Postgres>::new("UPDATE ");
        qb.push(quote_ident(&self.table(&model)));
        push_set(&mut qb, &update);
        push_where(&mut qb, &r#where);
        qb.push(" RETURNING *");
        let row = qb
            .build()
            .fetch_optional(&self.pool)
            .await
            .map_err(backend_err)?;
        Ok(row.as_ref().map(row_to_row))
    }

    async fn update_many(&self, args: UpdateArgs) -> Result<u64, AdapterError> {
        let UpdateArgs {
            model,
            r#where,
            update,
        } = args;
        if update.is_empty() {
            return Ok(0);
        }
        let mut qb = QueryBuilder::<Postgres>::new("UPDATE ");
        qb.push(quote_ident(&self.table(&model)));
        push_set(&mut qb, &update);
        push_where(&mut qb, &r#where);
        let res = qb.build().execute(&self.pool).await.map_err(backend_err)?;
        Ok(res.rows_affected())
    }

    async fn delete(&self, args: DeleteArgs) -> Result<(), AdapterError> {
        let DeleteArgs { model, r#where } = args;
        if r#where.is_empty() {
            return Ok(()); // never delete every row from a singular delete
        }
        let mut qb = QueryBuilder::<Postgres>::new("DELETE FROM ");
        qb.push(quote_ident(&self.table(&model)));
        push_where(&mut qb, &r#where);
        qb.build().execute(&self.pool).await.map_err(backend_err)?;
        Ok(())
    }

    async fn delete_many(&self, args: DeleteArgs) -> Result<u64, AdapterError> {
        let DeleteArgs { model, r#where } = args;
        let mut qb = QueryBuilder::<Postgres>::new("DELETE FROM ");
        qb.push(quote_ident(&self.table(&model)));
        push_where(&mut qb, &r#where);
        let res = qb.build().execute(&self.pool).await.map_err(backend_err)?;
        Ok(res.rows_affected())
    }

    async fn consume_one(&self, args: DeleteArgs) -> Result<Option<Row>, AdapterError> {
        let DeleteArgs { model, r#where } = args;
        let table = quote_ident(&self.table(&model));
        // DELETE the single matching row atomically and return it.
        let mut qb = QueryBuilder::<Postgres>::new("DELETE FROM ");
        qb.push(&table)
            .push(" WHERE \"id\" IN (SELECT \"id\" FROM ")
            .push(&table);
        push_where(&mut qb, &r#where);
        qb.push(" LIMIT 1) RETURNING *");
        let row = qb
            .build()
            .fetch_optional(&self.pool)
            .await
            .map_err(backend_err)?;
        Ok(row.as_ref().map(row_to_row))
    }

    async fn increment_one(&self, args: IncrementArgs) -> Result<Option<Row>, AdapterError> {
        let IncrementArgs {
            model,
            r#where,
            increment,
            set,
        } = args;
        if increment.is_empty() && set.as_ref().is_none_or(Row::is_empty) {
            return Ok(None);
        }
        let table = quote_ident(&self.table(&model));
        let mut qb = QueryBuilder::<Postgres>::new("UPDATE ");
        qb.push(&table).push(" SET ");
        let mut first = true;
        for (field, delta) in &increment {
            if !first {
                qb.push(", ");
            }
            first = false;
            let col = quote_ident(field);
            qb.push(&col).push(" = ").push(&col).push(" + ");
            // Whole deltas bind as bigint so an int column stays int.
            if delta.fract() == 0.0 {
                qb.push_bind(*delta as i64);
            } else {
                qb.push_bind(*delta);
            }
        }
        if let Some(set) = &set {
            for (field, value) in set {
                if !first {
                    qb.push(", ");
                }
                first = false;
                qb.push(quote_ident(field)).push(" = ");
                if value.is_null() {
                    qb.push("NULL");
                } else {
                    bind_value(&mut qb, value);
                }
            }
        }
        qb.push(" WHERE \"id\" IN (SELECT \"id\" FROM ")
            .push(&table);
        push_where(&mut qb, &r#where);
        qb.push(" LIMIT 1) RETURNING *");
        let row = qb
            .build()
            .fetch_optional(&self.pool)
            .await
            .map_err(backend_err)?;
        Ok(row.as_ref().map(row_to_row))
    }
}

/// Append `SET col = $n, …`, rendering null assignments as literal `NULL`.
fn push_set(qb: &mut QueryBuilder<Postgres>, update: &Row) {
    qb.push(" SET ");
    for (i, (k, v)) in update.iter().enumerate() {
        if i > 0 {
            qb.push(", ");
        }
        qb.push(quote_ident(k)).push(" = ");
        if v.is_null() {
            qb.push("NULL");
        } else {
            bind_value(qb, v);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // Integration tests run only when DATABASE_URL points at a reachable Postgres
    // (CI provides one); otherwise they skip, so the suite stays green without a DB.
    async fn adapter() -> Option<SqlxPostgresAdapter> {
        let url = std::env::var("DATABASE_URL").ok()?;
        let a = SqlxPostgresAdapter::connect(&url).await.ok()?;
        a.run_migrations(&better_auth_rs_core::db::core_tables())
            .await
            .ok()?;
        a.delete_many(DeleteArgs {
            model: "user".into(),
            r#where: vec![],
        })
        .await
        .ok()?;
        Some(a)
    }

    fn sample_user(email: &str) -> Row {
        let now = OffsetDateTime::now_utc();
        [
            ("email".to_string(), DbValue::from(email)),
            ("name".to_string(), DbValue::from("Ann")),
            ("emailVerified".to_string(), DbValue::Bool(false)),
            ("createdAt".to_string(), DbValue::DateTime(now)),
            ("updatedAt".to_string(), DbValue::DateTime(now)),
        ]
        .into_iter()
        .collect()
    }

    #[tokio::test]
    async fn create_find_update_delete() {
        let Some(a) = adapter().await else {
            eprintln!("skipping: DATABASE_URL not set / Postgres unreachable");
            return;
        };

        let created = a
            .create(CreateArgs::new("user", sample_user("a@b.com")))
            .await
            .unwrap();
        assert!(created.get("id").and_then(DbValue::as_str).is_some());
        assert_eq!(created.get("email"), Some(&DbValue::from("a@b.com")));

        let found = a
            .find_one(FindOneArgs::new(
                "user",
                vec![Where::eq("email", "a@b.com")],
            ))
            .await
            .unwrap();
        assert!(found.is_some());

        // case-insensitive lookup
        let ci = a
            .find_one(FindOneArgs::new(
                "user",
                vec![Where::eq("email", "A@B.COM").insensitive()],
            ))
            .await
            .unwrap();
        assert!(ci.is_some());

        let count = a
            .count(CountArgs {
                model: "user".into(),
                r#where: vec![],
            })
            .await
            .unwrap();
        assert_eq!(count, 1);

        let updated = a
            .update(UpdateArgs {
                model: "user".into(),
                r#where: vec![Where::eq("email", "a@b.com")],
                update: [("name".to_string(), DbValue::from("Bob"))]
                    .into_iter()
                    .collect(),
            })
            .await
            .unwrap();
        assert_eq!(
            updated.and_then(|r| r.get("name").cloned()),
            Some(DbValue::from("Bob"))
        );

        let deleted = a
            .delete_many(DeleteArgs {
                model: "user".into(),
                r#where: vec![Where::eq("email", "a@b.com")],
            })
            .await
            .unwrap();
        assert_eq!(deleted, 1);
    }

    #[tokio::test]
    async fn conformance() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            eprintln!("skipping: DATABASE_URL not set");
            return;
        };
        let Ok(a) = SqlxPostgresAdapter::connect(&url).await else {
            eprintln!("skipping: Postgres unreachable");
            return;
        };
        a.run_migrations(&better_auth_rs_test_utils::test_schema())
            .await
            .unwrap();
        better_auth_rs_test_utils::run_conformance(&a).await;
    }
}
