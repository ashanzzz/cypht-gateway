use crate::{GatewayError, GatewayResult};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Account,
    Mailbox,
    Message,
    Attachment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignedObjectId {
    v: u8,
    kind: ObjectKind,
    parts: Vec<String>,
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
        let json = serde_json::to_vec(&payload)
            .map_err(|e| GatewayError::Internal(format!("encode object id: {e}")))?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| GatewayError::Crypto)?;
        mac.update(&json);
        let signature = mac.finalize().into_bytes();
        Ok(format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(json),
            URL_SAFE_NO_PAD.encode(signature)
        ))
    }

    pub fn decode(&self, expected: ObjectKind, value: &str) -> GatewayResult<Vec<String>> {
        let (payload, signature) = value
            .split_once('.')
            .ok_or_else(|| GatewayError::InvalidRequest("malformed opaque id".into()))?;
        let json = URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| GatewayError::InvalidRequest("malformed opaque id".into()))?;
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| GatewayError::InvalidRequest("malformed opaque id".into()))?;
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).map_err(|_| GatewayError::Crypto)?;
        mac.update(&json);
        mac.verify_slice(&signature)
            .map_err(|_| GatewayError::InvalidRequest("invalid opaque id signature".into()))?;
        let parsed: SignedObjectId = serde_json::from_slice(&json)
            .map_err(|_| GatewayError::InvalidRequest("malformed opaque id payload".into()))?;
        if parsed.v != 1 || parsed.kind != expected {
            return Err(GatewayError::InvalidRequest("opaque id type mismatch".into()));
        }
        Ok(parsed.parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_ids_round_trip() {
        let codec = ObjectIdCodec::new([7_u8; 32]).unwrap();
        let id = codec
            .encode(ObjectKind::Message, ["account-1", "INBOX", "42"])
            .unwrap();
        assert_eq!(
            codec.decode(ObjectKind::Message, &id).unwrap(),
            vec!["account-1", "INBOX", "42"]
        );
    }

    #[test]
    fn signed_ids_reject_tampering_and_wrong_kind() {
        let codec = ObjectIdCodec::new([7_u8; 32]).unwrap();
        let id = codec.encode(ObjectKind::Message, ["a", "INBOX", "1"]).unwrap();
        assert!(codec.decode(ObjectKind::Account, &id).is_err());

        let mut tampered = id.into_bytes();
        tampered[0] = if tampered[0] == b'A' { b'B' } else { b'A' };
        let tampered = String::from_utf8(tampered).unwrap();
        assert!(codec.decode(ObjectKind::Message, &tampered).is_err());
    }
}
