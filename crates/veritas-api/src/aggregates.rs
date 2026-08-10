use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};

use crate::{ApiError, AppState};

const SUPPRESSION_THRESHOLD: i64 = 5;

#[derive(Deserialize)]
pub(crate) struct AggregateQuery {
    region_id: String,
    month_key: String,
    employment: String,
}

#[derive(Serialize)]
pub(crate) struct AggregateResponse {
    necessities_avg_pct: Option<f64>,
    n: i64,
    suppressed: bool,
}

pub(crate) async fn get(
    State(state): State<AppState>,
    Query(query): Query<AggregateQuery>,
) -> Result<Json<AggregateResponse>, ApiError> {
    if query.region_id.is_empty() || query.month_key.is_empty() {
        return Err(ApiError::bad_request(
            "region_id and month_key are required",
        ));
    }
    if !matches!(query.employment.as_str(), "all" | "employed" | "unemployed") {
        return Err(ApiError::bad_request(
            "employment must be all, employed, or unemployed",
        ));
    }

    let (n, necessities_sum) = state
        .database
        .aggregate(&query.region_id, &query.month_key, &query.employment)
        .await?;
    let (_, employed_n, unemployed_n) = state
        .database
        .employment_counts(&query.region_id, &query.month_key)
        .await?;
    let suppressed = n < SUPPRESSION_THRESHOLD
        || [employed_n, unemployed_n]
            .into_iter()
            .any(|count| (1..SUPPRESSION_THRESHOLD).contains(&count));
    Ok(Json(AggregateResponse {
        necessities_avg_pct: (!suppressed).then(|| necessities_sum as f64 / n as f64),
        n,
        suppressed,
    }))
}
