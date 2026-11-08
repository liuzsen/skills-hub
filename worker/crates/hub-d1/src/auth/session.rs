use super::{AuthStore, failure, integer, prepare};
use hub_application::auth::{authenticate_session::SessionReader, logout::SessionRevoker};
use hub_domain::account::{Account, AccountId, Capabilities, EmailAddress};
use worker::send::SendFuture;

#[derive(serde::Deserialize)]
struct AccountRow {
    id: String,
    email_key: String,
    is_admin: i64,
    view_all: i64,
    download_all: i64,
    upload_edit: i64,
}

impl AccountRow {
    fn into_account(self) -> anyhow::Result<Account> {
        let id =
            AccountId::parse(self.id).map_err(|_| anyhow::anyhow!("invalid stored account id"))?;
        let email = EmailAddress::parse(&self.email_key)
            .map_err(|_| anyhow::anyhow!("invalid stored email"))?;
        anyhow::ensure!(
            [
                self.is_admin,
                self.view_all,
                self.download_all,
                self.upload_edit
            ]
            .iter()
            .all(|value| matches!(value, 0 | 1)),
            "invalid stored capability"
        );
        Ok(Account::new(
            id,
            email,
            self.is_admin == 1,
            Capabilities {
                view_all: self.view_all == 1,
                download_all: self.download_all == 1,
                upload_edit: self.upload_edit == 1,
            },
        ))
    }
}

impl SessionReader for AuthStore {
    async fn find_active(&self, token_hash: String, now: i64) -> anyhow::Result<Option<Account>> {
        SendFuture::new(async move {
            let row = prepare(&self.db, "SELECT a.id, a.email_key, EXISTS(SELECT 1 FROM admin_roles r WHERE r.account_id=a.id) AS is_admin, COALESCE(c.view_all,0) AS view_all, COALESCE(c.download_all,0) AS download_all, COALESCE(c.upload_edit,0) AS upload_edit FROM sessions s JOIN accounts a ON a.id=s.account_id LEFT JOIN account_capabilities c ON c.account_id=a.id WHERE s.token_hash=?1 AND s.expires_at>?2 AND s.revoked_at IS NULL", &[token_hash.as_str().into(), integer(now)?])?
                .first::<AccountRow>(None).await.map_err(|error| failure("session lookup", error))?;
            row.map(AccountRow::into_account).transpose()
        }).await
    }
}

impl SessionRevoker for AuthStore {
    async fn revoke(&self, token_hash: String, now: i64) -> anyhow::Result<()> {
        SendFuture::new(async move {
            let result = prepare(
                &self.db,
                "UPDATE sessions SET revoked_at=?2 WHERE token_hash=?1 AND revoked_at IS NULL",
                &[token_hash.as_str().into(), integer(now)?],
            )?
            .run()
            .await
            .map_err(|error| failure("session revoke", error))?;
            anyhow::ensure!(result.success(), "D1 session revoke reported failure");
            Ok(())
        })
        .await
    }
}
