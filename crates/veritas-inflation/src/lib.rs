//! Official price-series ingestion and normalization.

use std::{collections::HashSet, fmt, future::Future};

use serde::Deserialize;
use sqlx::{Row, SqlitePool};

pub const REQUIRED_SERIES_KEYS: [&str; 4] = [
    "prices_overall",
    "prices_food",
    "prices_housing",
    "prices_energy",
];

#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    pub id: String,
    pub source_area: String,
    pub measure: String,
    pub series: Vec<ConfiguredSeries>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SourceConfig {
    pub name: String,
    pub base_url: String,
    pub methodology_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InflationConfig {
    pub source: SourceConfig,
    pub regions: Vec<Region>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ConfiguredSeries {
    pub key: String,
    pub label: String,
    pub series_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OfficialPoint {
    pub month_key: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OfficialSeries {
    pub key: String,
    pub label: String,
    pub points: Vec<OfficialPoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OfficialRegionData {
    pub unavailable: bool,
    pub series: Vec<OfficialSeries>,
}

#[derive(Debug)]
pub struct InflationError(String);

impl InflationError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for InflationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for InflationError {}

#[derive(Deserialize)]
struct RegionFile {
    source: SourceConfig,
    regions: Vec<Region>,
}

impl<'de> Deserialize<'de> for Region {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RegionWire {
            id: String,
            source_area: String,
            measure: String,
            series: Vec<ConfiguredSeries>,
        }

        let wire = RegionWire::deserialize(deserializer)?;
        Ok(Self {
            id: wire.id,
            source_area: wire.source_area,
            measure: wire.measure,
            series: wire.series,
        })
    }
}

pub fn parse_regions(json: &str) -> Result<Vec<Region>, InflationError> {
    Ok(parse_config(json)?.regions)
}

pub fn parse_config(json: &str) -> Result<InflationConfig, InflationError> {
    let file: RegionFile =
        serde_json::from_str(json).map_err(|error| InflationError::new(error.to_string()))?;
    Ok(InflationConfig {
        source: file.source,
        regions: file.regions,
    })
}

pub fn region_is_enabled(region: &Region) -> bool {
    let configured_keys: HashSet<&str> = region
        .series
        .iter()
        .filter(|series| !series.series_key.is_empty() && !series.label.is_empty())
        .map(|series| series.key.as_str())
        .collect();

    !region.id.is_empty()
        && REQUIRED_SERIES_KEYS
            .iter()
            .all(|required| configured_keys.contains(required))
}

pub fn parse_oecd_csv(body: &str) -> Result<Vec<(String, f64)>, InflationError> {
    let mut lines = body.lines();
    let header = lines
        .next()
        .ok_or_else(|| InflationError::new("OECD response has no header"))?;
    let columns: Vec<&str> = header.split(',').collect();
    let month_index = columns
        .iter()
        .position(|column| *column == "TIME_PERIOD")
        .ok_or_else(|| InflationError::new("OECD response has no TIME_PERIOD column"))?;
    let value_index = columns
        .iter()
        .position(|column| *column == "OBS_VALUE")
        .ok_or_else(|| InflationError::new("OECD response has no OBS_VALUE column"))?;

    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            let month = fields
                .get(month_index)
                .ok_or_else(|| InflationError::new("OECD response row has no month"))?;
            let value = fields
                .get(value_index)
                .ok_or_else(|| InflationError::new("OECD response row has no value"))?
                .parse::<f64>()
                .map_err(|_| InflationError::new("OECD response contains an invalid value"))?;
            if !is_month_key(month) {
                return Err(InflationError::new(
                    "OECD response contains an invalid month",
                ));
            }
            Ok(((*month).to_owned(), value))
        })
        .collect()
}

pub async fn migrate_store(pool: &SqlitePool) -> Result<(), InflationError> {
    for statement in [
        "CREATE TABLE IF NOT EXISTS official_region_status (
            region_id TEXT PRIMARY KEY,
            official_unavailable INTEGER NOT NULL CHECK(official_unavailable IN (0, 1)),
            last_success_at TEXT
        )",
        "CREATE TABLE IF NOT EXISTS official_series_points (
            region_id TEXT NOT NULL,
            series_key TEXT NOT NULL,
            label TEXT NOT NULL,
            month_key TEXT NOT NULL,
            value REAL NOT NULL,
            PRIMARY KEY(region_id, series_key, month_key)
        )",
    ] {
        sqlx::query(statement)
            .execute(pool)
            .await
            .map_err(|error| InflationError::new(error.to_string()))?;
    }
    Ok(())
}

pub async fn replace_region(
    pool: &SqlitePool,
    region_id: &str,
    series: &[OfficialSeries],
) -> Result<(), InflationError> {
    let mut transaction = pool
        .begin()
        .await
        .map_err(|error| InflationError::new(error.to_string()))?;
    sqlx::query("DELETE FROM official_series_points WHERE region_id = ?")
        .bind(region_id)
        .execute(&mut *transaction)
        .await
        .map_err(|error| InflationError::new(error.to_string()))?;

    for configured_series in series {
        for point in &configured_series.points {
            sqlx::query(
                "INSERT INTO official_series_points
                 (region_id, series_key, label, month_key, value)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(region_id)
            .bind(&configured_series.key)
            .bind(&configured_series.label)
            .bind(&point.month_key)
            .bind(point.value)
            .execute(&mut *transaction)
            .await
            .map_err(|error| InflationError::new(error.to_string()))?;
        }
    }
    sqlx::query(
        "INSERT INTO official_region_status
         (region_id, official_unavailable, last_success_at)
         VALUES (?, 0, CURRENT_TIMESTAMP)
         ON CONFLICT(region_id) DO UPDATE SET
           official_unavailable = 0,
           last_success_at = CURRENT_TIMESTAMP",
    )
    .bind(region_id)
    .execute(&mut *transaction)
    .await
    .map_err(|error| InflationError::new(error.to_string()))?;
    transaction
        .commit()
        .await
        .map_err(|error| InflationError::new(error.to_string()))?;
    Ok(())
}

pub async fn mark_region_unavailable(
    pool: &SqlitePool,
    region_id: &str,
) -> Result<(), InflationError> {
    sqlx::query(
        "INSERT INTO official_region_status (region_id, official_unavailable, last_success_at)
         VALUES (?, 1, NULL)
         ON CONFLICT(region_id) DO UPDATE SET official_unavailable = 1",
    )
    .bind(region_id)
    .execute(pool)
    .await
    .map_err(|error| InflationError::new(error.to_string()))?;
    Ok(())
}

pub async fn region_data(
    pool: &SqlitePool,
    region_id: &str,
) -> Result<OfficialRegionData, InflationError> {
    let unavailable =
        sqlx::query("SELECT official_unavailable FROM official_region_status WHERE region_id = ?")
            .bind(region_id)
            .fetch_optional(pool)
            .await
            .map_err(|error| InflationError::new(error.to_string()))?
            .map(|row| row.get::<i64, _>("official_unavailable") != 0)
            .unwrap_or(true);

    let rows = sqlx::query(
        "SELECT series_key, label, month_key, value
         FROM official_series_points
         WHERE region_id = ?
         ORDER BY series_key, month_key",
    )
    .bind(region_id)
    .fetch_all(pool)
    .await
    .map_err(|error| InflationError::new(error.to_string()))?;
    let mut series = Vec::<OfficialSeries>::new();
    for row in rows {
        let key: String = row.get("series_key");
        let series_index = match series.iter().position(|current| current.key == key) {
            Some(index) => index,
            None => {
                series.push(OfficialSeries {
                    key: key.clone(),
                    label: row.get("label"),
                    points: Vec::new(),
                });
                series.len() - 1
            }
        };
        series[series_index].points.push(OfficialPoint {
            month_key: row.get("month_key"),
            value: row.get("value"),
        });
    }
    Ok(OfficialRegionData {
        unavailable,
        series,
    })
}

pub trait HttpClient {
    fn get(&self, url: &str) -> impl Future<Output = Result<String, String>> + Send;
}

pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

impl Default for ReqwestHttpClient {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl HttpClient for ReqwestHttpClient {
    fn get(&self, url: &str) -> impl Future<Output = Result<String, String>> + Send {
        let client = self.client.clone();
        let url = url.to_owned();
        async move {
            client
                .get(url)
                .header(reqwest::header::ACCEPT, "text/csv")
                .send()
                .await
                .map_err(|error| error.to_string())?
                .error_for_status()
                .map_err(|error| error.to_string())?
                .text()
                .await
                .map_err(|error| error.to_string())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshResult {
    Updated,
    Skipped,
}

pub async fn fetch_region(
    client: &impl HttpClient,
    pool: &SqlitePool,
    source: &SourceConfig,
    region: &Region,
) -> Result<RefreshResult, InflationError> {
    if !region_is_enabled(region) {
        mark_region_unavailable(pool, &region.id).await?;
        return Ok(RefreshResult::Skipped);
    }

    let fetched = fetch_required_series(client, source, region).await;
    match fetched {
        Ok(series) => {
            replace_region(pool, &region.id, &series).await?;
            Ok(RefreshResult::Updated)
        }
        Err(error) => {
            mark_region_unavailable(pool, &region.id).await?;
            Err(error)
        }
    }
}

async fn fetch_required_series(
    client: &impl HttpClient,
    source: &SourceConfig,
    region: &Region,
) -> Result<Vec<OfficialSeries>, InflationError> {
    let mut fetched = Vec::with_capacity(REQUIRED_SERIES_KEYS.len());
    for key in REQUIRED_SERIES_KEYS {
        let Some(configured_series) = region.series.iter().find(|series| series.key == key) else {
            return Err(InflationError::new("region is missing a required series"));
        };
        let url = format!(
            "{}/{}?startPeriod=2020-01",
            source.base_url.trim_end_matches('/'),
            configured_series.series_key
        );
        let body = client.get(&url).await.map_err(InflationError::new)?;
        let points = parse_oecd_csv(&body)?;
        if points.is_empty() {
            return Err(InflationError::new("OECD response has no observations"));
        }
        fetched.push(OfficialSeries {
            key: configured_series.key.clone(),
            label: configured_series.label.clone(),
            points: points
                .into_iter()
                .map(|(month_key, value)| OfficialPoint { month_key, value })
                .collect(),
        });
    }
    Ok(fetched)
}

fn is_month_key(value: &str) -> bool {
    value.len() == 7
        && value.as_bytes().get(4) == Some(&b'-')
        && value[..4]
            .chars()
            .all(|character| character.is_ascii_digit())
        && value[5..]
            .chars()
            .all(|character| character.is_ascii_digit())
}
