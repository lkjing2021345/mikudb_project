use criterion::{criterion_group, criterion_main, Criterion};
use mikudb_boml::{decode, encode_to_vec, Document};

fn bench_document_creation(c: &mut Criterion) {
    c.bench_function("document_create", |b| {
        b.iter(|| {
            let mut doc = Document::new();
            doc.insert("name", "Miku");
            doc.insert("age", 16i64);
            doc.insert("version", "v3");
            doc
        })
    });
}

fn bench_document_serialize(c: &mut Criterion) {
    let mut doc = Document::new();
    doc.insert("name", "Miku");
    doc.insert("age", 16i64);
    doc.insert("active", true);
    doc.insert("score", 99.5f64);

    let value = doc.to_boml_value();

    c.bench_function("document_serialize", |b| {
        b.iter(|| encode_to_vec(&value))
    });
}

fn bench_document_deserialize(c: &mut Criterion) {
    let mut doc = Document::new();
    doc.insert("name", "Miku");
    doc.insert("age", 16i64);
    doc.insert("active", true);
    doc.insert("score", 99.5f64);

    let value = doc.to_boml_value();
    let encoded = encode_to_vec(&value).unwrap();

    c.bench_function("document_deserialize", |b| {
        b.iter(|| decode(&encoded))
    });
}

fn bench_nested_document(c: &mut Criterion) {
    c.bench_function("nested_document_create", |b| {
        b.iter(|| {
            let mut inner = Document::without_id();
            inner.insert("street", "123 Main St");
            inner.insert("city", "Tokyo");

            let mut doc = Document::new();
            doc.insert("name", "Miku");
            doc.insert("address", inner.to_boml_value());
            doc
        })
    });
}

criterion_group!(
    benches,
    bench_document_creation,
    bench_document_serialize,
    bench_document_deserialize,
    bench_nested_document,
);

criterion_main!(benches);
