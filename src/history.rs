#[cfg(feature = "history")]
use rusqlite::{Connection, Result as SqlResult};
// use serde_json;
use std::path::Path;
use anyhow::Result;

#[cfg(feature = "history")]
pub struct HistoryManager {
    conn: Connection,
}

#[cfg(feature = "history")]
impl HistoryManager {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS command_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                working_dir TEXT NOT NULL,
                command TEXT NOT NULL,
                args TEXT,
                output TEXT,
                session_history TEXT
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn record_command(
        &self,
        working_dir: &str,
        command: &str,
        args: Option<&str>,
        output: Option<&str>,
        session_history: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO command_history (working_dir, command, args, output, session_history)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (working_dir, command, args, output, session_history),
        )?;
        Ok(())
    }

    pub fn get_recent_history(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT timestamp, working_dir, command, args, output, session_history
             FROM command_history 
             ORDER BY timestamp DESC 
             LIMIT ?1"
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(HistoryEntry {
                timestamp: row.get(0)?,
                working_dir: row.get(1)?,
                command: row.get(2)?,
                args: row.get(3)?,
                output: row.get(4)?,
                session_history: row.get(5)?,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }
}

#[cfg(not(feature = "history"))]
pub struct HistoryManager;

#[cfg(not(feature = "history"))]
impl HistoryManager {
    pub fn new(_db_path: &Path) -> Result<Self> {
        Ok(Self)
    }

    pub fn record_command(
        &self,
        _working_dir: &str,
        _command: &str,
        _args: Option<&str>,
        _output: Option<&str>,
        _session_history: Option<&str>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn get_recent_history(&self, _limit: usize) -> Result<Vec<HistoryEntry>> {
        Ok(Vec::new())
    }
}

#[derive(Debug)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub working_dir: String,
    pub command: String,
    pub args: Option<String>,
    pub output: Option<String>,
    pub session_history: Option<String>,
}