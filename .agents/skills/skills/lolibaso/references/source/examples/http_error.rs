//! HttpErrorCode 完整用法示例。
//! 运行：cargo run --example http_error --features http
use lolibaso::http::error::HttpErrorCode;

// 必须是非空、无泛型的 enum，并显式指定整数 repr。
#[derive(HttpErrorCode)]
#[repr(i32)]
#[http(status = 500, rename_all = "SCREAMING_SNAKE_CASE")]
enum ApiError {
    Internal = 1000,
    #[http(status = 404)]
    UserNotFound {
        user_id: String,
    },
    #[http(code = "BAD_NAME", status = 422)]
    InvalidName(String),
}

// 未指定时：code 使用变体原名，status 为 400。
#[derive(HttpErrorCode)]
#[repr(u8)]
enum RequestError {
    InvalidRequest,
}

fn main() {
    for (error, expected) in [
        (ApiError::Internal, ("INTERNAL", 1000, 500)),
        (
            ApiError::UserNotFound {
                user_id: "42".into(),
            },
            ("USER_NOT_FOUND", 1001, 404),
        ),
        (ApiError::InvalidName(" ".into()), ("BAD_NAME", 1002, 422)),
    ] {
        let metadata: (&'static str, i32, u16) =
            (error.code(), error.discriminant(), error.http_status());
        assert_eq!(metadata, expected);
    }
    assert_eq!(RequestError::InvalidRequest.code(), "InvalidRequest");
    assert_eq!(RequestError::InvalidRequest.http_status(), 400);
}

// code 必须非空且在 enum 内唯一；宏接受的 status 范围为 100..=999。
// rename_all 支持 lowercase / UPPERCASE / PascalCase / camelCase /
// snake_case / SCREAMING_SNAKE_CASE / kebab-case / SCREAMING-KEBAB-CASE。
