use rusqlite::{Connection, Result, params};
use std::sync::{Arc, Mutex};


// sample struct for now. will change later
#[derive(Debug)]
struct User {
    id: i32,
    name: String,
    age: i32,
}

fn main() -> Result<()> {
    // Open a connection to an in-memory SQLite database
    let conn_init = Connection::open_in_memory()?;
    let db_conn = Arc::new(Mutex::new(conn));

    let conn = Arc::clone(&db_conn);

    // Create: Set up the table
    conn.execute(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            age INTEGER
        )",
        [],
    )?;

    // Create: Insert
    let user = User {
        id: 1,
        name: "Somrishi".to_string(),
        age: 30,
    };
    conn.execute(
        "INSERT INTO users (id, name, age) VALUES (?1, ?2, ?3)",
        params![user.id, user.name, user.age],
    )?;

    // Read: Query
    let mut stmt = conn.prepare("SELECT id, name, age FROM users WHERE id = ?1")?;
    let user = stmt.query_row(params![1], |row| {
        Ok(User {
            id: row.get(0)?,
            name: row.get(1)?,
            age: row.get(2)?,
        })
    })?;
    println!("Read user: {:?}", user);

    // Update
    conn.execute(
        "UPDATE users SET age = ?1 WHERE id = ?2",
        params![31, 1],
    )?;

    // Read again to verify the update
    let updated_user = stmt.query_row(params![1], |row| {
        Ok(User {
            id: row.get(0)?,
            name: row.get(1)?,
            age: row.get(2)?,
        })
    })?;
    println!("Updated user: {:?}", updated_user);

    // Delete: Remove
    conn.execute("DELETE FROM users WHERE id = ?1", params![1])?;

    // Attempt to read the deleted user (this should fail)
    // user doesn't exist testing
    match stmt.query_row(params![1], |row| {
        Ok(User {
            id: row.get(0)?,
            name: row.get(1)?,
            age: row.get(2)?,
        })
    }) {
        Ok(_) => println!("User still exists (unexpected)"),
        Err(e) => println!("User successfully deleted: {}", e),
    }

    Ok(())
}