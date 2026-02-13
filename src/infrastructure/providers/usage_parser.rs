//! Provider CLI 출력에서 토큰 사용량을 best-effort로 추출한다.

use crate::domain::review::TokenUsage;

pub(super) fn parse_usage(stdout: &str, stderr: &str) -> TokenUsage {
    // CLI마다 출력 형식이 달라 best-effort 방식으로 숫자를 추출한다.
    let merged = format!("{}\n{}", stdout, stderr);
    let prompt = extract_metric(
        &merged,
        &[
            "prompt_tokens",
            "prompt tokens",
            "input_tokens",
            "input tokens",
        ],
    );
    let completion = extract_metric(
        &merged,
        &[
            "completion_tokens",
            "completion tokens",
            "output_tokens",
            "output tokens",
        ],
    );
    let mut total = extract_metric(&merged, &["total_tokens", "total tokens", "tokens total"]);

    if total.is_none() {
        total = match (prompt, completion) {
            (Some(p), Some(c)) => Some(p + c),
            _ => None,
        };
    }

    TokenUsage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        total_tokens: total,
    }
}

fn extract_metric(text: &str, keys: &[&str]) -> Option<u64> {
    let lower = text.to_lowercase();
    for line in lower.lines() {
        for key in keys {
            if line.contains(key) && let Some(v) = first_number(line) {
                return Some(v);
            }
        }
    }

    for key in keys {
        if let Some(idx) = lower.find(key) {
            let tail = &lower[idx + key.len()..];
            if let Some(v) = first_number(tail) {
                return Some(v);
            }
        }
    }

    None
}

fn first_number(s: &str) -> Option<u64> {
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            break;
        }
    }

    if cur.is_empty() {
        None
    } else {
        cur.parse::<u64>().ok()
    }
}
