use std::{collections::HashSet, time::Duration};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditRiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditFinding {
    pub title: String,
    pub explanation: String,
    pub event_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuditAnalysis {
    pub summary: String,
    pub risk_level: AuditRiskLevel,
    pub findings: Vec<AuditFinding>,
}

#[derive(Debug, Error)]
pub enum AuditAnalysisError {
    #[error("AI provider request failed: {0}")]
    Provider(#[from] reqwest::Error),
    #[error("AI provider returned an invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Clone)]
pub struct AuditAnalyzer {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: [ChatMessage<'a>; 2],
    response_format: ResponseFormat,
    reasoning_effort: &'static str,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'static str,
    content: &'a str,
}

#[derive(Serialize)]
struct ResponseFormat {
    r#type: &'static str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalysisEvent<'a> {
    id: i64,
    action: &'a str,
    resource_type: &'a str,
    resource_id: Option<&'a str>,
    result: &'a str,
    reason_code: Option<&'a str>,
    created_at: &'a str,
}

impl AuditAnalyzer {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            model: model.into(),
        }
    }

    pub async fn analyze(
        &self,
        events: &[crate::AuditEventView],
    ) -> Result<AuditAnalysis, AuditAnalysisError> {
        if events.is_empty() {
            return Ok(AuditAnalysis {
                summary: "No audit events matched the current filters.".to_owned(),
                risk_level: AuditRiskLevel::Low,
                findings: Vec::new(),
            });
        }

        let safe_events = events
            .iter()
            .map(|event| AnalysisEvent {
                id: event.id,
                action: &event.action,
                resource_type: &event.resource_type,
                resource_id: event.resource_id.as_deref(),
                result: &event.result,
                reason_code: event.reason_code.as_deref(),
                created_at: &event.created_at,
            })
            .collect::<Vec<_>>();
        let content = serde_json::to_string(&safe_events)
            .map_err(|error| AuditAnalysisError::InvalidResponse(error.to_string()))?;
        let request = ChatRequest {
            model: &self.model,
            messages: [
                ChatMessage {
                    role: "system",
                    content: "Analyze the supplied audit events. Return only JSON with this exact shape: {\"summary\":string,\"riskLevel\":\"low\"|\"medium\"|\"high\",\"findings\":[{\"title\":string,\"explanation\":string,\"eventIds\":number[]}]}. Reference only supplied event IDs. Do not invent facts.",
                },
                ChatMessage {
                    role: "user",
                    content: &content,
                },
            ],
            response_format: ResponseFormat {
                r#type: "json_object",
            },
            reasoning_effort: "none",
            stream: false,
        };
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .timeout(Duration::from_secs(60))
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let response: ChatResponse = serde_json::from_str(&response)
            .map_err(|error| AuditAnalysisError::InvalidResponse(error.to_string()))?;
        let content = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| AuditAnalysisError::InvalidResponse("missing choice".to_owned()))?
            .message
            .content;
        let allowed_event_ids = events.iter().map(|event| event.id).collect();

        decode_analysis(&content, &allowed_event_ids)
    }
}

fn decode_analysis(
    content: &str,
    allowed_event_ids: &HashSet<i64>,
) -> Result<AuditAnalysis, AuditAnalysisError> {
    let analysis: AuditAnalysis = serde_json::from_str(content.trim())
        .map_err(|error| AuditAnalysisError::InvalidResponse(error.to_string()))?;

    for event_id in analysis
        .findings
        .iter()
        .flat_map(|finding| &finding.event_ids)
    {
        if !allowed_event_ids.contains(event_id) {
            return Err(AuditAnalysisError::InvalidResponse(format!(
                "unknown audit event {event_id}"
            )));
        }
    }

    Ok(analysis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_analysis_with_source_event_references() {
        let allowed_event_ids = HashSet::from([7, 9]);
        let content = r#"{
            "summary": "Two failed changes need review.",
            "riskLevel": "high",
            "findings": [{
                "title": "Repeated failures",
                "explanation": "The same action failed twice.",
                "eventIds": [7, 9]
            }]
        }"#;

        let analysis = decode_analysis(content, &allowed_event_ids).unwrap();

        assert_eq!(analysis.risk_level, AuditRiskLevel::High);
        assert_eq!(analysis.findings[0].event_ids, vec![7, 9]);
    }

    #[test]
    fn rejects_event_references_not_present_in_the_source() {
        let allowed_event_ids = HashSet::from([7]);
        let content = r#"{
            "summary": "One event needs review.",
            "riskLevel": "medium",
            "findings": [{
                "title": "Unexpected event",
                "explanation": "The model invented an event reference.",
                "eventIds": [8]
            }]
        }"#;

        let error = decode_analysis(content, &allowed_event_ids).unwrap_err();

        assert!(error.to_string().contains("unknown audit event 8"));
    }
}
