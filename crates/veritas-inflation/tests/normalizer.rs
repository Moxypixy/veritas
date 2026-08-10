use veritas_inflation::{REQUIRED_SERIES_KEYS, parse_oecd_csv, parse_regions, region_is_enabled};

const RECORDED_OECD_RESPONSE: &str = include_str!("fixtures/oecd-prices.csv");

#[test]
fn normalizes_recorded_oecd_csv_to_monthly_price_points() {
    let points = parse_oecd_csv(RECORDED_OECD_RESPONSE).unwrap();

    assert_eq!(
        points,
        vec![("2026-01".into(), 3.0), ("2026-02".into(), 2.8),]
    );
}

#[test]
fn keeps_only_regions_with_all_required_plain_language_series() {
    let regions = parse_regions(include_str!("../regions.json")).unwrap();
    let euro_area = regions
        .iter()
        .find(|region| region.id == "euro-area")
        .unwrap();

    assert!(region_is_enabled(euro_area));
    assert_eq!(REQUIRED_SERIES_KEYS.len(), 4);

    let incomplete = r#"{
        "source": {
            "name": "OECD",
            "base_url": "https://example.invalid",
            "methodology_url": "https://example.invalid/methodology"
        },
        "regions": [{
            "id": "missing-energy",
            "source_area": "XXX",
            "measure": "CPI",
            "series": [
                {"key": "prices_overall", "label": "Overall", "series_key": "a"},
                {"key": "prices_food", "label": "Food", "series_key": "b"},
                {"key": "prices_housing", "label": "Housing", "series_key": "c"}
            ]
        }]
    }"#;
    let incomplete_region = &parse_regions(incomplete).unwrap()[0];

    assert!(!region_is_enabled(incomplete_region));
}
