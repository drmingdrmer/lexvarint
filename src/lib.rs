//! Lexicographically ordered encoding for unsigned integers.
//!
//! Each encoding contains a three-digit segment count followed by that many
//! underscore-prefixed, three-digit, big-endian base-1000 segments. Bytewise
//! lexicographic order of canonical encodings matches the numeric order of
//! their values.
//!
//! ```
//! assert_eq!(lexvarint::encode(1_000), "002_001_000");
//! assert_eq!(lexvarint::decode("002_001_000"), Ok(1_000));
//! ```

#![forbid(unsafe_code)]

use thiserror::Error;

const SEGMENT_RADIX: u128 = 1_000;
const SEGMENT_WIDTH: usize = 3;
const SEPARATOR_WIDTH: usize = 1;
const ENCODED_SEGMENT_WIDTH: usize = SEPARATOR_WIDTH + SEGMENT_WIDTH;
const U128_SEGMENTS: usize = 13;

/// The encoded length of zero.
pub const MIN_ENCODED_LEN: usize = SEGMENT_WIDTH;

/// The maximum encoded length of a [`u128`].
pub const MAX_ENCODED_LEN: usize = SEGMENT_WIDTH + U128_SEGMENTS * ENCODED_SEGMENT_WIDTH;

/// An error returned when an encoded value is invalid.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum DecodeError {
    /// A segment does not contain exactly three ASCII digits.
    #[error("encoded segment {index} must contain exactly three ASCII digits")]
    InvalidSegment { index: usize },

    /// The declared segment count differs from the payload length.
    #[error("encoded segment count is {declared}, but the payload contains {actual} segments")]
    SegmentCountMismatch { declared: usize, actual: usize },

    /// The first value segment is zero, making the encoding noncanonical.
    #[error("the first value segment is 000")]
    LeadingZeroSegment,

    /// The encoded value exceeds [`u128::MAX`].
    #[error("encoded value exceeds u128::MAX")]
    Overflow,
}

/// Encodes an unsigned integer into its canonical ASCII representation.
pub fn encode(value: u128) -> String {
    let (count, segments) = split_segments(value);

    let length = SEGMENT_WIDTH + count * ENCODED_SEGMENT_WIDTH;

    let mut encoded = String::with_capacity(length);

    push_segment(&mut encoded, count as u16);

    for index in (0..count).rev() {
        encoded.push('_');
        push_segment(&mut encoded, segments[index]);
    }
    encoded
}

/// Decodes a canonical ASCII representation into an unsigned integer.
pub fn decode(encoded: &str) -> Result<u128, DecodeError> {
    let mut segments = encoded.split('_');
    let header = segments.next().expect("split always yields a header");
    let declared = parse_segment(header, 0)? as usize;
    let actual = segments.clone().count();

    if declared != actual {
        return Err(DecodeError::SegmentCountMismatch { declared, actual });
    }

    if segments.clone().next() == Some("000") {
        return Err(DecodeError::LeadingZeroSegment);
    }

    let mut value = 0_u128;
    for (index, segment) in segments.enumerate() {
        let segment = parse_segment(segment, index + 1)?;
        let product = value.checked_mul(SEGMENT_RADIX).ok_or(DecodeError::Overflow)?;
        value = product.checked_add(segment).ok_or(DecodeError::Overflow)?;
    }
    Ok(value)
}

fn split_segments(mut value: u128) -> (usize, [u16; U128_SEGMENTS]) {
    let mut segments = [0_u16; U128_SEGMENTS];
    let mut count = 0;

    while value > 0 {
        let segment = (value % SEGMENT_RADIX) as u16;
        value /= SEGMENT_RADIX;
        segments[count] = segment;
        count += 1;
    }

    (count, segments)
}

fn push_segment(encoded: &mut String, segment: u16) {
    debug_assert!(segment < SEGMENT_RADIX as u16);
    let hundreds = segment / 100;
    let tens = segment / 10 % 10;
    let ones = segment % 10;
    encoded.push(char::from(b'0' + hundreds as u8));
    encoded.push(char::from(b'0' + tens as u8));
    encoded.push(char::from(b'0' + ones as u8));
}

fn parse_segment(segment: &str, index: usize) -> Result<u128, DecodeError> {
    let bytes = segment.as_bytes();
    let has_width = bytes.len() == SEGMENT_WIDTH;
    let has_only_digits = bytes.iter().all(|byte| byte.is_ascii_digit());
    if !has_width || !has_only_digits {
        return Err(DecodeError::InvalidSegment { index });
    }

    let hundreds = u128::from(bytes[0] - b'0') * 100;
    let tens = u128::from(bytes[1] - b'0') * 10;
    let ones = u128::from(bytes[2] - b'0');
    Ok(hundreds + tens + ones)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_examples_and_boundaries() {
        let cases = [
            (0, "000"),
            (1, "001_001"),
            (2, "001_002"),
            (999, "001_999"),
            (1_000, "002_001_000"),
            (1_003, "002_001_003"),
            (999_999, "002_999_999"),
            (1_000_000, "003_001_000_000"),
            (
                u128::MAX,
                "013_340_282_366_920_938_463_463_374_607_431_768_211_455",
            ),
        ];

        for (value, expected) in cases {
            let actual = encode(value);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn round_trips_representative_values() {
        let values = representative_values();
        for expected in values {
            let encoded = encode(expected);
            let actual = decode(&encoded);
            assert_eq!(actual, Ok(expected));
        }
    }

    #[test]
    fn encoded_order_matches_numeric_order() {
        let values = representative_values();
        let actual: Vec<_> = values.into_iter().map(encode).collect();
        let mut expected = actual.clone();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }

    #[test]
    fn rejects_invalid_encodings() {
        let cases = [
            ("", DecodeError::InvalidSegment { index: 0 }),
            ("00", DecodeError::InvalidSegment { index: 0 }),
            ("0000", DecodeError::InvalidSegment { index: 0 }),
            ("00a", DecodeError::InvalidSegment { index: 0 }),
            ("001001", DecodeError::InvalidSegment { index: 0 }),
            ("001-001", DecodeError::InvalidSegment { index: 0 }),
            ("001_00a", DecodeError::InvalidSegment { index: 1 }),
            (
                "001",
                DecodeError::SegmentCountMismatch {
                    declared: 1,
                    actual: 0,
                },
            ),
            (
                "000_000",
                DecodeError::SegmentCountMismatch {
                    declared: 0,
                    actual: 1,
                },
            ),
            ("001_000", DecodeError::LeadingZeroSegment),
            ("002_000_001", DecodeError::LeadingZeroSegment),
            (
                "013_340_282_366_920_938_463_463_374_607_431_768_211_456",
                DecodeError::Overflow,
            ),
        ];

        for (encoded, expected) in cases {
            let actual = decode(encoded);
            assert_eq!(actual, Err(expected));
        }
    }

    fn representative_values() -> Vec<u128> {
        let mut values: Vec<_> = (0..=10_000).collect();
        let boundaries = [
            999,
            1_000,
            999_999,
            1_000_000,
            u64::MAX as u128,
            u128::MAX - 1,
            u128::MAX,
        ];
        values.extend(boundaries);

        let mut state = 0x4d59_5df4_d0f3_3173_u128;
        for _ in 0..10_000 {
            let multiplied = state.wrapping_mul(0xda94_2042_e4dd_58b5);
            state = multiplied.wrapping_add(1);
            values.push(state);
        }
        values.sort_unstable();
        values.dedup();
        values
    }
}
