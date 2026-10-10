//! Printed units and exact conversion to a base unit.
//!
//! Records keep the quantity exactly as printed on the pack (`qty_milli` in
//! thousandths of `unit`). Comparison across pack sizes and countries happens
//! on a derived price per base unit (litre, kilogram or item). Every factor
//! below is exact, expressed in pico-base units so integer math never rounds
//! the conversion itself.
//!
//! A bare "pint" or "gallon" is never accepted: a UK pint is about 20% larger
//! than a US pint.

use serde::{Deserialize, Serialize};

/// What a unit measures; prices only compare within one dimension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    /// Base unit: litre.
    Volume,
    /// Base unit: kilogram.
    Mass,
    /// Base unit: one item.
    Count,
}

/// A unit as printed on a pack or shelf label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Unit {
    #[serde(rename = "l")]
    Litre,
    #[serde(rename = "ml")]
    Millilitre,
    #[serde(rename = "us_gal")]
    UsGallon,
    #[serde(rename = "us_pt")]
    UsPint,
    #[serde(rename = "imp_gal")]
    ImperialGallon,
    #[serde(rename = "imp_pt")]
    ImperialPint,
    #[serde(rename = "kg")]
    Kilogram,
    #[serde(rename = "g")]
    Gram,
    #[serde(rename = "lb")]
    Pound,
    #[serde(rename = "oz")]
    Ounce,
    #[serde(rename = "each")]
    Each,
}

impl Unit {
    pub const ALL: [Unit; 11] = [
        Unit::Litre,
        Unit::Millilitre,
        Unit::UsGallon,
        Unit::UsPint,
        Unit::ImperialGallon,
        Unit::ImperialPint,
        Unit::Kilogram,
        Unit::Gram,
        Unit::Pound,
        Unit::Ounce,
        Unit::Each,
    ];

    /// Stable code used in JSON and in the canonical record encoding.
    pub fn code(self) -> &'static str {
        match self {
            Unit::Litre => "l",
            Unit::Millilitre => "ml",
            Unit::UsGallon => "us_gal",
            Unit::UsPint => "us_pt",
            Unit::ImperialGallon => "imp_gal",
            Unit::ImperialPint => "imp_pt",
            Unit::Kilogram => "kg",
            Unit::Gram => "g",
            Unit::Pound => "lb",
            Unit::Ounce => "oz",
            Unit::Each => "each",
        }
    }

    /// Parses a unit code. Ambiguous codes such as `pt` or `gal` are rejected.
    pub fn from_code(code: &str) -> Option<Unit> {
        Unit::ALL.into_iter().find(|unit| unit.code() == code)
    }

    pub fn dimension(self) -> Dimension {
        match self {
            Unit::Litre
            | Unit::Millilitre
            | Unit::UsGallon
            | Unit::UsPint
            | Unit::ImperialGallon
            | Unit::ImperialPint => Dimension::Volume,
            Unit::Kilogram | Unit::Gram | Unit::Pound | Unit::Ounce => Dimension::Mass,
            Unit::Each => Dimension::Count,
        }
    }

    /// Size of one unit in pico-base units (10^-12 litre, kilogram or item).
    pub fn pico_base(self) -> u128 {
        match self {
            Unit::Litre => 1_000_000_000_000,
            Unit::Millilitre => 1_000_000_000,
            // 1 US gallon = 3.785411784 L (exact, 231 cubic inches).
            Unit::UsGallon => 3_785_411_784_000,
            // 1 US pint = 0.473176473 L (exact, 1/8 US gallon).
            Unit::UsPint => 473_176_473_000,
            // 1 imperial gallon = 4.54609 L (exact by definition).
            Unit::ImperialGallon => 4_546_090_000_000,
            // 1 imperial pint = 0.56826125 L (exact, 1/8 imperial gallon).
            Unit::ImperialPint => 568_261_250_000,
            Unit::Kilogram => 1_000_000_000_000,
            Unit::Gram => 1_000_000_000,
            // 1 lb = 0.45359237 kg (exact, international pound).
            Unit::Pound => 453_592_370_000,
            // 1 oz = 0.028349523125 kg (exact, 1/16 lb).
            Unit::Ounce => 28_349_523_125,
            Unit::Each => 1_000_000_000_000,
        }
    }
}

/// Price per base unit (litre, kilogram or item) in thousandths of the minor
/// currency unit, rounded half up. `None` when the quantity is zero.
///
/// Example: 1 US gallon at 431 cents is 113 858 milli-cents per litre
/// (113.858 cents/L).
pub fn price_per_base_milli(amount_minor: u64, qty_milli: u64, unit: Unit) -> Option<u128> {
    // amount / (qty_milli / 1000 * pico / 1e12) * 1000
    //   = amount * 1e3 * 1e3 * 1e12 / (qty_milli * pico)
    let numerator = u128::from(amount_minor) * 1_000_000_000_000_000_000;
    let denominator = u128::from(qty_milli) * unit.pico_base();
    if denominator == 0 {
        return None;
    }
    Some((numerator + denominator / 2) / denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip_and_reject_ambiguous_names() {
        for unit in Unit::ALL {
            assert_eq!(Unit::from_code(unit.code()), Some(unit));
        }
        assert_eq!(Unit::from_code("pt"), None);
        assert_eq!(Unit::from_code("gal"), None);
        assert_eq!(Unit::from_code("pint"), None);
    }

    #[test]
    fn uk_pint_is_about_twenty_percent_larger_than_us_pint() {
        let ratio = Unit::ImperialPint.pico_base() as f64 / Unit::UsPint.pico_base() as f64;
        assert!((ratio - 1.2009).abs() < 0.0001, "ratio was {ratio}");
    }

    #[test]
    fn eight_pints_make_a_gallon_on_both_systems() {
        assert_eq!(Unit::UsPint.pico_base() * 8, Unit::UsGallon.pico_base());
        assert_eq!(
            Unit::ImperialPint.pico_base() * 8,
            Unit::ImperialGallon.pico_base()
        );
        assert_eq!(Unit::Ounce.pico_base() * 16, Unit::Pound.pico_base());
    }

    #[test]
    fn us_gallon_price_converts_to_per_litre() {
        // 4.31 USD per US gallon -> 113.858 cents per litre.
        assert_eq!(
            price_per_base_milli(431, 1_000, Unit::UsGallon),
            Some(113_858)
        );
    }

    #[test]
    fn danish_litre_and_half_litre_compare_directly() {
        // 12.95 DKK for 1 L and 7.50 DKK for 0.5 L.
        assert_eq!(
            price_per_base_milli(1_295, 1_000, Unit::Litre),
            Some(1_295_000)
        );
        assert_eq!(price_per_base_milli(750, 500, Unit::Litre), Some(1_500_000));
        assert_eq!(
            price_per_base_milli(750, 500_000, Unit::Millilitre),
            Some(1_500_000)
        );
    }

    #[test]
    fn zero_quantity_has_no_unit_price() {
        assert_eq!(price_per_base_milli(100, 0, Unit::Litre), None);
    }
}
