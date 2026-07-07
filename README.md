# samples

[![Cargo Test](https://github.com/quietscroll/samples/actions/workflows/cargo-test.yml/badge.svg)](https://github.com/quietscroll/samples/actions/workflows/cargo-test.yml)

Normalized f32 audio samples newtype with conversions to and from L16 PCM.

## Overview

`Samples` wraps a `Vec<f32>` of normalized mono audio in the range `[-1.0, 1.0]`.
It is the floating-point counterpart to [`pcm::PCM`] and serves as the working
representation for DSP steps that need f32 arithmetic.

```rust
use samples::Samples;
use pcm::PCM;

// Build from a slice of floats.
let s = Samples::from(vec![0.0f32, 0.5, -0.5]);
assert_eq!(s.len(), 3);

// Round-trip to L16 mono PCM and back.
let pcm = PCM::from(&s);
let back = Samples::from(&pcm);
assert_eq!(back.len(), 3);
assert!((back[1] - 0.5).abs() < 1.0 / i16::MAX as f32 + 1e-6);
```

## Features

| feature | what it adds |
|---------|-------------|
| `serde` | derives `Serialize` / `Deserialize` on `Samples` as a JSON array of f32 |

```toml
[dependencies]
samples = { version = "0.1", features = ["serde"] }
```

## Conversions

| from | to | via |
|------|----|-----|
| `Vec<f32>` | `Samples` | `From<Vec<f32>>` |
| `&[f32]` | `Samples` | `From<&[f32]>` |
| `&pcm::PCM` | `Samples` | `From<&PCM>` — i16 LE bytes → normalized f32 |
| `&[u8]` | `Samples` | `TryFrom<&[u8]>` — rejects odd-length slices |
| `Samples` | `Vec<f32>` | `From<Samples>` / `into_inner()` |
| `&Samples` / `Samples` | `pcm::PCM` | `From<&Samples>` / `From<Samples>` — normalized f32 → i16 LE bytes |

## API highlights

| item | description |
|------|-------------|
| `Samples::new()` | empty buffer |
| `Samples::len()` | number of f32 samples |
| `Samples::is_empty()` | true when the buffer contains no samples |
| `Samples::trim_tail(n)` | keep only the trailing `n` samples; clone unchanged if shorter |
| `Samples::to_bytes()` | convert to L16 mono PCM bytes (clamps to `[-1.0, 1.0]`, scales to i16 LE) |
| `Deref<Target = [f32]>` | slice access, iteration, and indexing work directly on `Samples` |
| `Extend<f32>` | accumulate samples from an iterator |
| `IntoIterator` | consume as an iterator of `f32` |

## License

MIT
