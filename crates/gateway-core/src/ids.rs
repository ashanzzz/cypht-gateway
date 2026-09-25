use crate::{GatewayError, GatewayResult};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

const MAX_OBJECT_ID_LENGTH: usize = 4_096;
const PRIVATE_ID_SIGNATURE_DOMAIN: &[u8] = b"cypht-gateway/object-id/signature/v2\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Account,
    Mailbox,
    Message,
    Attachment,
    Profile,
    Upload,
    Contact,
    Tag,
    SavedSearch,
    Calendar,
    CalendarEvent,
    Feed,
    FeedItem,
}

impl ObjectKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Mailbox => "mailbox",
            Self::Message => "message",
            Self::Attachment => "attachment",
            Self::Profile => "profile",
            Self::Upload => "upload",
            Self::Contact => "contact",
            Self::Tag => "tag",
            Self::SavedSearch => "saved_search",
            Self::Calendar => "calendar",
            Self::CalendarEvent => "calendar_event",
            Self::Feed => "feed",
            Self::FeedItem => "feed_item",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignedObjectId {
    v: u8,
    kind: ObjectKind,
    parts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedObjectId {
    pub version: u8,
    pub parts: Vec<String>,
}

#[derive(Clone)]
pub struct ObjectIdCodec {
    key: Vec<u8>,
}

impl ObjectIdCodec {
    pub fn new(key: impl AsRef<[u8]>) -> GatewayResult<Self> {
        let key = key.as_ref();
        if key.len() < 32 {
            return Err(GatewayError::Configuration(
                "object id signing key must contain at least 32 bytes".into(),
            ));
        }
        Ok(Self { key: key.to_vec() })
    }

    pub fn encode<I, S>(&self, kind: ObjectKind, parts: I) -> GatewayResult<String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let payload = SignedObjectId {
            v: 1,
            kind,
            parts: parts.into_iter().map(Into::into).collect(),
        };
        self.encode_payload(payload, false)
    }

    /// Encodes an ID for a new object without exposing its internal identifiers.
    ///
    /// The public payload contains only a keyed digest bound to the owner,
    /// kind, source, and serialized internal parts. Legacy `encode` stays v1.
    pub fn encode_private<I, S>(
        &self,
        kind: ObjectKind,
        owner: impl AsRef<str>,
        source: &str,
        internal_parts: I,
    ) -> GatewayResult<String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let owner = owner.as_ref();
        if source.is_empty() {
            return Err(GatewayError::InvalidRequest(
                "object ID source cannot be empty".into(),
            ));
        }
        let internal_parts: Vec<String> = internal_parts.into_iter().map(Into::into).collect();
        let kind_bytes = serde_json::to_vec(&kind)
            .map_err(|e| GatewayError::Internal(format!("encode private object kind: {e}")))?;
        let parts_bytes = serde_json::to_vec(&internal_parts)
            .map_err(|e| GatewayError::Internal(format!("encode private object parts: {e}")))?;

        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| GatewayError::Crypto)?;
        mac.update(b"cypht-gateway/private-object-id/v2\0");
        update_framed(&mut mac, &kind_bytes);
        update_framed(&mut mac, owner.as_bytes());
        update_framed(&mut mac, source.as_bytes());
        update_framed(&mut mac, &parts_bytes);
        let digest = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

        let payload = SignedObjectId {
            v: 2,
            kind,
            parts: vec![digest],
        };
        self.encode_payload(payload, true)
    }

    pub fn decode(&self, expected: ObjectKind, value: &str) -> GatewayResult<Vec<String>> {
        self.decode_versioned(expected, value)
            .map(|decoded| decoded.parts)
    }

    pub fn decode_versioned(
        &self,
        expected: ObjectKind,
        value: &str,
    ) -> GatewayResult<DecodedObjectId> {
        if value.len() > MAX_OBJECT_ID_LENGTH {
            return Err(GatewayError::InvalidRequest("malformed opaque id".into()));
        }
        let (expected_version, encoded) = match value.strip_prefix("v2.") {
            Some(encoded) => (2, encoded),
            None => (1, value),
        };
        let (payload, signature) = encoded
            .split_once('.')
            .ok_or_else(|| GatewayError::InvalidRequest("malformed opaque id".into()))?;
        let json = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| GatewayError::InvalidRequest("malformed opaque id".into()))?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| GatewayError::InvalidRequest("malformed opaque id".into()))?;
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| GatewayError::Crypto)?;
        if expected_version == 2 {
            mac.update(PRIVATE_ID_SIGNATURE_DOMAIN);
        }
        mac.update(&json);
        mac.verify_slice(&signature)
            .map_err(|_| GatewayError::InvalidRequest("invalid opaque id signature".into()))?;
        let parsed: SignedObjectId = serde_json::from_slice(&json)
            .map_err(|_| GatewayError::InvalidRequest("malformed opaque id payload".into()))?;
        if parsed.v != expected_version || parsed.kind != expected {
            return Err(GatewayError::InvalidRequest(
                "opaque id type mismatch".into(),
            ));
        }
        Ok(DecodedObjectId {
            version: parsed.v,
            parts: parsed.parts,
        })
    }

    fn encode_payload(&self, payload: SignedObjectId, private: bool) -> GatewayResult<String> {
        let json = serde_json::to_vec(&payload)
            .map_err(|e| GatewayError::Internal(format!("encode object id: {e}")))?;
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| GatewayError::Crypto)?;
        if payload.v == 2 {
            mac.update(PRIVATE_ID_SIGNATURE_DOMAIN);
        }
        mac.update(&json);
        let signature = mac.finalize().into_bytes();
        let encoded = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(json),
            URL_SAFE_NO_PAD.encode(signature)
        );
        Ok(if private {
            format!("v2.{encoded}")
        } else {
            encoded
        })
    }
}

fn update_framed(mac: &mut Hmac<Sha256>, value: &[u8]) {
    mac.update(&(value.len() as u64).to_be_bytes());
    mac.update(value);
}

#[cfg(test)]
#[path = "ids_tests.rs"]
mod tests;
