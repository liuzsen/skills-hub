mod access;
mod bind;
mod claim;

use worker::{D1Database, D1PreparedStatement, wasm_bindgen::JsValue};

pub struct RedemptionStore {
    db: D1Database,
}

impl RedemptionStore {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }
}

fn failure(operation: &'static str, _error: worker::Error) -> anyhow::Error {
    anyhow::anyhow!("D1 redemption {operation} failed (SDK details redacted)")
}

fn prepare(db: &D1Database, sql: &str, values: &[JsValue]) -> anyhow::Result<D1PreparedStatement> {
    db.prepare(sql)
        .bind(values)
        .map_err(|error| failure("bind", error))
}

fn integer(value: i64) -> anyhow::Result<JsValue> {
    anyhow::ensure!(
        (0..=9_007_199_254_740_991).contains(&value),
        "D1 integer out of safe range"
    );
    Ok(JsValue::from_f64(value as f64))
}
