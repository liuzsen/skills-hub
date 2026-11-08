use super::{AdministrationStore, failure, prepare};
use hub_application::administration::grant_capabilities::{Capability, CapabilityGranter};
use hub_domain::account::AccountId;
use worker::send::SendFuture;

#[derive(serde::Deserialize)]
struct GrantRow {
    account_id: String,
}

impl CapabilityGranter for AdministrationStore {
    async fn grant(&self, target: AccountId, capability: Capability) -> anyhow::Result<bool> {
        SendFuture::new(async move {
            let (view, download, upload) = match capability {
                Capability::ViewAll => (1, 0, 0),
                Capability::DownloadAll => (0, 1, 0),
                Capability::UploadEdit => (0, 0, 1),
            };
            let result = prepare(&self.db,
                "INSERT INTO account_capabilities(account_id, view_all, download_all, upload_edit) SELECT id, ?2, ?3, ?4 FROM accounts WHERE id=?1 ON CONFLICT(account_id) DO UPDATE SET view_all=MAX(account_capabilities.view_all,excluded.view_all), download_all=MAX(account_capabilities.download_all,excluded.download_all), upload_edit=MAX(account_capabilities.upload_edit,excluded.upload_edit) RETURNING account_id",
                &[target.as_str().into(), view.into(), download.into(), upload.into()])?
                .all().await.map_err(|error| failure("capability grant", error))?;
            anyhow::ensure!(result.success(), "D1 capability grant reported failure");
            let rows = result.results::<GrantRow>().map_err(|error| failure("grant receipt", error))?;
            anyhow::ensure!(rows.len() <= 1, "duplicate capability grant receipt");
            anyhow::ensure!(rows.first().is_none_or(|row| row.account_id == target.as_str()), "wrong capability grant receipt");
            Ok(!rows.is_empty())
        }).await
    }
}
