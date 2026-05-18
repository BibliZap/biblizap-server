//! Defines a custom type for Lens.org specific IDs and associated parsing/validation logic.

use std::marker::PhantomData;

use arrayvec::ArrayString;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use thiserror::Error;

/// Represents errors that can occur when parsing or validating a Lens.org ID string.
#[derive(Error, Debug)]
pub enum LensIdError {
    /// The candidate string for a LensID is not exactly 19 characters long.
    #[error("LensID candidate string is not 19 characters long")]
    Not19Characters,
    /// The parsed integer value of the LensID is zero, which is considered invalid.
    #[error("LensID is zero")]
    ZeroLensID,
}

/// A custom type representing a validated Lens.org specific ID.
///
/// Stores both the original string representation and a parsed integer value
/// for efficient hashing and comparison.
///
/// Note: Equality comparison is based solely on the `int` field (first 18 digits).
/// The string representation is assumed to be correct for performance.
#[derive(Debug, Clone)]
pub struct LensId {
    /// The integer representation of the first 18 digits of the Lens ID string.
    int: u64,
    /// The original 19-character string representation of the Lens ID.
    string: ArrayString<19>,
}

impl Serialize for LensId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.string.as_str())
    }
}

impl<'de> Deserialize<'de> for LensId {
    /// Custom deserialization logic for `LensId`.
    ///
    /// Deserializes a string value into a validated `LensId` struct.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = LensIdVisitor(PhantomData);
        deserializer.deserialize_str(visitor)
    }
}

/// A visitor for deserializing a string into a `LensId`.
struct LensIdVisitor(PhantomData<fn() -> LensId>);
impl Visitor<'_> for LensIdVisitor {
    type Value = LensId;

    /// Indicates the expected format for deserialization.
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a 19-character Lens.org ID string")
    }

    /// Visits a string value and attempts to convert it into a `LensId`.
    fn visit_str<E>(self, lensid_str: &str) -> Result<LensId, E>
    where
        E: de::Error,
    {
        // Use the TryFrom implementation to validate and create the LensId
        LensId::try_from(lensid_str).map_err(|e| de::Error::custom(format!("invalid lensid: {e}")))
    }
}

impl TryFrom<&str> for LensId {
    type Error = LensIdError;
    /// Attempts to create a `LensId` from a string slice.
    ///
    /// Validates that the string is 19 characters long and that the parsed
    /// integer value is not zero.
    fn try_from(lensid_str: &str) -> Result<Self, Self::Error> {
        if lensid_str.len() != 19 {
            return Err(LensIdError::Not19Characters);
        }

        let lensid = Self::from_str_unchecked(lensid_str);

        match lensid.int {
            0 => Err(LensIdError::ZeroLensID),
            _ => Ok(lensid),
        }
    }
}

impl AsRef<str> for LensId {
    /// Returns the string slice representation of the Lens ID.
    fn as_ref(&self) -> &str {
        self.string.as_str()
    }
}

impl PartialEq for LensId {
    /// Compares two `LensId` instances based only on their integer representation.
    ///
    /// This assumes the string representation is correct and doesn't need to be compared.
    fn eq(&self, other: &Self) -> bool {
        self.int == other.int
    }
}

impl Eq for LensId {}

impl PartialOrd for LensId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LensId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.int.cmp(&other.int)
    }
}

impl std::hash::Hash for LensId {
    /// Implements hashing for `LensId` based on its integer representation.
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) {
        hasher.write_u64(self.int)
    }
}

/// Enables the use of `LensId` with `nohash_hasher::NoHashHasher`.
impl nohash_hasher::IsEnabled for LensId {}

impl LensId {
    /// Creates a `LensId` from a string slice without performing length or zero validation.
    ///
    /// This is an internal helper function and should be used with caution.
    /// The input string is expected to be exactly 19 characters long.
    fn from_str_unchecked(lensid_str: &str) -> Self {
        let lensid_int: u64 = lensid_str
            .chars()
            .take(18) // Take the first 18 characters
            .filter_map(|c| c.to_digit(10)) // Convert digits to u32
            .fold(0, |acc, digit| acc * 10 + (digit as u64)); // Build the u64

        LensId {
            string: ArrayString::from(lensid_str).unwrap(), // Safe because length is checked by caller
            int: lensid_int,
        }
    }

    /// Calculates the checksum character for a Lens ID.
    ///
    /// Uses a modulo-11 algorithm similar to ISBN checksums.
    /// The checksum is calculated from the first 14 digits.
    fn calculate_checksum(int_value: u64) -> char {
        // Extract digits directly from the number without string conversion
        // We need to process up to 14 digits
        let mut digits = [0u8; 14];
        let mut count = 0;

        // Extract digits from right to left
        let mut temp = int_value;
        while temp > 0 && count < 14 {
            digits[count] = (temp % 10) as u8;
            temp /= 10;
            count += 1;
        }

        // Calculate checksum (process from left to right, so reverse)
        let mut checksum_total = 0u64;
        for i in (0..14).rev() {
            let digit = if i < count { digits[i] } else { 0 };
            checksum_total = (checksum_total + digit as u64) * 2;
        }

        let remainder = checksum_total % 11;
        let checksum = (12 - remainder) % 11;

        // Map to character: 0-9 are digits, 10 is 'X'
        if checksum == 10 {
            'X'
        } else {
            char::from_digit(checksum as u32, 10).unwrap()
        }
    }

    /// Returns the integer representation (first 18 digits) of this Lens ID.
    pub fn as_u64(&self) -> u64 {
        self.int
    }
}

impl From<u64> for LensId {
    /// Creates a `LensId` from a u64 representing the first 14 digits.
    ///
    /// Automatically calculates the checksum (15th character) and constructs
    /// the full 19-character string representation with hyphens.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let id = LensId::from(20200401307330u64);
    /// assert_eq!(id.as_ref(), "020-200-401-307-33X");
    /// ```
    fn from(int_value: u64) -> Self {
        let checksum = Self::calculate_checksum(int_value);

        // Format directly into ArrayString without heap allocation
        let mut result = ArrayString::<19>::new();

        // Extract 14 digits (with leading zeros)
        let mut digits = [0u8; 14];
        let mut temp = int_value;
        for i in 0..14 {
            digits[13 - i] = (temp % 10) as u8;
            temp /= 10;
        }

        // Build string: XXX-XXX-XXX-XXX-XXC
        (0..3).for_each(|i| {
            result.push((b'0' + digits[i]) as char);
        });
        result.push('-');
        (3..6).for_each(|i| {
            result.push((b'0' + digits[i]) as char);
        });
        result.push('-');
        (6..9).for_each(|i| {
            result.push((b'0' + digits[i]) as char);
        });
        result.push('-');
        (9..12).for_each(|i| {
            result.push((b'0' + digits[i]) as char);
        });
        result.push('-');
        (12..14).for_each(|i| {
            result.push((b'0' + digits[i]) as char);
        });
        result.push(checksum);

        LensId {
            int: int_value,
            string: result,
        }
    }
}

impl fmt::Display for LensId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.string.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_u64_with_known_ids() {
        // Test with known Lens IDs to verify checksum algorithm
        // Note: These tests will help us verify the correct checksum algorithm

        // Parse existing IDs to get their int values
        let id1 = LensId::try_from("020-200-401-307-33X").unwrap();
        let int1 = id1.as_u64();
        println!("{int1}");

        // Recreate from u64 and verify it matches
        let recreated1 = LensId::from(int1);
        assert_eq!(
            recreated1.as_ref(),
            id1.as_ref(),
            "Recreated ID should match original for 020-200-401-307-33X"
        );

        let id2 = LensId::try_from("050-708-976-791-252").unwrap();
        let int2 = id2.as_u64();

        let recreated2 = LensId::from(int2);
        assert_eq!(
            recreated2.as_ref(),
            id2.as_ref(),
            "Recreated ID should match original for 050-708-976-791-252"
        );
    }

    #[test]
    fn test_equality_compares_only_u64() {
        // Two LensIds with the same int value should be equal
        let id1 = LensId::try_from("020-200-401-307-33X").unwrap();
        let int_value = id1.as_u64();
        let id2 = LensId::from(int_value);

        assert_eq!(id1, id2, "LensIds with same int should be equal");
        assert_eq!(id1.as_u64(), id2.as_u64());
    }

    #[test]
    fn test_hash_uses_only_u64() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let id1 = LensId::try_from("020-200-401-307-33X").unwrap();
        let int_value = id1.as_u64();
        let id2 = LensId::from(int_value);

        let mut hasher1 = DefaultHasher::new();
        id1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        id2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2, "LensIds with same int should have same hash");
    }

    #[test]
    fn test_ordering_by_u64() {
        let id1 = LensId::try_from("020-200-401-307-33X").unwrap();
        let id2 = LensId::try_from("050-708-976-791-252").unwrap();

        assert!(id1 < id2, "020... should be less than 050...");
        assert!(id2 > id1, "050... should be greater than 020...");
    }

    #[test]
    fn test_as_u64() {
        let id = LensId::try_from("020-200-401-307-33X").unwrap();
        let int_value = id.as_u64();

        // The int value should be the first 14 digits (leading zero dropped by u64)
        assert_eq!(int_value, 2020040130733u64);
    }

    #[test]
    fn test_display_trait() {
        let id = LensId::try_from("020-200-401-307-33X").unwrap();
        assert_eq!(format!("{id}"), "020-200-401-307-33X");
    }
}
