use worker::{Request, Result, Url};

pub struct HttpPolicy {
    origin: String,
    secure_cookie: bool,
}

impl HttpPolicy {
    /// insecure_local is set only by the composition root's feature + explicit local config gate.
    /// Local mode requires a configured loopback origin even when using HTTPS.
    pub fn new(origin: String, insecure_local: bool) -> anyhow::Result<Self> {
        let url = Url::parse(&origin)?;
        anyhow::ensure!(
            url.origin().ascii_serialization() == origin
                && url.username().is_empty()
                && url.password().is_none(),
            "APP_ORIGIN must be an exact origin"
        );
        let is_loopback = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
        anyhow::ensure!(
            !insecure_local || is_loopback,
            "local configuration requires a loopback APP_ORIGIN"
        );
        anyhow::ensure!(
            url.scheme() == "https" || (insecure_local && is_loopback && url.scheme() == "http"),
            "insecure origin requires explicit local configuration"
        );
        Ok(Self {
            origin,
            secure_cookie: !insecure_local || url.scheme() == "https",
        })
    }

    pub fn check_origin(&self, request: &Request) -> Result<bool> {
        Ok(
            request.headers().get("Origin")?.as_deref() == Some(self.origin.as_str())
                && request.url()?.origin().ascii_serialization() == self.origin,
        )
    }

    pub fn secure_cookie(&self) -> bool {
        self.secure_cookie
    }
}
