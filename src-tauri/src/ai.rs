use crate::{
    db::AppStore,
    models::{AiAnalysis, AiRequest, SecretStatus},
};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use reqwest::Client;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::time::Duration;
use uuid::Uuid;

const INTEGRITY_WARNING: &str = "IB does not regard AI-produced material as your own work. If you use generated or paraphrased text in assessed work, credit the tool in the text and bibliography and follow your school's policy.";

pub async fn status(store: &AppStore) -> SecretStatus {
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    let ollama_available = client
        .get("http://127.0.0.1:11434/api/tags")
        .send()
        .await
        .is_ok();
    SecretStatus {
        openai_configured: store.get_secret("openai-api-key").is_some(),
        google_configured: store.get_secret("google-client-id").is_some(),
        google_connected: store.get_secret("google-token").is_some(),
        ollama_available,
    }
}

pub async fn analyze(store: &AppStore, request: AiRequest) -> Result<AiAnalysis> {
    let warning = request.assessed_work.then(|| INTEGRITY_WARNING.to_string());
    let result = if let Some(api_key) = store.get_secret("openai-api-key") {
        openai_analysis(store, &api_key, &request, warning.clone()).await
    } else {
        ollama_analysis(&request, warning.clone()).await
    };
    let analysis = match result {
        Ok(value) => value,
        Err(cloud_error) if store.get_secret("openai-api-key").is_some() => {
            ollama_analysis(&request, warning)
                .await
                .map_err(|local_error| {
                    anyhow!(
                        "Cloud analysis failed: {cloud_error}. Local fallback failed: {local_error}"
                    )
                })?
        }
        Err(error) => return Err(error),
    };
    store.save_ai_analysis(&analysis, &request.prompt)?;
    Ok(analysis)
}

async fn openai_analysis(
    store: &AppStore,
    api_key: &str,
    request: &AiRequest,
    warning: Option<String>,
) -> Result<AiAnalysis> {
    let (model, effort) = match request.mode.as_str() {
        "classify" | "extract" => ("gpt-5.6-luna", "low"),
        "deep" => ("gpt-5.6-sol", "high"),
        _ => ("gpt-5.6-terra", "medium"),
    };
    let profile_id = store.get_setting("profile_safety_id").unwrap_or_else(|| {
        let id = hex::encode(Sha256::digest(b"ib45-primary-profile"));
        let _ = store.set_setting("profile_safety_id", &id);
        id
    });
    let system = format!(
        "You are the evidence-focused planning engine inside IB 45 Coach. Treat 45 as an aspiration, never a promise. Separate observed facts from inference. Do not invent grade boundaries, deadlines, citations, or completed work. Prefer the smallest realistic next actions that fit the stated capacity. Return only the requested JSON. {}",
        if request.assessed_work {
            INTEGRITY_WARNING
        } else {
            "For assessed work, identify integrity risks and never imply generated text is the student's own."
        }
    );
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "summary": {"type":"string"},
            "claims": {"type":"array","items":{"type":"string"}},
            "uncertainty": {"type":"string"},
            "evidence": {"type":"array","items":{"type":"string"}},
            "recommendedActions": {"type":"array","items":{"type":"string"}}
        },
        "required": ["summary","claims","uncertainty","evidence","recommendedActions"]
    });
    let payload = json!({
        "model": model,
        "store": false,
        "safety_identifier": &profile_id[..profile_id.len().min(64)],
        "reasoning": {"effort": effort, "context": "current_turn"},
        "text": {"verbosity":"medium", "format":{"type":"json_schema","name":"ib_coach_analysis","strict":true,"schema":schema}},
        "input": [
            {"role":"system","content":[{"type":"input_text","text":system}]},
            {"role":"user","content":[{"type":"input_text","text":format!("Request:\n{}\n\nVerified local context:\n{}", request.prompt, serde_json::to_string_pretty(&request.context).unwrap_or_default())}]}
        ]
    });
    let response = Client::builder()
        .timeout(Duration::from_secs(if request.mode == "deep" {
            180
        } else {
            90
        }))
        .build()?
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .context("Could not reach OpenAI")?;
    let status = response.status();
    let raw: Value = response
        .json()
        .await
        .context("OpenAI returned an unreadable response")?;
    if !status.is_success() {
        return Err(anyhow!(
            "OpenAI returned {}: {}",
            status,
            raw.get("error")
                .and_then(|value| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
        ));
    }
    let text = extract_output_text(&raw)
        .ok_or_else(|| anyhow!("OpenAI response did not contain output text"))?;
    let structured: Value =
        serde_json::from_str(text).context("OpenAI output was not valid structured JSON")?;
    Ok(to_analysis(
        "openai",
        model,
        &request.mode,
        structured,
        raw,
        warning,
    ))
}

async fn ollama_analysis(request: &AiRequest, warning: Option<String>) -> Result<AiAnalysis> {
    let prompt = format!(
        "You are the offline fallback for IB 45 Coach. Return JSON only with keys summary, claims, uncertainty, evidence, recommendedActions. Never promise a score. Never invent grade boundaries or evidence.\nRequest: {}\nContext: {}\n{}",
        request.prompt,
        serde_json::to_string(&request.context).unwrap_or_default(),
        if request.assessed_work {
            INTEGRITY_WARNING
        } else {
            ""
        }
    );
    let response = Client::builder()
        .timeout(Duration::from_secs(120))
        .build()?
        .post("http://127.0.0.1:11434/api/generate")
        .json(&json!({"model":"qwen3:4b","prompt":prompt,"stream":false,"format":"json"}))
        .send()
        .await
        .context(
            "Ollama is not available. Configure an OpenAI API key or start Ollama with qwen3:4b.",
        )?;
    if !response.status().is_success() {
        return Err(anyhow!("Ollama returned {}", response.status()));
    }
    let raw: Value = response.json().await?;
    let text = raw
        .get("response")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Ollama returned no response"))?;
    let structured: Value = serde_json::from_str(text).unwrap_or_else(|_| {
        json!({
            "summary": text,
            "claims": [],
            "uncertainty": "Offline fallback output could not be fully validated.",
            "evidence": [],
            "recommendedActions": []
        })
    });
    Ok(to_analysis(
        "ollama",
        "qwen3:4b",
        &request.mode,
        structured,
        raw,
        warning,
    ))
}

fn extract_output_text(value: &Value) -> Option<&str> {
    value
        .get("output")?
        .as_array()?
        .iter()
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .find(|content| content.get("type").and_then(Value::as_str) == Some("output_text"))
        .and_then(|content| content.get("text"))
        .and_then(Value::as_str)
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn to_analysis(
    provider: &str,
    model: &str,
    mode: &str,
    structured: Value,
    raw: Value,
    warning: Option<String>,
) -> AiAnalysis {
    AiAnalysis {
        id: Uuid::new_v4().to_string(),
        provider: provider.to_string(),
        model: model.to_string(),
        mode: mode.to_string(),
        summary: structured
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("Analysis completed.")
            .to_string(),
        claims: strings(structured.get("claims")),
        uncertainty: structured
            .get("uncertainty")
            .and_then(Value::as_str)
            .unwrap_or("Confidence has not been calibrated.")
            .to_string(),
        evidence: strings(structured.get("evidence")),
        recommended_actions: strings(structured.get("recommendedActions")),
        academic_integrity_warning: warning,
        raw,
        created_at: Utc::now().to_rfc3339(),
    }
}
