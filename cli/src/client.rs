use std::error::Error;
use std::fmt;
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::models::{Runtime, RuntimeCreate, RuntimeList, RuntimeStatus, RuntimeUpdate};

pub const DEFAULT_API_URL: &str = "http://127.0.0.1:8000";

#[derive(Debug)]
pub enum ApiError {
    InvalidBaseUrl(String),
    Transport(reqwest::Error),
    Response { status: StatusCode, detail: String },
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBaseUrl(url) => write!(formatter, "invalid API URL: {url}"),
            Self::Transport(error) => write!(formatter, "HTTP request failed: {error}"),
            Self::Response { status, detail } => {
                write!(formatter, "API returned {}: {detail}", status.as_u16())
            }
        }
    }
}

impl Error for ApiError {}

impl From<reqwest::Error> for ApiError {
    fn from(error: reqwest::Error) -> Self {
        Self::Transport(error)
    }
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    base_url: reqwest::Url,
    http: Client,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Result<Self, ApiError> {
        let normalized = format!("{}/", base_url.trim_end_matches('/'));
        let parsed = reqwest::Url::parse(&normalized)
            .map_err(|_| ApiError::InvalidBaseUrl(base_url.to_owned()))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(ApiError::InvalidBaseUrl(base_url.to_owned()));
        }
        let http = Client::builder().timeout(Duration::from_secs(30)).build()?;
        Ok(Self {
            base_url: parsed,
            http,
        })
    }

    pub fn create_runtime(&self, request: &RuntimeCreate) -> Result<Runtime, ApiError> {
        let response = self
            .http
            .post(self.endpoint("v1/runtimes"))
            .json(request)
            .send()?;
        parse_json(response)
    }

    pub fn list_runtimes(
        &self,
        limit: u32,
        offset: u64,
        status: Option<RuntimeStatus>,
    ) -> Result<RuntimeList, ApiError> {
        let mut request = self
            .http
            .get(self.endpoint("v1/runtimes"))
            .query(&[("limit", limit.to_string()), ("offset", offset.to_string())]);
        if let Some(status) = status {
            request = request.query(&[("status", status.to_string())]);
        }
        parse_json(request.send()?)
    }

    pub fn get_runtime(&self, runtime_id: &str) -> Result<Runtime, ApiError> {
        parse_json(self.http.get(self.runtime_endpoint(runtime_id)).send()?)
    }

    pub fn update_runtime(
        &self,
        runtime_id: &str,
        request: &RuntimeUpdate,
    ) -> Result<Runtime, ApiError> {
        let response = self
            .http
            .patch(self.runtime_endpoint(runtime_id))
            .json(request)
            .send()?;
        parse_json(response)
    }

    pub fn delete_runtime(&self, runtime_id: &str) -> Result<(), ApiError> {
        let response = self.http.delete(self.runtime_endpoint(runtime_id)).send()?;
        ensure_success(response).map(|_| ())
    }

    fn endpoint(&self, path: &str) -> reqwest::Url {
        self.base_url
            .join(path)
            .expect("static API paths must be valid")
    }

    fn runtime_endpoint(&self, runtime_id: &str) -> reqwest::Url {
        let mut endpoint = self.endpoint("v1/runtimes/");
        endpoint
            .path_segments_mut()
            .expect("HTTP API URL must support path segments")
            .pop_if_empty()
            .push(runtime_id);
        endpoint
    }
}

fn parse_json<T: DeserializeOwned>(response: Response) -> Result<T, ApiError> {
    ensure_success(response)?
        .json()
        .map_err(ApiError::Transport)
}

fn ensure_success(response: Response) -> Result<Response, ApiError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().unwrap_or_default();
    let detail = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|value| value.get("detail").cloned())
        .map(|detail| match detail {
            Value::String(message) => message,
            other => other.to_string(),
        })
        .filter(|detail| !detail.is_empty())
        .unwrap_or_else(|| {
            if body.is_empty() {
                "request failed without an error body".to_owned()
            } else {
                body
            }
        });
    Err(ApiError::Response { status, detail })
}
