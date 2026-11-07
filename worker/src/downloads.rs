use hub_application::skills::artifacts::ArtifactDescriptor;
use hub_http::skills::DownloadTransport;
use hub_r2::R2ArtifactStore;
use worker::ResponseBody;

/// Runtime-only bridge; neither the application nor HTTP adapter imports R2.
pub struct R2Downloads(pub R2ArtifactStore);

impl DownloadTransport for R2Downloads {
    async fn open_body(&self, artifact: &ArtifactDescriptor) -> anyhow::Result<ResponseBody> {
        self.0.open_body(artifact).await
    }
}
