use libsql::Connection;
use super::repository::VaultRow;

/// Column names for SELECT queries on memory_vault_entries.
/// Every query must use this exact column order so that `ColumnMap` indexing is stable.
pub const VAULT_COLUMNS: &[&str] = &[
    "id",
    "workspace_id",
    "source",
    "sensitivity",
    "created_at",
    "last_accessed",
    "access_count",
    "content_ciphertext",
    "content_nonce",
    "tags_ciphertext",
    "tags_nonce",
    "metadata_ciphertext",
    "metadata_nonce",
    "embedding",
    "embedding_model",
    "embedding_provider",
    "embedding_dim",
];

/// Maps named columns to positional indices for safe `row.get::<T>(idx)` access.
pub struct ColumnMap {
    names: Vec<String>,
}

impl ColumnMap {
    pub fn from_columns(cols: &[&str]) -> Self {
        Self {
            names: cols.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn index_of(&self, field: &str) -> Result<usize, String> {
        self.names
            .iter()
            .position(|n| n == field)
            .ok_or_else(|| format!("ColumnMap: unknown field '{}'", field))
    }

    pub fn get_string(&self, row: &libsql::Row, field: &str) -> Result<String, String> {
        let idx = self.index_of(field)?;
        row.get::<String>(idx as i32).map_err(|e| format!("ColumnMap get_string '{}': {}", field, e))
    }

    pub fn get_i64(&self, row: &libsql::Row, field: &str) -> Result<i64, String> {
        let idx = self.index_of(field)?;
        row.get::<i64>(idx as i32).map_err(|e| format!("ColumnMap get_i64 '{}': {}", field, e))
    }

    pub fn get_blob(&self, row: &libsql::Row, field: &str) -> Result<Vec<u8>, String> {
        let idx = self.index_of(field)?;
        row.get::<Vec<u8>>(idx as i32).map_err(|e| format!("ColumnMap get_blob '{}': {}", field, e))
    }

    pub fn get_opt_blob(&self, row: &libsql::Row, field: &str) -> Option<Vec<u8>> {
        let idx = self.index_of(field).ok()?;
        row.get::<Option<Vec<u8>>>(idx as i32).unwrap_or(None)
    }

    pub fn get_opt_string(&self, row: &libsql::Row, field: &str) -> Option<String> {
        let idx = self.index_of(field).ok()?;
        row.get::<Option<String>>(idx as i32).unwrap_or(None)
    }

    pub fn get_opt_i64(&self, row: &libsql::Row, field: &str) -> Option<i64> {
        let idx = self.index_of(field).ok()?;
        row.get::<Option<i64>>(idx as i32).unwrap_or(None)
    }

    /// Convert a libsql Row into a VaultRow using named field access.
    pub fn to_vault_row(&self, row: &libsql::Row) -> Result<VaultRow, String> {
        Ok(VaultRow {
            id: self.get_string(row, "id")?,
            workspace_id: self.get_string(row, "workspace_id")?,
            source: self.get_string(row, "source")?,
            sensitivity: self.get_string(row, "sensitivity")?,
            created_at: self.get_i64(row, "created_at")?,
            last_accessed: self.get_i64(row, "last_accessed")?,
            access_count: self.get_i64(row, "access_count")?,
            content_ciphertext: self.get_blob(row, "content_ciphertext")?,
            content_nonce: self.get_blob(row, "content_nonce")?,
            tags_ciphertext: self.get_blob(row, "tags_ciphertext")?,
            tags_nonce: self.get_blob(row, "tags_nonce")?,
            metadata_ciphertext: self.get_opt_blob(row, "metadata_ciphertext"),
            metadata_nonce: self.get_opt_blob(row, "metadata_nonce"),
            embedding: self.get_opt_blob(row, "embedding"),
            embedding_model: self.get_opt_string(row, "embedding_model"),
            embedding_provider: self.get_opt_string(row, "embedding_provider"),
            embedding_dim: self.get_opt_i64(row, "embedding_dim").map(|v| v as usize),
        })
    }
}

/// Lazy singleton for the standard vault column map.
pub fn vault_column_map() -> ColumnMap {
    ColumnMap::from_columns(VAULT_COLUMNS)
}

/// Returns the SELECT column list as a comma-separated string.
pub fn vault_select_columns() -> String {
    VAULT_COLUMNS.join(", ")
}

#[derive(Debug, Clone, Copy)]
pub enum OrderBy {
    CreatedDesc,
    LastAccessedDesc,
    AccessCountDesc,
}

impl OrderBy {
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::CreatedDesc => "created_at DESC",
            Self::LastAccessedDesc => "last_accessed DESC",
            Self::AccessCountDesc => "access_count DESC",
        }
    }

    pub fn from_str_loose(s: &str) -> Self {
        match s {
            "last_accessed" => Self::LastAccessedDesc,
            "access_count" => Self::AccessCountDesc,
            _ => Self::CreatedDesc,
        }
    }
}

/// Typed query builder for filtered, paginated queries on memory_vault_entries.
pub struct VaultQuery {
    pub workspace_id: Option<String>,
    pub sensitivity: Option<String>,
    pub source_prefix: Option<String>,
    pub created_after: Option<i64>,
    pub created_before: Option<i64>,
    pub order_by: OrderBy,
    pub limit: usize,
    pub offset: usize,
}

impl Default for VaultQuery {
    fn default() -> Self {
        Self {
            workspace_id: None,
            sensitivity: None,
            source_prefix: None,
            created_after: None,
            created_before: None,
            order_by: OrderBy::CreatedDesc,
            limit: 20,
            offset: 0,
        }
    }
}

impl VaultQuery {
    /// Build WHERE clause fragments and corresponding parameter values.
    /// Returns (where_clause, params) — where_clause includes leading "WHERE" if non-empty.
    fn build_where(&self) -> (String, Vec<libsql::Value>) {
        let mut clauses = Vec::new();
        let mut values: Vec<libsql::Value> = Vec::new();

        if let Some(ref ws) = self.workspace_id {
            values.push(ws.clone().into());
            clauses.push(format!("workspace_id = ?{}", values.len()));
        }
        if let Some(ref s) = self.sensitivity {
            values.push(s.clone().into());
            clauses.push(format!("sensitivity = ?{}", values.len()));
        }
        if let Some(ref sp) = self.source_prefix {
            values.push(format!("{}%", sp).into());
            clauses.push(format!("source LIKE ?{}", values.len()));
        }
        if let Some(after) = self.created_after {
            values.push(after.into());
            clauses.push(format!("created_at >= ?{}", values.len()));
        }
        if let Some(before) = self.created_before {
            values.push(before.into());
            clauses.push(format!("created_at <= ?{}", values.len()));
        }

        if clauses.is_empty() {
            (String::new(), values)
        } else {
            (format!("WHERE {}", clauses.join(" AND ")), values)
        }
    }

    /// Execute the query and return matching VaultRows.
    pub async fn execute(&self, conn: &Connection) -> Result<Vec<VaultRow>, String> {
        let cols = vault_select_columns();
        let (where_clause, mut values) = self.build_where();

        let sql = format!(
            "SELECT {} FROM memory_vault_entries {} ORDER BY {} LIMIT ?{} OFFSET ?{}",
            cols,
            where_clause,
            self.order_by.as_sql(),
            values.len() + 1,
            values.len() + 2,
        );

        values.push((self.limit as i64).into());
        values.push((self.offset as i64).into());

        let mut rows = conn
            .query(&sql, libsql::params::Params::Positional(values))
            .await
            .map_err(|e| format!("VaultQuery execute: {}", e))?;

        let cmap = vault_column_map();
        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            results.push(cmap.to_vault_row(&row)?);
        }
        Ok(results)
    }

    /// Count matching rows (ignores limit/offset).
    pub async fn count(&self, conn: &Connection) -> Result<usize, String> {
        let (where_clause, values) = self.build_where();

        let sql = format!(
            "SELECT COUNT(*) FROM memory_vault_entries {}",
            where_clause,
        );

        let mut rows = conn
            .query(&sql, libsql::params::Params::Positional(values))
            .await
            .map_err(|e| format!("VaultQuery count: {}", e))?;

        if let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
            let count: i64 = row.get(0).unwrap_or(0);
            Ok(count as usize)
        } else {
            Ok(0)
        }
    }
}
