mod create;
mod grant;

use worker::{D1Database, D1PreparedStatement, wasm_bindgen::JsValue};

pub struct AdministrationStore {
    db: D1Database,
}

impl AdministrationStore {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }
}

fn failure(operation: &'static str, _error: worker::Error) -> anyhow::Error {
    anyhow::anyhow!("D1 administration {operation} failed (SDK details redacted)")
}

fn prepare(db: &D1Database, sql: &str, values: &[JsValue]) -> anyhow::Result<D1PreparedStatement> {
    db.prepare(sql)
        .bind(values)
        .map_err(|error| failure("bind", error))
}
