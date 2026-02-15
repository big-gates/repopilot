//! Provider HTTP API 호출 공용 유틸리티.

use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::{Client, RequestBuilder};
use serde_json::Value;

/// Provider API 호출용 기본 HTTP 클라이언트를 생성한다.
pub fn build_api_client() -> Client {
    // TLS 설정 실패 등 예외 상황에서는 기본 클라이언트로 폴백한다.
    Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| Client::new())
}

/// JSON 응답을 기대하는 요청을 전송하고 실패/파싱 오류를 표준화한다.
pub async fn send_json(
    provider_name: &str,
    action: &str,
    request: RequestBuilder,
) -> Result<Value> {
    let response = request
        .send()
        .await
        .with_context(|| format!("{provider_name}: failed to {action}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("{provider_name}: failed to read {action} response body"))?;

    if !status.is_success() {
        bail!("{provider_name}: {action} failed ({status}): {body}");
    }

    serde_json::from_str(&body)
        .with_context(|| format!("{provider_name}: invalid JSON response while {action}"))
}

/// API 응답 구조에서 텍스트를 재귀적으로 추출한다.
pub fn collect_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.to_string(),
        Value::Array(items) => items
            .iter()
            .map(collect_text)
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(""),
        Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(Value::as_str) {
                return text.to_string();
            }
            if let Some(message) = map.get("message") {
                let text = collect_text(message);
                if !text.is_empty() {
                    return text;
                }
            }
            if let Some(content) = map.get("content") {
                let text = collect_text(content);
                if !text.is_empty() {
                    return text;
                }
            }
            if let Some(output_text) = map.get("output_text").and_then(Value::as_str) {
                return output_text.to_string();
            }
            String::new()
        }
        _ => String::new(),
    }
}
