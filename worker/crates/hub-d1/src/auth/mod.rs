mod consume;
mod issue;
mod session;

use worker::{D1Database, D1PreparedStatement, D1Result, wasm_bindgen::JsValue};

pub struct AuthStore {
    pub(crate) db: D1Database,
}

impl AuthStore {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }
}

// SDK errors can include SQL and bound values. Only operation context crosses this boundary.
fn failure(operation: &'static str, _error: worker::Error) -> anyhow::Error {
    anyhow::anyhow!("D1 {operation} failed (SDK details redacted)")
}

fn prepare(db: &D1Database, sql: &str, values: &[JsValue]) -> anyhow::Result<D1PreparedStatement> {
    db.prepare(sql)
        .bind(values)
        .map_err(|error| failure("bind", error))
}

async fn batch(
    db: &D1Database,
    statements: Vec<D1PreparedStatement>,
) -> anyhow::Result<Vec<D1Result>> {
    let results = db
        .batch(statements)
        .await
        .map_err(|error| failure("atomic batch", error))?;
    anyhow::ensure!(
        results.iter().all(D1Result::success),
        "D1 batch reported failure"
    );
    Ok(results)
}

fn integer(value: i64) -> anyhow::Result<JsValue> {
    anyhow::ensure!(
        (0..=9_007_199_254_740_991).contains(&value),
        "D1 integer out of safe range"
    );
    Ok(JsValue::from_f64(value as f64))
}

#[derive(serde::Deserialize)]
struct Receipt {
    receipt: String,
}

fn receipt(results: &[D1Result], expected: &str) -> anyhow::Result<bool> {
    let rows = results
        .last()
        .ok_or_else(|| anyhow::anyhow!("D1 batch missing receipt"))?
        .results::<Receipt>()
        .map_err(|error| failure("receipt decode", error))?;
    anyhow::ensure!(rows.len() <= 1, "D1 returned duplicate receipts");
    Ok(rows.first().is_some_and(|row| row.receipt == expected))
}
