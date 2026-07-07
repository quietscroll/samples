//! Normalized f32 audio samples newtype with conversions to and from L16 PCM.
//!
//! # Overview
//!
//! [`Samples`] wraps a `Vec<f32>` of normalized mono audio in the range
//! `[-1.0, 1.0]`. It is the floating-point counterpart to [`pcm::PCM`] and
//! serves as the working representation for DSP steps that need f32 arithmetic.
//!
//! ```
//! use samples::Samples;
//!
//! // Build from raw floats.
//! let s = Samples::from(vec![0.0f32, 0.5, -0.5]);
//! assert_eq!(s.len(), 3);
//!
//! // Round-trip to L16 PCM and back.
//! let pcm = s.to_pcm();
//! let back = Samples::from(&pcm);
//! assert_eq!(back.len(), 3);
//! assert!((back[1] - 0.5).abs() < 1.0 / i16::MAX as f32 + 1e-6);
//! ```
//!
//! # Features
//!
//! | feature | effect |
//! |---------|--------|
//! | `serde` | derives `Serialize` / `Deserialize` on [`Samples`] as a JSON array of f32 |

#![deny(missing_docs, unreachable_pub)]

use std::ops::Deref;

use pcm::PCM;

/// Errors that can arise from PCM operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The byte buffer has an odd length. L16 mono uses 2 bytes per sample,
    /// so an odd byte count cannot represent valid PCM data.
    #[error("PCM byte length must be even for i16 LE samples")]
    ByteLengthNotEven,
}

/// Normalized f32 audio samples ([-1.0, 1.0], mono, same sample rate as PCM).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Samples(Vec<f32>);

impl Samples {
    /// Create an empty sample buffer.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Number of samples.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True when the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consume the wrapper and return the inner `Vec<f32>`.
    pub fn into_inner(self) -> Vec<f32> {
        self.0
    }

    /// Return only the trailing `max_len` samples, discarding the head.
    ///
    /// If `self` is shorter than `max_len`, returns a clone unchanged.
    pub fn trim_tail(&self, max_len: usize) -> Samples {
        if self.0.len() > max_len {
            Samples(self.0[self.0.len() - max_len..].to_vec())
        } else {
            self.clone()
        }
    }

    /// Convert to L16 mono PCM (i16 little-endian bytes) by clamping each
    /// sample to [-1.0, 1.0] and scaling to the i16 range.
    pub fn to_pcm(&self) -> PCM {
        let bytes: Vec<u8> = self
            .0
            .iter()
            .flat_map(|&s| sample_to_i16(s).to_le_bytes())
            .collect();
        PCM::from(bytes)
    }
}

/// Convert a normalized f32 sample to i16, clamping to [-1.0, 1.0] and scaling.
pub fn sample_to_i16(sample: f32) -> i16 {
    let clamped = sample.clamp(-1.0, 1.0);
    let scaled = if clamped >= 0.0 {
        clamped * i16::MAX as f32
    } else {
        clamped * -(i16::MIN as f32)
    };

    scaled as i16
}

#[cfg(feature = "serde")]
impl Samples {
    /// Encode this Samples buffer as a base64 string (STANDARD alphabet).
    pub fn to_b64(&self) -> String {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let bytes: Vec<u8> = self.0.iter().flat_map(|&f| f.to_le_bytes()).collect();
        STANDARD.encode(&bytes)
    }

    /// Decode a base64 string (STANDARD alphabet) into a Samples buffer.
    ///
    /// Returns [`base64::DecodeError`] when the input is not valid base64.
    pub fn from_b64(s: &str) -> Result<Self, base64::DecodeError> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let bytes = STANDARD.decode(s)?;
        let floats = bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        Ok(Self(floats))
    }
}

/// Serde helpers for serialising [`Samples`] as a base64 string.
///
/// Use `#[serde(with = "pcm::b64")]` on a `Samples` field.
#[cfg(feature = "serde")]
pub mod b64 {
    use super::Samples;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    /// Serialize `Samples` as a base64 string.
    pub fn serialize<S: Serializer>(pcm: &Samples, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&pcm.to_b64())
    }

    /// Deserialize `Samples` from a base64 string.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Samples, D::Error> {
        let raw = String::deserialize(d)?;
        Samples::from_b64(&raw).map_err(D::Error::custom)
    }
}

/// Serde helpers for serialising `Option<`[`Samples`]`>` as a nullable base64 string.
///
/// Use `#[serde(with = "pcm::b64_option")]` on an `Option<Samples>` field.
#[cfg(feature = "serde")]
pub mod b64_option {
    use super::Samples;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    /// Serialize `Option<Samples>` as a base64 string or `null`.
    pub fn serialize<S: Serializer>(opt: &Option<Samples>, s: S) -> Result<S::Ok, S::Error> {
        match opt {
            Some(pcm) => s.serialize_str(&pcm.to_b64()),
            None => s.serialize_none(),
        }
    }

    /// Deserialize `Option<Samples>` from a base64 string or `null`.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Samples>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|raw| Samples::from_b64(&raw).map_err(D::Error::custom))
            .transpose()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Samples {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_b64())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Samples {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let raw = <String as serde::Deserialize>::deserialize(d)?;
        Samples::from_b64(&raw).map_err(D::Error::custom)
    }
}

impl Deref for Samples {
    type Target = [f32];

    fn deref(&self) -> &[f32] {
        &self.0
    }
}

impl From<Vec<f32>> for Samples {
    fn from(v: Vec<f32>) -> Self {
        Self(v)
    }
}

impl From<Samples> for Vec<f32> {
    fn from(s: Samples) -> Self {
        s.0
    }
}

impl From<&[f32]> for Samples {
    fn from(s: &[f32]) -> Self {
        Self(s.to_vec())
    }
}

impl Extend<f32> for Samples {
    fn extend<I: IntoIterator<Item = f32>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl IntoIterator for Samples {
    type Item = f32;
    type IntoIter = std::vec::IntoIter<f32>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Convert L16 mono PCM bytes to normalized f32 samples.
///
/// Each i16 LE pair is decoded and divided by 32768 to normalize to [-1.0, 1.0].
impl From<&PCM> for Samples {
    fn from(p: &PCM) -> Self {
        let floats = p
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / i16::MAX as f32)
            .collect();
        Self(floats)
    }
}

impl TryFrom<Vec<u8>> for Samples {
    type Error = Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(bytes.as_slice())
    }
}

/// Convert raw i16 LE bytes to [`Samples`], validating even length.
///
/// Returns an error string if the byte slice has an odd length (not valid L16).
impl TryFrom<&[u8]> for Samples {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() % 2 != 0 {
            return Err(Self::Error::ByteLengthNotEven);
        }
        let floats = bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / i16::MAX as f32)
            .collect();
        Ok(Self(floats))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_round_trip() {
        let one_sec = PCM::from(vec![0u8; 24000 * 2]);
        let samples = Samples::from(&one_sec);
        assert_eq!(samples.len(), 24000);
        assert!(samples.iter().all(|&s| s == 0.0));
        let back = samples.to_pcm();
        assert_eq!(back.len(), one_sec.len());
    }

    #[test]
    fn try_from_odd_bytes_fails() {
        assert!(Samples::try_from([0u8, 1, 2].as_slice()).is_err());
    }

    #[test]
    fn try_from_even_bytes_succeeds() {
        let samples = Samples::try_from([0x00u8, 0x80].as_slice()).unwrap();
        assert_eq!(samples.len(), 1);
        assert!(samples[0] < 0.0); // i16::MIN / 32768 is negative
    }

    #[test]
    fn extend_accumulates() {
        let mut s = Samples::new();
        s.extend([0.5f32, -0.5]);
        s.extend([0.25f32]);
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn deref_coerces_to_slice() {
        let s = Samples::from(vec![1.0f32, 2.0, 3.0]);
        let sum: f32 = s.iter().sum();
        assert_eq!(sum, 6.0);
    }

    #[test]
    fn into_iterator_yields_f32() {
        let s = Samples::from(vec![0.1f32, 0.2, 0.3]);
        let v: Vec<f32> = s.into_iter().collect();
        assert_eq!(v, vec![0.1f32, 0.2, 0.3]);
    }

    #[test]
    fn extend_with_another_samples() {
        let mut a = Samples::from(vec![0.1f32]);
        let b = Samples::from(vec![0.2f32, 0.3]);
        a.extend(b);
        assert_eq!(a.len(), 3);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn test_pcm_serde() {
        let s = Samples::from(vec![0.1f32, 0.2, 0.3]);
        let serialized = serde_json::to_string(&s).unwrap();
        let deserialized: Samples = serde_json::from_str(&serialized).unwrap();
        assert_eq!(s, deserialized);
    }
}
