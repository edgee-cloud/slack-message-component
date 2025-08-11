use crate::bindings::wasi::http::types::IncomingBody;
use anyhow::Result;
use bytes::Bytes;

pub trait FromBody: Sized {
    fn from_data(data: Bytes) -> Result<Self>;

    fn from_body(body: IncomingBody) -> Result<Self> {
        Self::from_data(body.read()?)
    }
}

pub trait IntoBody: Sized {
    fn into_body(self) -> Result<Bytes>;

    #[allow(unused_variables)]
    fn extend_response_parts(&self, parts: &mut http::response::Parts) {}
}

impl FromBody for IncomingBody {
    fn from_data(_: Bytes) -> Result<Self> {
        unimplemented!("Should never be called")
    }

    fn from_body(body: IncomingBody) -> Result<Self> {
        Ok(body)
    }
}

impl FromBody for Bytes {
    fn from_data(data: Bytes) -> Result<Self> {
        Ok(data)
    }
}

impl IntoBody for Bytes {
    fn into_body(self) -> Result<Bytes> {
        Ok(self)
    }
}

impl FromBody for () {
    fn from_data(_: Bytes) -> Result<Self> {
        Ok(())
    }

    fn from_body(_: IncomingBody) -> Result<Self> {
        Ok(())
    }
}

impl IntoBody for () {
    fn into_body(self) -> Result<Bytes> {
        Ok(Bytes::new())
    }
}

impl FromBody for String {
    fn from_data(data: Bytes) -> Result<Self> {
        String::from_utf8(data.into()).map_err(Into::into)
    }
}

impl IntoBody for String {
    fn into_body(self) -> Result<Bytes> {
        Ok(Bytes::from(self))
    }
}

impl<T: FromBody> FromBody for Option<T> {
    fn from_data(data: Bytes) -> Result<Self> {
        if data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(T::from_data(data)?))
        }
    }
}

impl<T: IntoBody> IntoBody for Option<T> {
    fn into_body(self) -> Result<Bytes> {
        match self {
            Some(value) => value.into_body(),
            None => Ok(Bytes::new()),
        }
    }

    fn extend_response_parts(&self, parts: &mut http::response::Parts) {
        if let Some(value) = self {
            value.extend_response_parts(parts);
        }
    }
}

// Data types

#[derive(Debug, Clone)]
pub struct Json<T>(pub T);

impl<T: serde::de::DeserializeOwned> FromBody for Json<T> {
    fn from_data(bytes: Bytes) -> Result<Self> {
        let data = serde_json::from_slice(&bytes)?;
        Ok(Self(data))
    }
}

impl<T: serde::Serialize> IntoBody for Json<T> {
    fn into_body(self) -> Result<Bytes> {
        use bytes::{BufMut, BytesMut};

        let mut buf = BytesMut::with_capacity(128).writer();
        serde_json::to_writer(&mut buf, &self.0)?;
        Ok(buf.into_inner().freeze())
    }

    fn extend_response_parts(&self, parts: &mut http::response::Parts) {
        parts
            .headers
            .entry(http::header::CONTENT_TYPE)
            .or_insert(http::HeaderValue::from_static("application/json"));
    }
}

#[derive(Debug, Clone)]
pub struct RawJson<T>(pub T);

impl<T: Into<Bytes>> IntoBody for RawJson<T> {
    fn into_body(self) -> Result<Bytes> {
        Ok(self.0.into())
    }

    fn extend_response_parts(&self, parts: &mut http::response::Parts) {
        Json(()).extend_response_parts(parts)
    }
}

#[derive(Debug, Clone)]
pub struct Html<T>(pub T);

impl<T: Into<Bytes>> IntoBody for Html<T> {
    fn into_body(self) -> Result<Bytes> {
        Ok(self.0.into())
    }

    fn extend_response_parts(&self, parts: &mut http::response::Parts) {
        parts
            .headers
            .entry(http::header::CONTENT_TYPE)
            .or_insert(http::HeaderValue::from_static("text/html; charset=utf-8"));
    }
}
