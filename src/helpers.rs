use crate::world::bindings::wasi::http::types::{
    Fields, IncomingRequest, OutgoingBody, OutgoingResponse,
};

use crate::world::bindings::exports::wasi::http::incoming_handler::ResponseOutparam;
use crate::world::bindings::wasi::io::streams::StreamError;
use std::collections::HashMap;

pub struct ResponseBuilder {
    headers: Fields,
    status_code: u16,
    body_content: Option<String>,
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        ResponseBuilder::new()
    }
}

impl ResponseBuilder {
    pub fn new() -> Self {
        ResponseBuilder {
            headers: Fields::new(),
            status_code: 200,
            body_content: None,
        }
    }

    pub fn set_header(&mut self, key: &str, value: &str) -> &mut Self {
        let _ = self
            .headers
            .set(key, vec![value.as_bytes().to_vec()].as_slice());
        self
    }

    pub fn set_status_code(&mut self, status_code: u16) -> &mut Self {
        self.status_code = status_code;
        self
    }

    pub fn set_body(&mut self, body: &str) -> &mut Self {
        self.body_content = Some(body.to_string());
        self
    }

    pub fn build(self, resp: ResponseOutparam) {
        let resp_tx = OutgoingResponse::new(self.headers);
        let _ = resp_tx.set_status_code(self.status_code);

        let body = resp_tx.body().unwrap();
        ResponseOutparam::set(resp, Ok(resp_tx));
        let stream = body.write().unwrap();
        if let Some(body_content) = self.body_content {
            stream.write(body_content.as_bytes()).unwrap();
        }
        drop(stream);
        let _ = OutgoingBody::finish(body, None);
    }
}

pub fn parse_headers(headers: &Fields) -> HashMap<String, Vec<String>> {
    let mut output: HashMap<String, Vec<String>> = HashMap::new();
    for (header_name, header_value) in headers.entries() {
        let header_name = header_name.to_string();
        let header_value = String::from_utf8_lossy(&header_value).to_string();
        output
            .entry(header_name.clone())
            .or_default()
            .push(header_value);
    }

    output
}

pub fn parse_body(req: IncomingRequest) -> Result<Vec<u8>, String> {
    let mut request_body = Vec::new();
    let stream = match req.consume() {
        Ok(stream) => stream,
        Err(e) => {
            return Err(format!("Failed to consume request stream"));
        }
    };
    let stream = match stream.stream() {
        Ok(stream) => stream,
        Err(e) => {
            return Err(format!("Failed to get request stream: "));
        }
    };

    loop {
        match stream.read(4096) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                request_body.extend_from_slice(&chunk);
            }
            Err(StreamError::Closed) => {
                // Stream is closed, we can stop reading
                break;
            }
            Err(e) => {
                return Err(format!("Failed to read from request stream: {e}"));
            }
        }
    }
    Ok(request_body)
}

pub fn error_response(msg: &str, status_code: u16, resp: ResponseOutparam) {
    let mut builder = ResponseBuilder::new();
    builder
        .set_header("content-type", "application/json")
        .set_status_code(status_code)
        .set_body(&format!("{{\"error\": \"{msg}\"}}"));
    builder.build(resp);
    return;
}
