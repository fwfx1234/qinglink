use anyhow::{Context, Result};
use rusqlite::{Connection, Transaction};

pub const CURRENT_SCHEMA_VERSION: i64 = 2;

pub fn migrate(connection: &mut Connection) -> Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL DEFAULT (unixepoch())
        );",
    )?;
    let current = connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get::<_, i64>(0),
        )
        .context("cannot read QingLink schema version")?;
    for version in (current + 1)..=CURRENT_SCHEMA_VERSION {
        let transaction = connection.transaction()?;
        apply(&transaction, version)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version) VALUES (?1)",
            [version],
        )?;
        transaction.commit()?;
    }
    Ok(())
}

fn apply(transaction: &Transaction<'_>, version: i64) -> Result<()> {
    match version {
        1 => transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT (unixepoch())
            );
            CREATE TABLE IF NOT EXISTS paired_devices (
                device_id TEXT PRIMARY KEY,
                device_name TEXT NOT NULL,
                token_prefix TEXT NOT NULL,
                paired_at INTEGER NOT NULL,
                last_seen_at INTEGER
            );",
        )?,
        2 => transaction.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_paired_devices_last_seen
             ON paired_devices(last_seen_at DESC);",
        )?,
        unknown => anyhow::bail!("unknown QingLink migration version {unknown}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_versioned_and_idempotent() {
        let mut connection = Connection::open_in_memory().expect("database");
        migrate(&mut connection).expect("first migration");
        migrate(&mut connection).expect("second migration");
        let version: i64 = connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("schema version");
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration count");
        assert_eq!(count, CURRENT_SCHEMA_VERSION);
    }
}
