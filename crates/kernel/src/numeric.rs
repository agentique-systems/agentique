//! Exact finite numeric literals. No binary floating point or serialization identity.
use num_bigint::{BigInt, Sign};
use std::{cmp::Ordering, fmt, str::FromStr};

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid finite numeric lexical value")]
pub struct InvalidNumericLexical;

/// Arbitrary precision signed integer, with canonical equality and numeric order.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Integer(BigInt);
impl FromStr for Integer {
    type Err = InvalidNumericLexical;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(InvalidNumericLexical);
        }
        BigInt::from_str(s)
            .map(Self)
            .map_err(|_| InvalidNumericLexical)
    }
}
impl From<i64> for Integer {
    fn from(value: i64) -> Self {
        Self(value.into())
    }
}
impl fmt::Display for Integer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A finite decimal literal denotes coefficient × 10^exponent exactly.
///
/// Coefficient and exponent are arbitrary precision. Trailing coefficient zeros
/// are removed and zero has one representation. This represents the finite
/// decimal/scientific literals used for normative Real-valued literal properties;
/// it does not claim to encode every mathematical real, infinity or NaN.
/// Arithmetic and language expression evaluation are separate responsibilities.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ExactDecimal {
    coefficient: BigInt,
    exponent: BigInt,
}
impl FromStr for ExactDecimal {
    type Err = InvalidNumericLexical;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (mantissa, exponent) = match s.find(['e', 'E']) {
            Some(i) => (&s[..i], s[i + 1..].parse::<Integer>()?.0),
            None => (s, BigInt::from(0)),
        };
        let negative = mantissa.starts_with('-');
        let unsigned = mantissa.strip_prefix(['+', '-']).unwrap_or(mantissa);
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        if whole.is_empty() && fraction.is_empty()
            || !whole
                .bytes()
                .chain(fraction.bytes())
                .all(|b| b.is_ascii_digit())
        {
            return Err(InvalidNumericLexical);
        }
        let digits = format!("{whole}{fraction}");
        let digits = digits.trim_start_matches('0');
        if digits.is_empty() {
            return Ok(Self {
                coefficient: 0.into(),
                exponent: 0.into(),
            });
        }
        let canonical = digits.trim_end_matches('0');
        let exponent =
            exponent - BigInt::from(fraction.len()) + BigInt::from(digits.len() - canonical.len());
        let mut coefficient = BigInt::from_str(canonical).map_err(|_| InvalidNumericLexical)?;
        if negative {
            coefficient = -coefficient;
        }
        Ok(Self {
            coefficient,
            exponent,
        })
    }
}
impl fmt::Display for ExactDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}e{}", self.coefficient, self.exponent)
    }
}
impl Ord for ExactDecimal {
    fn cmp(&self, other: &Self) -> Ordering {
        let sign_rank = |s| match s {
            Sign::Minus => 0,
            Sign::NoSign => 1,
            Sign::Plus => 2,
        };
        let sign = self.coefficient.sign();
        let order = sign_rank(sign).cmp(&sign_rank(other.coefficient.sign()));
        if order != Ordering::Equal || sign == Sign::NoSign {
            return order;
        }
        let a = self.coefficient.magnitude().to_str_radix(10);
        let b = other.coefficient.magnitude().to_str_radix(10);
        let magnitude = (&self.exponent + BigInt::from(a.len()))
            .cmp(&(&other.exponent + BigInt::from(b.len())));
        let order = magnitude.then_with(|| {
            let n = a.len().max(b.len());
            a.bytes()
                .chain(std::iter::repeat(b'0'))
                .take(n)
                .cmp(b.bytes().chain(std::iter::repeat(b'0')).take(n))
        });
        if sign == Sign::Minus {
            order.reverse()
        } else {
            order
        }
    }
}
impl PartialOrd for ExactDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
