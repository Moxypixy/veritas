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
