use criterion::{criterion_group, criterion_main, Criterion};
use sqffy::ui::{DataTable, sql_value_cell};

fn bench_datatable(c: &mut Criterion) {
    c.bench_function("datatable_100x5", |b| {
        b.iter(|| {
            let mut t = DataTable::new().headers_with_types([
                ("id", "INTEGER"),
                ("name", "TEXT"),
                ("email", "TEXT"),
                ("score", "REAL"),
                ("active", "BOOLEAN"),
            ]);
            for i in 0..100 {
                t.push_row([
                    sql_value_cell(&i.to_string()),
                    sql_value_cell("alice"),
                    sql_value_cell("a@b.c"),
                    sql_value_cell("1.5"),
                    sql_value_cell("TRUE"),
                ]);
            }
            t.render()
        })
    });
}

criterion_group!(benches, bench_datatable);
criterion_main!(benches);
