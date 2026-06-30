//! Behavior tests for the adapter query model and the `CustomAdapter` default methods.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Mutex;

use super::*;

#[test]
fn where_constructors_and_defaults() {
    let w = Where::eq("email", "a@b.com");
    assert_eq!(w.field, "email");
    assert_eq!(w.operator, WhereOperator::Eq);
    assert_eq!(w.connector, Connector::And);
    assert_eq!(w.mode, MatchMode::Sensitive);

    let w2 = Where::new("count", WhereOperator::Gt, 0_i64)
        .with_connector(Connector::Or)
        .with_mode(MatchMode::Insensitive);
    assert_eq!(w2.operator, WhereOperator::Gt);
    assert_eq!(w2.connector, Connector::Or);
    assert_eq!(w2.mode, MatchMode::Insensitive);
    assert_eq!(w2.value, DbValue::Int(0));
}

#[test]
fn operator_and_relation_defaults() {
    assert_eq!(WhereOperator::default(), WhereOperator::Eq);
    assert_eq!(Connector::default(), Connector::And);
    assert_eq!(MatchMode::default(), MatchMode::Sensitive);
    assert_eq!(RelationType::default(), RelationType::OneToMany);
}

// A minimal in-memory `CustomAdapter` (Eq-only matching on the given conditions) — just enough to
// exercise the default `consume_one` / `increment_one` implementations.
#[derive(Default)]
struct MiniAdapter {
    // model -> rows
    data: Mutex<HashMap<String, Vec<Row>>>,
}

impl MiniAdapter {
    fn matches(row: &Row, conditions: &[Where]) -> bool {
        conditions
            .iter()
            .all(|c| row.get(&c.field) == Some(&c.value))
    }
}

#[async_trait::async_trait]
impl CustomAdapter for MiniAdapter {
    async fn create(&self, args: CreateArgs) -> AdapterResult<Row> {
        self.data
            .lock()
            .unwrap()
            .entry(args.model)
            .or_default()
            .push(args.data.clone());
        Ok(args.data)
    }
    async fn update(&self, args: UpdateArgs) -> AdapterResult<Option<Row>> {
        let mut g = self.data.lock().unwrap();
        let rows = g.entry(args.model).or_default();
        for row in rows.iter_mut() {
            if Self::matches(row, &args.conditions) {
                for (k, v) in &args.update {
                    row.insert(k.clone(), v.clone());
                }
                return Ok(Some(row.clone()));
            }
        }
        Ok(None)
    }
    async fn update_many(&self, args: UpdateManyArgs) -> AdapterResult<u64> {
        let mut g = self.data.lock().unwrap();
        let rows = g.entry(args.model).or_default();
        let mut n = 0;
        for row in rows.iter_mut() {
            if Self::matches(row, &args.conditions) {
                for (k, v) in &args.update {
                    row.insert(k.clone(), v.clone());
                }
                n += 1;
            }
        }
        Ok(n)
    }
    async fn find_one(&self, args: FindOneArgs) -> AdapterResult<Option<Row>> {
        let g = self.data.lock().unwrap();
        Ok(g.get(&args.model).and_then(|rows| {
            rows.iter()
                .find(|r| Self::matches(r, &args.conditions))
                .cloned()
        }))
    }
    async fn find_many(&self, args: FindManyArgs) -> AdapterResult<Vec<Row>> {
        let g = self.data.lock().unwrap();
        Ok(g.get(&args.model)
            .map(|rows| {
                rows.iter()
                    .filter(|r| Self::matches(r, &args.conditions))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }
    async fn delete(&self, args: DeleteArgs) -> AdapterResult<()> {
        let mut g = self.data.lock().unwrap();
        if let Some(rows) = g.get_mut(&args.model) {
            rows.retain(|r| !Self::matches(r, &args.conditions));
        }
        Ok(())
    }
    async fn delete_many(&self, args: DeleteArgs) -> AdapterResult<u64> {
        let mut g = self.data.lock().unwrap();
        let rows = g.entry(args.model).or_default();
        let before = rows.len();
        rows.retain(|r| !Self::matches(r, &args.conditions));
        Ok((before - rows.len()) as u64)
    }
    async fn count(&self, args: CountArgs) -> AdapterResult<u64> {
        let g = self.data.lock().unwrap();
        Ok(g.get(&args.model)
            .map(|rows| {
                rows.iter()
                    .filter(|r| Self::matches(r, &args.conditions))
                    .count() as u64
            })
            .unwrap_or(0))
    }
}

fn row(pairs: &[(&str, DbValue)]) -> Row {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.clone()))
        .collect()
}

#[tokio::test]
async fn default_consume_one_finds_then_deletes() {
    let a = MiniAdapter::default();
    a.create(CreateArgs {
        model: "verification".into(),
        data: row(&[
            ("id", DbValue::from("v1")),
            ("value", DbValue::from("code")),
        ]),
        select: None,
    })
    .await
    .unwrap();

    let consumed = a
        .consume_one(ConsumeOneArgs {
            model: "verification".into(),
            conditions: vec![Where::eq("id", "v1")],
        })
        .await
        .unwrap();
    assert_eq!(consumed.unwrap().get("value"), Some(&DbValue::from("code")));

    // gone now -> second consume returns None
    let again = a
        .consume_one(ConsumeOneArgs {
            model: "verification".into(),
            conditions: vec![Where::eq("id", "v1")],
        })
        .await
        .unwrap();
    assert!(again.is_none());
}

#[tokio::test]
async fn default_increment_one_applies_delta_and_set() {
    let a = MiniAdapter::default();
    a.create(CreateArgs {
        model: "rateLimit".into(),
        data: row(&[("id", DbValue::from("k")), ("count", DbValue::Int(2))]),
        select: None,
    })
    .await
    .unwrap();

    let mut inc = BTreeMap::new();
    inc.insert("count".to_owned(), 3.0);
    let mut set = Row::new();
    set.insert("lastRequest".to_owned(), DbValue::Int(999));

    let updated = a
        .increment_one(IncrementOneArgs {
            model: "rateLimit".into(),
            conditions: vec![Where::eq("id", "k")],
            increment: inc,
            set: Some(set),
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.get("count"), Some(&DbValue::Int(5)));
    assert_eq!(updated.get("lastRequest"), Some(&DbValue::Int(999)));

    // guard that matches nothing -> None
    let none = a
        .increment_one(IncrementOneArgs {
            model: "rateLimit".into(),
            conditions: vec![Where::eq("id", "nope")],
            increment: BTreeMap::new(),
            set: None,
        })
        .await
        .unwrap();
    assert!(none.is_none());
}
