use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};

use anyhow::Result;
use serde_json::Value;

use super::{
    Connector, DetectionResult, DiscoveredSourceFile, NormalizedConversation, NormalizedMessage,
    ScanContext, parse_timestamp, reindex_messages,
};

const MAX_INDEXED_TOOL_OUTPUT_CHARS: usize = 128 * 1024;

pub struct CodexConnector {
    inner: franken_agent_detection::CodexConnector,
}

impl Default for CodexConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexConnector {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: franken_agent_detection::CodexConnector::new(),
        }
    }
}

impl Connector for CodexConnector {
    fn detect(&self) -> DetectionResult {
        self.inner.detect()
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
        let mut conversations = self.inner.scan(ctx)?;
        for conversation in &mut conversations {
            augment_modern_codex_messages(conversation);
        }
        Ok(conversations)
    }

    fn supports_streaming_scan(&self) -> bool {
        self.inner.supports_streaming_scan()
    }

    fn discover_source_files(&self, ctx: &ScanContext) -> Result<Vec<DiscoveredSourceFile>> {
        self.inner.discover_source_files(ctx)
    }

    fn scan_with_callback(
        &self,
        ctx: &ScanContext,
        on_conversation: &mut dyn FnMut(NormalizedConversation) -> Result<()>,
    ) -> Result<()> {
        self.inner.scan_with_callback(ctx, &mut |mut conversation| {
            augment_modern_codex_messages(&mut conversation);
            on_conversation(conversation)
        })
    }
}

fn augment_modern_codex_messages(conversation: &mut NormalizedConversation) {
    if conversation
        .source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_none_or(|ext| !ext.eq_ignore_ascii_case("jsonl"))
    {
        return;
    }

    let Ok(file) = File::open(&conversation.source_path) else {
        return;
    };

    let mut seen_messages: HashSet<ModernCodexMessageSignature> = conversation
        .messages
        .iter()
        .map(modern_codex_message_signature)
        .collect();
    let mut seen_call_ids: HashSet<String> = conversation
        .messages
        .iter()
        .flat_map(modern_codex_message_call_ids)
        .collect();
    let mut seen_raw_entries: HashSet<[u8; 32]> = conversation
        .messages
        .iter()
        .map(|message| modern_codex_raw_signature(&message.extra))
        .collect();
    let mut added = false;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(raw) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let raw_signature = modern_codex_raw_signature(&raw);
        if seen_raw_entries.contains(&raw_signature) {
            continue;
        }
        let Some(message) = modern_codex_message(&raw) else {
            continue;
        };
        if message_already_indexed(&seen_messages, &seen_call_ids, &message) {
            seen_raw_entries.insert(raw_signature);
            continue;
        }
        seen_messages.insert(modern_codex_message_signature(&message));
        seen_call_ids.extend(modern_codex_message_call_ids(&message));
        seen_raw_entries.insert(raw_signature);
        conversation.messages.push(message);
        added = true;
    }

    if added {
        conversation.messages.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.idx.cmp(&right.idx))
        });
        reindex_messages(&mut conversation.messages);
    }
}

fn modern_codex_message(raw: &Value) -> Option<NormalizedMessage> {
    let entry_type = raw.get("type").and_then(Value::as_str)?;
    let payload = raw.get("payload")?;
    let created_at = raw.get("timestamp").and_then(parse_timestamp);

    match entry_type {
        "response_item" => response_item_message(payload, created_at, raw),
        "event_msg" => event_message(payload, created_at, raw),
        _ => None,
    }
}

fn response_item_message(
    payload: &Value,
    created_at: Option<i64>,
    raw: &Value,
) -> Option<NormalizedMessage> {
    match payload.get("type").and_then(Value::as_str) {
        Some("message") | None => {
            let content = payload.get("content").and_then(flatten_modern_content)?;
            let role = payload
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("agent")
                .to_string();
            Some(normalized_message(
                role,
                None,
                created_at,
                content,
                raw.clone(),
                payload.get("content").map_or_else(
                    Vec::new,
                    franken_agent_detection::extract_invocations_from_content_blocks,
                ),
            ))
        }
        Some("function_call") => {
            let tool_name = payload
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let arguments = payload.get("arguments").cloned();
            let content = tool_call_content(tool_name, arguments.as_ref());
            let call_id = payload
                .get("call_id")
                .or_else(|| payload.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string);
            Some(normalized_message(
                "assistant".to_string(),
                None,
                created_at,
                content,
                raw.clone(),
                vec![franken_agent_detection::NormalizedInvocation {
                    kind: "tool".to_string(),
                    name: tool_name.to_string(),
                    raw_name: None,
                    call_id,
                    arguments: arguments.and_then(normalize_invocation_arguments),
                }],
            ))
        }
        Some("function_call_output") => {
            let output = payload.get("output").and_then(Value::as_str)?;
            let call_id = payload.get("call_id").and_then(Value::as_str);
            Some(normalized_message(
                "tool".to_string(),
                None,
                created_at,
                tool_output_content(call_id, output),
                raw.clone(),
                Vec::new(),
            ))
        }
        _ => None,
    }
}

fn event_message(
    payload: &Value,
    created_at: Option<i64>,
    raw: &Value,
) -> Option<NormalizedMessage> {
    match payload.get("type").and_then(Value::as_str) {
        Some("agent_message") => {
            let content = payload
                .get("message")
                .or_else(|| payload.get("text"))
                .and_then(Value::as_str)?
                .trim()
                .to_string();
            non_empty_message("assistant".to_string(), None, created_at, content, raw)
        }
        Some("tool_result") => {
            let output = payload
                .get("output")
                .or_else(|| payload.get("result"))
                .and_then(Value::as_str)?;
            let call_id = payload
                .get("call_id")
                .or_else(|| payload.get("id"))
                .and_then(Value::as_str);
            Some(normalized_message(
                "tool".to_string(),
                None,
                created_at,
                tool_output_content(call_id, output),
                raw.clone(),
                Vec::new(),
            ))
        }
        _ => None,
    }
}

fn normalized_message(
    role: String,
    author: Option<String>,
    created_at: Option<i64>,
    content: String,
    extra: Value,
    invocations: Vec<franken_agent_detection::NormalizedInvocation>,
) -> NormalizedMessage {
    NormalizedMessage {
        idx: 0,
        role,
        author,
        created_at,
        content,
        extra,
        invocations,
        snippets: Vec::new(),
    }
}

fn non_empty_message(
    role: String,
    author: Option<String>,
    created_at: Option<i64>,
    content: String,
    raw: &Value,
) -> Option<NormalizedMessage> {
    (!content.trim().is_empty())
        .then(|| normalized_message(role, author, created_at, content, raw.clone(), Vec::new()))
}

fn flatten_modern_content(content: &Value) -> Option<String> {
    if let Some(text) = content
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        return Some(text.to_string());
    }

    let mut parts = Vec::new();
    for item in content.as_array()? {
        let text = modern_content_part_text(item);

        let text = text.trim();
        if !text.is_empty() {
            parts.push(text.to_string());
        }
    }

    (!parts.is_empty()).then(|| parts.join("\n"))
}

fn modern_content_part_text(item: &Value) -> String {
    if let Some(text) = item.as_str() {
        return text.to_string();
    }

    let item_type = item.get("type").and_then(Value::as_str);
    if matches!(
        item_type,
        None | Some("text") | Some("input_text") | Some("output_text")
    ) {
        return item
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
    }

    if item_type == Some("tool_use") {
        let tool_name = item
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let detail = item
            .get("input")
            .and_then(|input| {
                input
                    .get("description")
                    .or_else(|| input.get("file_path"))
                    .or_else(|| input.get("path"))
                    .or_else(|| input.get("command"))
            })
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        return if detail.is_empty() {
            format!("[Tool: {tool_name}]")
        } else {
            format!("[Tool: {tool_name} - {detail}]")
        };
    }

    String::new()
}

fn tool_call_content(tool_name: &str, arguments: Option<&Value>) -> String {
    let mut content = format!("[Tool: {tool_name}]");
    if let Some(arguments) = arguments.and_then(argument_text) {
        content.push('\n');
        content.push_str(&arguments);
    }
    content
}

fn tool_output_content(call_id: Option<&str>, output: &str) -> String {
    let label = call_id.map_or_else(
        || "[Tool output]".to_string(),
        |id| format!("[Tool output: {id}]"),
    );
    let output = truncate_tool_output(output.trim());
    if output.is_empty() {
        label
    } else {
        format!("{label}\n{output}")
    }
}

fn argument_text(arguments: &Value) -> Option<String> {
    let text = match arguments {
        Value::String(text) => text.trim().to_string(),
        other => serde_json::to_string(other).ok()?,
    };
    (!text.is_empty()).then_some(text)
}

fn normalize_invocation_arguments(arguments: Value) -> Option<Value> {
    match arguments {
        Value::String(text) => serde_json::from_str(&text)
            .ok()
            .or_else(|| (!text.trim().is_empty()).then_some(Value::String(text))),
        Value::Null => None,
        other => Some(other),
    }
}

fn truncate_tool_output(output: &str) -> String {
    let mut truncated = String::new();
    let mut chars = output.chars();
    for _ in 0..MAX_INDEXED_TOOL_OUTPUT_CHARS {
        let Some(ch) = chars.next() else {
            return output.to_string();
        };
        truncated.push(ch);
    }
    let omitted = chars.count();
    truncated.push_str(&format!(
        "\n[truncated {omitted} additional chars from tool output]"
    ));
    truncated
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ModernCodexMessageSignature {
    role: String,
    author: Option<String>,
    created_at: Option<i64>,
    content_hash: [u8; 32],
}

fn modern_codex_message_signature(message: &NormalizedMessage) -> ModernCodexMessageSignature {
    ModernCodexMessageSignature {
        role: message.role.clone(),
        author: message.author.clone(),
        created_at: message.created_at,
        content_hash: *blake3::hash(message.content.as_bytes()).as_bytes(),
    }
}

fn modern_codex_raw_signature(raw: &Value) -> [u8; 32] {
    let mut bytes = Vec::new();
    if serde_json::to_writer(&mut bytes, raw).is_err() {
        bytes.extend_from_slice(raw.to_string().as_bytes());
    }
    *blake3::hash(&bytes).as_bytes()
}

fn modern_codex_message_call_ids(message: &NormalizedMessage) -> impl Iterator<Item = String> + '_ {
    message
        .invocations
        .iter()
        .filter_map(|invocation| invocation.call_id.clone())
}

fn message_already_indexed(
    seen_messages: &HashSet<ModernCodexMessageSignature>,
    seen_call_ids: &HashSet<String>,
    candidate: &NormalizedMessage,
) -> bool {
    seen_messages.contains(&modern_codex_message_signature(candidate))
        || candidate
            .invocations
            .iter()
            .filter_map(|invocation| invocation.call_id.as_deref())
            .any(|call_id| seen_call_ids.contains(call_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(content: &str, call_id: Option<&str>) -> NormalizedMessage {
        NormalizedMessage {
            idx: 0,
            role: "assistant".to_string(),
            author: None,
            created_at: Some(1_700_000_000_000),
            content: content.to_string(),
            extra: Value::Null,
            invocations: call_id
                .map(|call_id| {
                    vec![franken_agent_detection::NormalizedInvocation {
                        kind: "tool".to_string(),
                        name: "shell".to_string(),
                        raw_name: None,
                        call_id: Some(call_id.to_string()),
                        arguments: None,
                    }]
                })
                .unwrap_or_default(),
            snippets: Vec::new(),
        }
    }

    #[test]
    fn modern_codex_duplicate_detection_uses_precomputed_sets() {
        let existing = message("canonical response", Some("call-1"));
        let mut seen_messages = HashSet::from([modern_codex_message_signature(&existing)]);
        let mut seen_call_ids: HashSet<String> = modern_codex_message_call_ids(&existing).collect();

        assert!(message_already_indexed(
            &seen_messages,
            &seen_call_ids,
            &message("canonical response", None)
        ));
        assert!(message_already_indexed(
            &seen_messages,
            &seen_call_ids,
            &message("same tool call, changed wording", Some("call-1"))
        ));

        let fresh = message("fresh response", Some("call-2"));
        assert!(!message_already_indexed(
            &seen_messages,
            &seen_call_ids,
            &fresh
        ));
        seen_messages.insert(modern_codex_message_signature(&fresh));
        seen_call_ids.extend(modern_codex_message_call_ids(&fresh));
        assert!(message_already_indexed(
            &seen_messages,
            &seen_call_ids,
            &fresh
        ));
    }
}
