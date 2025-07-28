use std::collections::HashMap;

use bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};

#[cfg(not(test))]
use waki::Response;

mod bindings {
    wit_bindgen::generate!({
        path: ".edgee/wit",
        world: "edge-function",
        generate_all,
        pub_export_macro: true,
        default_bindings_module: "$crate::bindings",
    });
}
mod helpers;

struct Component;
bindings::export!(Component);

impl bindings::exports::wasi::http::incoming_handler::Guest for Component {
    fn handle(req: IncomingRequest, resp: ResponseOutparam) {
        helpers::run_json(req, resp, Self::handle_json_request);
    }
}

impl Component {
    fn handle_json_request(
        req: http::Request<serde_json::Value>,
    ) -> Result<http::Response<serde_json::Value>, anyhow::Error> {
        let settings = Settings::from_req(&req)?;

        // Extract message from request body
        let request_body = req.body();
        let message = match request_body.get("message") {
            Some(value) => value.as_str().unwrap_or_default().to_string(),
            None => return Err(anyhow::anyhow!("Missing 'message' field in request body")),
        };

        // Build Slack API payload for simple text message and send it
        let slack_message_payload = SlackMessagePayload::new(message);
        let slack_response = slack_message_payload
            .send(&settings.webhook_url)
            .expect("Failed to send Slack message");

        // create response body based on Slack response's status code
        let response_status = slack_response.status_code();
        let component_response = SlackResponse::from_status(response_status);

        // note: Content-type is already set by helpers::run_json
        Ok(http::Response::builder()
            .status(response_status)
            .body(serde_json::json!(component_response))?)
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

    #[cfg(not(test))]
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

#[derive(serde::Deserialize, serde::Serialize)]
struct SlackResponse {
    ok: bool,
}

impl SlackResponse {
    fn from_status(status: u16) -> Self {
        Self { ok: status == 200 }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Settings {
    pub webhook_url: String,
}

impl Settings {
    pub fn new(headers: &http::header::HeaderMap) -> anyhow::Result<Self> {
        let value = headers
            .get("x-edgee-component-settings")
            .ok_or_else(|| anyhow::anyhow!("Missing 'x-edgee-component-settings' header"))
            .and_then(|value| value.to_str().map_err(Into::into))?;
        let data: HashMap<String, String> = serde_json::from_str(value)?;

        Ok(Self {
            webhook_url: data
                .get("webhook_url")
                .ok_or_else(|| anyhow::anyhow!("Missing webhook_url setting"))?
                .to_string(),
        })
    }

    pub fn from_req<B>(req: &http::Request<B>) -> anyhow::Result<Self> {
        Self::new(req.headers())
    }
}

#[cfg(test)]
mod tests {
    use http::{HeaderValue, Request};
    use lazy_static;
    use serde_json::json;
    use std::sync::Mutex;

    use super::*;

    // Patch SlackMessagePayload::send for this test
    lazy_static::lazy_static! {
        static ref SEND_CALLED: Mutex<bool> = Mutex::new(false);
    }

    // Mock SlackMessagePayload::send to avoid real HTTP call
    pub struct MockResponse;
    impl MockResponse {
        pub fn status_code(&self) -> u16 {
            200
        }
    }

    impl SlackMessagePayload {
        pub fn send(&self, _webhook_url: &str) -> anyhow::Result<MockResponse> {
            *SEND_CALLED.lock().unwrap() = true;
            Ok(MockResponse)
        }
    }

    #[test]
    fn test_settings_new() {
        let mut headers = http::header::HeaderMap::new();
        headers.insert(
            "x-edgee-component-settings",
            HeaderValue::from_static(r#"{"webhook_url": "test_value"}"#),
        );

        let settings = Settings::new(&headers).unwrap();
        assert_eq!(settings.webhook_url, "test_value");
    }

    #[test]
    fn test_settings_new_missing_header() {
        let headers = http::header::HeaderMap::new();
        let result = Settings::new(&headers);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing 'x-edgee-component-settings' header"
        );
    }

    #[test]
    fn test_settings_new_invalid_json() {
        let mut headers = http::header::HeaderMap::new();
        headers.insert(
            "x-edgee-component-settings",
            HeaderValue::from_static("not a json"),
        );
        let result = Settings::new(&headers);
        assert!(result.is_err());
    }

    #[test]
    fn test_settings_new_missing_webhook_url() {
        let mut headers = http::header::HeaderMap::new();
        headers.insert(
            "x-edgee-component-settings",
            HeaderValue::from_static(r#"{"not_webhook_url": "value"}"#),
        );
        let result = Settings::new(&headers);
        assert!(result.is_err());
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

    #[test]
    fn test_handle_json_request_success() {
        // Prepare request with headers and body
        let body = json!({ "message": "Hello, Slack!" });
        let req = Request::builder()
            .header(
                "x-edgee-component-settings",
                r#"{"webhook_url": "http://example.com/webhook"}"#,
            )
            .body(body)
            .unwrap();

        // Call the handler
        let result = Component::handle_json_request(req);

        // Assert
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body().to_string(), "{\"ok\":true}");
        assert!(*SEND_CALLED.lock().unwrap());
    }

    #[test]
    fn test_handle_json_request_missing_message() {
        let body = json!({});
        let req = Request::builder()
            .header(
                "x-edgee-component-settings",
                r#"{"webhook_url": "http://example.com/webhook"}"#,
            )
            .body(body)
            .unwrap();

        let result = Component::handle_json_request(req);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing 'message' field in request body"
        );
    }

    #[test]
    fn test_handle_json_request_invalid_settings() {
        let body = json!({ "message": "Test" });
        let req = Request::builder().body(body).unwrap();

        let result = Component::handle_json_request(req);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Missing 'x-edgee-component-settings' header"
        );
    }
}
