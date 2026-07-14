use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use samples::Samples;

const SAMPLE_RATE: usize = 24_000;

fn pcm_bytes(sample_count: usize) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(sample_count * 2);

    for index in 0..sample_count {
        let value = match index % 8 {
            0 => i16::MIN,
            1 => -24_576,
            2 => -12_288,
            3 => -1,
            4 => 0,
            5 => 1,
            6 => 12_288,
            _ => i16::MAX,
        };
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    bytes
}

fn samples(sample_count: usize) -> Samples {
    let values: Vec<f32> = (0..sample_count)
        .map(|index| match index % 8 {
            0 => -1.25_f32,
            1 => -1.0,
            2 => -0.5,
            3 => -0.000_03,
            4 => 0.0,
            5 => 0.000_03,
            6 => 0.5,
            _ => 1.25,
        })
        .collect();

    Samples::from(values)
}

fn bench_conversions(c: &mut Criterion) {
    let mut group = c.benchmark_group("conversions");

    for sample_count in [SAMPLE_RATE / 10, SAMPLE_RATE, SAMPLE_RATE * 10] {
        let pcm_bytes = pcm_bytes(sample_count);
        group.bench_with_input(
            BenchmarkId::new("pcm_bytes_to_samples", sample_count),
            &pcm_bytes,
            |b, bytes| {
                b.iter(|| {
                    Samples::try_from(black_box(bytes.as_slice()))
                        .expect("benchmark PCM bytes are valid")
                });
            },
        );
        group.bench_with_input(
            BenchmarkId::new("pcm_bytes_to_samples_reuse", sample_count),
            &pcm_bytes,
            |b, bytes| {
                let mut samples = Vec::with_capacity(bytes.len() / 2);
                b.iter(|| {
                    Samples::try_from_bytes_into(black_box(bytes.as_slice()), &mut samples)
                        .expect("benchmark PCM bytes are valid");
                    black_box(samples.as_slice());
                });
            },
        );

        let samples = samples(sample_count);
        group.bench_with_input(
            BenchmarkId::new("to_bytes", sample_count),
            &samples,
            |b, samples| b.iter(|| black_box(samples).to_bytes()),
        );
        group.bench_with_input(
            BenchmarkId::new("to_bytes_reuse", sample_count),
            &samples,
            |b, samples| {
                let mut bytes = Vec::with_capacity(samples.len() * 2);
                b.iter(|| {
                    black_box(samples).write_bytes_to(&mut bytes);
                    black_box(bytes.as_slice());
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_conversions);
criterion_main!(benches);
