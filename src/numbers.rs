//! Numbers: rounding, exact trigonometry and the fit tolerance (clause 5).

/// The fit tolerance of clause 5.7, in millimetres.
pub const FIT_TOL_MM: f64 = 1e-6;

/// Round `value` to `places` decimal places and return the result as a whole number of units of
/// `10^-places`, with its sign. The exact binary value is used, with the half-even rule (clause 5.5).
///
/// Returns `None` when the value is not finite or too large for exact work (about 1e15 times
/// `10^places`); the callers then use the value as it is.
fn round_scaled(value: f64, places: u32) -> Option<(bool, u128)> {
    if !value.is_finite() {
        return None;
    }
    let bits = value.to_bits();
    let negative = (bits >> 63) != 0;
    let exp_bits = ((bits >> 52) & 0x7ff) as i32;
    let frac = bits & ((1u64 << 52) - 1);
    let (mantissa, exp) = if exp_bits == 0 {
        (frac, -1074)
    } else {
        (frac | (1u64 << 52), exp_bits - 1075)
    };
    let scale = 10u128.checked_pow(places)?;
    let numerator = u128::from(mantissa).checked_mul(scale)?;
    if exp >= 0 {
        let shifted = numerator.checked_mul(1u128.checked_shl(exp as u32)?)?;
        return Some((negative, shifted));
    }
    let shift = (-exp) as u32;
    if shift >= 127 {
        // below 2^-74 of one unit: rounds to zero (the numerator is under 2^73).
        return Some((negative, 0));
    }
    let quotient = numerator >> shift;
    let remainder = numerator & ((1u128 << shift) - 1);
    let half = 1u128 << (shift - 1);
    let up = remainder > half || (remainder == half && quotient & 1 == 1);
    Some((negative, quotient + u128::from(up)))
}

/// Clause 5.5: round the exact value half-even; a negative zero is written as zero.
pub fn round_half_even(value: f64, places: u32) -> f64 {
    match round_scaled(value, places) {
        None => value,
        Some((_, 0)) => 0.0,
        Some((negative, q)) => {
            // Decimal text to the nearest binary number: the same value as Python's float(Decimal).
            let text = format!("{}{}e-{}", if negative { "-" } else { "" }, q, places);
            text.parse::<f64>().unwrap_or(value)
        }
    }
}

/// Whole units of `10^-places` after half-even rounding (clause 8.3.2 works in micrometres).
pub fn round_to_units(value: f64, places: u32) -> Option<i64> {
    let (negative, q) = round_scaled(value, places)?;
    let q = i64::try_from(q).ok()?;
    Some(if negative { -q } else { q })
}

/// The value as text with exactly `places` decimal places, rounded half-even on the exact binary
/// value (clause 12.1.1). A result of zero has no sign.
pub fn format_fixed(value: f64, places: u32) -> String {
    let Some((negative, q)) = round_scaled(value, places) else {
        return format!("{value}");
    };
    let mut digits = q.to_string();
    let places = places as usize;
    if digits.len() <= places {
        digits = format!("{}{}", "0".repeat(places + 1 - digits.len()), digits);
    }
    let split = digits.len() - places;
    let mut text = if places == 0 {
        digits.clone()
    } else {
        format!("{}.{}", &digits[..split], &digits[split..])
    };
    if negative && q != 0 {
        text.insert(0, '-');
    }
    text
}

/// A paper length, 3 places (clause 5.4).
pub fn mm(value: f64) -> f64 {
    round_half_even(value, 3)
}

/// An angle on the paper in `[0, 360)`, 3 places; a rounded 360 is written 0 (clause 16.4).
pub fn angle(value: f64) -> f64 {
    let result = round_half_even(value.rem_euclid(360.0), 3);
    if result >= 360.0 { 0.0 } else { result }
}

/// Cosine and sine of an angle in degrees, exact at multiples of 90 degrees (clause 5.6.1).
pub fn cos_sin(degrees: f64) -> (f64, f64) {
    let d = degrees.rem_euclid(360.0);
    if d == 0.0 {
        (1.0, 0.0)
    } else if d == 90.0 {
        (0.0, 1.0)
    } else if d == 180.0 {
        (-1.0, 0.0)
    } else if d == 270.0 {
        (0.0, -1.0)
    } else {
        let r = d.to_radians();
        (libm::cos(r), libm::sin(r))
    }
}

/// `a % b` with the sign of `b`, as the standard writes `mod`.
pub fn modulo(a: f64, b: f64) -> f64 {
    a.rem_euclid(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ties_go_to_the_even_digit() {
        assert_eq!(round_half_even(0.0625, 3), 0.062);
        assert_eq!(round_half_even(0.1875, 3), 0.188);
        assert_eq!(round_half_even(-0.0625, 3), -0.062);
        assert_eq!(round_half_even(2.5, 0), 2.0);
        assert_eq!(round_half_even(3.5, 0), 4.0);
    }

    #[test]
    fn the_exact_binary_value_decides_not_the_text() {
        assert_eq!(round_half_even(1.0005, 3), 1.0);
        assert_eq!(round_half_even(0.0005, 3), 0.001);
    }

    #[test]
    fn a_negative_zero_is_written_as_zero() {
        let r = round_half_even(-0.0004, 3);
        assert_eq!(r, 0.0);
        assert!(r.is_sign_positive());
        assert!(round_half_even(-0.0, 3).is_sign_positive());
    }

    #[test]
    fn extreme_values_do_not_panic() {
        assert_eq!(round_half_even(f64::MAX, 3), f64::MAX);
        assert_eq!(round_half_even(1e-300, 3), 0.0);
        assert_eq!(round_half_even(5e-324, 6), 0.0);
        assert!(round_half_even(f64::NAN, 3).is_nan());
        assert_eq!(round_half_even(1e22, 6), 1e22);
    }

    #[test]
    fn sine_and_cosine_are_exact_at_right_angles() {
        assert_eq!(cos_sin(90.0), (0.0, 1.0));
        assert_eq!(cos_sin(-90.0), (0.0, -1.0));
        assert_eq!(cos_sin(450.0), (0.0, 1.0));
        assert_eq!(cos_sin(180.0), (-1.0, 0.0));
        assert_eq!(cos_sin(0.0), (1.0, 0.0));
    }

    #[test]
    fn an_angle_stays_below_360() {
        assert_eq!(angle(359.9999), 0.0);
        assert_eq!(angle(-10.0), 350.0);
        assert_eq!(angle(405.0), 45.0);
    }
}
