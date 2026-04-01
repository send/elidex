//! History persistence sub-store (design doc §22.4.2 HistoryStore).
//!
//! Two-table design: `urls` (aggregated) + `visits` (per-visit).
//! Frecency scoring follows the Chromium/Firefox pattern with time-decay buckets.

use std::time::SystemTime;

use crate::error::StorageError;

/// Transition type for a visit (how the user got there).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum TransitionType {
    Link = 0,
    Typed = 1,
    Bookmark = 2,
    Reload = 3,
    FormSubmit = 4,
}

impl TransitionType {
    fn from_i32(v: i32) -> Self {
        match v {
            1 => Self::Typed,
            2 => Self::Bookmark,
            3 => Self::Reload,
            4 => Self::FormSubmit,
            _ => Self::Link,
        }
    }

    /// Frecency bonus multiplier for transition type.
    fn frecency_bonus(self) -> i64 {
        match self {
            Self::Typed => 2000,
            Self::Bookmark => 1400,
            Self::Link => 1000,
            Self::FormSubmit => 800,
            Self::Reload => 500,
        }
    }
}

/// A URL aggregate row from the `urls` table.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub visit_count: i64,
    pub typed_count: i64,
    pub frecency: i64,
    pub last_visit_time: i64,
}

/// A suggestion entry for address bar auto-complete.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub url: String,
    pub title: String,
    pub frecency: i64,
}

/// Zero-cost borrow wrapper around the browser.sqlite connection.
pub struct HistoryStore<'db> {
    conn: &'db rusqlite::Connection,
}

impl<'db> HistoryStore<'db> {
    pub(crate) fn new(conn: &'db rusqlite::Connection) -> Self {
        Self { conn }
    }

    /// Record a page visit (design doc §22.4.2).
    ///
    /// Inserts a row into `visits` and upserts the `urls` aggregate.
    pub fn record_visit(
        &self,
        url: &url::Url,
        title: &str,
        transition: TransitionType,
    ) -> Result<(), StorageError> {
        let url_str = url.as_str();
        let now = super::system_time_to_unix(SystemTime::now());
        let typed_inc = i32::from(transition == TransitionType::Typed);

        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(StorageError::from)?;

        // Upsert url aggregate.
        tx.execute(
            "INSERT INTO urls (url, title, visit_count, typed_count, last_visit_time, frecency) \
             VALUES (?1, ?2, 1, ?3, ?4, 0) \
             ON CONFLICT(url) DO UPDATE SET \
               title = CASE WHEN ?2 != '' THEN ?2 ELSE title END, \
               visit_count = visit_count + 1, \
               typed_count = typed_count + ?3, \
               last_visit_time = ?4",
            rusqlite::params![url_str, title, typed_inc, now],
        )
        .map_err(StorageError::from)?;

        // Get the url_id.
        let url_id: i64 = tx
            .query_row("SELECT id FROM urls WHERE url = ?1", [url_str], |row| {
                row.get(0)
            })
            .map_err(StorageError::from)?;

        // Insert visit record.
        tx.execute(
            "INSERT INTO visits (url_id, visit_time, transition_type) VALUES (?1, ?2, ?3)",
            rusqlite::params![url_id, now, transition as i32],
        )
        .map_err(StorageError::from)?;

        // Recalculate frecency.
        let frecency = calculate_frecency_on(&tx, url_id, now)?;
        tx.execute(
            "UPDATE urls SET frecency = ?1 WHERE id = ?2",
            rusqlite::params![frecency, url_id],
        )
        .map_err(StorageError::from)?;

        tx.commit().map_err(StorageError::from)?;
        Ok(())
    }

    /// Search history by URL/title substring.
    pub fn query(&self, text: &str, limit: usize) -> Result<Vec<HistoryEntry>, StorageError> {
        let pattern = format!("%{text}%");
        let mut stmt = self
            .conn
            .prepare(
                "SELECT url, title, visit_count, typed_count, frecency, last_visit_time \
                 FROM urls WHERE url LIKE ?1 OR title LIKE ?1 \
                 ORDER BY frecency DESC LIMIT ?2",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(rusqlite::params![pattern, limit as i64], |row| {
                Ok(HistoryEntry {
                    url: row.get(0)?,
                    title: row.get(1)?,
                    visit_count: row.get(2)?,
                    typed_count: row.get(3)?,
                    frecency: row.get(4)?,
                    last_visit_time: row.get(5)?,
                })
            })
            .map_err(StorageError::from)?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(StorageError::from)?);
        }
        Ok(entries)
    }

    /// Frecency-based suggestions for address bar auto-complete.
    pub fn frecency_suggest(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<Suggestion>, StorageError> {
        let pattern = format!("{prefix}%");
        let mut stmt = self
            .conn
            .prepare(
                "SELECT url, title, frecency FROM urls \
                 WHERE url LIKE ?1 ORDER BY frecency DESC LIMIT ?2",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map(rusqlite::params![pattern, limit as i64], |row| {
                Ok(Suggestion {
                    url: row.get(0)?,
                    title: row.get(1)?,
                    frecency: row.get(2)?,
                })
            })
            .map_err(StorageError::from)?;

        let mut suggestions = Vec::new();
        for row in rows {
            suggestions.push(row.map_err(StorageError::from)?);
        }
        Ok(suggestions)
    }

    /// Delete visits in a time range and recalculate affected URL frecencies.
    pub fn delete_range(&self, from: SystemTime, to: SystemTime) -> Result<(), StorageError> {
        let from_ts = from
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let to_ts = to
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(i64::MAX);

        // Find affected url_ids before deleting.
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT url_id FROM visits \
                 WHERE visit_time >= ?1 AND visit_time <= ?2",
            )
            .map_err(StorageError::from)?;
        let affected: Vec<i64> = stmt
            .query_map(rusqlite::params![from_ts, to_ts], |row| row.get(0))
            .map_err(StorageError::from)?
            .filter_map(Result::ok)
            .collect();

        // Delete visits in range.
        self.conn
            .execute(
                "DELETE FROM visits WHERE visit_time >= ?1 AND visit_time <= ?2",
                rusqlite::params![from_ts, to_ts],
            )
            .map_err(StorageError::from)?;

        // Recalculate frecency for affected URLs and clean up orphans.
        let now = super::system_time_to_unix(SystemTime::now());

        for url_id in affected {
            let visit_count: i64 = self
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM visits WHERE url_id = ?1",
                    [url_id],
                    |row| row.get(0),
                )
                .map_err(StorageError::from)?;

            if visit_count == 0 {
                // No visits remain — remove the URL entry.
                self.conn
                    .execute("DELETE FROM urls WHERE id = ?1", [url_id])
                    .map_err(StorageError::from)?;
            } else {
                let frecency = self.calculate_frecency(url_id, now)?;
                let last_visit: i64 = self
                    .conn
                    .query_row(
                        "SELECT MAX(visit_time) FROM visits WHERE url_id = ?1",
                        [url_id],
                        |row| row.get(0),
                    )
                    .map_err(StorageError::from)?;
                self.conn
                    .execute(
                        "UPDATE urls SET frecency = ?1, visit_count = ?2, last_visit_time = ?3 WHERE id = ?4",
                        rusqlite::params![frecency, visit_count, last_visit, url_id],
                    )
                    .map_err(StorageError::from)?;
            }
        }

        Ok(())
    }

    /// Delete all visits for a URL (cascades from `urls` table).
    pub fn delete_url(&self, url: &url::Url) -> Result<(), StorageError> {
        self.conn
            .execute(
                "DELETE FROM urls WHERE url = ?1",
                rusqlite::params![url.as_str()],
            )
            .map_err(StorageError::from)?;
        Ok(())
    }

    /// Calculate frecency for a URL based on its visit history.
    ///
    /// Uses time-decay buckets (Chromium/Firefox pattern):
    /// - Within 1 day: weight 100
    /// - Within 1 week: weight 70
    /// - Within 1 month: weight 50
    /// - Within 3 months: weight 30
    /// - Older: weight 10
    fn calculate_frecency(&self, url_id: i64, now: i64) -> Result<i64, StorageError> {
        calculate_frecency_on(self.conn, url_id, now)
    }
}

/// Frecency calculation — works on any `rusqlite::Connection`-like type.
fn calculate_frecency_on(
    conn: &rusqlite::Connection,
    url_id: i64,
    now: i64,
) -> Result<i64, StorageError> {
    let mut stmt = conn
        .prepare("SELECT visit_time, transition_type FROM visits WHERE url_id = ?1")
        .map_err(StorageError::from)?;

    let visits: Vec<(i64, i32)> = stmt
        .query_map([url_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(StorageError::from)?
        .filter_map(Result::ok)
        .collect();

    if visits.is_empty() {
        return Ok(0);
    }

    let mut score: i64 = 0;
    for (visit_time, transition) in &visits {
        let age_secs = now.saturating_sub(*visit_time);
        let age_weight = time_decay_weight(age_secs);
        let bonus = TransitionType::from_i32(*transition).frecency_bonus();
        score += (age_weight * bonus) / 100;
    }

    Ok(score / visits.len() as i64)
}

/// Time-decay weight based on visit age in seconds.
fn time_decay_weight(age_secs: i64) -> i64 {
    const DAY: i64 = 86_400;
    const WEEK: i64 = 7 * DAY;
    const MONTH: i64 = 30 * DAY;
    const QUARTER: i64 = 90 * DAY;

    if age_secs < DAY {
        100
    } else if age_secs < WEEK {
        70
    } else if age_secs < MONTH {
        50
    } else if age_secs < QUARTER {
        30
    } else {
        10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_db::BrowserDb;

    fn test_db() -> (tempfile::TempDir, BrowserDb) {
        let dir = tempfile::tempdir().unwrap();
        let db = BrowserDb::open(dir.path()).unwrap();
        (dir, db)
    }

    #[test]
    fn record_and_query() {
        let (_dir, db) = test_db();
        let store = db.history();
        let url = url::Url::parse("https://example.com/page").unwrap();

        store
            .record_visit(&url, "Example Page", TransitionType::Link)
            .unwrap();

        let results = store.query("example", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://example.com/page");
        assert_eq!(results[0].title, "Example Page");
        assert_eq!(results[0].visit_count, 1);
    }

    #[test]
    fn multiple_visits_increment_count() {
        let (_dir, db) = test_db();
        let store = db.history();
        let url = url::Url::parse("https://example.com/").unwrap();

        store
            .record_visit(&url, "Home", TransitionType::Typed)
            .unwrap();
        store
            .record_visit(&url, "Home", TransitionType::Link)
            .unwrap();
        store
            .record_visit(&url, "Home", TransitionType::Link)
            .unwrap();

        let results = store.query("example", 10).unwrap();
        assert_eq!(results[0].visit_count, 3);
        assert_eq!(results[0].typed_count, 1);
    }

    #[test]
    fn frecency_suggest_ordered() {
        let (_dir, db) = test_db();
        let store = db.history();

        let url1 = url::Url::parse("https://example.com/a").unwrap();
        let url2 = url::Url::parse("https://example.com/b").unwrap();

        // url2 gets more visits → higher frecency.
        store
            .record_visit(&url1, "A", TransitionType::Link)
            .unwrap();
        for _ in 0..5 {
            store
                .record_visit(&url2, "B", TransitionType::Typed)
                .unwrap();
        }

        let suggestions = store.frecency_suggest("https://example", 10).unwrap();
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].url, "https://example.com/b");
        assert!(suggestions[0].frecency > suggestions[1].frecency);
    }

    #[test]
    fn delete_url_cascades() {
        let (_dir, db) = test_db();
        let store = db.history();
        let url = url::Url::parse("https://example.com/delete-me").unwrap();

        store
            .record_visit(&url, "Delete", TransitionType::Link)
            .unwrap();
        store.delete_url(&url).unwrap();

        let results = store.query("delete-me", 10).unwrap();
        assert!(results.is_empty());

        // Visits should also be gone (CASCADE).
        let count: i64 = db
            .raw_connection()
            .query_row("SELECT COUNT(*) FROM visits", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn time_decay_weights() {
        assert_eq!(time_decay_weight(0), 100);
        assert_eq!(time_decay_weight(3600), 100); // 1 hour
        assert_eq!(time_decay_weight(86_400 + 1), 70); // >1 day
        assert_eq!(time_decay_weight(86_400 * 8), 50); // >1 week
        assert_eq!(time_decay_weight(86_400 * 31), 30); // >1 month
        assert_eq!(time_decay_weight(86_400 * 91), 10); // >3 months
    }
}
