use crate::{client::limited_text_snippet, DmApiError, Result};
use reqwest::{header::LOCATION, Response};
use std::collections::BTreeMap;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawResponse {
    pub url: Url,
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub location: Option<Url>,
    pub content_type: Option<String>,
    pub body_snippet: Option<String>,
}

impl RawResponse {
    pub async fn from_response(res: Response) -> Result<Self> {
        Self::from_response_with_body_limit(res, 2048).await
    }

    pub async fn from_response_with_body_limit(res: Response, body_limit: usize) -> Result<Self> {
        let url = res.url().clone();
        let status = res.status();
        let headers = res
            .headers()
            .iter()
            .filter_map(|(key, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (key.as_str().to_owned(), value.to_owned()))
            })
            .collect::<BTreeMap<_, _>>();
        let location = res
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(|location| url.join(location))
            .transpose()
            .map_err(|source| DmApiError::InvalidLocationHeader {
                endpoint: url.clone(),
                location: headers.get("location").cloned().unwrap_or_default(),
                source,
            })?;
        let content_type = headers.get("content-type").cloned();
        let body_snippet = if status.is_redirection() {
            None
        } else {
            res.text()
                .await
                .ok()
                .map(|body| limited_text_snippet(&body, body_limit))
        };

        Ok(Self {
            url,
            status: status.as_u16(),
            headers,
            location,
            content_type,
            body_snippet,
        })
    }
}
