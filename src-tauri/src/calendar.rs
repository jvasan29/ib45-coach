use crate::{
    db::AppStore,
    models::{CalendarBinding, CalendarStatus, StudyBlockInput},
};
use anyhow::{Context, Result, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration as ChronoDuration, Utc};
use rand::RngCore;
use reqwest::{Client, StatusCode};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant},
};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoogleToken {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: i64,
    token_type: String,
}

pub async fn connect(store: &AppStore) -> Result<CalendarStatus> {
    let client_id = store
        .get_secret("google-client-id")
        .ok_or_else(|| anyhow!("Save a Google desktop OAuth client ID first"))?;
    let client_secret = store.get_secret("google-client-secret").unwrap_or_default();
    let oauth = tokio::task::spawn_blocking({
        let client_id = client_id.clone();
        move || receive_authorization_code(&client_id)
    })
    .await??;
    let response = Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", oauth.code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", oauth.redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", oauth.verifier.as_str()),
        ])
        .send()
        .await
        .context("Could not exchange the Google authorization code")?;
    let status = response.status();
    let raw: Value = response.json().await?;
    if !status.is_success() {
        return Err(anyhow!(
            "Google authorization failed: {}",
            raw.get("error_description")
                .or_else(|| raw.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
        ));
    }
    let token = GoogleToken {
        access_token: required_string(&raw, "access_token")?,
        refresh_token: raw
            .get("refresh_token")
            .and_then(Value::as_str)
            .map(str::to_string),
        expires_at: Utc::now().timestamp()
            + raw
                .get("expires_in")
                .and_then(Value::as_i64)
                .unwrap_or(3600)
            - 60,
        token_type: raw
            .get("token_type")
            .and_then(Value::as_str)
            .unwrap_or("Bearer")
            .to_string(),
    };
    store.save_secret("google-token", &serde_json::to_string(&token)?)?;
    ensure_coach_calendar(store).await?;
    sync(store).await
}

struct OAuthResult {
    code: String,
    verifier: String,
    redirect_uri: String,
}

fn receive_authorization_code(client_id: &str) -> Result<OAuthResult> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let redirect_uri = format!("http://127.0.0.1:{}", listener.local_addr()?.port());
    let mut random = [0u8; 48];
    rand::rng().fill_bytes(&mut random);
    let verifier = URL_SAFE_NO_PAD.encode(random);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = Uuid::new_v4().to_string();
    let mut authorization = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")?;
    authorization.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "https://www.googleapis.com/auth/calendar https://www.googleapis.com/auth/userinfo.email")
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state);
    open::that(authorization.as_str()).context("Could not open the Google authorization page")?;
    let started = Instant::now();
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer)?;
                let request = String::from_utf8_lossy(&buffer[..read]);
                let target = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .ok_or_else(|| anyhow!("Invalid OAuth redirect"))?;
                let redirect = Url::parse(&format!("http://localhost{target}"))?;
                let parameters = redirect
                    .query_pairs()
                    .collect::<std::collections::HashMap<_, _>>();
                let returned_state = parameters
                    .get("state")
                    .map(|value| value.as_ref())
                    .unwrap_or("");
                let (body, result) = if returned_state != state {
                    (
                        "Authorization was rejected because the security state did not match.",
                        Err(anyhow!("OAuth state mismatch")),
                    )
                } else if let Some(error) = parameters.get("error") {
                    (
                        "Google Calendar connection was cancelled. You can close this tab.",
                        Err(anyhow!("Google authorization was cancelled: {error}")),
                    )
                } else {
                    let code = parameters
                        .get("code")
                        .map(|value| value.to_string())
                        .ok_or_else(|| anyhow!("Google returned no authorization code"))?;
                    (
                        "IB 45 Coach is connected to Google Calendar. You can close this tab and return to the app.",
                        Ok(OAuthResult {
                            code,
                            verifier: verifier.clone(),
                            redirect_uri: redirect_uri.clone(),
                        }),
                    )
                };
                let html = format!(
                    "<!doctype html><title>IB 45 Coach</title><body style='font-family:system-ui;padding:48px;background:#f4f1e8;color:#172033'><h1>IB 45 Coach</h1><p>{body}</p></body>"
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    html.len(),
                    html
                );
                let _ = stream.write_all(response.as_bytes());
                return result;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if started.elapsed() > Duration::from_secs(180) {
                    return Err(anyhow!("Google authorization timed out"));
                }
                thread::sleep(Duration::from_millis(200));
            }
            Err(error) => return Err(error.into()),
        }
    }
}

async fn valid_token(store: &AppStore) -> Result<GoogleToken> {
    let encoded = store
        .get_secret("google-token")
        .ok_or_else(|| anyhow!("Google Calendar is not connected"))?;
    let mut token: GoogleToken = serde_json::from_str(&encoded)?;
    if token.expires_at > Utc::now().timestamp() + 30 {
        return Ok(token);
    }
    let refresh_token = token
        .refresh_token
        .clone()
        .ok_or_else(|| anyhow!("Google access expired; reconnect Calendar"))?;
    let client_id = store
        .get_secret("google-client-id")
        .ok_or_else(|| anyhow!("Google client ID is missing"))?;
    let client_secret = store.get_secret("google-client-secret").unwrap_or_default();
    let response = Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;
    let status = response.status();
    let raw: Value = response.json().await?;
    if !status.is_success() {
        return Err(anyhow!("Could not refresh Google access: {}", raw));
    }
    token.access_token = required_string(&raw, "access_token")?;
    token.expires_at = Utc::now().timestamp()
        + raw
            .get("expires_in")
            .and_then(Value::as_i64)
            .unwrap_or(3600)
        - 60;
    store.save_secret("google-token", &serde_json::to_string(&token)?)?;
    Ok(token)
}

async fn ensure_coach_calendar(store: &AppStore) -> Result<()> {
    let token = valid_token(store).await?;
    let client = Client::new();
    let calendars: Value = client
        .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
        .bearer_auth(&token.access_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let existing = calendars
        .get("items")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("summary").and_then(Value::as_str) == Some("IB 45 Coach"))
        });
    let calendar_id = if let Some(value) = existing {
        required_string(value, "id")?
    } else {
        let created: Value = client.post("https://www.googleapis.com/calendar/v3/calendars")
            .bearer_auth(&token.access_token).json(&json!({"summary":"IB 45 Coach","description":"Study blocks managed by your local IB 45 Coach application.","timeZone":"Asia/Bangkok"}))
            .send().await?.error_for_status()?.json().await?;
        required_string(&created, "id")?
    };
    store.connect()?.execute(
        "INSERT INTO calendar_bindings(calendar_id,name,selected,auto_edit,is_coach_calendar,updated_at) VALUES(?1,'IB 45 Coach',1,1,1,?2)
         ON CONFLICT(calendar_id) DO UPDATE SET name='IB 45 Coach',selected=1,auto_edit=1,is_coach_calendar=1,updated_at=excluded.updated_at",
        params![calendar_id,Utc::now().to_rfc3339()],
    )?;
    Ok(())
}

pub async fn sync(store: &AppStore) -> Result<CalendarStatus> {
    let token = valid_token(store).await?;
    let client = Client::new();
    let response = client
        .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
        .bearer_auth(&token.access_token)
        .send()
        .await?;
    let status = response.status();
    let calendars: Value = response.json().await?;
    if !status.is_success() {
        return Err(anyhow!("Calendar list failed: {calendars}"));
    }
    let connection = store.connect()?;
    for calendar in calendars
        .get("items")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let id = required_string(&calendar, "id")?;
        let name = calendar
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("Untitled calendar");
        let coach = name == "IB 45 Coach";
        connection.execute(
            "INSERT INTO calendar_bindings(calendar_id,name,selected,auto_edit,is_coach_calendar,updated_at) VALUES(?1,?2,?3,?3,?3,?4)
             ON CONFLICT(calendar_id) DO UPDATE SET name=excluded.name,is_coach_calendar=CASE WHEN excluded.is_coach_calendar=1 THEN 1 ELSE calendar_bindings.is_coach_calendar END,updated_at=excluded.updated_at",
            params![id,name,coach as i64,Utc::now().to_rfc3339()],
        )?;
    }
    drop(connection);
    let bindings = bindings(store)?;
    for binding in bindings.iter().filter(|binding| binding.selected) {
        sync_events(store, &client, &token, binding).await?;
    }
    store.set_setting("calendar_last_sync", &Utc::now().to_rfc3339())?;
    status_view(store).await
}

async fn sync_events(
    store: &AppStore,
    client: &Client,
    token: &GoogleToken,
    binding: &CalendarBinding,
) -> Result<()> {
    let sync_token: Option<String> = store
        .connect()?
        .query_row(
            "SELECT sync_token FROM calendar_bindings WHERE calendar_id=?1",
            [&binding.calendar_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let mut page_token: Option<String> = None;
    let mut final_sync_token: Option<String> = None;
    let mut retried_full = false;
    loop {
        let mut url = Url::parse(&format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            url::form_urlencoded::byte_serialize(binding.calendar_id.as_bytes())
                .collect::<String>()
        ))?;
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("singleEvents", "true")
                .append_pair("showDeleted", "true")
                .append_pair("maxResults", "2500");
            if let Some(page) = &page_token {
                query.append_pair("pageToken", page);
            }
            if let Some(sync_value) = &sync_token {
                if !retried_full {
                    query.append_pair("syncToken", sync_value);
                }
            }
            if sync_token.is_none() || retried_full {
                query.append_pair(
                    "timeMin",
                    &(Utc::now() - ChronoDuration::days(30)).to_rfc3339(),
                );
                query.append_pair(
                    "timeMax",
                    &(Utc::now() + ChronoDuration::days(180)).to_rfc3339(),
                );
            }
        }
        let response = client
            .get(url)
            .bearer_auth(&token.access_token)
            .send()
            .await?;
        if response.status() == StatusCode::GONE && !retried_full {
            retried_full = true;
            page_token = None;
            store.connect()?.execute(
                "UPDATE calendar_bindings SET sync_token=NULL WHERE calendar_id=?1",
                [&binding.calendar_id],
            )?;
            continue;
        }
        let status = response.status();
        let raw: Value = response.json().await?;
        if !status.is_success() {
            return Err(anyhow!(
                "Calendar sync failed for {}: {}",
                binding.name,
                raw
            ));
        }
        let connection = store.connect()?;
        for event in raw
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            let event_id = required_string(&event, "id")?;
            let start = event
                .get("start")
                .and_then(|value| value.get("dateTime").or_else(|| value.get("date")))
                .and_then(Value::as_str)
                .map(str::to_string);
            let end = event
                .get("end")
                .and_then(|value| value.get("dateTime").or_else(|| value.get("date")))
                .and_then(Value::as_str)
                .map(str::to_string);
            let has_attendees = event
                .get("attendees")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty());
            connection.execute(
                "INSERT INTO calendar_events(event_id,calendar_id,summary,start_at,end_at,has_attendees,etag,status,raw_json,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
                 ON CONFLICT(event_id,calendar_id) DO UPDATE SET summary=excluded.summary,start_at=excluded.start_at,end_at=excluded.end_at,has_attendees=excluded.has_attendees,etag=excluded.etag,status=excluded.status,raw_json=excluded.raw_json,updated_at=excluded.updated_at",
                params![event_id,binding.calendar_id,event.get("summary").and_then(Value::as_str).unwrap_or("Busy"),start,end,has_attendees as i64,event.get("etag").and_then(Value::as_str),event.get("status").and_then(Value::as_str).unwrap_or("confirmed"),serde_json::to_string(&event)?,Utc::now().to_rfc3339()],
            )?;
        }
        page_token = raw
            .get("nextPageToken")
            .and_then(Value::as_str)
            .map(str::to_string);
        final_sync_token = raw
            .get("nextSyncToken")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or(final_sync_token);
        if page_token.is_none() {
            break;
        }
    }
    let connection = store.connect()?;
    let event_count: i64 = connection.query_row(
        "SELECT count(*) FROM calendar_events WHERE calendar_id=?1 AND status<>'cancelled'",
        [&binding.calendar_id],
        |row| row.get(0),
    )?;
    connection.execute("UPDATE calendar_bindings SET sync_token=?1,event_count=?2,updated_at=?3 WHERE calendar_id=?4", params![final_sync_token,event_count,Utc::now().to_rfc3339(),binding.calendar_id])?;
    Ok(())
}

pub async fn status_view(store: &AppStore) -> Result<CalendarStatus> {
    let connected = store.get_secret("google-token").is_some();
    let account_email = if connected {
        if let Ok(token) = valid_token(store).await {
            match Client::new()
                .get("https://www.googleapis.com/oauth2/v2/userinfo")
                .bearer_auth(token.access_token)
                .send()
                .await
            {
                Ok(response) => response.json::<Value>().await.ok().and_then(|value| {
                    value
                        .get("email")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                }),
                Err(_) => None,
            }
        } else {
            None
        }
    } else {
        None
    };
    Ok(CalendarStatus {
        connected,
        account_email,
        last_sync_at: store.get_setting("calendar_last_sync"),
        bindings: bindings(store)?,
    })
}

fn bindings(store: &AppStore) -> Result<Vec<CalendarBinding>> {
    let connection = store.connect()?;
    let mut statement = connection.prepare("SELECT calendar_id,name,selected,auto_edit,is_coach_calendar,event_count FROM calendar_bindings ORDER BY is_coach_calendar DESC,name")?;
    let rows = statement.query_map([], |row| {
        Ok(CalendarBinding {
            calendar_id: row.get(0)?,
            name: row.get(1)?,
            selected: row.get::<_, i64>(2)? == 1,
            auto_edit: row.get::<_, i64>(3)? == 1,
            is_coach_calendar: row.get::<_, i64>(4)? == 1,
            event_count: row.get(5)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn set_binding(
    store: &AppStore,
    calendar_id: &str,
    selected: bool,
    auto_edit: bool,
) -> Result<()> {
    let coach: bool = store
        .connect()?
        .query_row(
            "SELECT is_coach_calendar FROM calendar_bindings WHERE calendar_id=?1",
            [calendar_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .unwrap_or(0)
        == 1;
    store.connect()?.execute(
        "UPDATE calendar_bindings SET selected=?1,auto_edit=?2,updated_at=?3 WHERE calendar_id=?4",
        params![
            (selected || coach) as i64,
            (auto_edit || coach) as i64,
            Utc::now().to_rfc3339(),
            calendar_id
        ],
    )?;
    Ok(())
}

pub async fn schedule_block(store: &AppStore, input: StudyBlockInput) -> Result<String> {
    let allowed = store
        .connect()?
        .query_row(
            "SELECT selected=1 AND auto_edit=1 FROM calendar_bindings WHERE calendar_id=?1",
            [&input.calendar_id],
            |row| row.get::<_, bool>(0),
        )
        .optional()?
        .unwrap_or(false);
    if !allowed {
        return Err(anyhow!(
            "This calendar is not authorized for automatic study-block edits"
        ));
    }
    let token = valid_token(store).await?;
    let event_id = format!("ib45{}", Uuid::new_v4().simple());
    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events?sendUpdates=none",
        url::form_urlencoded::byte_serialize(input.calendar_id.as_bytes()).collect::<String>()
    );
    let payload = json!({
        "id": event_id,
        "summary": input.title,
        "description": input.description,
        "start": {"dateTime": input.start_at},
        "end": {"dateTime": input.end_at},
        "extendedProperties": {"private":{"ib45TaskId":input.task_id}},
        "reminders": {"useDefault":false,"overrides":[{"method":"popup","minutes":10}]}
    });
    let response = Client::new()
        .post(url)
        .bearer_auth(token.access_token)
        .json(&payload)
        .send()
        .await?;
    let status = response.status();
    let raw: Value = response.json().await?;
    if !status.is_success() {
        return Err(anyhow!("Could not create study block: {}", raw));
    }
    store.connect()?.execute(
        "UPDATE tasks SET calendar_event_id=?1,updated_at=?2 WHERE id=?3",
        params![event_id, Utc::now().to_rfc3339(), input.task_id],
    )?;
    Ok(event_id)
}

pub fn disconnect(store: &AppStore) -> Result<()> {
    store.delete_secret("google-token");
    store
        .connect()?
        .execute_batch("DELETE FROM calendar_events; DELETE FROM calendar_bindings;")?;
    Ok(())
}

fn required_string(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("Missing {key} in provider response"))
}
