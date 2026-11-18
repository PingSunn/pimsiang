use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row, params};

pub const SCHEMA_VERSION: i64 = 2;
const TRIGRAM_MIN_CHARS: usize = 3;

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("History database: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("History folder: {0}")]
    Io(#[from] std::io::Error),
    #[error("History database has schema version {found}, this app understands up to {supported}")]
    TooNew { found: i64, supported: i64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Pasted,
    Note,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pasted => "pasted",
            Self::Note => "note",
        }
    }

    fn parse(text: &str) -> Self {
        match text {
            "note" => Self::Note,
            _ => Self::Pasted,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: i64,
    pub text: String,
    pub created: i64,
    pub kind: Kind,
    pub model_id: Option<String>,
}

pub struct History {
    connection: Connection,
}

const MIGRATIONS: [&str; 2] = [
    "CREATE TABLE entries (
        id INTEGER PRIMARY KEY,
        text TEXT NOT NULL,
        created INTEGER NOT NULL,
        kind TEXT NOT NULL
    );
    CREATE INDEX entries_created ON entries (created DESC, id DESC);
    CREATE VIRTUAL TABLE entries_fts USING fts5(
        text,
        content = 'entries',
        content_rowid = 'id',
        tokenize = 'trigram'
    );
    CREATE TRIGGER entries_after_insert AFTER INSERT ON entries BEGIN
        INSERT INTO entries_fts (rowid, text) VALUES (new.id, new.text);
    END;
    CREATE TRIGGER entries_after_delete AFTER DELETE ON entries BEGIN
        INSERT INTO entries_fts (entries_fts, rowid, text) VALUES ('delete', old.id, old.text);
    END;",
    "ALTER TABLE entries ADD COLUMN model_id TEXT;",
];

const COLUMNS: &str = "id, text, created, kind, model_id";

impl History {
    pub fn open(path: &Path) -> Result<Self, HistoryError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        let history = Self { connection };
        history.migrate()?;
        Ok(history)
    }

    pub fn version(&self) -> Result<i64, HistoryError> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    fn migrate(&self) -> Result<(), HistoryError> {
        let found = self.version()?;
        if found > SCHEMA_VERSION {
            return Err(HistoryError::TooNew {
                found,
                supported: SCHEMA_VERSION,
            });
        }
        for (index, script) in MIGRATIONS.iter().enumerate() {
            let target = index as i64 + 1;
            if found >= target {
                continue;
            }
            let transaction = self.connection.unchecked_transaction()?;
            transaction.execute_batch(script)?;
            transaction.pragma_update(None, "user_version", target)?;
            transaction.commit()?;
        }
        Ok(())
    }

    pub fn add(
        &self,
        text: &str,
        created: i64,
        kind: Kind,
        model_id: Option<&str>,
    ) -> Result<i64, HistoryError> {
        self.connection.execute(
            "INSERT INTO entries (text, created, kind, model_id) VALUES (?1, ?2, ?3, ?4)",
            params![text, created, kind.as_str(), model_id],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn list(&self, offset: usize, limit: usize) -> Result<Vec<Entry>, HistoryError> {
        let mut statement = self.connection.prepare(&format!(
            "SELECT {COLUMNS} FROM entries ORDER BY created DESC, id DESC LIMIT ?1 OFFSET ?2"
        ))?;
        let rows = statement.query_map(params![limit as i64, offset as i64], entry)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn search(
        &self,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Entry>, HistoryError> {
        let query = query.trim();
        if query.is_empty() {
            return self.list(offset, limit);
        }
        if query.chars().count() < TRIGRAM_MIN_CHARS {
            let pattern = format!("%{}%", escape_like(query));
            let mut statement = self.connection.prepare(&format!(
                "SELECT {COLUMNS} FROM entries WHERE text LIKE ?1 ESCAPE '\\'
                 ORDER BY created DESC, id DESC LIMIT ?2 OFFSET ?3"
            ))?;
            let rows = statement.query_map(params![pattern, limit as i64, offset as i64], entry)?;
            return Ok(rows.collect::<Result<_, _>>()?);
        }
        let phrase = format!("\"{}\"", query.replace('"', "\"\""));
        let mut statement = self.connection.prepare(
            "SELECT e.id, e.text, e.created, e.kind, e.model_id
             FROM entries_fts f JOIN entries e ON e.id = f.rowid
             WHERE entries_fts MATCH ?1
             ORDER BY e.created DESC, e.id DESC LIMIT ?2 OFFSET ?3",
        )?;
        let rows = statement.query_map(params![phrase, limit as i64, offset as i64], entry)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn delete(&self, id: i64) -> Result<bool, HistoryError> {
        Ok(self
            .connection
            .execute("DELETE FROM entries WHERE id = ?1", params![id])?
            > 0)
    }

    pub fn clear(&self) -> Result<(), HistoryError> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM entries", [])?;
        transaction.execute(
            "INSERT INTO entries_fts (entries_fts) VALUES ('rebuild')",
            [],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn latest(&self) -> Result<Option<Entry>, HistoryError> {
        Ok(self
            .connection
            .query_row(
                &format!("SELECT {COLUMNS} FROM entries ORDER BY created DESC, id DESC LIMIT 1"),
                [],
                entry,
            )
            .optional()?)
    }
}

fn entry(row: &Row<'_>) -> rusqlite::Result<Entry> {
    Ok(Entry {
        id: row.get(0)?,
        text: row.get(1)?,
        created: row.get(2)?,
        kind: Kind::parse(&row.get::<_, String>(3)?),
        model_id: row.get(4)?,
    })
}

fn escape_like(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}
