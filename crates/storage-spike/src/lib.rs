#![forbid(unsafe_code)]

use rusqlite::Connection;

/// Opens an in-memory SQLite connection for the isolated M1A storage spike.
///
/// This is deliberately not a public persistence contract for DXBOT.
pub fn open_reference_connection() -> rusqlite::Result<Connection> {
    Connection::open_in_memory()
}

#[cfg(test)]
mod tests {
    use super::open_reference_connection;

    #[test]
    fn reference_connection_is_available() -> rusqlite::Result<()> {
        let connection = open_reference_connection()?;
        let value: i64 = connection.query_row("SELECT 1", [], |row| row.get(0))?;
        assert_eq!(value, 1);
        Ok(())
    }
}
