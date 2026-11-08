use lolibaso::entity::{Entity, EntityId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AccountId(String);

#[derive(Debug)]
pub struct InvalidAccountId;

impl AccountId {
    pub fn parse(value: String) -> Result<Self, InvalidAccountId> {
        if value.len() == 64
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            Ok(Self(value))
        } else {
            Err(InvalidAccountId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl EntityId for AccountId {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailAddress(String);

#[derive(Debug)]
pub struct InvalidEmail;

impl EmailAddress {
    pub fn parse(value: &str) -> Result<Self, InvalidEmail> {
        let value = value.trim().to_ascii_lowercase();
        let Some((local, domain)) = value.split_once('@') else {
            return Err(InvalidEmail);
        };
        let valid_local = !local.is_empty()
            && local.len() <= 64
            && !local.starts_with('.')
            && !local.ends_with('.')
            && !local.contains("..")
            && local
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&b));
        let valid_domain = domain.contains('.')
            && domain.split('.').all(|part| {
                !part.is_empty()
                    && part.len() <= 63
                    && !part.starts_with('-')
                    && !part.ends_with('-')
                    && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            });
        if value.len() <= 254 && value.is_ascii() && valid_local && valid_domain {
            Ok(Self(value))
        } else {
            Err(InvalidEmail)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    pub view_all: bool,
    pub download_all: bool,
    pub upload_edit: bool,
}

impl Capabilities {
    pub fn can_view_all(&self) -> bool {
        self.view_all || self.download_all || self.upload_edit
    }
    pub fn can_download_all(&self) -> bool {
        self.download_all
    }
    pub fn can_edit_all(&self) -> bool {
        self.upload_edit
    }
}

#[derive(Debug, Clone, Entity)]
#[entity(no_state)]
pub struct Account {
    id: AccountId,
    email: EmailAddress,
    is_admin: bool,
    capabilities: Capabilities,
}

impl Account {
    pub fn new(
        id: AccountId,
        email: EmailAddress,
        is_admin: bool,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            id,
            email,
            is_admin,
            capabilities,
        }
    }
    pub fn id(&self) -> &AccountId {
        &self.id
    }
    pub fn email(&self) -> &EmailAddress {
        &self.email
    }
    pub fn is_admin(&self) -> bool {
        self.is_admin
    }
    pub fn capabilities(&self) -> Capabilities {
        self.capabilities
    }
}

#[derive(Debug, Clone)]
pub enum Actor {
    Anonymous,
    Account(Account),
}

impl Actor {
    pub fn account(&self) -> Option<&Account> {
        match self {
            Self::Anonymous => None,
            Self::Account(account) => Some(account),
        }
    }
}
