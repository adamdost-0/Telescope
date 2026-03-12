//! SQLite-backed resource store for Kubernetes objects.
//!
//! Uses a document-store pattern: K8s resources are stored as JSON blobs
//! indexed by GVK (group/version/kind), namespace, and name.
//! This avoids schema migrations when CRDs change.

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

/// A stored Kubernetes resource entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceEntry {
    /// Group/Version/Kind identifier (e.g., "apps/v1/Deployment").
    pub gvk: String,
    /// Kubernetes namespace (empty string for cluster-scoped resources).
    pub namespace: String,
    /// Resource name.
    pub name: String,
    /// Kubernetes resourceVersion for watch consistency.
    pub resource_version: String,
    /// Full JSON representation of the resource.
    pub content: String,
    /// ISO 8601 timestamp of last update.
    pub updated_at: String,
}

/// SQLite-backed store for Kubernetes resources.
pub struct ResourceStore {
    conn: Connection,
}

impl ResourceStore {
    /// Open or create a resource store at the given path.
    /// Use ":memory:" for an in-memory database (testing).
    pub fn open(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init()?;
        store.migrate()?;
        Ok(store)
    }

    /// Initialize the database schema.
    fn init(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS resources (
                gvk              TEXT NOT NULL,
                namespace        TEXT NOT NULL DEFAULT '',
                name             TEXT NOT NULL,
                resource_version TEXT NOT NULL DEFAULT '',
                content          TEXT NOT NULL,
                updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                PRIMARY KEY (gvk, namespace, name)
            );

            CREATE INDEX IF NOT EXISTS idx_resources_gvk_ns
                ON resources (gvk, namespace);

            CREATE TABLE IF NOT EXISTS user_preferences (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY
            );
        ",
        )
    }

    /// Returns the current schema version, or 0 if not yet set.
    fn get_schema_version(&self) -> SqlResult<u32> {
        let mut stmt = self
            .conn
            .prepare("SELECT COALESCE(MAX(version), 0) FROM schema_version")?;
        stmt.query_row([], |row| row.get(0))
    }

    /// Record the schema version after a migration step.
    fn set_schema_version(&self, version: u32) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
            params![version],
        )?;
        Ok(())
    }

    /// Run sequential schema migrations.
    fn migrate(&self) -> SqlResult<()> {
        let current = self.get_schema_version()?;

        if current < 1 {
            // Version 1 is the initial schema created in init().
            self.set_schema_version(1)?;
        }

        // Future migrations:
        // if current < 2 { ... self.set_schema_version(2)?; }

        Ok(())
    }

    /// Insert or update a resource.
    pub fn upsert(&self, entry: &ResourceEntry) -> SqlResult<()> {
        self.conn.execute(
            "INSERT INTO resources (gvk, namespace, name, resource_version, content, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (gvk, namespace, name) DO UPDATE SET
                resource_version = excluded.resource_version,
                content = excluded.content,
                updated_at = excluded.updated_at",
            params![
                entry.gvk,
                entry.namespace,
                entry.name,
                entry.resource_version,
                entry.content,
                entry.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Delete a resource by GVK, namespace, and name.
    pub fn delete(&self, gvk: &str, namespace: &str, name: &str) -> SqlResult<bool> {
        let rows = self.conn.execute(
            "DELETE FROM resources WHERE gvk = ?1 AND namespace = ?2 AND name = ?3",
            params![gvk, namespace, name],
        )?;
        Ok(rows > 0)
    }

    /// Delete all resources of a given GVK (used on full resync).
    pub fn delete_all_by_gvk(&self, gvk: &str) -> SqlResult<usize> {
        self.conn
            .execute("DELETE FROM resources WHERE gvk = ?1", params![gvk])
    }

    /// List resources by GVK and optional namespace.
    /// If namespace is `None`, returns all namespaces.
    pub fn list(&self, gvk: &str, namespace: Option<&str>) -> SqlResult<Vec<ResourceEntry>> {
        match namespace {
            Some(ns) => {
                let mut stmt = self.conn.prepare(
                    "SELECT gvk, namespace, name, resource_version, content, updated_at
                     FROM resources WHERE gvk = ?1 AND namespace = ?2
                     ORDER BY name",
                )?;
                let rows = stmt
                    .query_map(params![gvk, ns], Self::row_to_entry)?
                    .collect::<SqlResult<Vec<_>>>()?;
                Ok(rows)
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT gvk, namespace, name, resource_version, content, updated_at
                     FROM resources WHERE gvk = ?1
                     ORDER BY namespace, name",
                )?;
                let rows = stmt
                    .query_map(params![gvk], Self::row_to_entry)?
                    .collect::<SqlResult<Vec<_>>>()?;
                Ok(rows)
            }
        }
    }

    /// Get a single resource by GVK, namespace, and name.
    pub fn get(&self, gvk: &str, namespace: &str, name: &str) -> SqlResult<Option<ResourceEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT gvk, namespace, name, resource_version, content, updated_at
             FROM resources WHERE gvk = ?1 AND namespace = ?2 AND name = ?3",
        )?;
        let mut rows = stmt.query_map(params![gvk, namespace, name], Self::row_to_entry)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Count resources by GVK and optional namespace.
    pub fn count(&self, gvk: &str, namespace: Option<&str>) -> SqlResult<u64> {
        match namespace {
            Some(ns) => self.conn.query_row(
                "SELECT COUNT(*) FROM resources WHERE gvk = ?1 AND namespace = ?2",
                params![gvk, ns],
                |row| row.get(0),
            ),
            None => self.conn.query_row(
                "SELECT COUNT(*) FROM resources WHERE gvk = ?1",
                params![gvk],
                |row| row.get(0),
            ),
        }
    }

    /// Get a user preference by key.
    pub fn get_preference(&self, key: &str) -> SqlResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM user_preferences WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get(0))?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Set a user preference (insert or replace).
    pub fn set_preference(&self, key: &str, value: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO user_preferences (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Delete a user preference by key. Returns true if a row was removed.
    pub fn delete_preference(&self, key: &str) -> SqlResult<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM user_preferences WHERE key = ?1", params![key])?;
        Ok(rows > 0)
    }

    fn row_to_entry(row: &rusqlite::Row<'_>) -> SqlResult<ResourceEntry> {
        Ok(ResourceEntry {
            gvk: row.get(0)?,
            namespace: row.get(1)?,
            name: row.get(2)?,
            resource_version: row.get(3)?,
            content: row.get(4)?,
            updated_at: row.get(5)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(gvk: &str, ns: &str, name: &str) -> ResourceEntry {
        ResourceEntry {
            gvk: gvk.to_string(),
            namespace: ns.to_string(),
            name: name.to_string(),
            resource_version: "1".to_string(),
            content: format!(r#"{{"kind":"{}","name":"{}"}}"#, gvk, name),
            updated_at: "2025-01-01T00:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn open_in_memory() {
        let store = ResourceStore::open(":memory:").expect("open");
        assert_eq!(store.count("any/v1/Kind", None).unwrap(), 0);
    }

    #[test]
    fn upsert_and_get() {
        let store = ResourceStore::open(":memory:").unwrap();
        let entry = make_entry("apps/v1/Deployment", "default", "nginx");
        store.upsert(&entry).unwrap();

        let got = store
            .get("apps/v1/Deployment", "default", "nginx")
            .unwrap()
            .expect("should exist");
        assert_eq!(got.gvk, entry.gvk);
        assert_eq!(got.name, entry.name);
        assert_eq!(got.content, entry.content);
    }

    #[test]
    fn upsert_updates_existing() {
        let store = ResourceStore::open(":memory:").unwrap();
        let mut entry = make_entry("apps/v1/Deployment", "default", "nginx");
        store.upsert(&entry).unwrap();

        entry.resource_version = "2".to_string();
        entry.content = r#"{"updated":true}"#.to_string();
        entry.updated_at = "2025-06-01T00:00:00.000Z".to_string();
        store.upsert(&entry).unwrap();

        let got = store
            .get("apps/v1/Deployment", "default", "nginx")
            .unwrap()
            .unwrap();
        assert_eq!(got.resource_version, "2");
        assert_eq!(got.content, r#"{"updated":true}"#);
        assert_eq!(got.updated_at, "2025-06-01T00:00:00.000Z");
        // Only one entry should exist
        assert_eq!(store.count("apps/v1/Deployment", None).unwrap(), 1);
    }

    #[test]
    fn delete_existing_returns_true() {
        let store = ResourceStore::open(":memory:").unwrap();
        store
            .upsert(&make_entry("v1/Pod", "default", "pod-1"))
            .unwrap();
        assert!(store.delete("v1/Pod", "default", "pod-1").unwrap());
        assert!(store.get("v1/Pod", "default", "pod-1").unwrap().is_none());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let store = ResourceStore::open(":memory:").unwrap();
        assert!(!store.delete("v1/Pod", "default", "no-such").unwrap());
    }

    #[test]
    fn delete_all_by_gvk() {
        let store = ResourceStore::open(":memory:").unwrap();
        store
            .upsert(&make_entry("v1/Pod", "ns-a", "pod-1"))
            .unwrap();
        store
            .upsert(&make_entry("v1/Pod", "ns-b", "pod-2"))
            .unwrap();
        store
            .upsert(&make_entry("apps/v1/Deployment", "ns-a", "deploy-1"))
            .unwrap();

        let deleted = store.delete_all_by_gvk("v1/Pod").unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(store.count("v1/Pod", None).unwrap(), 0);
        // Other GVKs untouched
        assert_eq!(store.count("apps/v1/Deployment", None).unwrap(), 1);
    }

    #[test]
    fn list_with_namespace_filter() {
        let store = ResourceStore::open(":memory:").unwrap();
        store
            .upsert(&make_entry("v1/Pod", "ns-a", "pod-1"))
            .unwrap();
        store
            .upsert(&make_entry("v1/Pod", "ns-a", "pod-2"))
            .unwrap();
        store
            .upsert(&make_entry("v1/Pod", "ns-b", "pod-3"))
            .unwrap();

        let ns_a = store.list("v1/Pod", Some("ns-a")).unwrap();
        assert_eq!(ns_a.len(), 2);
        assert!(ns_a.iter().all(|e| e.namespace == "ns-a"));
    }

    #[test]
    fn list_without_namespace_returns_all() {
        let store = ResourceStore::open(":memory:").unwrap();
        store
            .upsert(&make_entry("v1/Pod", "ns-a", "pod-1"))
            .unwrap();
        store
            .upsert(&make_entry("v1/Pod", "ns-b", "pod-2"))
            .unwrap();

        let all = store.list("v1/Pod", None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn count_with_and_without_namespace() {
        let store = ResourceStore::open(":memory:").unwrap();
        store
            .upsert(&make_entry("v1/Pod", "ns-a", "pod-1"))
            .unwrap();
        store
            .upsert(&make_entry("v1/Pod", "ns-a", "pod-2"))
            .unwrap();
        store
            .upsert(&make_entry("v1/Pod", "ns-b", "pod-3"))
            .unwrap();

        assert_eq!(store.count("v1/Pod", Some("ns-a")).unwrap(), 2);
        assert_eq!(store.count("v1/Pod", Some("ns-b")).unwrap(), 1);
        assert_eq!(store.count("v1/Pod", None).unwrap(), 3);
    }

    #[test]
    fn multiple_gvks_independent() {
        let store = ResourceStore::open(":memory:").unwrap();
        store
            .upsert(&make_entry("v1/Pod", "default", "pod-1"))
            .unwrap();
        store
            .upsert(&make_entry("v1/Service", "default", "svc-1"))
            .unwrap();

        assert_eq!(store.count("v1/Pod", None).unwrap(), 1);
        assert_eq!(store.count("v1/Service", None).unwrap(), 1);
        assert!(store.get("v1/Pod", "default", "svc-1").unwrap().is_none());
    }

    #[test]
    fn cluster_scoped_empty_namespace() {
        let store = ResourceStore::open(":memory:").unwrap();
        let entry = make_entry("v1/Namespace", "", "kube-system");
        store.upsert(&entry).unwrap();

        let got = store
            .get("v1/Namespace", "", "kube-system")
            .unwrap()
            .unwrap();
        assert_eq!(got.namespace, "");
        assert_eq!(got.name, "kube-system");
    }

    #[test]
    fn large_content_roundtrip() {
        let store = ResourceStore::open(":memory:").unwrap();
        // ~10KB JSON content
        let large_json = format!(r#"{{"data":"{}"}}"#, "x".repeat(10_000));
        let entry = ResourceEntry {
            gvk: "v1/ConfigMap".to_string(),
            namespace: "default".to_string(),
            name: "big-config".to_string(),
            resource_version: "99".to_string(),
            content: large_json.clone(),
            updated_at: "2025-01-01T00:00:00.000Z".to_string(),
        };
        store.upsert(&entry).unwrap();

        let got = store
            .get("v1/ConfigMap", "default", "big-config")
            .unwrap()
            .unwrap();
        assert_eq!(got.content, large_json);
        assert!(got.content.len() > 10_000);
    }

    // ── Preference tests ─────────────────────────────────────────────

    #[test]
    fn preference_set_and_get() {
        let store = ResourceStore::open(":memory:").unwrap();
        store.set_preference("theme", "dark").unwrap();
        assert_eq!(
            store.get_preference("theme").unwrap(),
            Some("dark".to_string())
        );
    }

    #[test]
    fn preference_missing_returns_none() {
        let store = ResourceStore::open(":memory:").unwrap();
        assert_eq!(store.get_preference("nonexistent").unwrap(), None);
    }

    #[test]
    fn preference_upsert_replaces() {
        let store = ResourceStore::open(":memory:").unwrap();
        store.set_preference("theme", "dark").unwrap();
        store.set_preference("theme", "light").unwrap();
        assert_eq!(
            store.get_preference("theme").unwrap(),
            Some("light".to_string())
        );
    }

    #[test]
    fn preference_delete() {
        let store = ResourceStore::open(":memory:").unwrap();
        store.set_preference("theme", "dark").unwrap();
        assert!(store.delete_preference("theme").unwrap());
        assert_eq!(store.get_preference("theme").unwrap(), None);
        assert!(!store.delete_preference("theme").unwrap());
    }

    // ── Schema migration tests ───────────────────────────────────────

    #[test]
    fn schema_version_set_on_open() {
        let store = ResourceStore::open(":memory:").unwrap();
        assert_eq!(store.get_schema_version().unwrap(), 1);
    }

    #[test]
    fn reopen_does_not_change_version() {
        // Simulate re-open by calling migrate again on the same connection
        let store = ResourceStore::open(":memory:").unwrap();
        assert_eq!(store.get_schema_version().unwrap(), 1);
        store.migrate().unwrap();
        assert_eq!(store.get_schema_version().unwrap(), 1);
    }

    #[test]
    fn schema_version_table_exists() {
        let store = ResourceStore::open(":memory:").unwrap();
        let count: u32 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
