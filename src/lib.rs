mod helpers;
mod world;

use std::collections::HashMap;

use waki::Response;
use world::bindings::exports::wasi::http::incoming_handler::Guest;
use world::bindings::wasi::http::types::IncomingRequest;
use world::bindings::wasi::http::types::ResponseOutparam;
use world::bindings::Component;

impl Guest for Component {
    fn handle(req: IncomingRequest, resp: ResponseOutparam) {
        // check if settings are valid
        let settings = match Settings::from_req(&req) {
            Ok(settings) => settings,
            Err(_) => {
                let response = helpers::build_response_json_error(
                    "Failed to parse component settings, missing Slack webhook URL",
                    500,
                );
                response.send(resp);
                return;
            }
        };

        // read request body
        let request_body = match helpers::parse_body(req) {
            Ok(body) => body,
            Err(e) => {
                let response = helpers::build_response_json_error(&e, 400);
                response.send(resp);
                return;
            }
        };

        // parse body to JSON
        let body_json: serde_json::Value = match serde_json::from_slice(&request_body) {
            Ok(json) => json,
            Err(_) => {
                let response =
                    helpers::build_response_json_error("Invalid JSON in request body", 400);
                response.send(resp);
                return;
            }
        };

        // extract message from request body
        let message = match body_json.get("message") {
            Some(value) => value.as_str().unwrap_or("").to_string(), // this removes quotes and converts to String
            None => {
                let response = helpers::build_response_json_error(
                    "Missing 'message' field in request body",
                    400,
                );
                response.send(resp);
                return;
            }
        };

        // build Slack API payload for simple text message and send it
        let slack_message_payload = SlackMessagePayload::new(message.clone());
        let slack_response = slack_message_payload.send(&settings.webhook_url);

        // handle error in case request couldn't be sent
        if let Err(e) = slack_response {
            let response = helpers::build_response_json_error(&e.to_string(), 500);
            response.send(resp);
            return;
        }

        let slack_response = slack_response.unwrap();
        let response_status = slack_response.status_code();
        let response_body =
            String::from_utf8_lossy(&slack_response.body().unwrap_or_default()).to_string();

        let response = helpers::build_response_json(&response_body, response_status);
        response.send(resp);
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct SlackMessagePayload {
    text: String,
}

impl SlackMessagePayload {
    fn new(text: String) -> Self {
        Self { text }
    }

    fn send(&self, webhook_url: &str) -> anyhow::Result<Response> {
        let client = waki::Client::new();
        let response = client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_vec(self)?)
            .send()?;
        Ok(response)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Settings {
    pub webhook_url: String,
}

impl Settings {
    pub fn from_req(req: &IncomingRequest) -> anyhow::Result<Self> {
        let map = helpers::parse_headers(&IncomingRequest::headers(req));
        Self::new(&map)
    }

    pub fn new(headers: &HashMap<String, Vec<String>>) -> anyhow::Result<Self> {
        let settings = headers
            .get("x-edgee-component-settings")
            .ok_or_else(|| anyhow::anyhow!("Missing 'x-edgee-component-settings' header"))?;

        if settings.len() != 1 {
            return Err(anyhow::anyhow!(
                "Expected exactly one 'x-edgee-component-settings' header, found {}",
                settings.len()
            ));
        }
        let setting = settings[0].clone();
        let setting: HashMap<String, String> = serde_json::from_str(&setting)?;

        let webhook_url = setting
            .get("webhook_url")
            .map(String::to_string)
            .unwrap_or_default();

        Ok(Self { webhook_url })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_new() {
        let mut headers = HashMap::new();
        headers.insert(
            "x-edgee-component-settings".to_string(),
            vec![r#"{"webhook_url": "test_value"}"#.to_string()],
        );

        let settings = Settings::new(&headers).unwrap();
        assert_eq!(settings.webhook_url, "test_value");
    }

    #[test]
    fn test_settings_new_missing_header() {
        let headers = HashMap::new();
        let result = Settings::new(&headers);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing 'x-edgee-component-settings' header"
        );
    }

    #[test]
    fn test_settings_new_multiple_headers() {
        let mut headers = HashMap::new();
        headers.insert(
            "x-edgee-component-settings".to_string(),
            vec![
                r#"{"webhook_url": "test_value"}"#.to_string(),
                r#"{"webhook_url": "another_value"}"#.to_string(),
            ],
        );
        let result = Settings::new(&headers);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Expected exactly one 'x-edgee-component-settings' header"));
    }

    #[test]
    fn test_settings_new_invalid_json() {
        let mut headers = HashMap::new();
        headers.insert(
            "x-edgee-component-settings".to_string(),
            vec!["not a json".to_string()],
        );
        let result = Settings::new(&headers);
        assert!(result.is_err());
    }

    #[test]
    fn test_settings_new_missing_webhook_url() {
        let mut headers = HashMap::new();
        headers.insert(
            "x-edgee-component-settings".to_string(),
            vec![r#"{"not_webhook_url": "value"}"#.to_string()],
        );
        let settings = Settings::new(&headers).unwrap();
        assert_eq!(settings.webhook_url, "");
    }

    #[test]
    fn test_slack_message_payload_new() {
        let payload = SlackMessagePayload::new("Hello, Slack!".to_string());
        assert_eq!(payload.text, "Hello, Slack!");
    }

    #[test]
    fn test_slack_message_payload_serialize() {
        let payload = SlackMessagePayload::new("Test message".to_string());
        let json = serde_json::to_string(&payload).unwrap();
        assert_eq!(json, r#"{"text":"Test message"}"#);
    }
}
