# SIMD Allocation Cycle Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve conversion performance by reducing allocation, copy, and per-chunk overhead in the existing SIMD conversion paths.

**Architecture:** Keep the public API unchanged. `Samples::to_bytes()` allocates the exact output size once and writes each SIMD/scalar result directly into the output buffer. `pcm_bytes_to_samples()` allocates the exact sample count once and writes each SIMD/scalar result directly into initialized output slots.

**Tech Stack:** Rust nightly 1.98, portable SIMD, existing Criterion conversion benchmarks.

## Global Constraints

- Do not change public APIs.
- Preserve existing conversion semantics exactly.
- Keep portable SIMD rather than architecture-specific intrinsics.
- Verify with `cargo test`.
- Compare performance with `cargo bench --bench conversions`.

---

### Task 1: Optimize Conversion Writes

**Files:**
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `Samples::to_bytes(&self) -> Vec<u8>` and private `pcm_bytes_to_samples(bytes: &[u8]) -> Vec<f32>`.
- Produces: same function signatures and same conversion semantics.

- [ ] **Step 1: Capture baseline**

Run:

```bash
cargo bench --bench conversions
```

Expected baseline on this machine:

```text
conversions/pcm_bytes_to_samples/240000: approximately 51 us
conversions/to_bytes/240000: approximately 41 us
```

- [ ] **Step 2: Optimize `Samples::to_bytes()`**

Change `Samples::to_bytes()` to:

```rust
pub fn to_bytes(&self) -> Vec<u8> {
    let sample_count = self.0.len();
    let mut bytes = vec![0u8; sample_count * 2];

    let sample_chunks = self.0.chunks_exact(8);
    let remainder = sample_chunks.remainder();
    let mut byte_chunks = bytes.chunks_exact_mut(16);

    for (chunk, out) in sample_chunks.zip(byte_chunks.by_ref()) {
        let sample_vec = f32x8::from_slice(chunk);
        let clamped = sample_vec.simd_clamp(f32x8::splat(-1.0), f32x8::splat(1.0));
        let is_positive = clamped.simd_ge(f32x8::splat(0.0));
        let scale = is_positive.select(
            f32x8::splat(i16::MAX as f32),
            f32x8::splat(-(i16::MIN as f32)),
        );
        let ints: i16x8 = (clamped * scale).cast();
        let byte_array: [u8; 16] = unsafe { std::mem::transmute(ints) };
        out.copy_from_slice(&byte_array);
    }

    let mut offset = sample_count / 8 * 16;
    for &s in remainder {
        let clamped = s.clamp(-1.0, 1.0);
        let scale = if clamped >= 0.0 {
            i16::MAX as f32
        } else {
            -(i16::MIN as f32)
        };
        let scaled = (clamped * scale) as i16;
        bytes[offset..offset + 2].copy_from_slice(&scaled.to_le_bytes());
        offset += 2;
    }

    #[cfg(target_endian = "big")]
    {
        for chunk in bytes.chunks_exact_mut(2) {
            chunk.swap(0, 1);
        }
    }

    bytes
}
```

- [ ] **Step 3: Optimize `pcm_bytes_to_samples()`**

Change `pcm_bytes_to_samples()` to:

```rust
fn pcm_bytes_to_samples(bytes: &[u8]) -> Vec<f32> {
    let sample_count = bytes.len() / 2;
    let mut floats = vec![0.0f32; sample_count];

    let byte_chunks = bytes.chunks_exact(16);
    let remainder = byte_chunks.remainder();
    let mut float_chunks = floats.chunks_exact_mut(8);

    for (chunk, out) in byte_chunks.zip(float_chunks.by_ref()) {
        let byte_arr: [u8; 16] = chunk.try_into().unwrap();
        let ints: i16x8 = unsafe { std::mem::transmute(byte_arr) };

        #[cfg(target_endian = "big")]
        let ints = ints.swap_bytes();

        let sample_vec: f32x8 = ints.cast();
        let normalized = sample_vec / f32x8::splat(i16::MAX as f32);
        normalized.copy_to_slice(out);
    }

    let mut offset = sample_count / 8 * 8;
    for chunk in remainder.chunks_exact(2) {
        floats[offset] = i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / i16::MAX as f32;
        offset += 1;
    }

    floats
}
```

- [ ] **Step 4: Verify correctness**

Run:

```bash
cargo test
```

Expected: 8 unit tests and 1 doctest pass.

- [ ] **Step 5: Verify benchmark target**

Run:

```bash
cargo bench --bench conversions -- --test
```

Expected: all six Criterion test-mode benchmark cases report `Success`.

- [ ] **Step 6: Compare performance**

Run:

```bash
cargo bench --bench conversions
```

Expected: report before/after Criterion medians for the two 240000-sample benchmarks.
