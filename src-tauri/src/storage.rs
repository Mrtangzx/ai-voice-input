use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub id: Option<i64>,
    pub raw_text: String,
    pub clean_text: String,
    pub duration_ms: i64,
    pub created_at: DateTime<Utc>,
    pub app_name: Option<String>,
}

pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    pub async fn in_memory() -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .connect(":memory:")
            .await?;
        let storage = Self { pool };
        storage.migrate().await?;
        Ok(storage)
    }

    pub async fn open(path: &str) -> Result<Self> {
        let opts = SqliteConnectOptions::from_str(path)?.create_if_missing(true);
        let pool = SqlitePoolOptions::new().connect_with(opts).await?;
        let storage = Self { pool };
        storage.migrate().await?;
        Ok(storage)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS transcripts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                raw_text TEXT NOT NULL,
                clean_text TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                app_name TEXT
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS transcripts_fts USING fts5(
                raw_text, clean_text, content='transcripts', content_rowid='id'
            );
        "#).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn insert(&self, t: &Transcript) -> Result<i64> {
        let id = sqlx::query(
            "INSERT INTO transcripts (raw_text, clean_text, duration_ms, created_at, app_name)
             VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&t.raw_text).bind(&t.clean_text).bind(t.duration_ms)
        .bind(t.created_at.to_rfc3339()).bind(&t.app_name)
        .execute(&self.pool).await?
        .last_insert_rowid();

        sqlx::query("INSERT INTO transcripts_fts (rowid, raw_text, clean_text) VALUES (?, ?, ?)")
            .bind(id).bind(&t.raw_text).bind(&t.clean_text)
            .execute(&self.pool).await?;
        Ok(id)
    }

    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<Transcript>> {
        let rows = sqlx::query_as::<_, TranscriptRow>(
            "SELECT id, raw_text, clean_text, duration_ms, created_at, app_name
             FROM transcripts ORDER BY id DESC LIMIT ? OFFSET ?"
        ).bind(limit).bind(offset).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn search(&self, q: &str, limit: i64) -> Result<Vec<Transcript>> {
        let rows = sqlx::query_as::<_, TranscriptRow>(
            "SELECT t.id, t.raw_text, t.clean_text, t.duration_ms, t.created_at, t.app_name
             FROM transcripts t JOIN transcripts_fts f ON t.id = f.rowid
             WHERE transcripts_fts MATCH ? ORDER BY rank LIMIT ?"
        ).bind(q).bind(limit).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM transcripts WHERE id = ?").bind(id).execute(&self.pool).await?;
        sqlx::query("DELETE FROM transcripts_fts WHERE rowid = ?").bind(id).execute(&self.pool).await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct TranscriptRow {
    id: i64,
    raw_text: String,
    clean_text: String,
    duration_ms: i64,
    created_at: String,
    app_name: Option<String>,
}

impl From<TranscriptRow> for Transcript {
    fn from(r: TranscriptRow) -> Self {
        Self {
            id: Some(r.id),
            raw_text: r.raw_text,
            clean_text: r.clean_text,
            duration_ms: r.duration_ms,
            created_at: DateTime::parse_from_rfc3339(&r.created_at).unwrap().with_timezone(&Utc),
            app_name: r.app_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert_and_list_transcripts() {
        let storage = Storage::in_memory().await.unwrap();
        storage.insert(&Transcript {
            id: None,
            raw_text: "嗯那个今天天气不错".into(),
            clean_text: "今天天气不错。".into(),
            duration_ms: 3000,
            created_at: Utc::now(),
            app_name: Some("notepad".into()),
        }).await.unwrap();

        let list = storage.list(10, 0).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].clean_text, "今天天气不错。");
    }

    #[tokio::test]
    async fn fts_search_finds_match() {
        let storage = Storage::in_memory().await.unwrap();
        storage.insert(&Transcript {
            id: None,
            raw_text: "今天呃天气真的不错".into(),
            clean_text: "今天天气真的不错。".into(),
            duration_ms: 3500,
            created_at: Utc::now(),
            app_name: None,
        }).await.unwrap();
        storage.insert(&Transcript {
            id: None,
            raw_text: "明天要开会".into(),
            clean_text: "明天要开会。".into(),
            duration_ms: 2000,
            created_at: Utc::now(),
            app_name: None,
        }).await.unwrap();

        let results = storage.search("天气", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].clean_text, "今天天气真的不错。");
    }

    #[tokio::test]
    async fn delete_removes_transcript() {
        let storage = Storage::in_memory().await.unwrap();
        let id = storage.insert(&Transcript {
            id: None,
            raw_text: "test".into(),
            clean_text: "test".into(),
            duration_ms: 100,
            created_at: Utc::now(),
            app_name: None,
        }).await.unwrap();

        storage.delete(id).await.unwrap();
        let list = storage.list(10, 0).await.unwrap();
        assert_eq!(list.len(), 0);
        let search = storage.search("test", 10).await.unwrap();
        assert_eq!(search.len(), 0);
    }
}
