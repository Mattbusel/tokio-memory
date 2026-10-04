//! # SQLite backend
//!
//! Durable implementations of [`EpisodicStore`] and [`SemanticStore`] in one
//! SQLite file (or in memory), so an agent's memory survives restarts.
//! Enable with the `sqlite` feature; SQLite is compiled in (`rusqlite` with
//! `bundled`), so there is nothing to install.
//!
//! Episodes are also indexed with SQLite's FTS5 full-text engine:
//! [`SqliteMemory::search`] finds episodes by words in their content, ranked
//! by BM25, which is how you answer "what did the user say about flights?"
//! without knowing an id, tag or time range.
//!
//! Calls run on Tokio's blocking pool, so they never stall the async runtime.
//!
//! ```no_run
//! # async fn demo() -> Result<(), tokio_memory::MemoryError> {
//! use tokio_memory::episodic::{Episode, EpisodeKind, EpisodicStore};
//! use tokio_memory::id::{AgentId, SessionId};
//! use tokio_memory::sqlite::SqliteMemory;
//!
//! let memory = SqliteMemory::open("agent-memory.db").await?;
//! let (agent, session) = (AgentId::new(), SessionId::new());
//! memory
//!     .record(Episode::new(agent, session, EpisodeKind::Observation,
//!         "The user wants a window seat on the Lisbon flight", 0.9, vec![])?)
//!     .await?;
//!
//! for (episode, score) in memory.search("lisbon flight", 5).await? {
//!     println!("{score:.2}  {}", episode.content);
//! }
//! # Ok(())
//! # }
//! ```

use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};

use crate::episodic::{Episode, EpisodicStore, TimeRange};
use crate::error::MemoryError;
use crate::id::{AgentId, EntityId, MemoryId, SessionId, Tag};
use crate::semantic::{Fact, SemanticStore};

const SCHEMA: &str = "
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS episodes (
    id         TEXT PRIMARY KEY,
    agent_id   TEXT NOT NULL,
    session_id TEXT NOT NULL,
    ts         TEXT NOT NULL,
    body       TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS episodes_agent   ON episodes(agent_id, ts);
CREATE INDEX IF NOT EXISTS episodes_session ON episodes(session_id, ts);
CREATE INDEX IF NOT EXISTS episodes_ts      ON episodes(ts);
CREATE TABLE IF NOT EXISTS episode_tags (
    episode_id TEXT NOT NULL REFERENCES episodes(id) ON DELETE CASCADE,
    tag        TEXT NOT NULL,
    PRIMARY KEY (tag, episode_id)
);
CREATE VIRTUAL TABLE IF NOT EXISTS episodes_fts USING fts5(id UNINDEXED, content);
CREATE TABLE IF NOT EXISTS facts (
    id      TEXT PRIMARY KEY,
    subject TEXT NOT NULL,
    body    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS facts_subject ON facts(subject);
";

fn db_err(e: impl std::fmt::Display) -> MemoryError {
    MemoryError::Persistence(e.to_string())
}

/// Timestamps are stored as RFC 3339 with fixed precision so that text
/// order equals time order.
fn ts_key(ts: &chrono::DateTime<chrono::Utc>) -> String {
    ts.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

/// Episodic and semantic memory in one SQLite database. Cheap to clone.
#[derive(Clone)]
pub struct SqliteMemory {
    conn: Arc<Mutex<Connection>>,
}

impl std::fmt::Debug for SqliteMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteMemory").finish_non_exhaustive()
    }
}

impl SqliteMemory {
    /// Open (or create) the database file at `path`.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let path = path.as_ref().to_owned();
        let conn = tokio::task::spawn_blocking(move || Connection::open(path))
            .await
            .map_err(db_err)?
            .map_err(db_err)?;
        Self::init(conn).await
    }

    /// A private in-memory database (gone when the last clone is dropped).
    pub async fn in_memory() -> Result<Self, MemoryError> {
        Self::init(Connection::open_in_memory().map_err(db_err)?).await
    }

    async fn init(conn: Connection) -> Result<Self, MemoryError> {
        let memory = Self { conn: Arc::new(Mutex::new(conn)) };
        memory
            .with(|c| {
                c.execute_batch("PRAGMA foreign_keys = ON;")?;
                c.execute_batch(SCHEMA)
            })
            .await?;
        Ok(memory)
    }

    /// Run `f` with the connection on the blocking pool.
    async fn with<T, F>(&self, f: F) -> Result<T, MemoryError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let mut guard = conn.lock().map_err(|_| db_err("connection lock poisoned"))?;
            f(&mut guard).map_err(db_err)
        })
        .await
        .map_err(db_err)?
    }

    /// Episodes whose content matches `query`, best first, with their BM25
    /// relevance (higher is better).
    ///
    /// Plain words are matched as all-of, in any order and any case; FTS5
    /// syntax (`"exact phrase"`, `OR`, `NOT`, `prefix*`) is also accepted.
    /// Words are matched exactly (no stemming).
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<(Episode, f64)>, MemoryError> {
        let fts_query = to_fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self
            .with(move |c| {
                let mut stmt = c.prepare(
                    "SELECT e.body, -bm25(episodes_fts) AS score
                     FROM episodes_fts JOIN episodes e ON e.id = episodes_fts.id
                     WHERE episodes_fts MATCH ?1
                     ORDER BY bm25(episodes_fts) LIMIT ?2",
                )?;
                let rows = stmt
                    .query_map(params![fts_query, limit as i64], |r| {
                        Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            })
            .await?;
        rows.into_iter()
            .map(|(body, score)| Ok((decode::<Episode>(&body)?, score)))
            .collect()
    }

    /// Delete an episode. Returns whether it existed.
    pub async fn forget(&self, id: &MemoryId) -> Result<bool, MemoryError> {
        let id = id.to_string();
        self.with(move |c| {
            let tx = c.transaction()?;
            tx.execute("DELETE FROM episodes_fts WHERE id = ?1", [&id])?;
            let n = tx.execute("DELETE FROM episodes WHERE id = ?1", [&id])?;
            tx.commit()?;
            Ok(n > 0)
        })
        .await
    }

    /// Number of stored episodes.
    pub async fn episode_count(&self) -> Result<usize, MemoryError> {
        self.with(|c| c.query_row("SELECT COUNT(*) FROM episodes", [], |r| r.get::<_, i64>(0)))
            .await
            .map(|n| n as usize)
    }

    async fn episodes_where(&self, sql: &'static str, arg: Vec<String>) -> Result<Vec<Episode>, MemoryError> {
        let bodies = self
            .with(move |c| {
                let mut stmt = c.prepare(sql)?;
                let rows = stmt
                    .query_map(rusqlite::params_from_iter(arg.iter()), |r| r.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            })
            .await?;
        bodies.iter().map(|b| decode(b)).collect()
    }
}

/// Turn free text into an FTS5 query: words joined with AND, each quoted so
/// punctuation cannot break the syntax. Queries already using FTS5 operators
/// or quotes are passed through.
fn to_fts_query(query: &str) -> String {
    let q = query.trim();
    let uses_syntax = q.contains('"')
        || q.ends_with('*')
        || q.split_whitespace().any(|w| matches!(w, "AND" | "OR" | "NOT" | "NEAR"));
    if uses_syntax {
        return q.to_owned();
    }
    q.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{w}\""))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn decode<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, MemoryError> {
    serde_json::from_str(body).map_err(|e| MemoryError::Serialization(e.to_string()))
}

fn encode<T: serde::Serialize>(value: &T) -> Result<String, MemoryError> {
    serde_json::to_string(value).map_err(|e| MemoryError::Serialization(e.to_string()))
}

#[async_trait]
impl EpisodicStore for SqliteMemory {
    /// Inserts the episode, or replaces the one with the same id.
    async fn record(&self, episode: Episode) -> Result<(), MemoryError> {
        let body = encode(&episode)?;
        let id = episode.id.to_string();
        let tags: Vec<String> = episode.tags.iter().map(|t| t.as_str().to_owned()).collect();
        let (agent, session, ts, content) = (
            episode.agent_id.to_string(),
            episode.session_id.to_string(),
            ts_key(&episode.timestamp),
            episode.content,
        );
        self.with(move |c| {
            let tx = c.transaction()?;
            tx.execute(
                "INSERT INTO episodes (id, agent_id, session_id, ts, body) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET agent_id = ?2, session_id = ?3, ts = ?4, body = ?5",
                params![id, agent, session, ts, body],
            )?;
            tx.execute("DELETE FROM episode_tags WHERE episode_id = ?1", [&id])?;
            for tag in &tags {
                tx.execute("INSERT OR IGNORE INTO episode_tags (episode_id, tag) VALUES (?1, ?2)", params![id, tag])?;
            }
            tx.execute("DELETE FROM episodes_fts WHERE id = ?1", [&id])?;
            tx.execute("INSERT INTO episodes_fts (id, content) VALUES (?1, ?2)", params![id, content])?;
            tx.commit()
        })
        .await
    }

    async fn recall(&self, id: &MemoryId) -> Result<Episode, MemoryError> {
        let key = id.to_string();
        let body = self
            .with(move |c| {
                c.query_row("SELECT body FROM episodes WHERE id = ?1", [key], |r| r.get::<_, String>(0))
                    .optional()
            })
            .await?;
        match body {
            Some(b) => decode(&b),
            None => Err(MemoryError::NotFound(id.clone())),
        }
    }

    async fn recall_range(&self, range: &TimeRange) -> Result<Vec<Episode>, MemoryError> {
        self.episodes_where(
            "SELECT body FROM episodes WHERE ts >= ?1 AND ts <= ?2 ORDER BY ts",
            vec![ts_key(&range.start), ts_key(&range.end)],
        )
        .await
    }

    async fn recall_by_session(&self, session_id: &SessionId) -> Result<Vec<Episode>, MemoryError> {
        self.episodes_where(
            "SELECT body FROM episodes WHERE session_id = ?1 ORDER BY ts",
            vec![session_id.to_string()],
        )
        .await
    }

    async fn recall_by_tag(&self, tag: &Tag) -> Result<Vec<Episode>, MemoryError> {
        self.episodes_where(
            "SELECT e.body FROM episode_tags t JOIN episodes e ON e.id = t.episode_id
             WHERE t.tag = ?1 ORDER BY e.ts",
            vec![tag.as_str().to_owned()],
        )
        .await
    }

    async fn all(&self) -> Result<Vec<Episode>, MemoryError> {
        self.episodes_where("SELECT body FROM episodes ORDER BY ts", Vec::new()).await
    }

    async fn recall_by_agent(&self, agent_id: &AgentId) -> Result<Vec<Episode>, MemoryError> {
        self.episodes_where(
            "SELECT body FROM episodes WHERE agent_id = ?1 ORDER BY ts",
            vec![agent_id.to_string()],
        )
        .await
    }
}

#[async_trait]
impl SemanticStore for SqliteMemory {
    /// Inserts the fact, or replaces the one with the same id.
    async fn assert_fact(&self, fact: Fact) -> Result<(), MemoryError> {
        let body = encode(&fact)?;
        let (id, subject) = (fact.id.to_string(), fact.subject.as_str().to_owned());
        self.with(move |c| {
            c.execute(
                "INSERT INTO facts (id, subject, body) VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET subject = ?2, body = ?3",
                params![id, subject, body],
            )
            .map(|_| ())
        })
        .await
    }

    async fn retract_fact(&self, id: &MemoryId) -> Result<(), MemoryError> {
        let key = id.to_string();
        let removed = self.with(move |c| c.execute("DELETE FROM facts WHERE id = ?1", [key])).await?;
        if removed == 0 {
            return Err(MemoryError::NotFound(id.clone()));
        }
        Ok(())
    }

    async fn query_subject(&self, subject: &EntityId) -> Result<Vec<Fact>, MemoryError> {
        let key = subject.as_str().to_owned();
        let bodies = self
            .with(move |c| {
                let mut stmt = c.prepare("SELECT body FROM facts WHERE subject = ?1")?;
                let rows = stmt
                    .query_map([key], |r| r.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            })
            .await?;
        bodies.iter().map(|b| decode(b)).collect()
    }

    async fn all_facts(&self) -> Result<Vec<Fact>, MemoryError> {
        let bodies = self
            .with(|c| {
                let mut stmt = c.prepare("SELECT body FROM facts")?;
                let rows = stmt
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(rows)
            })
            .await?;
        bodies.iter().map(|b| decode(b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::to_fts_query;

    #[test]
    fn free_text_becomes_quoted_and_terms() {
        assert_eq!(to_fts_query("Lisbon flight?"), "\"Lisbon\" AND \"flight\"");
        assert_eq!(to_fts_query("  "), "");
        assert_eq!(to_fts_query("\"window seat\" OR aisle"), "\"window seat\" OR aisle");
        assert_eq!(to_fts_query("lisb*"), "lisb*");
    }
}
