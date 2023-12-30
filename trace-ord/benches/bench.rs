use criterion::{criterion_group, criterion_main};
use trace_ord::{enumerate::ByFuel, lfenumerate::PredicateAbstraction};

fn enumeration_benchmark(c: &mut criterion::Criterion) {
    let mut group = c.benchmark_group("enumerate");
    group.sample_size(10);
    group.bench_function("enumerate", |b| {
        b.iter(|| {
            let mut predicates = ByFuel::<PredicateAbstraction>::default();
            predicates.advance(5).cloned().collect::<Vec<_>>()
        })
    });
    group.finish();
}

criterion_group!(benches, enumeration_benchmark);
criterion_main!(benches);
