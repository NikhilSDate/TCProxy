use tarpc::server::incoming::Incoming;
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



/*
pub fn test_sql() -> anyhow::Result<()> {
    // Open a connection to an in-memory SQLite database
    let conn = Connection::open_in_memory()?;

    // Create: Set up the table
    conn.execute(
        "CREATE TABLE rulefiles (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            content TEXT NOT NULL
        )",
        [],
    )?;

    // Create: Insert
    let file = RuleFile {
        id: 1,
        name: "TestFile".to_string(),
        content: "rulefilecontent".to_string(),
    };
    conn.execute(
        "INSERT INTO rulefiles (id, name, content) VALUES (?1, ?2, ?3)",
        params![file.id, file.name, file.content],
    )?;

    // Read: Query
    let mut stmt = conn.prepare("SELECT id, name, content FROM rulefiles WHERE id = ?1")?;
    let file = stmt.query_row(params![1], |row| {
        Ok(RuleFile {
            id: row.get(0)?,
            name: row.get(1)?,
            content: row.get(2)?,
        })
    })?;
    println!("Read rule file: {:?}", file);

    // Update
    conn.execute(
        "UPDATE rulefiles SET content = ?1 WHERE id = ?2",
        params![31, 1],
    )?;

    // Read again to verify the update
    let updated_file = stmt.query_row(params![1], |row| {
        Ok(RuleFile {
            id: row.get(0)?,
            name: row.get(1)?,
            content: row.get(2)?,
        })
    })?;
    println!("Updated rule file: {:?}", updated_file);

    // Delete: Remove
    conn.execute("DELETE FROM rulefiles WHERE id = ?1", params![1])?;

    // Attempt to read the deleted user (this should fail)
    // user doesn't exist testing
    match stmt.query_row(params![1], |row| {
        Ok(RuleFile {
            id: row.get(0)?,
            name: row.get(1)?,
            content: row.get(2)?,
        })
    }) {
        Ok(_) => println!("RuleFile still exists (unexpected)"),
        Err(e) => println!("RuleFile successfully deleted: {}", e),
    }

    Ok(())
}
*/