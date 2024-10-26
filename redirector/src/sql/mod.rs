use crate::model::AppState;

/// Sets up the SQL database. Should only be called once
pub fn init_sql(app_state: AppState) -> anyhow::Result<()> {
    let conn = match app_state.conn.lock() {
        Ok(conn) => conn,
        Err(e) => anyhow::bail!(e.to_string()),
    };

    // Create: Set up the table
    conn.execute(
        "CREATE TABLE rulefiles (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            content TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}
