use gateway_core::{AdvancedSavedSearchData, GatewayError, GatewayResult};
use serde_json::Value;
pub(crate) fn validate_advanced(data: &AdvancedSavedSearchData) -> GatewayResult<()> {
    if data.terms.is_empty()
        || data.terms.len() > 10
        || data.targets.is_empty()
        || data.targets.len() > 10
        || data.sources.is_empty()
        || data.sources.len() > 50
        || data.times.is_empty()
        || data.times.len() > 10
    {
        return Err(GatewayError::InvalidRequest(
            "advanced saved search is incomplete or exceeds limits".into(),
        ));
    }
    for term in &data.terms {
        if term.term.trim().is_empty() || term.term.len() > 500 {
            return Err(GatewayError::InvalidRequest(
                "advanced saved search term is invalid".into(),
            ));
        }
        validate_condition(term.condition.as_ref())?;
    }
    for target in &data.targets {
        let valid = match target.orig.as_str() {
            "TEXT" => target.target == "TEXT",
            "BODY" => target.target == "BODY",
            "header" => matches!(target.target.as_str(), "FROM" | "SUBJECT" | "TO" | "CC"),
            "custom" => valid_custom_header(&target.target),
            _ => false,
        };
        if !valid {
            return Err(GatewayError::InvalidRequest(
                "advanced saved search target is invalid".into(),
            ));
        }
        validate_condition(target.condition.as_ref())?;
    }
    if data
        .sources
        .iter()
        .any(|source| source.all_folders && source.subfolders)
    {
        return Err(GatewayError::InvalidRequest(
            "all_folders cannot be combined with subfolders".into(),
        ));
    }
    if !(1..=1000).contains(&data.other.limit) {
        return Err(GatewayError::InvalidRequest(
            "advanced saved search limit must be between 1 and 1000".into(),
        ));
    }
    if data.other.flags.len() > 8
        || data.other.flags.iter().any(|flag| {
            !matches!(
                flag.as_str(),
                "SEEN"
                    | "UNSEEN"
                    | "ANSWERED"
                    | "UNANSWERED"
                    | "FLAGGED"
                    | "UNFLAGGED"
                    | "DELETED"
                    | "UNDELETED"
            )
        })
    {
        return Err(GatewayError::InvalidRequest(
            "advanced saved search flags are invalid".into(),
        ));
    }
    if data
        .other
        .charset
        .as_deref()
        .is_some_and(|charset| !matches!(charset, "UTF-8" | "ASCII"))
    {
        return Err(GatewayError::InvalidRequest(
            "advanced saved search charset is invalid".into(),
        ));
    }
    for time in &data.times {
        if !valid_date(&time.from) || !valid_date(&time.to) || time.from > time.to {
            return Err(GatewayError::InvalidRequest(
                "advanced saved search time range is invalid".into(),
            ));
        }
    }
    Ok(())
}

fn valid_custom_header(target: &str) -> bool {
    let Some(name) = target.strip_prefix("HEADER ") else {
        return false;
    };
    !name.is_empty()
        && name.len() <= 78
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn validate_condition(value: Option<&Value>) -> GatewayResult<()> {
    if value.is_none_or(|condition| {
        condition == &Value::Bool(false)
            || condition == &Value::Null
            || condition == "and"
            || condition == "or"
    }) {
        Ok(())
    } else {
        Err(GatewayError::InvalidRequest(
            "advanced saved search condition is invalid".into(),
        ))
    }
}

pub(crate) fn valid_date(value: &str) -> bool {
    let mut parts = value.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    if year == 0 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

pub(crate) fn unsupported_advanced_search() -> GatewayError {
    GatewayError::CapabilityUnavailable("unsupported Cypht advanced search format".into())
}
