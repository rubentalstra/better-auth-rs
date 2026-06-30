//! In-memory [`DatabaseAdapter`] for tests and development (port of the `memory-adapter`
//! package's store behavior).
//!
//! Backed by a `model → rows` map under a `std::sync::Mutex`; every operation locks, mutates,
//! and releases without awaiting, so `consume_one`/`increment_one` are atomic. Where-clause
//! evaluation, the empty-`where` no-op rule for singular mutations, sort, and limit/offset mirror
//! the upstream memory adapter. Joins are not yet applied (a later step in Phase 1).

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use async_trait::async_trait;
use better_auth_rs_core::db::{
    AdapterError, Connector, CountArgs, CreateArgs, DatabaseAdapter, DbValue, DeleteArgs,
    FindManyArgs, FindOneArgs, IncrementArgs, MatchMode, Row, SortBy, SortDirection, UpdateArgs,
    Where, WhereOperator,
};

/// An in-memory adapter. Cheap to clone-free share behind an `Arc`.
#[derive(Debug, Default)]
pub struct MemoryAdapter {
    store: Mutex<HashMap<String, Vec<Row>>>,
}

impl MemoryAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    fn store(&self) -> MutexGuard<'_, HashMap<String, Vec<Row>>> {
        // Recover rather than panic if a prior holder panicked mid-op.
        self.store.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

// --- value helpers ---------------------------------------------------------

fn as_f64(v: &DbValue) -> Option<f64> {
    match v {
        DbValue::Int(i) => Some(*i as f64),
        DbValue::Float(f) => Some(*f),
        _ => None,
    }
}

/// Total order across comparable values (used by gt/lt and sort). `None` when incomparable.
fn db_cmp(a: &DbValue, b: &DbValue) -> Option<Ordering> {
    match (a, b) {
        (DbValue::String(x), DbValue::String(y)) => Some(x.cmp(y)),
        (DbValue::DateTime(x), DbValue::DateTime(y)) => Some(x.cmp(y)),
        _ => match (as_f64(a), as_f64(b)) {
            (Some(x), Some(y)) => x.partial_cmp(&y),
            _ => None,
        },
    }
}

/// `eq` semantics: `eq null` matches a missing or null field; strings honor case `mode`;
/// numbers compare by value (Int/Float interchangeable).
fn value_eq(rv: Option<&DbValue>, target: &DbValue, mode: MatchMode) -> bool {
    if matches!(target, DbValue::Null) {
        return rv.is_none_or(DbValue::is_null);
    }
    let Some(v) = rv else { return false };
    if v.is_null() {
        return false;
    }
    match (v, target) {
        (DbValue::String(a), DbValue::String(b)) => {
            if mode == MatchMode::Insensitive {
                a.to_lowercase() == b.to_lowercase()
            } else {
                a == b
            }
        }
        _ => match (as_f64(v), as_f64(target)) {
            (Some(x), Some(y)) => x == y,
            _ => v == target,
        },
    }
}

fn in_array(rv: Option<&DbValue>, target: &DbValue, mode: MatchMode) -> bool {
    let Some(v) = rv else { return false };
    match (v, target) {
        (DbValue::String(s), DbValue::StringArray(arr)) => {
            if mode == MatchMode::Insensitive {
                let s = s.to_lowercase();
                arr.iter().any(|x| x.to_lowercase() == s)
            } else {
                arr.contains(s)
            }
        }
        (DbValue::Int(i), DbValue::IntArray(arr)) => arr.contains(i),
        _ => false,
    }
}

#[derive(Clone, Copy)]
enum StrOp {
    Contains,
    StartsWith,
    EndsWith,
}

fn str_op(rv: Option<&DbValue>, target: &DbValue, mode: MatchMode, op: StrOp) -> bool {
    let (Some(DbValue::String(s)), DbValue::String(t)) = (rv, target) else {
        return false;
    };
    let (s, t) = if mode == MatchMode::Insensitive {
        (s.to_lowercase(), t.to_lowercase())
    } else {
        (s.clone(), t.clone())
    };
    match op {
        StrOp::Contains => s.contains(&t),
        StrOp::StartsWith => s.starts_with(&t),
        StrOp::EndsWith => s.ends_with(&t),
    }
}

fn cmp_match(rv: Option<&DbValue>, target: &DbValue, accepted: &[Ordering]) -> bool {
    if matches!(target, DbValue::Null) {
        return false; // `value != null` guard, matching upstream
    }
    match rv {
        Some(v) => db_cmp(v, target).is_some_and(|o| accepted.contains(&o)),
        None => false,
    }
}

fn eval_clause(row: &Row, c: &Where) -> bool {
    let rv = row.get(&c.field);
    match c.operator {
        WhereOperator::In => in_array(rv, &c.value, c.mode),
        WhereOperator::NotIn => !in_array(rv, &c.value, c.mode),
        WhereOperator::Contains => str_op(rv, &c.value, c.mode, StrOp::Contains),
        WhereOperator::StartsWith => str_op(rv, &c.value, c.mode, StrOp::StartsWith),
        WhereOperator::EndsWith => str_op(rv, &c.value, c.mode, StrOp::EndsWith),
        WhereOperator::Ne => !value_eq(rv, &c.value, c.mode),
        WhereOperator::Gt => cmp_match(rv, &c.value, &[Ordering::Greater]),
        WhereOperator::Gte => cmp_match(rv, &c.value, &[Ordering::Greater, Ordering::Equal]),
        WhereOperator::Lt => cmp_match(rv, &c.value, &[Ordering::Less]),
        WhereOperator::Lte => cmp_match(rv, &c.value, &[Ordering::Less, Ordering::Equal]),
        WhereOperator::Eq => value_eq(rv, &c.value, c.mode),
    }
}

/// Combine clauses left-to-right by each clause's connector (mirrors the upstream fold).
fn matches(row: &Row, wheres: &[Where]) -> bool {
    let Some(first) = wheres.first() else {
        return true;
    };
    let mut result = eval_clause(row, first);
    for c in wheres {
        let r = eval_clause(row, c);
        result = match c.connector {
            Connector::Or => result || r,
            Connector::And => result && r,
        };
    }
    result
}

fn disp(v: &DbValue) -> String {
    v.to_json().to_string()
}

fn sort_cmp(a: Option<&DbValue>, b: Option<&DbValue>) -> Ordering {
    let av = a.filter(|v| !v.is_null());
    let bv = b.filter(|v| !v.is_null());
    match (av, bv) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => db_cmp(a, b).unwrap_or_else(|| disp(a).cmp(&disp(b))),
    }
}

fn apply_sort(rows: &mut [Row], sort: &SortBy) {
    rows.sort_by(|a, b| {
        let ord = sort_cmp(a.get(&sort.field), b.get(&sort.field));
        match sort.direction {
            SortDirection::Asc => ord,
            SortDirection::Desc => ord.reverse(),
        }
    });
}

fn project(mut row: Row, select: Option<&[String]>) -> Row {
    if let Some(sel) = select
        && !sel.is_empty()
    {
        row.retain(|k, _| sel.iter().any(|s| s == k));
    }
    row
}

#[async_trait]
impl DatabaseAdapter for MemoryAdapter {
    fn id(&self) -> &str {
        "memory"
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
            .or_insert_with(|| DbValue::String(super::generate_id()));
        self.store().entry(model).or_default().push(data.clone());
        Ok(project(data, select.as_deref()))
    }

    async fn find_one(&self, args: FindOneArgs) -> Result<Option<Row>, AdapterError> {
        let FindOneArgs {
            model,
            r#where,
            select,
            join: _,
        } = args;
        let store = self.store();
        let found = store
            .get(&model)
            .and_then(|rows| rows.iter().find(|r| matches(r, &r#where)).cloned());
        Ok(found.map(|r| project(r, select.as_deref())))
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
        let store = self.store();
        let mut rows: Vec<Row> = store
            .get(&model)
            .map(|rows| {
                rows.iter()
                    .filter(|r| matches(r, &r#where))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if let Some(sb) = &sort_by {
            apply_sort(&mut rows, sb);
        }
        if let Some(off) = offset {
            rows.drain(..(off as usize).min(rows.len()));
        }
        if let Some(lim) = limit {
            rows.truncate(lim as usize);
        }
        Ok(rows
            .into_iter()
            .map(|r| project(r, select.as_deref()))
            .collect())
    }

    async fn count(&self, args: CountArgs) -> Result<u64, AdapterError> {
        let CountArgs { model, r#where } = args;
        let store = self.store();
        let n = match store.get(&model) {
            Some(rows) if r#where.is_empty() => rows.len(),
            Some(rows) => rows.iter().filter(|r| matches(r, &r#where)).count(),
            None => 0,
        };
        Ok(n as u64)
    }

    async fn update(&self, args: UpdateArgs) -> Result<Option<Row>, AdapterError> {
        let UpdateArgs {
            model,
            r#where,
            update,
        } = args;
        // Singular mutation with an empty predicate is a no-op (match-all is for update_many).
        if r#where.is_empty() {
            return Ok(None);
        }
        let mut store = self.store();
        let Some(rows) = store.get_mut(&model) else {
            return Ok(None);
        };
        let mut first = None;
        for row in rows.iter_mut() {
            if matches(row, &r#where) {
                for (k, v) in &update {
                    row.insert(k.clone(), v.clone());
                }
                if first.is_none() {
                    first = Some(row.clone());
                }
            }
        }
        Ok(first)
    }

    async fn update_many(&self, args: UpdateArgs) -> Result<u64, AdapterError> {
        let UpdateArgs {
            model,
            r#where,
            update,
        } = args;
        let mut store = self.store();
        let Some(rows) = store.get_mut(&model) else {
            return Ok(0);
        };
        let mut count = 0;
        for row in rows.iter_mut() {
            if matches(row, &r#where) {
                for (k, v) in &update {
                    row.insert(k.clone(), v.clone());
                }
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete(&self, args: DeleteArgs) -> Result<(), AdapterError> {
        let DeleteArgs { model, r#where } = args;
        if r#where.is_empty() {
            return Ok(());
        }
        if let Some(rows) = self.store().get_mut(&model) {
            rows.retain(|r| !matches(r, &r#where));
        }
        Ok(())
    }

    async fn delete_many(&self, args: DeleteArgs) -> Result<u64, AdapterError> {
        let DeleteArgs { model, r#where } = args;
        let mut store = self.store();
        let Some(rows) = store.get_mut(&model) else {
            return Ok(0);
        };
        let before = rows.len();
        rows.retain(|r| !matches(r, &r#where));
        Ok((before - rows.len()) as u64)
    }

    async fn consume_one(&self, args: DeleteArgs) -> Result<Option<Row>, AdapterError> {
        let DeleteArgs { model, r#where } = args;
        let mut store = self.store();
        let Some(rows) = store.get_mut(&model) else {
            return Ok(None);
        };
        let idx = rows.iter().position(|r| matches(r, &r#where));
        Ok(idx.map(|i| rows.remove(i)))
    }

    async fn increment_one(&self, args: IncrementArgs) -> Result<Option<Row>, AdapterError> {
        let IncrementArgs {
            model,
            r#where,
            increment,
            set,
        } = args;
        let mut store = self.store();
        let Some(rows) = store.get_mut(&model) else {
            return Ok(None);
        };
        let Some(i) = rows.iter().position(|r| matches(r, &r#where)) else {
            return Ok(None);
        };
        let row = &mut rows[i];
        for (field, delta) in &increment {
            let was_int = matches!(row.get(field), Some(DbValue::Int(_)) | None);
            let current = match row.get(field) {
                Some(DbValue::Int(n)) => *n as f64,
                Some(DbValue::Float(f)) => *f,
                _ => 0.0,
            };
            let next = current + delta;
            let value = if was_int && next.fract() == 0.0 {
                DbValue::Int(next as i64)
            } else {
                DbValue::Float(next)
            };
            row.insert(field.clone(), value);
        }
        if let Some(set) = &set {
            for (k, v) in set {
                row.insert(k.clone(), v.clone());
            }
        }
        Ok(Some(row.clone()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn row(pairs: &[(&str, DbValue)]) -> Row {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    fn user(name: &str, age: i64) -> Row {
        row(&[("name", name.into()), ("age", DbValue::Int(age))])
    }

    async fn seed() -> MemoryAdapter {
        let a = MemoryAdapter::new();
        for r in [user("Alice", 30), user("bob", 25), user("Carol", 40)] {
            a.create(CreateArgs::new("user", r)).await.unwrap();
        }
        a
    }

    #[tokio::test]
    async fn create_assigns_id_and_finds_by_field() {
        let a = MemoryAdapter::new();
        let created = a
            .create(CreateArgs::new("user", user("Alice", 30)))
            .await
            .unwrap();
        assert!(created.get("id").and_then(DbValue::as_str).is_some());

        let found = a
            .find_one(FindOneArgs::new("user", vec![Where::eq("name", "Alice")]))
            .await
            .unwrap();
        assert_eq!(found.unwrap().get("name"), Some(&DbValue::from("Alice")));
    }

    #[tokio::test]
    async fn find_many_filters_sorts_and_paginates() {
        let a = seed().await;
        // age > 26 → Alice(30), Carol(40)
        let rows = a
            .find_many(FindManyArgs::new("user").filter(vec![Where::op(
                "age",
                WhereOperator::Gt,
                DbValue::Int(26),
            )]))
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);

        // sorted desc by age, limit 1 → Carol
        let top = a
            .find_many(
                FindManyArgs::new("user")
                    .sort_by(SortBy {
                        field: "age".into(),
                        direction: SortDirection::Desc,
                    })
                    .limit(1),
            )
            .await
            .unwrap();
        assert_eq!(top[0].get("name"), Some(&DbValue::from("Carol")));
    }

    #[tokio::test]
    async fn case_insensitive_and_string_ops() {
        let a = seed().await;
        let ci = a
            .find_one(FindOneArgs::new(
                "user",
                vec![Where::eq("name", "BOB").insensitive()],
            ))
            .await
            .unwrap();
        assert_eq!(ci.unwrap().get("name"), Some(&DbValue::from("bob")));

        let starts = a
            .find_many(FindManyArgs::new("user").filter(vec![Where::op(
                "name",
                WhereOperator::StartsWith,
                "Car",
            )]))
            .await
            .unwrap();
        assert_eq!(starts.len(), 1);
    }

    #[tokio::test]
    async fn update_empty_where_is_noop_but_update_mutates() {
        let a = seed().await;
        let noop = a
            .update(UpdateArgs {
                model: "user".into(),
                r#where: vec![],
                update: user("X", 1),
            })
            .await
            .unwrap();
        assert!(noop.is_none());

        let updated = a
            .update(UpdateArgs {
                model: "user".into(),
                r#where: vec![Where::eq("name", "Alice")],
                update: row(&[("age", DbValue::Int(31))]),
            })
            .await
            .unwrap();
        assert_eq!(updated.unwrap().get("age"), Some(&DbValue::Int(31)));
        assert_eq!(
            a.count(CountArgs {
                model: "user".into(),
                r#where: vec![]
            })
            .await
            .unwrap(),
            3
        );
    }

    #[tokio::test]
    async fn consume_one_removes_and_returns() {
        let a = seed().await;
        let taken = a
            .consume_one(DeleteArgs {
                model: "user".into(),
                r#where: vec![Where::eq("name", "bob")],
            })
            .await
            .unwrap();
        assert!(taken.is_some());
        // gone now
        let again = a
            .consume_one(DeleteArgs {
                model: "user".into(),
                r#where: vec![Where::eq("name", "bob")],
            })
            .await
            .unwrap();
        assert!(again.is_none());
        assert_eq!(
            a.count(CountArgs {
                model: "user".into(),
                r#where: vec![]
            })
            .await
            .unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn increment_one_is_guarded() {
        let a = MemoryAdapter::new();
        a.create(CreateArgs::new(
            "counter",
            row(&[("remaining", DbValue::Int(1))]),
        ))
        .await
        .unwrap();

        // remaining > 0 → decrement to 0
        let dec = a
            .increment_one(IncrementArgs {
                model: "counter".into(),
                r#where: vec![Where::op("remaining", WhereOperator::Gt, DbValue::Int(0))],
                increment: [("remaining".to_string(), -1.0)].into_iter().collect(),
                set: None,
            })
            .await
            .unwrap();
        assert_eq!(dec.unwrap().get("remaining"), Some(&DbValue::Int(0)));

        // guard now fails (remaining == 0) → no mutation, None
        let blocked = a
            .increment_one(IncrementArgs {
                model: "counter".into(),
                r#where: vec![Where::op("remaining", WhereOperator::Gt, DbValue::Int(0))],
                increment: [("remaining".to_string(), -1.0)].into_iter().collect(),
                set: None,
            })
            .await
            .unwrap();
        assert!(blocked.is_none());
    }

    #[tokio::test]
    async fn conformance() {
        let adapter = MemoryAdapter::new();
        crate::adapters::conformance::run_conformance(&adapter).await;
    }
}
