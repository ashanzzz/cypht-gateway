use crate::GatewayService;
use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use gateway_auth::{Principal, SCOPE_CALENDAR_READ, SCOPE_CALENDAR_WRITE};
use gateway_core::{
    Calendar, CalendarEvent, CalendarEventCreateRequest, CalendarRepeatInterval, GatewayError,
    GatewayResult, ObjectKind,
};
use gateway_cypht::BridgeCalendarEvent;
use serde_json::json;

const CALENDAR_PART: &str = "calendar_events";

impl GatewayService {
    pub async fn calendar(&self, principal: &Principal) -> GatewayResult<Calendar> {
        principal.requires(SCOPE_CALENDAR_READ)?;
        self.assert_global_resource_allowed(principal)?;
        Ok(Calendar {
            id: self.calendar_public_id(principal)?,
            name: "Personal".into(),
            scope: "user".into(),
            provider: "cypht".into(),
            timezone: None,
            capabilities: vec![
                "read".into(),
                "create".into(),
                "delete".into(),
                "repeat_interval".into(),
            ],
        })
    }

    pub async fn calendar_events(
        &self,
        principal: &Principal,
        calendar_id: &str,
        start: &str,
        end: &str,
    ) -> GatewayResult<Vec<CalendarEvent>> {
        principal.requires(SCOPE_CALENDAR_READ)?;
        self.assert_global_resource_allowed(principal)?;
        self.assert_calendar_id(principal, calendar_id)?;
        let start = parse_timestamp(start, "start")?;
        let end = parse_timestamp(end, "end")?;
        if end <= start || end - start > 366 * 86_400 {
            return Err(GatewayError::InvalidRequest(
                "calendar range must be positive and no longer than 366 days".into(),
            ));
        }
        let session = self.ensure_cypht_session(principal).await?;
        let rows = self.cypht.calendar_events(&session, start, end).await?;
        let events = rows
            .into_iter()
            .map(|row| self.map_calendar_event(principal, row))
            .collect::<GatewayResult<Vec<_>>>()?;
        self.audit(
            &principal.username,
            principal.auth_id.as_deref(),
            "calendar.events.list",
            Some(calendar_id),
            true,
            None,
        );
        Ok(events)
    }

    pub async fn create_calendar_event(
        &self,
        principal: &Principal,
        calendar_id: &str,
        request: CalendarEventCreateRequest,
    ) -> GatewayResult<CalendarEvent> {
        let result = self
            .create_calendar_event_inner(principal, calendar_id, request)
            .await;
        self.audit_write(
            principal,
            "calendar.event.create",
            result.as_ref().ok().map(|event| event.id.as_str()),
            result.is_ok(),
        )?;
        result
    }

    pub async fn delete_calendar_event(
        &self,
        principal: &Principal,
        calendar_id: &str,
        event_id: &str,
        confirm: bool,
    ) -> GatewayResult<()> {
        let result = self
            .delete_calendar_event_inner(principal, calendar_id, event_id, confirm)
            .await;
        self.audit_write(
            principal,
            "calendar.event.delete",
            Some(event_id),
            result.is_ok(),
        )?;
        result
    }

    async fn create_calendar_event_inner(
        &self,
        principal: &Principal,
        calendar_id: &str,
        request: CalendarEventCreateRequest,
    ) -> GatewayResult<CalendarEvent> {
        principal.requires(SCOPE_CALENDAR_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        self.assert_calendar_id(principal, calendar_id)?;
        validate_calendar_text(&request.title, 500, true, "title")?;
        validate_calendar_text(&request.description, 20_000, false, "description")?;
        let starts_at = parse_timestamp(&request.starts_at, "starts_at")?;
        let session = self.ensure_cypht_session(principal).await?;
        let payload = json!({
            "title": request.title,
            "description": request.description,
            "starts_at": starts_at,
            "repeat_interval": repeat_to_cypht(request.repeat_interval),
        });
        let row = self
            .cypht
            .create_calendar_event(&session, &principal.credential, &payload)
            .await?;
        self.map_calendar_event(principal, row)
    }

    async fn delete_calendar_event_inner(
        &self,
        principal: &Principal,
        calendar_id: &str,
        event_id: &str,
        confirm: bool,
    ) -> GatewayResult<()> {
        principal.requires(SCOPE_CALENDAR_WRITE)?;
        self.assert_global_resource_allowed(principal)?;
        self.assert_calendar_id(principal, calendar_id)?;
        if !confirm {
            return Err(GatewayError::InvalidRequest(
                "confirm=true is required to delete a calendar event".into(),
            ));
        }
        let raw_id = self.decode_calendar_event_id(principal, event_id)?;
        let session = self.ensure_cypht_session(principal).await?;
        self.cypht
            .delete_calendar_event(
                &session,
                &principal.credential,
                &json!({"event_id": raw_id, "confirm": true}),
            )
            .await?;
        Ok(())
    }

    fn calendar_public_id(&self, principal: &Principal) -> GatewayResult<String> {
        self.encode_cypht_object_id(principal, ObjectKind::Calendar, [CALENDAR_PART])
    }

    fn assert_calendar_id(&self, principal: &Principal, id: &str) -> GatewayResult<()> {
        let decoded = self.decode_cypht_object_id(principal, ObjectKind::Calendar, id)?;
        if decoded.version != 2
            || decoded.parts.len() != 2
            || decoded.parts[0] != "cypht"
            || decoded.parts[1] != CALENDAR_PART
        {
            return Err(GatewayError::NotFound("calendar".into()));
        }
        Ok(())
    }

    fn calendar_event_public_id(
        &self,
        principal: &Principal,
        raw_id: &str,
    ) -> GatewayResult<String> {
        self.encode_cypht_object_id(principal, ObjectKind::CalendarEvent, [raw_id])
    }

    fn decode_calendar_event_id(&self, principal: &Principal, id: &str) -> GatewayResult<String> {
        let decoded = self.decode_cypht_object_id(principal, ObjectKind::CalendarEvent, id)?;
        if decoded.version != 2
            || decoded.parts.len() != 2
            || decoded.parts[0] != "cypht"
            || decoded.parts[1].is_empty()
        {
            return Err(GatewayError::NotFound("calendar event".into()));
        }
        Ok(decoded.parts[1].clone())
    }

    fn map_calendar_event(
        &self,
        principal: &Principal,
        row: BridgeCalendarEvent,
    ) -> GatewayResult<CalendarEvent> {
        Ok(CalendarEvent {
            id: self.calendar_event_public_id(principal, &row.id)?,
            title: row.title,
            description: row.description,
            starts_at: format_timestamp(row.starts_at),
            occurrence_at: format_timestamp(row.occurrence_at),
            repeat_interval: repeat_from_cypht(&row.repeat_interval)?,
        })
    }
}

fn parse_timestamp(value: &str, field: &str) -> GatewayResult<i64> {
    let parsed = DateTime::<FixedOffset>::parse_from_rfc3339(value).map_err(|_| {
        GatewayError::InvalidRequest(format!("{field} must be RFC3339 with an offset"))
    })?;
    Ok(parsed.timestamp())
}

fn format_timestamp(value: i64) -> String {
    DateTime::<Utc>::from_timestamp(value, 0)
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn validate_calendar_text(
    value: &str,
    max: usize,
    required: bool,
    field: &str,
) -> GatewayResult<()> {
    if (required && value.trim().is_empty())
        || value.len() > max
        || value.chars().any(char::is_control)
    {
        return Err(GatewayError::InvalidRequest(format!(
            "calendar {field} is invalid"
        )));
    }
    Ok(())
}

fn repeat_to_cypht(value: CalendarRepeatInterval) -> &'static str {
    match value {
        CalendarRepeatInterval::None => "",
        CalendarRepeatInterval::Day => "day",
        CalendarRepeatInterval::Week => "week",
        CalendarRepeatInterval::Month => "month",
        CalendarRepeatInterval::Year => "year",
    }
}

fn repeat_from_cypht(value: &str) -> GatewayResult<CalendarRepeatInterval> {
    match value {
        "" => Ok(CalendarRepeatInterval::None),
        "day" => Ok(CalendarRepeatInterval::Day),
        "week" => Ok(CalendarRepeatInterval::Week),
        "month" => Ok(CalendarRepeatInterval::Month),
        "year" => Ok(CalendarRepeatInterval::Year),
        _ => Err(GatewayError::CapabilityUnavailable(
            "unsupported Cypht calendar repeat interval".into(),
        )),
    }
}

#[cfg(test)]
#[path = "calendar_tests.rs"]
mod tests;
