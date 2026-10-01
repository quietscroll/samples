# Criterion Conversion Benchmarks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Criterion benchmarks for PCM byte-to-sample conversion and sample-to-byte conversion.

**Architecture:** Benchmark the public conversion paths so the crate API does not grow only for benchmarking. `Samples::try_from(&[u8])` exercises the private `pcm_bytes_to_samples` implementation, and `Samples::to_bytes()` exercises the outbound conversion path.

**Tech Stack:** Rust nightly 1.98, Criterion, existing `samples` and `pcm` APIs.

## Global Constraints

- Do not expose `pcm_bytes_to_samples` publicly just for benchmarks.
- Keep benchmark inputs deterministic.
- Use `std::hint::black_box` through Criterion to prevent optimizer elimination.
- Verify the benchmark target compiles and runs in test mode.

---

### Task 1: Conversion Benchmarks

**Files:**
- Modify: `Cargo.toml`
- Create: `benches/conversions.rs`

**Interfaces:**
- Consumes: `samples::Samples`, `Samples::try_from(&[u8]) -> Result<Samples, samples::Error>`, and `Samples::to_bytes() -> Vec<u8>`.
- Produces: Criterion benchmark functions named `pcm_bytes_to_samples` and `to_bytes`, grouped under `conversions`.

- [ ] **Step 1: Add Criterion configuration**

Edit `Cargo.toml` to include:

```toml
[dev-dependencies]
criterion = { version = "0.7", default-features = false }
serde_json = { version = "1" }

[[bench]]
name = "conversions"
harness = false
```

- [ ] **Step 2: Create benchmark target**

Create `benches/conversions.rs` with deterministic PCM and sample buffers:

```rust
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
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
    let values = (0..sample_count)
        .map(|index| match index % 8 {
            0 => -1.25,
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

        let samples = samples(sample_count);
        group.bench_with_input(
            BenchmarkId::new("to_bytes", sample_count),
            &samples,
            |b, samples| b.iter(|| black_box(samples).to_bytes()),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_conversions);
criterion_main!(benches);
```

- [ ] **Step 3: Verify benchmark target**

Run:

```bash
cargo bench --bench conversions -- --test
```

Expected: command exits with status 0.

- [ ] **Step 4: Verify tests still pass**

Run:

```bash
cargo test
```

Expected: command exits with status 0.
