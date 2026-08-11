use std::time::Duration;

use veritas_inflation::{
    ConfiguredSeries, HttpClient, Region, ReqwestHttpClient, SourceConfig, parse_config,
    parse_oecd_csv,
};

const REQUEST_DELAY: Duration = Duration::from_secs(1);
const MAX_TRANSIENT_ATTEMPTS: usize = 3;

const CANDIDATES: &[Candidate] = &[
    Candidate::hicp("euro-area", "Euro area", "EA20"),
    Candidate::cpi("australia", "Australia", "AUS"),
    Candidate::cpi("austria", "Austria", "AUT"),
    Candidate::cpi("belgium", "Belgium", "BEL"),
    Candidate::cpi("canada", "Canada", "CAN"),
    Candidate::cpi("switzerland", "Switzerland", "CHE"),
    Candidate::cpi("chile", "Chile", "CHL"),
    Candidate::cpi("china", "China", "CHN"),
    Candidate::cpi("colombia", "Colombia", "COL"),
    Candidate::cpi("costa-rica", "Costa Rica", "CRI"),
    Candidate::cpi("czechia", "Czechia", "CZE"),
    Candidate::cpi("germany", "Germany", "DEU"),
    Candidate::cpi("denmark", "Denmark", "DNK"),
    Candidate::cpi("spain", "Spain", "ESP"),
    Candidate::cpi("estonia", "Estonia", "EST"),
    Candidate::cpi("finland", "Finland", "FIN"),
    Candidate::cpi("france", "France", "FRA"),
    Candidate::cpi("united-kingdom", "United Kingdom", "GBR"),
    Candidate::cpi("greece", "Greece", "GRC"),
    Candidate::cpi("hungary", "Hungary", "HUN"),
    Candidate::cpi("ireland", "Ireland", "IRL"),
    Candidate::cpi("iceland", "Iceland", "ISL"),
    Candidate::cpi("israel", "Israel", "ISR"),
    Candidate::cpi("italy", "Italy", "ITA"),
    Candidate::cpi("japan", "Japan", "JPN"),
    Candidate::cpi("south-korea", "South Korea", "KOR"),
    Candidate::cpi("lithuania", "Lithuania", "LTU"),
    Candidate::cpi("luxembourg", "Luxembourg", "LUX"),
    Candidate::cpi("latvia", "Latvia", "LVA"),
    Candidate::cpi("mexico", "Mexico", "MEX"),
    Candidate::cpi("netherlands", "Netherlands", "NLD"),
    Candidate::cpi("norway", "Norway", "NOR"),
    Candidate::cpi("new-zealand", "New Zealand", "NZL"),
    Candidate::cpi("poland", "Poland", "POL"),
    Candidate::cpi("portugal", "Portugal", "PRT"),
    Candidate::cpi("slovakia", "Slovakia", "SVK"),
    Candidate::cpi("slovenia", "Slovenia", "SVN"),
    Candidate::cpi("sweden", "Sweden", "SWE"),
    Candidate::cpi("turkey", "Turkey", "TUR"),
    Candidate::cpi("united-states", "United States", "USA"),
    Candidate::cpi("south-africa", "South Africa", "ZAF"),
];

struct Candidate {
    id: &'static str,
    label: &'static str,
    source_area: &'static str,
    measure: &'static str,
}

impl Candidate {
    const fn cpi(id: &'static str, label: &'static str, source_area: &'static str) -> Self {
        Self {
            id,
            label,
            source_area,
            measure: "CPI",
        }
    }

    const fn hicp(id: &'static str, label: &'static str, source_area: &'static str) -> Self {
        Self {
            id,
            label,
            source_area,
            measure: "HICP",
        }
    }

    fn to_region(&self) -> Region {
        let series_prefix = if self.measure == "HICP" {
            format!("{}.M.HICP.CPI.PA", self.source_area)
        } else {
            format!("{}.M.N.CPI.PA", self.source_area)
        };

        Region {
            id: self.id.to_owned(),
            label: self.label.to_owned(),
            source_area: self.source_area.to_owned(),
            measure: self.measure.to_owned(),
            series: [
                ("prices_overall", "How fast prices are rising overall", "_T"),
                ("prices_food", "Food prices", "CP01"),
                ("prices_housing", "Housing costs", "CP04"),
                ("prices_energy", "Energy / fuel prices", "CP045"),
            ]
            .into_iter()
            .map(|(key, label, coicop)| ConfiguredSeries {
                key: key.to_owned(),
                label: label.to_owned(),
                series_key: format!("{series_prefix}.{coicop}.N.GY"),
            })
            .collect(),
        }
    }
}

#[tokio::main]
async fn main() {
    let config = parse_config(include_str!("../../regions.json"))
        .expect("the checked-in regions file must contain a valid source configuration");
    let client = ReqwestHttpClient::default();
    // Preserve regions already known to be good. A 429 only means that this
    // verification run was rate-limited, never that a published series vanished.
    let mut regions = config.regions.clone();

    for candidate in CANDIDATES {
        let region = candidate.to_region();
        match verify_region(&client, &config.source, &region).await {
            Ok(()) => {
                println!("PASS {}", region.id);
                if !regions.iter().any(|existing| existing.id == region.id) {
                    regions.push(region);
                }
            }
            Err(error) if is_inconclusive_error(&error) => {
                println!("INCONCLUSIVE {}: {error}", region.id);
            }
            Err(error) => println!("FAIL {}: {error}", region.id),
        }
    }

    println!("\nVerified regions.json:");
    println!("{}", regions_json(&config.source, &regions));
}

async fn verify_region(
    client: &ReqwestHttpClient,
    source: &SourceConfig,
    region: &Region,
) -> Result<(), String> {
    for series in &region.series {
        let url = format!(
            "{}/{}?startPeriod=2020-01",
            source.base_url.trim_end_matches('/'),
            series.series_key
        );
        let body = get_with_transient_retries(client, &url).await?;
        let points = parse_oecd_csv(&body).map_err(|error| error.to_string())?;
        if points.is_empty() {
            return Err(format!("{} returned no YYYY-MM points", series.key));
        }
        tokio::time::sleep(REQUEST_DELAY).await;
    }
    Ok(())
}

async fn get_with_transient_retries(
    client: &ReqwestHttpClient,
    url: &str,
) -> Result<String, String> {
    let mut attempt = 0;

    loop {
        match client.get(url).await {
            Ok(body) => return Ok(body),
            Err(error) if is_inconclusive_error(&error) && attempt + 1 < MAX_TRANSIENT_ATTEMPTS => {
                attempt += 1;
                let backoff = REQUEST_DELAY * attempt as u32;
                eprintln!(
                    "Transient OECD error; retrying request {attempt}/{MAX_TRANSIENT_ATTEMPTS} after {}s: {error}",
                    backoff.as_secs()
                );
                tokio::time::sleep(backoff).await;
            }
            Err(error) => return Err(error),
        }
    }
}

fn is_inconclusive_error(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("429")
        || error.contains("error sending request")
        || error.contains("request or response body error")
        || error.contains("operation timed out")
        || error.contains("connection error")
        || error.contains("dns error")
}

fn regions_json(source: &SourceConfig, regions: &[Region]) -> String {
    let regions: Vec<_> = regions
        .iter()
        .map(|region| {
            serde_json::json!({
                "id": region.id,
                "label": region.label,
                "source_area": region.source_area,
                "measure": region.measure,
                "series": region.series.iter().map(|series| serde_json::json!({
                    "key": series.key,
                    "label": series.label,
                    "series_key": series.series_key,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "source": {
            "name": source.name,
            "base_url": source.base_url,
            "methodology_url": source.methodology_url,
        },
        "regions": regions,
    }))
    .expect("regions are serializable")
}

#[cfg(test)]
mod tests {
    use super::is_inconclusive_error;

    #[test]
    fn treats_rate_limits_and_network_failures_as_inconclusive() {
        assert!(is_inconclusive_error(
            "HTTP status client error (429 Too Many Requests)"
        ));
        assert!(is_inconclusive_error("error sending request for url"));
        assert!(!is_inconclusive_error(
            "HTTP status client error (404 Not Found)"
        ));
    }
}
