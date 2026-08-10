use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};

use crate::ApiError;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

pub(crate) struct NewVote<'a> {
    pub(crate) wallet: &'a str,
    pub(crate) month_key: &'a str,
    pub(crate) region_id: &'a str,
    pub(crate) ciphertext: &'a [u8],
    pub(crate) commitment: &'a [u8; 32],
    pub(crate) necessities_pct: u8,
    pub(crate) employment: &'a str,
    pub(crate) created_at: &'a str,
}

impl Database {
    pub async fn connect(database_url: &str) -> Result<Self, ApiError> {
        // One connection keeps `sqlite::memory:` deterministic in API tests.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await
            .map_err(|_| ApiError::unavailable("database unavailable"))?;
        let database = Self { pool };
        database.migrate().await?;
        Ok(database)
    }

    async fn migrate(&self) -> Result<(), ApiError> {
        for statement in [
            "CREATE TABLE IF NOT EXISTS auth_challenges (
                nonce TEXT PRIMARY KEY,
                wallet TEXT NOT NULL,
                challenge TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                consumed_at TEXT
            )",
            "CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                wallet TEXT NOT NULL,
                expires_at TEXT NOT NULL
            )",
            "CREATE TABLE IF NOT EXISTS answers (
                id INTEGER PRIMARY KEY,
                wallet TEXT NOT NULL,
                month_key TEXT NOT NULL,
                region_id TEXT NOT NULL,
                ciphertext BLOB NOT NULL,
                nonce BLOB NOT NULL,
                commitment BLOB NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(wallet, month_key)
            )",
            "CREATE TABLE IF NOT EXISTS aggregate_bins (
                region_id TEXT NOT NULL,
                month_key TEXT NOT NULL,
                employment TEXT NOT NULL,
                n INTEGER NOT NULL CHECK(n >= 0),
                necessities_sum INTEGER NOT NULL CHECK(necessities_sum >= 0),
                PRIMARY KEY(region_id, month_key, employment)
            )",
        ] {
            sqlx::query(statement)
                .execute(&self.pool)
                .await
                .map_err(|_| ApiError::unavailable("database migration failed"))?;
        }
        Ok(())
    }

    pub(crate) async fn create_challenge(
        &self,
        nonce: &str,
        wallet: &str,
        challenge: &str,
        expires_at: &str,
    ) -> Result<(), ApiError> {
        sqlx::query(
            "INSERT INTO auth_challenges (nonce, wallet, challenge, expires_at)
             VALUES (?, ?, ?, ?)",
        )
        .bind(nonce)
        .bind(wallet)
        .bind(challenge)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|_| ApiError::unavailable("could not create challenge"))?;
        Ok(())
    }

    pub(crate) async fn challenge_for_verification(
        &self,
        nonce: &str,
        wallet: &str,
        now: &str,
    ) -> Result<Option<String>, ApiError> {
        let row = sqlx::query(
            "SELECT challenge FROM auth_challenges
             WHERE nonce = ? AND wallet = ? AND consumed_at IS NULL AND expires_at > ?",
        )
        .bind(nonce)
        .bind(wallet)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| ApiError::unavailable("database unavailable"))?;
        Ok(row.map(|row| row.get("challenge")))
    }

    pub(crate) async fn consume_challenge(
        &self,
        nonce: &str,
        wallet: &str,
        now: &str,
    ) -> Result<bool, ApiError> {
        let result = sqlx::query(
            "UPDATE auth_challenges SET consumed_at = ?
             WHERE nonce = ? AND wallet = ? AND consumed_at IS NULL AND expires_at > ?",
        )
        .bind(now)
        .bind(nonce)
        .bind(wallet)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|_| ApiError::unavailable("database unavailable"))?;
        Ok(result.rows_affected() == 1)
    }

    pub(crate) async fn create_session(
        &self,
        token: &str,
        wallet: &str,
        expires_at: &str,
    ) -> Result<(), ApiError> {
        sqlx::query("INSERT INTO sessions (token, wallet, expires_at) VALUES (?, ?, ?)")
            .bind(token)
            .bind(wallet)
            .bind(expires_at)
            .execute(&self.pool)
            .await
            .map_err(|_| ApiError::unavailable("could not create session"))?;
        Ok(())
    }

    pub(crate) async fn wallet_for_session(
        &self,
        token: &str,
        now: &str,
    ) -> Result<Option<String>, ApiError> {
        let row = sqlx::query("SELECT wallet FROM sessions WHERE token = ? AND expires_at > ?")
            .bind(token)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| ApiError::unavailable("database unavailable"))?;
        Ok(row.map(|row| row.get("wallet")))
    }

    pub(crate) async fn store_vote(&self, vote: NewVote<'_>) -> Result<(), ApiError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| ApiError::unavailable("database unavailable"))?;
        let insert = sqlx::query(
            "INSERT INTO answers
             (wallet, month_key, region_id, ciphertext, nonce, commitment, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(wallet, month_key) DO NOTHING",
        )
        .bind(vote.wallet)
        .bind(vote.month_key)
        .bind(vote.region_id)
        .bind(vote.ciphertext)
        .bind(Vec::<u8>::new())
        .bind(vote.commitment.as_slice())
        .bind(vote.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::unavailable("could not store vote"))?;
        if insert.rows_affected() != 1 {
            tx.rollback()
                .await
                .map_err(|_| ApiError::unavailable("database unavailable"))?;
            return Err(ApiError::conflict(
                "wallet already submitted for this month",
            ));
        }

        for bin in ["all", vote.employment] {
            sqlx::query(
                "INSERT INTO aggregate_bins
                 (region_id, month_key, employment, n, necessities_sum)
                 VALUES (?, ?, ?, 1, ?)
                 ON CONFLICT(region_id, month_key, employment)
                 DO UPDATE SET
                   n = aggregate_bins.n + 1,
                   necessities_sum = aggregate_bins.necessities_sum + excluded.necessities_sum",
            )
            .bind(vote.region_id)
            .bind(vote.month_key)
            .bind(bin)
            .bind(i64::from(vote.necessities_pct))
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::unavailable("could not update aggregates"))?;
        }

        tx.commit()
            .await
            .map_err(|_| ApiError::unavailable("could not store vote"))?;
        Ok(())
    }

    pub(crate) async fn aggregate(
        &self,
        region_id: &str,
        month_key: &str,
        employment: &str,
    ) -> Result<(i64, i64), ApiError> {
        let row = sqlx::query(
            "SELECT n, necessities_sum FROM aggregate_bins
             WHERE region_id = ? AND month_key = ? AND employment = ?",
        )
        .bind(region_id)
        .bind(month_key)
        .bind(employment)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| ApiError::unavailable("database unavailable"))?;
        Ok(row
            .map(|row| (row.get("n"), row.get("necessities_sum")))
            .unwrap_or((0, 0)))
    }
}
