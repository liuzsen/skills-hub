/// Unix time in whole seconds. Implementations must use server time, never request input.
pub trait Clock: Send + Sync {
    fn now(&self) -> i64;
}
