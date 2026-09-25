# Calendar

This document defines the limited Cypht 2.12.0 Calendar capability.

## Runtime model

Cypht 2.12.0 stores a user-level `calendar_events` array. Each event contains only `title`, `description`, `date`, and `repeat_interval`. Cypht has no event end time, duration, location, event timezone, attendee list, reminder, or RFC 5545 recurrence rule.

The Gateway exposes one synthetic user calendar named `Personal`. It supports read, create, delete, and simple repeat intervals. Event update, all-day events, locations, complex recurrence, invitations, synchronization, and import/export remain deferred.

## REST contract

- `GET /api/v1/calendars`
- `GET /api/v1/calendars/{calendar_id}/events?start=<RFC3339>&end=<RFC3339>`
- `POST /api/v1/calendars/{calendar_id}/events`
- `DELETE /api/v1/calendars/{calendar_id}/events/{event_id}?confirm=true`

The range uses `[start, end)` semantics and is limited to 366 days. Inputs must include an RFC3339 offset. A naive local timestamp is rejected.

The create body contains:

```json
{
  "title": "Team meeting",
  "description": "Plain text",
  "starts_at": "2026-09-24T09:00:00-07:00",
  "repeat_interval": "week"
}
```

The event response contains `starts_at` and `occurrence_at` as RFC3339 UTC values. `repeat_interval` is one of `none`, `day`, `week`, `month`, or `year`.

## Security and persistence

Calendar data is user-wide Cypht configuration. Account-restricted PATs are denied before Bridge access. `calendar.read` and `calendar.write` are not in the default AI scope preset.

Calendar and event IDs use owner-bound version-2 opaque IDs. Cypht's content-derived MD5 event ID is never returned as a public ID. Delete fails when the Bridge cannot identify one source event unambiguously.

Writes require the durable user-config adapter. The Bridge takes a fresh snapshot under the per-user lock, changes `calendar_events`, saves atomically, and verifies a fresh-session readback. Missing durable storage returns `501 capability_unavailable`.

MCP starts read-only. Calendar write tools require runtime write opt-in, `calendar.write`, and `confirm=true` for delete. Calendar titles and descriptions are untrusted user data.

## Verification status

Cypht 2.12.0 source review and local Bridge create, range, persistence, and confirmation tests pass. The live Unraid Bridge and provider-backed Calendar behavior remain unverified.
