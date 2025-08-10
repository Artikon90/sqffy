use criterion::{criterion_group, criterion_main, Criterion};
use sqffy::complete::{Completer, SchemaCache};
use sqffy::db::{ColumnInfo, TableInfo};
use std::collections::HashMap;

fn sample() -> Completer {
    let mut columns = HashMap::new();
    columns.insert(
        "users".into(),
        vec![
            ColumnInfo {
                name: "id".into(),
                type_name: "INTEGER".into(),
                not_null: true,
                default_value: None,
                primary_key: true,
                comment: None,
            },
            ColumnInfo {
                name: "email".into(),
                type_name: "TEXT".into(),
                not_null: false,
                default_value: None,
                primary_key: false,
                comment: None,
            },
        ],
    );
    Completer::new(SchemaCache::from_parts(
        vec![TableInfo {
            name: "users".into(),
            schema: None,
            kind: "table".into(),
        }],
        columns,
    ))
}

fn bench_complete(c: &mut Criterion) {
    let completer = sample();
    c.bench_function("complete_where", |b| {
        b.iter(|| completer.candidates("SELECT * FROM users WHERE e"))
    });
}

criterion_group!(benches, bench_complete);
criterion_main!(benches);
