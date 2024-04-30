use futures::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest_eventsource::{Event, EventSource};
use serde_json::{json, Value};

use crate::util::DynResult;

fn parse_message(message: &str) -> DynResult<String> {
    let token: Option<String> = serde_json::from_str::<Value>(&message)
        .ok()
        .and_then(|data| {
            if !data["choices"][0]["finish_reason"].is_null() {
                return Some("".to_string());
            }

            data["choices"][0]["delta"]["content"]
                .as_str()
                .map(|token|
                    token.to_string())
        });

    if let Some(token) = token {
        return Ok(token.to_string());
    } else {
        return Err("Error parsing response.".into());
    }
}

pub fn fetch_tokens(
    api_key: &str,
    prompt: &str,
    system_prompt:&str 
) -> impl Stream<Item = DynResult<Option<String>>> {
    let mut headers = HeaderMap::new();
    headers.insert("Authorization", HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap());
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let request_builder = reqwest::Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .headers(headers)
        .body(json!({
            "model": "gpt-4-turbo",
            "max_tokens": 2048,
            "temperature": 1,
            "stream": true,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": prompt }
            ]
        }).to_string());

    let event_source = EventSource::new(request_builder).unwrap();
    return event_source.map(|event| {
        match event {
            Ok(Event::Open) => Ok(Some("".to_string())),
            Ok(Event::Message(message)) => {
                if message.data.trim() == "[DONE]" {
                    return Ok(None);
                }

                Ok(Some(parse_message(&message.data)?))
            }
            Err(reqwest_eventsource::Error::StreamEnded) => Ok(None),
            Err(error) => Err(error.to_string().into())
        }});
}