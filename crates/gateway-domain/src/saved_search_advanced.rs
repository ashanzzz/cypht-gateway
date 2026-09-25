use crate::saved_search_validation::{unsupported_advanced_search, validate_advanced};
use crate::GatewayService;
use gateway_auth::{CyphtSession, Principal};
use gateway_core::{AdvancedSavedSearchData, GatewayError, GatewayResult, ObjectKind};
use serde_json::{json, Value};

pub(super) async fn from_bridge(
    service: &GatewayService,
    principal: &Principal,
    session: &CyphtSession,
    raw: Value,
) -> GatewayResult<AdvancedSavedSearchData> {
    let mut data = raw
        .as_object()
        .cloned()
        .ok_or_else(unsupported_advanced_search)?;
    if let Some(other) = data.get_mut("other").and_then(Value::as_object_mut) {
        if let Some(Value::String(limit)) = other.get("limit").cloned() {
            let parsed = limit
                .parse::<u64>()
                .map_err(|_| unsupported_advanced_search())?;
            other.insert("limit".into(), Value::Number(parsed.into()));
        }
        if other.get("charset").and_then(Value::as_str) == Some("") {
            other.insert("charset".into(), Value::Null);
        }
    }
    let raw_sources = data
        .remove("sources")
        .and_then(|value| value.as_array().cloned())
        .ok_or_else(unsupported_advanced_search)?;
    let accounts = service.cypht.accounts(session).await?;
    let mut public_sources = Vec::with_capacity(raw_sources.len());
    for raw_source in raw_sources {
        let source = raw_source
            .as_object()
            .ok_or_else(unsupported_advanced_search)?;
        let source_id = source
            .get("source")
            .and_then(Value::as_str)
            .ok_or_else(unsupported_advanced_search)?;
        let (account, folder) =
            decode_source(source_id, &accounts).ok_or_else(unsupported_advanced_search)?;
        let account_id = service.account_public_id(principal, &account.id)?;
        let all_folders = source
            .get("allFolders")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || folder.is_none();
        let mailbox_id = if all_folders {
            None
        } else if let Some(folder) = folder {
            let mailboxes = service.cypht.mailboxes(session, &account.id).await?;
            if !mailboxes.iter().any(|mailbox| mailbox.name == folder) {
                return Err(unsupported_advanced_search());
            }
            Some(service.encode_cypht_object_id(
                principal,
                ObjectKind::Mailbox,
                [&account.id, &folder],
            )?)
        } else {
            None
        };
        public_sources.push(json!({
            "account_id": account_id,
            "mailbox_id": mailbox_id,
            "all_folders": all_folders,
            "subfolders": source.get("subFolders").and_then(Value::as_bool).unwrap_or(false)
        }));
    }
    data.insert("sources".into(), Value::Array(public_sources));
    let mut data: AdvancedSavedSearchData =
        serde_json::from_value(Value::Object(data)).map_err(|_| unsupported_advanced_search())?;
    if data.other.charset.as_deref() == Some("") {
        data.other.charset = None;
    }
    validate_advanced(&data)?;
    Ok(data)
}

fn decode_source<'a>(
    source: &str,
    accounts: &'a [gateway_cypht::BridgeAccount],
) -> Option<(&'a gateway_cypht::BridgeAccount, Option<String>)> {
    let raw = source.strip_prefix("imap_")?;
    let mut matches = accounts
        .iter()
        .filter_map(|account| {
            let rest = raw.strip_prefix(&account.id)?;
            if rest.is_empty() || rest == "_" {
                return Some((account, None));
            }
            let encoded_folder = rest.strip_prefix('_')?;
            if encoded_folder.is_empty() {
                return Some((account, None));
            }
            if encoded_folder.len() % 2 != 0
                || !encoded_folder.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return None;
            }
            let bytes = hex::decode(encoded_folder).ok()?;
            let folder = String::from_utf8(bytes).ok()?;
            Some((account, Some(folder)))
        })
        .collect::<Vec<_>>();
    matches.sort_by_key(|(account, _)| std::cmp::Reverse(account.id.len()));
    matches.into_iter().next()
}
pub(super) async fn to_bridge(
    service: &GatewayService,
    principal: &Principal,
    session: &CyphtSession,
    data: &AdvancedSavedSearchData,
) -> GatewayResult<Value> {
    validate_advanced(data)?;
    let accounts = service.cypht.accounts(session).await?;
    let mut sources = Vec::with_capacity(data.sources.len());
    for source in &data.sources {
        let account_ref =
            service.decode_cypht_object_id(principal, ObjectKind::Account, &source.account_id)?;
        if account_ref.version != 2 {
            return Err(GatewayError::InvalidRequest(
                "advanced sources require version-2 account IDs".into(),
            ));
        }
        let account = service.decode_account(principal, &source.account_id)?;
        service.assert_account_allowed(principal, &source.account_id)?;
        let details = accounts
            .iter()
            .find(|row| row.id == account)
            .ok_or_else(|| GatewayError::NotFound("saved search account".into()))?;
        let (cypht_source, label, all_folders) = if let Some(mailbox_id) = &source.mailbox_id {
            if source.all_folders {
                return Err(GatewayError::InvalidRequest(
                    "all_folders cannot be combined with mailbox_id".into(),
                ));
            }
            let mailbox_ref =
                service.decode_cypht_object_id(principal, ObjectKind::Mailbox, mailbox_id)?;
            if mailbox_ref.version != 2 {
                return Err(GatewayError::InvalidRequest(
                    "advanced sources require version-2 mailbox IDs".into(),
                ));
            }
            let folder = service.decode_mailbox_for_account(principal, mailbox_id, &account)?;
            let mailboxes = service.cypht.mailboxes(session, &account).await?;
            let mailbox = mailboxes
                .iter()
                .find(|row| row.name == folder)
                .ok_or_else(|| GatewayError::NotFound("saved search mailbox".into()))?;
            (
                format!("imap_{account}_{}", hex::encode(folder.as_bytes())),
                format!("{} > {}", details.name, mailbox.display_name),
                false,
            )
        } else if source.all_folders {
            (format!("imap_{account}_"), details.name.clone(), true)
        } else {
            return Err(GatewayError::InvalidRequest(
                "saved search source requires mailbox_id or all_folders".into(),
            ));
        };
        sources.push(json!({
            "source": cypht_source,
            "label": label,
            "allFolders": all_folders,
            "subFolders": source.subfolders
        }));
    }
    Ok(json!({
        "terms": &data.terms,
        "targets": &data.targets,
        "sources": sources,
        "times": &data.times,
        "other": &data.other
    }))
}

#[cfg(test)]
#[path = "saved_search_advanced_tests.rs"]
mod tests;
