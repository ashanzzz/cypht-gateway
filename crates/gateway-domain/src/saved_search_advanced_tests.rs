use super::decode_source;
use crate::saved_search_validation::{valid_date, validate_advanced};
use gateway_core::{
    AdvancedSavedSearchData, AdvancedSearchSource, AdvancedSearchTarget, AdvancedSearchTerm,
    AdvancedSearchTimeRange,
};

#[test]
fn cypht_sources_become_known_account_folder_parts_only() {
    let accounts = vec![gateway_cypht::BridgeAccount {
        id: "7".into(),
        name: "Test account".into(),
        email: None,
        protocol: "imap".into(),
        server: None,
        can_send: false,
    }];
    let (account, folder) = decode_source("imap_7_494e424f58", &accounts).expect("folder source");
    assert_eq!(account.id, "7");
    assert_eq!(folder.as_deref(), Some("INBOX"));
    assert!(decode_source("imap_7_", &accounts).is_some_and(|(_, folder)| folder.is_none()));
    assert!(decode_source("imap_99_494e424f58", &accounts).is_none());
    assert!(decode_source("imap_7_4xyz", &accounts).is_none());
}

#[test]
fn advanced_metadata_rejects_invalid_fields_and_ranges() {
    let data = AdvancedSavedSearchData {
        terms: vec![AdvancedSearchTerm {
            term: "invoice".into(),
            condition: None,
        }],
        targets: vec![AdvancedSearchTarget {
            target: "TEXT".into(),
            orig: "TEXT".into(),
            condition: None,
        }],
        sources: vec![AdvancedSearchSource {
            account_id: "opaque-account".into(),
            mailbox_id: None,
            all_folders: true,
            subfolders: false,
        }],
        times: vec![AdvancedSearchTimeRange {
            from: "2026-02-28".into(),
            to: "2026-03-01".into(),
        }],
        other: gateway_core::AdvancedSearchOther {
            limit: 100,
            flags: vec![],
            charset: None,
        },
    };
    assert!(validate_advanced(&data).is_ok());
    let mut invalid_source = data.clone();
    invalid_source.sources[0].subfolders = true;
    assert!(validate_advanced(&invalid_source).is_err());
    let mut invalid = data;
    invalid.times[0].from = "2026-02-30".into();
    assert!(validate_advanced(&invalid).is_err());
    assert!(!valid_date("0000-01-01"));
}
