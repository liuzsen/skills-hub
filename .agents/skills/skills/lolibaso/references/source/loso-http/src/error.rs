pub use loso_macros::HttpErrorCode;

pub trait HttpErrorCode {
    type Discriminant;

    fn code(&self) -> &'static str;

    fn discriminant(&self) -> Self::Discriminant;

    fn http_status(&self) -> u16;
}
