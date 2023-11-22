use criterion::{criterion_group, criterion_main, Criterion};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use streaming_transpositions::{random_traces, OgRank2CurRank};

fn criterion_benchmark(c: &mut Criterion) {
    let trace_len = 120;
    let traces = random_traces(trace_len, 500, 30, 10);
    c.bench_function("streaming transpositions benchmark", |b| {
        b.iter(|| {
            let mut st =
                streaming_transpositions::StreamingTranspositions::new(trace_len, 20, 0.01);
            st.record_all(traces.iter().map(|it| OgRank2CurRank(it.clone())));
        })
    });
}

fn par_benchmark(c: &mut Criterion) {
    let trace_len = 120;
    let traces = random_traces(trace_len, 500, 30, 10);
    c.bench_function("streaming transpositions benchmark", |b| {
        b.iter(|| {
            let st = streaming_transpositions::StreamingTranspositions::new(trace_len, 20, 0.01);
            st.par_record_all((0..10).into_par_iter().map(|start| {
                traces[start * 50..start * 50 + 50]
                    .iter()
                    .map(|it| OgRank2CurRank(it.clone()))
            }))
        })
    });
}

criterion_group!(benches, criterion_benchmark, par_benchmark);
criterion_main!(benches);
