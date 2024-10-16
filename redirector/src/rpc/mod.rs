use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use std::process::id;
use futures::{future, StreamExt};
use rusqlite::params;
use tarpc::{context, server, server::Channel};
use tarpc::server::incoming::Incoming;
use tarpc::tokio_serde::formats::Json;
use tracing::{event, Level};
use crate::model::{AppState, RuleFile};
use crate::sql::init_sql;
use crate::error::{Error, Result};

/// Constant value for where the RPC server binds to
const RPC_BIND: (Ipv4Addr, u16) = (Ipv4Addr::LOCALHOST, 50050);

#[tarpc::service]
trait RuleSvc {
    async fn create(name: String, content: String) -> Result<i64>;
    async fn request(id: i64) -> Result<RuleFile>;
    async fn update(id: i64, content: String) -> Result<()>;
    async fn delete(id: i64) -> Result<()>;
}

#[derive(Clone)]
struct Server {
    addr: SocketAddr,
    app_state: AppState
}

impl RuleSvc for Server {
    async fn create(self, _: context::Context, name: String, content: String) -> Result<i64> {
        let conn = match self.app_state.conn.lock() {
            Ok(conn) => conn,
            Err(e) => return Err(Error::Anyhow(format!("Failed to obtain lock on app state: {}", e)))
        };

        match conn.execute(
            "INSERT INTO rulefiles (name, content) VALUES (?1, ?2)",
            params![name, content],
        ) {
            Ok(_) => Ok(conn.last_insert_rowid()),
            Err(e) => Err(Error::Anyhow(format!("Failed to insert rulefile: {}", e)))
        }
    }
    async fn request(self, _: context::Context, id: i64) -> Result<RuleFile> {
        let conn = match self.app_state.conn.lock() {
            Ok(conn) => conn,
            Err(e) => return Err(Error::Anyhow(format!("Failed to obtain lock on app state: {}", e)))
        };

        let mut stmt = conn.prepare("SELECT id, name, content FROM rulefiles WHERE id = ?1");
        if stmt.is_err() {
            return Err(Error::Anyhow(format!("Failed to prepare statement: {}", stmt.err().unwrap())));
        }
        let mut stmt = stmt.unwrap();

        match stmt.query_row(params![id], |row| {
            Ok(RuleFile {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
            })
        }) {
            Ok(r) => Ok(r),
            Err(e) => Err(Error::Anyhow(format!("Failed to query rulefile: {}", e)))
        }
    }
    async fn update(self, _: context::Context, id: i64, content: String) -> Result<()> {
        let conn = match self.app_state.conn.lock() {
            Ok(conn) => conn,
            Err(e) => return Err(Error::Anyhow(format!("Failed to obtain lock on app state: {}", e)))
        };

        if conn.execute(
            "INSERT INTO rulefiles (content) VALUES (?1)",
            params![content],
        ).is_err() {
            return Err(Error::Anyhow(format!("Failed to insert rulefile: {}", content)))
        }
        Ok(())
    }
    async fn delete(self, _: context::Context, id: i64) -> Result<()> {
        let conn = match self.app_state.conn.lock() {
            Ok(conn) => conn,
            Err(e) => return Err(Error::Anyhow(format!("Failed to obtain lock on app state: {}", e)))
        };

        match conn.execute("DELETE FROM rulefiles WHERE id = ?1", params![id]) {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::Anyhow(format!("Failed to delete rulefile: {}", e)))
        }
    }
}

async fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
}

/// Start the RPC server
pub async fn init_rpc(app_state: AppState) -> anyhow::Result<()> {
    let mut listener = tarpc::serde_transport::tcp::listen(&RPC_BIND, Json::default).await.expect("Failed to bind RPC listener");

    event!(Level::INFO, "RPC listening on {}:{}", RPC_BIND.0, RPC_BIND.1);

    init_sql(app_state.clone())?;

    event!(Level::INFO, "Initialized SQL db");

    listener.config_mut().max_frame_length(usize::MAX);
    listener
        .filter_map(|r| future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        .max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        .map(|channel| {
            let server = Server {
                addr: channel.transport().peer_addr().unwrap(),
                app_state: app_state.clone()
            };
            channel.execute(server.serve()).for_each(spawn)
        })
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use rusqlite::Connection;
    use super::*;

    #[test]
    pub fn test_create() -> anyhow::Result<()> {
        let state = AppState { conn: Arc::new(Mutex::new(Connection::open_in_memory()?)) };
        init_sql(state.clone())?;

        let conn = match state.conn.lock() {
            Ok(conn) => conn,
            Err(e) => anyhow::bail!("Failed to obtain lock on app state: {}", e)
        };

        let name = "TestRule";
        let content = "TestContent";

        conn.execute(
            "INSERT INTO rulefiles (name, content) VALUES (?1, ?2)",
            params![name, content],
        )?;

        let id = conn.last_insert_rowid();
        assert_eq!(id, 1);

        let mut stmt = conn.prepare("SELECT id, name, content FROM rulefiles WHERE id = ?1")?;
        let file = stmt.query_row(params![1], |row| {
            Ok(RuleFile {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
            })
        })?;

        assert_eq!(file.id, id);
        assert_eq!(file.name, name);
        assert_eq!(file.content, content);

        Ok(())
    }
}