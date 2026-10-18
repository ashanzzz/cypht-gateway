use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountSecurity {
    Tls,
    StartTls,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountEndpoint {
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub security: AccountSecurity,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountIdentity {
    pub name: String,
    pub email: String,
    pub reply_to: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountHealthStatus {
    Healthy,
    Degraded,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountHealth {
    pub status: AccountHealthStatus,
    pub checked_at: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedAccount {
    pub id: String,
    pub revision: u32,
    pub name: String,
    pub email: Option<String>,
    pub enabled: bool,
    pub managed: bool,
    pub inbound: Option<AccountEndpoint>,
    pub outbound: Option<AccountEndpoint>,
    pub identity: Option<AccountIdentity>,
    pub can_send: bool,
    pub health: AccountHealth,
    #[serde(default)]
    pub capabilities: Vec<String>,
}
