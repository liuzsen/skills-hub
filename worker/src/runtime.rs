use hub_application::{time::Clock, tokens::TokenSource};
use worker::{js_sys, wasm_bindgen::JsCast};

pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> i64 {
        (js_sys::Date::now() / 1000.0).floor() as i64
    }
}

pub struct CryptoTokens;
impl TokenSource for CryptoTokens {
    fn fill(&self, bytes: &mut [u8]) -> anyhow::Result<()> {
        let scope: web_sys::WorkerGlobalScope = js_sys::global().unchecked_into();
        let crypto = scope
            .crypto()
            .map_err(|_| anyhow::anyhow!("Worker crypto unavailable"))?;
        crypto
            .get_random_values_with_u8_array(bytes)
            .map_err(|_| anyhow::anyhow!("cryptographic randomness failed"))?;
        Ok(())
    }
}
