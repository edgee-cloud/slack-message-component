mod helpers;
mod world;

use std::collections::HashMap;

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
                helpers::error_response("Failed to parse component settings, missing Slack webhook URL", 500, resp);
                return;
            }
        };

        // read request body
        let request_body = match helpers::parse_body(req) {
            Ok(body) => body,
            Err(e) => {
                helpers::error_response(&e,400, resp);
                return;
            }
        };

        // parse body to JSON
        let body_json: serde_json::Value = match serde_json::from_slice(&request_body) {
            Ok(json) => json,
            Err(_) => {
                helpers::error_response("Invalid JSON in request body", 400, resp);
                return;
            }
        };

        // extract the message from the request body
        let message = match body_json.get("message") {
            Some(msg) => msg.to_string(),
            None => {
                helpers::error_response("Missing 'message' field in request body", 400, resp);
                return;
            }
        };

        // send 
        let slack_response = waki::Client::new()
            .post(&settings.webhook_url)
            .header("Content-Type", "application/json")
            .body(format!("{{\"text\": \"{message}\"}}"))
            .send()
            .unwrap();

        let response_status = slack_response.status_code();
        let response_body = String::from_utf8_lossy(&slack_response.body().unwrap_or_default()).to_string();
        
        let mut builder = helpers::ResponseBuilder::new();
        builder
            .set_header("content-type", "application/json")
            .set_status_code(response_status)
            .set_body(&response_body);

        builder.build(resp);
    }



}



#[derive(serde::Deserialize, serde::Serialize)]
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
}
