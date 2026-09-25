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
fn private_ids_use_v2_while_legacy_signed_ids_still_decode() {
    let codec = ObjectIdCodec::new([7_u8; 32]).unwrap();
    let legacy = codec
        .encode(ObjectKind::Message, ["account-1", "INBOX", "42"])
        .unwrap();
    let private = codec
        .encode_private(
            ObjectKind::Message,
            "alice",
            "cypht",
            ["account-1", "INBOX", "42"],
        )
        .unwrap();

    assert!(!legacy.starts_with("v2."));
    assert!(private.starts_with("v2."));
    assert_eq!(
        codec
            .decode_versioned(ObjectKind::Message, &legacy)
            .unwrap(),
        DecodedObjectId {
            version: 1,
            parts: vec!["account-1".into(), "INBOX".into(), "42".into()],
        }
    );
    assert_eq!(
        codec
            .decode_versioned(ObjectKind::Message, &private)
            .unwrap()
            .version,
        2
    );
}

#[test]
fn signed_ids_reject_tampering_and_wrong_kind() {
    let codec = ObjectIdCodec::new([7_u8; 32]).unwrap();
    let id = codec
        .encode(ObjectKind::Message, ["a", "INBOX", "1"])
        .unwrap();
    assert!(codec.decode(ObjectKind::Account, &id).is_err());

    let mut tampered = id.into_bytes();
    tampered[0] = if tampered[0] == b'A' { b'B' } else { b'A' };
    let tampered = String::from_utf8(tampered).unwrap();
    assert!(codec.decode(ObjectKind::Message, &tampered).is_err());
}

#[test]
fn signed_ids_reject_cross_kind_decoding() {
    let codec = ObjectIdCodec::new([7_u8; 32]).unwrap();

    for (encoded_kind, expected_kind) in [
        (ObjectKind::Contact, ObjectKind::Tag),
        (ObjectKind::Tag, ObjectKind::CalendarEvent),
        (ObjectKind::Feed, ObjectKind::Message),
    ] {
        let id = codec.encode(encoded_kind, ["opaque-part"]).unwrap();
        assert!(codec.decode(expected_kind, &id).is_err());
    }
}

#[test]
fn private_ids_are_deterministic_and_hide_internal_parts() {
    let codec = ObjectIdCodec::new([7_u8; 32]).unwrap();
    let first = codec
        .encode_private(
            ObjectKind::Contact,
            "owner-1",
            "local",
            ["internal-contact-42", "folder"],
        )
        .unwrap();
    let repeated = codec
        .encode_private(
            ObjectKind::Contact,
            "owner-1",
            "local",
            ["internal-contact-42", "folder"],
        )
        .unwrap();

    assert_eq!(first, repeated);
    assert!(first.starts_with("v2."));
    let public_parts = codec.decode(ObjectKind::Contact, &first).unwrap();
    assert_eq!(public_parts.len(), 1);
    assert!(!public_parts.contains(&"owner-1".to_string()));
    assert!(!public_parts.contains(&"internal-contact-42".to_string()));
    assert!(!public_parts.contains(&"folder".to_string()));
    assert!(!first.contains("owner-1"));
    assert!(!first.contains("internal-contact-42"));
}

#[test]
fn private_ids_bind_kind_owner_source_and_internal_parts() {
    let codec = ObjectIdCodec::new([7_u8; 32]).unwrap();
    let base = codec
        .encode_private(
            ObjectKind::Contact,
            "owner-a",
            "local",
            ["same-internal-id"],
        )
        .unwrap();
    let other_kind = codec
        .encode_private(ObjectKind::Tag, "owner-a", "local", ["same-internal-id"])
        .unwrap();
    let other_owner = codec
        .encode_private(
            ObjectKind::Contact,
            "owner-b",
            "local",
            ["same-internal-id"],
        )
        .unwrap();
    let other_source = codec
        .encode_private(
            ObjectKind::Contact,
            "owner-a",
            "carddav",
            ["same-internal-id"],
        )
        .unwrap();
    let other_parts = codec
        .encode_private(
            ObjectKind::Contact,
            "owner-a",
            "local",
            ["different-internal-id"],
        )
        .unwrap();

    assert!(codec.decode(ObjectKind::Tag, &base).is_err());
    assert_ne!(base, other_kind);
    assert_ne!(base, other_owner);
    assert_ne!(base, other_source);
    assert_ne!(base, other_parts);
    assert_ne!(
        codec.decode(ObjectKind::Contact, &base).unwrap(),
        codec.decode(ObjectKind::Contact, &other_owner).unwrap()
    );
    assert!(codec
        .decode(ObjectKind::Contact, &base.replacen("v2.", "v3.", 1))
        .is_err());
}
