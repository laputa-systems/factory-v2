//! Exact provider-binary64 accounting observations.
//!
//! The adapter writes the raw IEEE-754 bits.  This module never rounds through
//! a Rust `f64`: it compares nonnegative bit patterns directly and derives the
//! ceiling in integer arithmetic, so a positive sub-micro observation cannot
//! become a zero charge at the Rust boundary.

use thiserror::Error;

use crate::protocol::{ProviderCostObservationV1, UsageObservation, UsageTotals};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct UsdMicros(u128);
impl UsdMicros {
    pub const ZERO: Self = Self(0);
    pub const fn value(self) -> u128 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProviderCost {
    bits: u64,
}
impl ProviderCost {
    pub fn from_observation(value: &ProviderCostObservationV1) -> Result<Self, CostDecodeError> {
        let bits = u64::from_str_radix(value.binary64_big_endian_hex.as_str(), 16)
            .map_err(|_| CostDecodeError::InvalidHex)?;
        if bits >> 63 != 0 || (bits >> 52) & 0x7ff == 0x7ff {
            return Err(CostDecodeError::NonFiniteOrNegative);
        }
        Ok(Self { bits })
    }

    pub const fn ieee754_bits(self) -> u64 {
        self.bits
    }

    /// `ceil(binary64 * 1_000_000)` exactly.  Values too large for this
    /// boundary-owned `u128` are refused, forcing the daemon to contain rather
    /// than silently undercharge an impossible observation.
    pub fn ceil_to_micro_usd(self) -> Result<UsdMicros, CostDecodeError> {
        let exponent = ((self.bits >> 52) & 0x7ff) as i32;
        let fraction = self.bits & ((1_u64 << 52) - 1);
        if exponent == 0 && fraction == 0 {
            return Ok(UsdMicros::ZERO);
        }
        let (significand, binary_exponent) = if exponent == 0 {
            (u128::from(fraction), -1074_i32)
        } else {
            (u128::from((1_u64 << 52) | fraction), exponent - 1023 - 52)
        };
        let numerator = significand
            .checked_mul(1_000_000)
            .ok_or(CostDecodeError::Overflow)?;
        if binary_exponent >= 0 {
            let shift = u32::try_from(binary_exponent).map_err(|_| CostDecodeError::Overflow)?;
            return numerator
                .checked_shl(shift)
                .map(UsdMicros)
                .ok_or(CostDecodeError::Overflow);
        }
        let denominator_shift =
            u32::try_from(-binary_exponent).map_err(|_| CostDecodeError::Overflow)?;
        // A denominator above u128 is larger than this nonzero numerator, so
        // the exact ceiling is one micro-USD.
        if denominator_shift >= 128 {
            return Ok(UsdMicros(1));
        }
        let denominator = 1_u128 << denominator_shift;
        let quotient = numerator / denominator;
        let remainder = numerator % denominator;
        Ok(UsdMicros(quotient + u128::from(remainder != 0)))
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CostDecodeError {
    #[error("provider cost is not 16 hexadecimal binary64 bytes")]
    InvalidHex,
    #[error("provider cost is negative or nonfinite")]
    NonFiniteOrNegative,
    #[error("provider cost does not fit boundary micro-USD representation")]
    Overflow,
    #[error("usage totals are internally inconsistent")]
    InconsistentUsage,
    #[error("cumulative usage regressed")]
    RegressedUsage,
    #[error("usage is unavailable")]
    UnavailableUsage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub total_tokens: u64,
    pub provider_cost: ProviderCost,
    pub ceiling_micro_usd: UsdMicros,
}

impl NormalizedUsage {
    pub fn from_totals(totals: &UsageTotals) -> Result<Self, CostDecodeError> {
        let input = totals.input_tokens.value();
        let output = totals.output_tokens.value();
        let cache_read = totals.cache_read_tokens.value();
        let cache_write = totals.cache_write_tokens.value();
        let calculated_total = input
            .checked_add(output)
            .and_then(|value| value.checked_add(cache_read))
            .and_then(|value| value.checked_add(cache_write))
            .ok_or(CostDecodeError::InconsistentUsage)?;
        if calculated_total != totals.total_tokens.value() {
            return Err(CostDecodeError::InconsistentUsage);
        }
        let provider_cost = ProviderCost::from_observation(&totals.provider_cost)?;
        let ceiling_micro_usd = provider_cost.ceil_to_micro_usd()?;
        Ok(Self {
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: cache_read,
            cache_write_tokens: cache_write,
            total_tokens: calculated_total,
            provider_cost,
            ceiling_micro_usd,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UsageDelta {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub total_tokens: u64,
    pub micro_usd: UsdMicros,
    /// A duplicate cumulative snapshot is a no-op.  It is not another cost
    /// charge merely because the SDK repeated a meaningful lifecycle event.
    pub idempotent: bool,
}

#[derive(Clone, Debug, Default)]
pub struct UsageTracker {
    latest: Option<NormalizedUsage>,
}
impl UsageTracker {
    pub fn observe(
        &mut self,
        observation: &UsageObservation,
    ) -> Result<Option<UsageDelta>, CostDecodeError> {
        let UsageObservation::Known(totals) = observation else {
            return Err(CostDecodeError::UnavailableUsage);
        };
        let current = NormalizedUsage::from_totals(totals)?;
        let previous = self.latest.as_ref();
        if let Some(previous) = previous
            && (current.input_tokens < previous.input_tokens
                || current.output_tokens < previous.output_tokens
                || current.cache_read_tokens < previous.cache_read_tokens
                || current.cache_write_tokens < previous.cache_write_tokens
                || current.total_tokens < previous.total_tokens
                || current.provider_cost < previous.provider_cost
                || current.ceiling_micro_usd < previous.ceiling_micro_usd)
        {
            return Err(CostDecodeError::RegressedUsage);
        }
        let delta = UsageDelta {
            input_tokens: current.input_tokens - previous.map_or(0, |value| value.input_tokens),
            output_tokens: current.output_tokens - previous.map_or(0, |value| value.output_tokens),
            cache_read_tokens: current.cache_read_tokens
                - previous.map_or(0, |value| value.cache_read_tokens),
            cache_write_tokens: current.cache_write_tokens
                - previous.map_or(0, |value| value.cache_write_tokens),
            total_tokens: current.total_tokens - previous.map_or(0, |value| value.total_tokens),
            micro_usd: UsdMicros(
                current.ceiling_micro_usd.0 - previous.map_or(0, |value| value.ceiling_micro_usd.0),
            ),
            idempotent: previous == Some(&current),
        };
        self.latest = Some(current);
        Ok(Some(delta))
    }
    pub fn latest(&self) -> Option<&NormalizedUsage> {
        self.latest.as_ref()
    }
}

#[cfg(test)]
mod tests {
    // These are closed fixture constructors and assertion boundaries; panicking
    // keeps invalid test data local and legible without weakening production code.
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::protocol::{Binary64BigEndianHex, ProviderCostObservationV1, UsageObservation};

    fn cost(hex: &str) -> ProviderCostObservationV1 {
        ProviderCostObservationV1 {
            binary64_big_endian_hex: Binary64BigEndianHex::parse(hex).unwrap(),
        }
    }
    #[test]
    fn exact_binary64_rounding_never_undercounts_sub_micro_positive_cost() {
        // 0.0000001, the regression that a `toFixed(6)` representation loses.
        let cents = ProviderCost::from_observation(&cost("3e7ad7f29abcaf48")).unwrap();
        assert_eq!(cents.ceil_to_micro_usd().unwrap().value(), 1);
        assert_eq!(
            ProviderCost::from_observation(&cost("3eb0c6f7a0b5ed8d"))
                .unwrap()
                .ceil_to_micro_usd()
                .unwrap()
                .value(),
            1
        );
        // The next representable value above one micro-USD must round upward.
        assert_eq!(
            ProviderCost::from_observation(&cost("3eb0c6f7a0b5ed8e"))
                .unwrap()
                .ceil_to_micro_usd()
                .unwrap()
                .value(),
            2
        );
    }
    #[test]
    fn rejects_nonfinite_and_negative_binary64() {
        assert!(matches!(
            ProviderCost::from_observation(&cost("7ff0000000000000")),
            Err(CostDecodeError::NonFiniteOrNegative)
        ));
        assert!(matches!(
            ProviderCost::from_observation(&cost("8000000000000000")),
            Err(CostDecodeError::NonFiniteOrNegative)
        ));
    }
    #[test]
    fn usage_tracker_rejects_cost_regression() {
        fn totals(cost_hex: &str) -> UsageObservation {
            UsageObservation::Known(UsageTotals {
                input_tokens: crate::protocol::NonNegativeInteger::parse(1).unwrap(),
                output_tokens: crate::protocol::NonNegativeInteger::parse(1).unwrap(),
                cache_read_tokens: crate::protocol::NonNegativeInteger::parse(0).unwrap(),
                cache_write_tokens: crate::protocol::NonNegativeInteger::parse(0).unwrap(),
                total_tokens: crate::protocol::NonNegativeInteger::parse(2).unwrap(),
                provider_cost: cost(cost_hex),
            })
        }
        let mut tracker = UsageTracker::default();
        tracker.observe(&totals("3f947ae147ae147b")).unwrap();
        assert_eq!(
            tracker.observe(&totals("3f847ae147ae147b")),
            Err(CostDecodeError::RegressedUsage)
        );
    }
}
