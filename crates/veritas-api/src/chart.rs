use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::{ApiError, AppState};

const VOTERS_NECESSITIES: &str = "voters_necessities";
const VOTERS_NECESSITIES_LABEL: &str = "What voters say they spend on necessities";

#[derive(Deserialize)]
pub(crate) struct ChartQuery {
    region_id: String,
    employment: Option<String>,
}

#[derive(Serialize)]
struct ChartPoint {
    month_key: String,
    value: f64,
}

#[derive(Serialize)]
struct ChartSeries {
    key: String,
    label: String,
    points: Vec<ChartPoint>,
}

#[derive(Serialize)]
pub(crate) struct ChartResponse {
    region_id: String,
    official_unavailable: bool,
    series: Vec<ChartSeries>,
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Query(query): Query<ChartQuery>,
) -> Result<Json<ChartResponse>, ApiError> {
    if query.region_id.is_empty() {
        return Err(ApiError::bad_request("region_id is required"));
    }
    let employment = query.employment.as_deref().unwrap_or("all");
    if !matches!(employment, "all" | "employed" | "unemployed") {
        return Err(ApiError::bad_request(
            "employment must be all, employed, or unemployed",
        ));
    }

    let official = state
        .database
        .official_region_data(&query.region_id)
        .await?;
    let votes = state
        .database
        .chart_vote_points(&query.region_id, employment)
        .await?;
    let mut series = if official.unavailable {
        Vec::new()
    } else {
        official
            .series
            .into_iter()
            .map(|official_series| ChartSeries {
                key: official_series.key,
                label: official_series.label,
                points: official_series
                    .points
                    .into_iter()
                    .map(|point| ChartPoint {
                        month_key: point.month_key,
                        value: point.value,
                    })
                    .collect(),
            })
            .collect()
    };
    series.push(ChartSeries {
        key: VOTERS_NECESSITIES.to_owned(),
        label: VOTERS_NECESSITIES_LABEL.to_owned(),
        points: votes
            .into_iter()
            .map(|point| ChartPoint {
                month_key: point.month_key,
                value: point.value,
            })
            .collect(),
    });

    Ok(Json(ChartResponse {
        region_id: query.region_id,
        official_unavailable: official.unavailable,
        series,
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::extract::{Query, State};
    use chrono::{TimeZone, Utc};
    use serde_json::json;
    use veritas_inflation::{HttpClient, RefreshResult, fetch_region, parse_config};

    use super::{ChartQuery, get};
    use crate::{ApiError, AppState, AuthVerifier, ChainVerifier, DataKey, Database, FixedClock};

    const RECORDED_OECD_RESPONSE: &str =
        include_str!("../../veritas-inflation/tests/fixtures/oecd-prices.csv");

    struct RecordedHttpClient;

    impl HttpClient for RecordedHttpClient {
        async fn get(&self, _url: &str) -> Result<String, String> {
            Ok(RECORDED_OECD_RESPONSE.to_owned())
        }
    }

    struct NoopChainVerifier;

    #[async_trait]
    impl ChainVerifier for NoopChainVerifier {
        async fn commitment_for_vote(
            &self,
            _wallet: &str,
            _month_key: &str,
        ) -> Result<Option<[u8; 32]>, ApiError> {
            Ok(None)
        }
    }

    struct NoopAuthVerifier;

    #[async_trait]
    impl AuthVerifier for NoopAuthVerifier {
        async fn verify(
            &self,
            _wallet: &str,
            _challenge: &str,
            _signature: &str,
            _public_key: &str,
        ) -> Result<bool, ApiError> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn hides_cached_official_series_after_region_loses_a_required_series() {
        let config = parse_config(include_str!("../../veritas-inflation/regions.json")).unwrap();
        let mut region = config
            .regions
            .iter()
            .find(|region| region.id == "united-states")
            .unwrap()
            .clone();
        let database = Database::connect("sqlite::memory:").await.unwrap();

        assert_eq!(
            fetch_region(
                &RecordedHttpClient,
                database.pool(),
                &config.source,
                &region
            )
            .await
            .unwrap(),
            RefreshResult::Updated
        );

        region.series.retain(|series| series.key != "prices_energy");
        assert_eq!(
            fetch_region(
                &RecordedHttpClient,
                database.pool(),
                &config.source,
                &region
            )
            .await
            .unwrap(),
            RefreshResult::Skipped
        );

        let state = AppState::new(
            database,
            DataKey::from_base64("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap(),
            Arc::new(NoopChainVerifier),
            Arc::new(NoopAuthVerifier),
            Arc::new(FixedClock::new(
                Utc.with_ymd_and_hms(2026, 8, 10, 12, 0, 0).unwrap(),
            )),
        );
        let response = get(
            State(state),
            Query(ChartQuery {
                region_id: "united-states".into(),
                employment: None,
            }),
        )
        .await
        .unwrap();

        assert_eq!(
            serde_json::to_value(response.0).unwrap(),
            json!({
                "region_id": "united-states",
                "official_unavailable": true,
                "series": [{
                    "key": "voters_necessities",
                    "label": "What voters say they spend on necessities",
                    "points": []
                }]
            })
        );
    }
}
