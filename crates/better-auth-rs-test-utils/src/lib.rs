//! Backend-agnostic conformance harness for [`DatabaseAdapter`] (port of the `test-utils`
//! adapter suites). One battery of assertions, run against every backend — the memory adapter
//! locally and `sqlx-postgres` in CI — so all adapters share one executable spec.
//!
//! Test-only: not compiled into the shipped crate.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use better_auth_rs_core::db::{
    BetterAuthDbSchema, CountArgs, CreateArgs, DatabaseAdapter, DbFieldType, DbValue, DeleteArgs,
    FieldAttribute, FindManyArgs, FindOneArgs, IncrementArgs, Row, SortBy, SortDirection,
    TableSchema, UpdateArgs, Where, WhereOperator,
};

/// The model the harness exercises. Has a string, an int, a bool, and an optional string column
/// so every operator (including numeric `gt`/`lt`/`increment`) and nullability can be tested.
pub const MODEL: &str = "ba_test";

/// Schema for [`MODEL`], used to migrate SQL backends before running the harness.
pub fn test_schema() -> BetterAuthDbSchema {
    let fields = vec![
        ("name".to_string(), FieldAttribute::new(DbFieldType::String)),
        ("age".to_string(), FieldAttribute::new(DbFieldType::Number)),
        (
            "active".to_string(),
            FieldAttribute::new(DbFieldType::Boolean),
        ),
        (
            "tag".to_string(),
            FieldAttribute::new(DbFieldType::String).optional(),
        ),
    ];
    let mut schema = BetterAuthDbSchema::new();
    schema.insert(MODEL.to_string(), TableSchema::new(MODEL, fields));
    schema
}

fn row(pairs: &[(&str, DbValue)]) -> Row {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn person(name: &str, age: i64, active: bool, tag: Option<&str>) -> Row {
    row(&[
        ("name", name.into()),
        ("age", DbValue::Int(age)),
        ("active", DbValue::Bool(active)),
        ("tag", tag.map(DbValue::from).unwrap_or(DbValue::Null)),
    ])
}

fn names(rows: &[Row]) -> Vec<String> {
    rows.iter()
        .filter_map(|r| r.get("name").and_then(DbValue::as_str).map(str::to_string))
        .collect()
}

/// Run the full conformance battery against `adapter`, using [`MODEL`].
/// Panics (failing the calling test) on the first violated assertion.
pub async fn run_conformance(adapter: &dyn DatabaseAdapter) {
    // Start from an empty table.
    adapter
        .delete_many(DeleteArgs {
            model: MODEL.into(),
            r#where: vec![],
        })
        .await
        .unwrap();
    assert_eq!(count_all(adapter).await, 0, "table should start empty");

    // --- create -----------------------------------------------------------
    let alice = adapter
        .create(CreateArgs::new(MODEL, person("Alice", 30, true, Some("x"))))
        .await
        .unwrap();
    let alice_id = alice
        .get("id")
        .and_then(DbValue::as_str)
        .expect("create returns an id")
        .to_string();
    assert_eq!(alice.get("name"), Some(&DbValue::from("Alice")));
    assert_eq!(alice.get("age"), Some(&DbValue::Int(30)));
    assert_eq!(alice.get("active"), Some(&DbValue::Bool(true)));
    adapter
        .create(CreateArgs::new(MODEL, person("bob", 25, true, Some("y"))))
        .await
        .unwrap();
    adapter
        .create(CreateArgs::new(MODEL, person("Carol", 40, false, None)))
        .await
        .unwrap();

    // --- find_one ---------------------------------------------------------
    let found = adapter
        .find_one(FindOneArgs::new(MODEL, vec![Where::eq("name", "Alice")]))
        .await
        .unwrap();
    assert_eq!(
        found.and_then(|r| r.get("age").cloned()),
        Some(DbValue::Int(30))
    );
    let by_id = adapter
        .find_one(FindOneArgs::new(
            MODEL,
            vec![Where::eq("id", alice_id.as_str())],
        ))
        .await
        .unwrap();
    assert!(by_id.is_some(), "find_one by id");
    let missing = adapter
        .find_one(FindOneArgs::new(MODEL, vec![Where::eq("name", "nobody")]))
        .await
        .unwrap();
    assert!(
        missing.is_none(),
        "find_one returns None when nothing matches"
    );

    // --- count ------------------------------------------------------------
    assert_eq!(count_all(adapter).await, 3);
    let active = adapter
        .count(CountArgs {
            model: MODEL.into(),
            r#where: vec![Where::eq("active", true)],
        })
        .await
        .unwrap();
    assert_eq!(active, 2, "count with predicate");

    // --- operators (find_many) -------------------------------------------
    assert_eq!(
        sorted(
            &filter(
                adapter,
                vec![Where::op("age", WhereOperator::Gt, DbValue::Int(26))]
            )
            .await
        ),
        vec!["Alice", "Carol"]
    );
    assert_eq!(
        sorted(
            &filter(
                adapter,
                vec![Where::op("age", WhereOperator::Lte, DbValue::Int(25))]
            )
            .await
        ),
        vec!["bob"]
    );
    assert_eq!(
        sorted(&filter(adapter, vec![Where::op("name", WhereOperator::Ne, "Alice")]).await),
        vec!["Carol", "bob"]
    );
    assert_eq!(
        sorted(
            &filter(
                adapter,
                vec![Where::op("name", WhereOperator::Contains, "ar")]
            )
            .await
        ),
        vec!["Carol"]
    );
    assert_eq!(
        sorted(
            &filter(
                adapter,
                vec![Where::op("name", WhereOperator::StartsWith, "Car")]
            )
            .await
        ),
        vec!["Carol"]
    );
    assert_eq!(
        sorted(
            &filter(
                adapter,
                vec![Where::op("name", WhereOperator::EndsWith, "ob")]
            )
            .await
        ),
        vec!["bob"]
    );
    assert_eq!(
        sorted(
            &filter(
                adapter,
                vec![Where::op(
                    "name",
                    WhereOperator::In,
                    DbValue::StringArray(vec!["Alice".into(), "Carol".into()])
                )]
            )
            .await
        ),
        vec!["Alice", "Carol"]
    );
    assert_eq!(
        sorted(
            &filter(
                adapter,
                vec![Where::op(
                    "name",
                    WhereOperator::NotIn,
                    DbValue::StringArray(vec!["Alice".into(), "Carol".into()])
                )]
            )
            .await
        ),
        vec!["bob"]
    );

    // AND (two default-connector clauses) vs OR (two OR-connector clauses)
    assert_eq!(
        sorted(
            &filter(
                adapter,
                vec![
                    Where::op("age", WhereOperator::Gte, DbValue::Int(25)),
                    Where::eq("active", true)
                ]
            )
            .await
        ),
        vec!["Alice", "bob"]
    );
    assert_eq!(
        sorted(
            &filter(
                adapter,
                vec![
                    Where::eq("name", "Alice").or(),
                    Where::eq("name", "bob").or()
                ]
            )
            .await
        ),
        vec!["Alice", "bob"]
    );

    // --- sort / limit / offset / select ----------------------------------
    let desc = adapter
        .find_many(
            FindManyArgs::new(MODEL)
                .sort_by(SortBy {
                    field: "age".into(),
                    direction: SortDirection::Desc,
                })
                .limit(1),
        )
        .await
        .unwrap();
    assert_eq!(names(&desc), vec!["Carol"], "sort desc + limit");
    let asc_off = adapter
        .find_many(
            FindManyArgs::new(MODEL)
                .sort_by(SortBy {
                    field: "age".into(),
                    direction: SortDirection::Asc,
                })
                .offset(1),
        )
        .await
        .unwrap();
    assert_eq!(names(&asc_off), vec!["Alice", "Carol"], "sort asc + offset");
    let projected = adapter
        .find_one(FindOneArgs {
            model: MODEL.into(),
            r#where: vec![Where::eq("name", "Alice")],
            select: Some(vec!["name".into()]),
            join: None,
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        projected.keys().cloned().collect::<Vec<_>>(),
        vec!["name".to_string()],
        "select projects columns"
    );

    // --- case-insensitive -------------------------------------------------
    assert!(
        adapter
            .find_one(FindOneArgs::new(
                MODEL,
                vec![Where::eq("name", "ALICE").insensitive()]
            ))
            .await
            .unwrap()
            .is_some(),
        "case-insensitive eq"
    );
    assert_eq!(
        sorted(
            &filter(
                adapter,
                vec![Where::op("name", WhereOperator::Contains, "AR").insensitive()]
            )
            .await
        ),
        vec!["Carol"],
        "case-insensitive contains"
    );

    // --- update / update_many --------------------------------------------
    let updated = adapter
        .update(UpdateArgs {
            model: MODEL.into(),
            r#where: vec![Where::eq("name", "Alice")],
            update: row(&[("age", DbValue::Int(31))]),
        })
        .await
        .unwrap();
    assert_eq!(
        updated.and_then(|r| r.get("age").cloned()),
        Some(DbValue::Int(31))
    );
    let noop = adapter
        .update(UpdateArgs {
            model: MODEL.into(),
            r#where: vec![],
            update: row(&[("age", DbValue::Int(0))]),
        })
        .await
        .unwrap();
    assert!(noop.is_none(), "update with empty where is a no-op");
    let many = adapter
        .update_many(UpdateArgs {
            model: MODEL.into(),
            r#where: vec![Where::eq("active", true)],
            update: row(&[("tag", "z".into())]),
        })
        .await
        .unwrap();
    assert_eq!(many, 2, "update_many affected count");

    // --- delete / delete_many --------------------------------------------
    adapter
        .delete(DeleteArgs {
            model: MODEL.into(),
            r#where: vec![Where::eq("name", "bob")],
        })
        .await
        .unwrap();
    assert_eq!(
        count_all(adapter).await,
        2,
        "delete removes the matching row"
    );
    let noop_del = adapter
        .delete(DeleteArgs {
            model: MODEL.into(),
            r#where: vec![],
        })
        .await;
    assert!(
        noop_del.is_ok() && count_all(adapter).await == 2,
        "delete with empty where is a no-op"
    );
    let removed = adapter
        .delete_many(DeleteArgs {
            model: MODEL.into(),
            r#where: vec![Where::eq("active", false)],
        })
        .await
        .unwrap();
    assert_eq!(removed, 1, "delete_many affected count");

    // --- consume_one (atomic) --------------------------------------------
    adapter
        .create(CreateArgs::new(MODEL, person("Dave", 50, true, None)))
        .await
        .unwrap();
    let taken = adapter
        .consume_one(DeleteArgs {
            model: MODEL.into(),
            r#where: vec![Where::eq("name", "Dave")],
        })
        .await
        .unwrap();
    assert_eq!(
        taken.and_then(|r| r.get("name").cloned()),
        Some(DbValue::from("Dave"))
    );
    let taken_again = adapter
        .consume_one(DeleteArgs {
            model: MODEL.into(),
            r#where: vec![Where::eq("name", "Dave")],
        })
        .await
        .unwrap();
    assert!(taken_again.is_none(), "consume_one removes the row");

    // --- increment_one (guarded) -----------------------------------------
    adapter
        .delete_many(DeleteArgs {
            model: MODEL.into(),
            r#where: vec![],
        })
        .await
        .unwrap();
    adapter
        .create(CreateArgs::new(MODEL, person("Counter", 1, true, None)))
        .await
        .unwrap();
    let inc = adapter
        .increment_one(IncrementArgs {
            model: MODEL.into(),
            r#where: vec![Where::op("age", WhereOperator::Gt, DbValue::Int(0))],
            increment: [("age".to_string(), 1.0)].into_iter().collect(),
            set: None,
        })
        .await
        .unwrap();
    assert_eq!(
        inc.and_then(|r| r.get("age").cloned()),
        Some(DbValue::Int(2)),
        "increment applies the delta"
    );
    let blocked = adapter
        .increment_one(IncrementArgs {
            model: MODEL.into(),
            r#where: vec![Where::op("age", WhereOperator::Gt, DbValue::Int(100))],
            increment: [("age".to_string(), 1.0)].into_iter().collect(),
            set: None,
        })
        .await
        .unwrap();
    assert!(
        blocked.is_none(),
        "increment_one guard blocks when nothing matches"
    );

    // clean up
    adapter
        .delete_many(DeleteArgs {
            model: MODEL.into(),
            r#where: vec![],
        })
        .await
        .unwrap();
}

async fn count_all(adapter: &dyn DatabaseAdapter) -> u64 {
    adapter
        .count(CountArgs {
            model: MODEL.into(),
            r#where: vec![],
        })
        .await
        .unwrap()
}

async fn filter(adapter: &dyn DatabaseAdapter, wheres: Vec<Where>) -> Vec<Row> {
    adapter
        .find_many(FindManyArgs::new(MODEL).filter(wheres))
        .await
        .unwrap()
}

fn sorted(rows: &[Row]) -> Vec<String> {
    let mut n = names(rows);
    n.sort();
    n
}
