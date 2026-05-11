use crate::{
    raw::RawResponse, ContentCount, ContentQuery, Credentials, DmApiError, DownloadByteRange,
    DownloadFile, DownloadFileKind, DownloadPlan, DownloadResolution, DownloadStreamRequest,
    DownloadUnavailableReason, Purchase, Result, SerialDownloadPage, SerialNumber, SessionSnapshot,
    SessionStatus, SplitDownloadPage, SplitDownloadPart, Work, WorkId, WorksResponse,
    DEFAULT_WORKS_BATCH_LIMIT,
};
use bytes::Bytes;
use cookie_store::CookieStore;
use futures_core::Stream;
use reqwest::{
    header::{HeaderMap, HeaderValue, CONTENT_TYPE, LOCATION, RANGE},
    redirect::Policy,
    Client, Response, StatusCode,
};
use reqwest_cookie_store::CookieStoreMutex;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{collections::BTreeMap, io::BufWriter, sync::Arc};
use tokio::sync::Mutex;
use url::Url;

const LOGIN_URL: &str = "https://login.dlsite.com/login";
const LOGIN_SKIP_URL: &str = "https://www.dlsite.com/home/login/=/skip_register/1";
const LOGIN_FINISH_URL: &str = "https://www.dlsite.com/home/login/finish";
const CONTENT_COUNT_URL: &str = "https://play.dlsite.com/api/v3/content/count";
const CONTENT_SALES_URL: &str = "https://play.dlsite.com/api/v3/content/sales";
const CONTENT_WORKS_URL: &str = "https://play.dlsite.com/api/v3/content/works";
const DOWNLOAD_URL: &str = "https://play.dlsite.com/api/v3/download";
const HOME_SERIAL_URL: &str = "https://www.dlsite.com/home/serial/=/product_id/";
const MAX_DOWNLOAD_REDIRECTS: usize = 8;
const DOWNLOAD_PAGE_BODY_LIMIT: usize = 512 * 1024;

#[derive(Debug, Clone)]
pub struct DlsiteClientConfig {
    pub user_agent: String,
}

impl Default for DlsiteClientConfig {
    fn default() -> Self {
        Self {
            user_agent: concat!("dlsite-manager/", env!("CARGO_PKG_VERSION")).to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct DlsiteClient {
    http: Client,
    cookie_store: Arc<CookieStoreMutex>,
    works_batch_limit: Arc<Mutex<Option<usize>>>,
}

impl DlsiteClient {
    pub fn new(config: DlsiteClientConfig) -> Result<Self> {
        let cookie_store = Arc::new(CookieStoreMutex::new(CookieStore::default()));
        let http = Client::builder()
            .cookie_provider(cookie_store.clone())
            .redirect(Policy::none())
            .user_agent(config.user_agent)
            .build()?;

        Ok(Self {
            http,
            cookie_store,
            works_batch_limit: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn login(&self, credentials: &Credentials) -> Result<SessionSnapshot> {
        self.http
            .get(LOGIN_URL)
            .query(&[("user", "self")])
            .send()
            .await?
            .error_for_status()?;

        let xsrf_token = self.xsrf_token()?;

        let auth_res = self
            .http
            .post(LOGIN_URL)
            .form(&[
                ("login_id", credentials.username.as_str()),
                ("password", credentials.password.as_str()),
                ("_token", xsrf_token.as_str()),
            ])
            .send()
            .await?;

        if auth_res.status() != StatusCode::FOUND {
            let endpoint = auth_res.url().clone();
            let status = auth_res.status();
            let body_snippet = response_text_snippet(auth_res).await;

            if body_snippet
                .as_deref()
                .is_some_and(|body| body.contains("ログインIDかパスワードが間違っています。"))
            {
                return Err(DmApiError::InvalidCredentials);
            }

            return Err(DmApiError::UnexpectedStatus {
                endpoint,
                status,
                body_snippet,
            });
        }

        let login_res = self.http.get(LOGIN_URL).send().await?;
        let login_res_status = login_res.status();
        let login_res_endpoint = login_res.url().clone();
        let login_res_body = response_text_snippet(login_res).await;

        if login_res_body
            .as_deref()
            .is_some_and(|body| body.contains("ログインIDかパスワードが間違っています。"))
        {
            return Err(DmApiError::InvalidCredentials);
        }

        if login_res_status != StatusCode::OK && login_res_status != StatusCode::FOUND {
            return Err(DmApiError::UnexpectedStatus {
                endpoint: login_res_endpoint,
                status: login_res_status,
                body_snippet: login_res_body,
            });
        }

        let skip_location = self.redirect_location_from_get(LOGIN_SKIP_URL).await?;
        let oauth_request_location = self.redirect_location_from_get(skip_location).await?;

        self.redirect_location_from_get(oauth_request_location)
            .await?;

        self.http
            .get(LOGIN_FINISH_URL)
            .send()
            .await?
            .error_for_status()?;

        match self.validate_session().await? {
            SessionStatus::Authorized => self.export_session(),
            SessionStatus::Unauthorized => Err(DmApiError::InvalidCredentials),
        }
    }

    pub fn export_session(&self) -> Result<SessionSnapshot> {
        let mut writer = BufWriter::new(Vec::new());
        let guard = self
            .cookie_store
            .lock()
            .map_err(|_| DmApiError::CookieStore("cookie store mutex is poisoned".to_owned()))?;

        cookie_store::serde::json::save(&guard, &mut writer)
            .map_err(|err| DmApiError::CookieStore(format!("{err:?}")))?;

        drop(guard);

        let bytes = writer
            .into_inner()
            .map_err(|err| DmApiError::CookieStore(format!("{err:?}")))?;
        let cookies_json =
            String::from_utf8(bytes).map_err(|err| DmApiError::CookieStore(format!("{err:?}")))?;

        Ok(SessionSnapshot { cookies_json })
    }

    pub fn import_session(&self, snapshot: &SessionSnapshot) -> Result<()> {
        let parsed = cookie_store::serde::json::load(snapshot.cookies_json.as_bytes())
            .map_err(|err| DmApiError::CookieStore(format!("{err:?}")))?;
        let mut guard = self
            .cookie_store
            .lock()
            .map_err(|_| DmApiError::CookieStore("cookie store mutex is poisoned".to_owned()))?;

        *guard = parsed;
        Ok(())
    }

    pub async fn validate_session(&self) -> Result<SessionStatus> {
        let res = self.http.get(CONTENT_COUNT_URL).send().await?;

        match res.status() {
            StatusCode::OK => {
                let endpoint = res.url().clone();
                let body = res.text().await?;
                let count = parse_json_body::<ContentCount>(endpoint, &body)?;
                self.cache_limits_from_count(&count).await;
                Ok(SessionStatus::Authorized)
            }
            StatusCode::UNAUTHORIZED => Ok(SessionStatus::Unauthorized),
            status => {
                let endpoint = res.url().clone();
                let body_snippet = response_text_snippet(res).await;
                Err(DmApiError::UnexpectedStatus {
                    endpoint,
                    status,
                    body_snippet,
                })
            }
        }
    }

    pub async fn content_count(&self, query: ContentQuery) -> Result<ContentCount> {
        let endpoint = Url::parse(CONTENT_COUNT_URL)?;
        let mut request = self.http.get(endpoint.clone());

        if let Some(last) = query.last {
            request = request.query(&[("last", last)]);
        }

        let count = parse_json_response(request.send().await?).await?;
        self.cache_limits_from_count(&count).await;

        Ok(count)
    }

    pub async fn sales(&self, query: ContentQuery) -> Result<Vec<Purchase>> {
        let endpoint = Url::parse(CONTENT_SALES_URL)?;
        let mut request = self.http.get(endpoint.clone());

        if let Some(last) = query.last {
            request = request.query(&[("last", last)]);
        }

        parse_json_response(request.send().await?).await
    }

    pub async fn works(&self, ids: &[WorkId]) -> Result<Vec<Work>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        'load: loop {
            let limit = self.works_batch_limit().await;
            let mut works = Vec::new();

            for chunk in ids.chunks(limit) {
                match self.works_batch(chunk).await {
                    Ok(chunk_works) => works.extend(chunk_works),
                    Err(DmApiError::BatchLimitExceeded { limit, .. }) if limit > 0 => {
                        self.set_works_batch_limit(limit).await;
                        continue 'load;
                    }
                    Err(err) => return Err(err),
                }
            }

            return Ok(works);
        }
    }

    pub async fn works_batch(&self, ids: &[WorkId]) -> Result<Vec<Work>> {
        let endpoint = Url::parse(CONTENT_WORKS_URL)?;
        let ids = ids
            .iter()
            .map(|id| id.as_ref().to_owned())
            .collect::<Vec<_>>();

        let res = self.http.post(endpoint).json(&ids).send().await?;
        let status = res.status();

        if status == StatusCode::UNAUTHORIZED {
            return Err(DmApiError::NotAuthorized);
        }

        if !status.is_success() {
            let endpoint = res.url().clone();
            let headers = res.headers().clone();
            let body_snippet = response_text_snippet(res).await;

            if let Some(limit) =
                detect_works_batch_limit(&headers, body_snippet.as_deref().unwrap_or_default())
            {
                return Err(DmApiError::BatchLimitExceeded {
                    limit,
                    body_snippet,
                });
            }

            return Err(DmApiError::UnexpectedStatus {
                endpoint,
                status,
                body_snippet,
            });
        }

        let endpoint = res.url().clone();
        let body = res.text().await?;
        let response = parse_json_body::<WorksResponse>(endpoint, &body)?;

        Ok(response.works)
    }

    pub async fn raw_works_batch(&self, ids: &[WorkId]) -> Result<RawResponse> {
        self.raw_works_batch_with_body_limit(ids, 2048).await
    }

    pub async fn raw_works_batch_with_body_limit(
        &self,
        ids: &[WorkId],
        body_limit: usize,
    ) -> Result<RawResponse> {
        let endpoint = Url::parse(CONTENT_WORKS_URL)?;
        let ids = ids
            .iter()
            .map(|id| id.as_ref().to_owned())
            .collect::<Vec<_>>();
        let res = self.http.post(endpoint).json(&ids).send().await?;

        RawResponse::from_response_with_body_limit(res, body_limit).await
    }

    pub async fn resolve_download(&self, work_id: &WorkId) -> Result<DownloadResolution> {
        Ok(self.probe_download(work_id).await?.resolution)
    }

    pub async fn probe_download(&self, work_id: &WorkId) -> Result<DownloadProbe> {
        let initial = self.raw_download_probe(work_id).await?;
        let resolution = download_resolution_from_raw_response(&initial);

        Ok(DownloadProbe {
            work_id: work_id.clone(),
            initial,
            resolution,
        })
    }

    pub async fn download_plan(&self, work_id: &WorkId) -> Result<DownloadPlan> {
        match self.resolve_download(work_id).await? {
            DownloadResolution::Direct { stream_request } => {
                let serial_page = self.optional_serial_download_page(work_id).await?;
                let (stream_request, serial_numbers) = match serial_page {
                    Some(page) => (page.stream_request, page.serial_numbers),
                    None => (stream_request, Vec::new()),
                };

                Ok(DownloadPlan {
                    work_id: work_id.clone(),
                    files: vec![DownloadFile {
                        kind: DownloadFileKind::Direct,
                        stream_request,
                    }],
                    serial_numbers,
                })
            }
            DownloadResolution::Split { location } => {
                let page = self.split_download_page(location).await?;
                Ok(DownloadPlan {
                    work_id: work_id.clone(),
                    files: page
                        .parts
                        .into_iter()
                        .map(|part| DownloadFile {
                            kind: DownloadFileKind::SplitPart {
                                number: part.number,
                            },
                            stream_request: part.stream_request,
                        })
                        .collect(),
                    serial_numbers: Vec::new(),
                })
            }
            DownloadResolution::SerialRequired { location } => {
                let page = self.serial_download_page(location).await?;
                Ok(DownloadPlan {
                    work_id: work_id.clone(),
                    files: vec![DownloadFile {
                        kind: DownloadFileKind::Direct,
                        stream_request: page.stream_request,
                    }],
                    serial_numbers: page.serial_numbers,
                })
            }
            DownloadResolution::UnknownRedirect { location } => {
                Err(DmApiError::DownloadUnknownRedirect {
                    work_id: work_id.clone(),
                    location,
                })
            }
            DownloadResolution::Unavailable { reason } => Err(DmApiError::DownloadUnavailable {
                work_id: work_id.clone(),
                reason,
            }),
        }
    }

    pub async fn open_download_stream(
        &self,
        request: &DownloadStreamRequest,
        range: Option<DownloadByteRange>,
    ) -> Result<DownloadStream> {
        let mut url = request.url.clone();

        for _ in 0..MAX_DOWNLOAD_REDIRECTS {
            let mut builder = self.http.get(url.clone());

            if let Some(range) = range {
                builder = builder.header(RANGE, range.header_value());
            }

            let res = builder.send().await?;
            let status = res.status();

            if status.is_redirection() {
                url = redirect_location(&res)?;
                continue;
            }

            if status != StatusCode::OK && status != StatusCode::PARTIAL_CONTENT {
                let endpoint = res.url().clone();
                let body_snippet = response_text_snippet(res).await;
                return Err(DmApiError::UnexpectedStatus {
                    endpoint,
                    status,
                    body_snippet,
                });
            }

            return Ok(DownloadStream { response: res });
        }

        Err(DmApiError::RedirectLimitExceeded {
            endpoint: url,
            limit: MAX_DOWNLOAD_REDIRECTS,
        })
    }

    pub async fn optional_serial_download_page(
        &self,
        work_id: &WorkId,
    ) -> Result<Option<SerialDownloadPage>> {
        let location = serial_page_url(work_id)?;
        let raw = self
            .raw_get_with_body_limit(location.clone(), DOWNLOAD_PAGE_BODY_LIMIT)
            .await?;

        if is_absent_optional_serial_page_status(raw.status) {
            return Ok(None);
        }

        parse_serial_download_page_from_raw(location, raw).map(Some)
    }

    pub async fn split_download_page(&self, location: Url) -> Result<SplitDownloadPage> {
        let raw = self
            .raw_get_with_body_limit(location.clone(), DOWNLOAD_PAGE_BODY_LIMIT)
            .await?;
        let body = raw.body_snippet.as_deref().unwrap_or_default();
        let parts = parse_split_download_parts(&location, body);

        if parts.is_empty() {
            return Err(DmApiError::DownloadPageLinkNotFound {
                page: location,
                kind: "split",
            });
        }

        Ok(SplitDownloadPage {
            page_url: raw.url,
            parts,
        })
    }

    pub async fn serial_download_page(&self, location: Url) -> Result<SerialDownloadPage> {
        let raw = self
            .raw_get_with_body_limit(location.clone(), DOWNLOAD_PAGE_BODY_LIMIT)
            .await?;

        parse_serial_download_page_from_raw(location, raw)
    }

    pub async fn raw_download_probe(&self, work_id: &WorkId) -> Result<RawResponse> {
        self.raw_download_probe_with_body_limit(work_id, 2048).await
    }

    pub async fn raw_download_probe_with_body_limit(
        &self,
        work_id: &WorkId,
        body_limit: usize,
    ) -> Result<RawResponse> {
        let res = self
            .http
            .get(DOWNLOAD_URL)
            .query(&[("workno", work_id.as_ref())])
            .send()
            .await?;

        RawResponse::from_response_with_body_limit(res, body_limit).await
    }

    pub async fn raw_get(&self, url: Url) -> Result<RawResponse> {
        self.raw_get_with_body_limit(url, 2048).await
    }

    pub async fn raw_get_with_body_limit(
        &self,
        url: Url,
        body_limit: usize,
    ) -> Result<RawResponse> {
        let res = self.http.get(url).send().await?;

        RawResponse::from_response_with_body_limit(res, body_limit).await
    }

    async fn works_batch_limit(&self) -> usize {
        self.works_batch_limit
            .lock()
            .await
            .unwrap_or(DEFAULT_WORKS_BATCH_LIMIT)
    }

    async fn set_works_batch_limit(&self, limit: usize) {
        *self.works_batch_limit.lock().await = Some(limit);
    }

    async fn cache_limits_from_count(&self, count: &ContentCount) {
        if let Some(limit) = count.page_limit.filter(|limit| *limit > 0) {
            self.set_works_batch_limit(limit).await;
        }
    }

    fn xsrf_token(&self) -> Result<String> {
        let guard = self
            .cookie_store
            .lock()
            .map_err(|_| DmApiError::CookieStore("cookie store mutex is poisoned".to_owned()))?;

        guard
            .get("login.dlsite.com", "/", "XSRF-TOKEN")
            .map(|cookie| cookie.value().to_owned())
            .ok_or(DmApiError::XsrfTokenNotFound)
    }

    async fn redirect_location_from_get(&self, url: impl reqwest::IntoUrl) -> Result<Url> {
        let res = self.http.get(url).send().await?;
        let status = res.status();

        if !status.is_redirection() {
            let endpoint = res.url().clone();
            let body_snippet = response_text_snippet(res).await;
            return Err(DmApiError::UnexpectedStatus {
                endpoint,
                status,
                body_snippet,
            });
        }

        redirect_location(&res)
    }
}

fn parse_serial_download_page_from_raw(
    location: Url,
    raw: RawResponse,
) -> Result<SerialDownloadPage> {
    if !(200..=299).contains(&raw.status) {
        return Err(DmApiError::UnexpectedStatus {
            endpoint: raw.url,
            status: StatusCode::from_u16(raw.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            body_snippet: raw.body_snippet,
        });
    }

    let body = raw.body_snippet.as_deref().unwrap_or_default();
    let serial_numbers = parse_serial_numbers(body);

    if let Some(stream_request) = parse_serial_download_link(&location, body) {
        return Ok(SerialDownloadPage {
            page_url: raw.url,
            serial_numbers,
            stream_request,
        });
    }

    Err(DmApiError::DownloadPageLinkNotFound {
        page: location,
        kind: "serial",
    })
}

fn is_absent_optional_serial_page_status(status: u16) -> bool {
    status == StatusCode::INTERNAL_SERVER_ERROR.as_u16() || (300..=399).contains(&status)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadProbe {
    pub work_id: WorkId,
    pub initial: RawResponse,
    pub resolution: DownloadResolution,
}

pub struct DownloadStream {
    response: Response,
}

impl DownloadStream {
    pub fn url(&self) -> &Url {
        self.response.url()
    }

    pub fn status(&self) -> StatusCode {
        self.response.status()
    }

    pub fn content_length(&self) -> Option<u64> {
        self.response.content_length()
    }

    pub fn headers(&self) -> BTreeMap<String, String> {
        self.response
            .headers()
            .iter()
            .filter_map(|(key, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (key.as_str().to_owned(), value.to_owned()))
            })
            .collect()
    }

    pub async fn next_chunk(&mut self) -> std::result::Result<Option<Bytes>, reqwest::Error> {
        self.response.chunk().await
    }

    pub fn into_bytes_stream(
        self,
    ) -> impl Stream<Item = std::result::Result<Bytes, reqwest::Error>> {
        self.response.bytes_stream()
    }
}

fn download_resolution_from_raw_response(raw: &RawResponse) -> DownloadResolution {
    match raw.status {
        300..=399 => match raw.location.clone() {
            Some(location) => DownloadResolution::from_redirect_location(location),
            None => DownloadResolution::Unavailable {
                reason: DownloadUnavailableReason::UnexpectedStatus {
                    status: raw.status,
                    body_snippet: raw.body_snippet.clone(),
                },
            },
        },
        401 => DownloadResolution::Unavailable {
            reason: DownloadUnavailableReason::NotAuthorized,
        },
        404 => DownloadResolution::Unavailable {
            reason: DownloadUnavailableReason::NotFound,
        },
        status => DownloadResolution::Unavailable {
            reason: DownloadUnavailableReason::UnexpectedStatus {
                status,
                body_snippet: raw.body_snippet.clone(),
            },
        },
    }
}

fn parse_split_download_parts(page_url: &Url, body: &str) -> Vec<SplitDownloadPart> {
    let mut parts = extract_download_links(page_url, body)
        .into_iter()
        .filter_map(|url| {
            let number = download_part_number(&url)?;
            Some(SplitDownloadPart {
                number,
                stream_request: DownloadStreamRequest { url },
            })
        })
        .collect::<Vec<_>>();

    parts.sort_by(|left, right| {
        left.number
            .cmp(&right.number)
            .then_with(|| left.stream_request.url.cmp(&right.stream_request.url))
    });
    parts.dedup_by(|left, right| {
        left.number == right.number && left.stream_request.url == right.stream_request.url
    });
    parts
}

fn parse_serial_download_link(page_url: &Url, body: &str) -> Option<DownloadStreamRequest> {
    extract_download_links(page_url, body)
        .into_iter()
        .find(|url| {
            matches!(
                DownloadResolution::from_redirect_location(url.clone()),
                DownloadResolution::Direct { .. }
            )
        })
        .map(|url| DownloadStreamRequest { url })
}

fn parse_serial_numbers(body: &str) -> Vec<SerialNumber> {
    let Some(section_start) = body.find("<h2>シリアル番号</h2>") else {
        return Vec::new();
    };
    let section = &body[section_start..];
    let Some(table_start) = section.find("<table") else {
        return Vec::new();
    };
    let section = &section[table_start..];
    let table_end = section.find("</table>").unwrap_or(section.len());
    let table = &section[..table_end];
    let mut serial_numbers = Vec::new();
    let mut rest = table;

    while let Some(row_start) = rest.find("<tr") {
        rest = &rest[row_start..];
        let Some(row_end) = rest.find("</tr>") else {
            break;
        };
        let row = &rest[..row_end];

        if let Some(serial_number) = parse_serial_number_row(row) {
            serial_numbers.push(serial_number);
        }

        rest = &rest[row_end + "</tr>".len()..];
    }

    serial_numbers
}

fn parse_serial_number_row(row: &str) -> Option<SerialNumber> {
    let label = extract_element_text(row, "th")?;

    if !label.contains("シリアル") && !label.to_ascii_lowercase().contains("serial") {
        return None;
    }

    let value = extract_element_text(row, "td")?;

    if value.is_empty() {
        return None;
    }

    Some(SerialNumber { label, value })
}

fn extract_element_text(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = html.find(&open)?;
    let after_open = html[start..].find('>')? + start + 1;
    let end = html[after_open..].find(&close)? + after_open;
    let text = strip_html_tags(&html[after_open..end]);
    let text = decode_basic_html_entities(&text);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");

    Some(text)
}

fn strip_html_tags(html: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }

    output
}

fn decode_basic_html_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
}

fn extract_download_links(page_url: &Url, body: &str) -> Vec<Url> {
    let mut links = extract_quoted_values(body)
        .into_iter()
        .filter_map(|value| page_url.join(&value).ok())
        .filter(|url| {
            let host = url.host_str().unwrap_or_default();
            let path = url.path();
            host == "www.dlsite.com"
                && path.starts_with("/home/download")
                && path.contains("product_id")
        })
        .collect::<Vec<_>>();

    links.sort();
    links.dedup();
    links
}

fn extract_quoted_values(body: &str) -> Vec<String> {
    let mut values = Vec::new();

    for quote in ['"', '\''] {
        let mut start = None;

        for (index, value) in body.char_indices() {
            if value != quote {
                continue;
            }

            if let Some(open) = start.take() {
                values.push(body[open..index].to_owned());
            } else {
                start = Some(index + value.len_utf8());
            }
        }
    }

    values
}

fn download_part_number(url: &Url) -> Option<u32> {
    let mut segments = url.path_segments()?;

    while let Some(segment) = segments.next() {
        if segment == "number" {
            return segments.next()?.parse().ok();
        }
    }

    None
}

fn serial_page_url(work_id: &WorkId) -> Result<Url> {
    Url::parse(&format!("{HOME_SERIAL_URL}{work_id}.html")).map_err(Into::into)
}

async fn parse_json_response<T>(res: Response) -> Result<T>
where
    T: DeserializeOwned,
{
    let status = res.status();

    if status == StatusCode::UNAUTHORIZED {
        return Err(DmApiError::NotAuthorized);
    }

    if !status.is_success() {
        let endpoint = res.url().clone();
        let body_snippet = response_text_snippet(res).await;
        return Err(DmApiError::UnexpectedStatus {
            endpoint,
            status,
            body_snippet,
        });
    }

    let endpoint = res.url().clone();
    let body = res.text().await?;
    parse_json_body(endpoint, &body)
}

fn parse_json_body<T>(endpoint: Url, body: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut deserializer = serde_json::Deserializer::from_str(body);
    serde_path_to_error::deserialize(&mut deserializer).map_err(|err| DmApiError::UnexpectedJson {
        endpoint,
        path: err.path().to_string(),
        source: err.into_inner(),
    })
}

pub fn detect_works_batch_limit(headers: &HeaderMap, body: &str) -> Option<usize> {
    const HEADER_KEYS: &[&str] = &[
        "x-batch-limit",
        "x-max-batch-size",
        "x-works-batch-limit",
        "x-dlsite-batch-limit",
    ];

    for key in HEADER_KEYS {
        if let Some(limit) = headers.get(*key).and_then(header_value_to_usize) {
            return Some(limit);
        }
    }

    let body = body.trim();
    if body.is_empty() {
        return None;
    }

    let json = serde_json::from_str::<Value>(body).ok()?;
    find_batch_limit_in_json(&json)
}

fn find_batch_limit_in_json(value: &Value) -> Option<usize> {
    const JSON_KEYS: &[&str] = &[
        "expected_batch_size",
        "batch_size",
        "batchSize",
        "max_batch_size",
        "maxBatchSize",
        "batch_limit",
        "batchLimit",
        "page_limit",
        "pageLimit",
    ];

    match value {
        Value::Object(map) => {
            for key in JSON_KEYS {
                if let Some(limit) = map
                    .get(*key)
                    .and_then(|value| value.as_u64())
                    .and_then(|value| usize::try_from(value).ok())
                {
                    return Some(limit);
                }
            }

            map.values().find_map(find_batch_limit_in_json)
        }
        Value::Array(values) => values.iter().find_map(find_batch_limit_in_json),
        _ => None,
    }
}

fn header_value_to_usize(value: &HeaderValue) -> Option<usize> {
    value.to_str().ok()?.parse().ok()
}

pub(crate) fn redirect_location(res: &Response) -> Result<Url> {
    let endpoint = res.url().clone();
    let location = res
        .headers()
        .get(LOCATION)
        .ok_or_else(|| DmApiError::LocationHeaderMissing {
            endpoint: endpoint.clone(),
        })?
        .to_str()
        .map_err(|err| DmApiError::CookieStore(format!("invalid Location header: {err:?}")))?;

    endpoint
        .join(location)
        .map_err(|source| DmApiError::InvalidLocationHeader {
            endpoint,
            location: location.to_owned(),
            source,
        })
}

pub(crate) fn text_snippet(value: &str) -> String {
    limited_text_snippet(value, 2048)
}

pub(crate) fn limited_text_snippet(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

async fn response_text_snippet(res: Response) -> Option<String> {
    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if !content_type.is_empty()
        && !content_type.contains("json")
        && !content_type.contains("text")
        && !content_type.contains("html")
    {
        return None;
    }

    res.text().await.ok().map(|body| text_snippet(&body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::HeaderValue;

    #[test]
    fn detects_batch_limit_from_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-batch-limit", HeaderValue::from_static("50"));

        assert_eq!(detect_works_batch_limit(&headers, ""), Some(50));
    }

    #[test]
    fn detects_batch_limit_from_json_body() {
        let headers = HeaderMap::new();
        let body = r#"{ "error": { "maxBatchSize": 50 } }"#;

        assert_eq!(detect_works_batch_limit(&headers, body), Some(50));
    }

    #[test]
    fn detects_batch_limit_from_page_limit_json_body() {
        let headers = HeaderMap::new();
        let body = r#"{ "user": 1223, "page_limit": 50, "concurrency": 500 }"#;

        assert_eq!(detect_works_batch_limit(&headers, body), Some(50));
    }

    #[test]
    fn classifies_raw_download_responses() {
        let raw = RawResponse {
            url: Url::parse(DOWNLOAD_URL).unwrap(),
            status: 302,
            headers: BTreeMap::new(),
            location: Some(
                Url::parse("https://www.dlsite.com/home/download/=/product_id/RJ123456.html")
                    .unwrap(),
            ),
            content_type: None,
            body_snippet: None,
        };

        assert!(matches!(
            download_resolution_from_raw_response(&raw),
            DownloadResolution::Direct { .. }
        ));

        let raw = RawResponse {
            status: 401,
            body_snippet: Some("unauthorized".to_owned()),
            ..raw
        };

        assert!(matches!(
            download_resolution_from_raw_response(&raw),
            DownloadResolution::Unavailable {
                reason: DownloadUnavailableReason::NotAuthorized
            }
        ));
    }

    #[test]
    fn parses_split_download_links() {
        let page_url =
            Url::parse("https://www.dlsite.com/home/download/split/=/product_id/RJ123456.html")
                .unwrap();
        let body = r#"
            <a href="https://www.dlsite.com/home/download/=/number/2/product_id/RJ123456.html">2</a>
            <a href="https://www.dlsite.com/home/download/=/number/1/product_id/RJ123456.html">1</a>
            <a href="/home/download/split/=/product_id/RJ123456.html">self</a>
        "#;

        let parts = parse_split_download_parts(&page_url, body);

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].number, 1);
        assert_eq!(parts[1].number, 2);
    }

    #[test]
    fn parses_serial_download_link() {
        let page_url =
            Url::parse("https://www.dlsite.com/home/serial/=/product_id/VJ123456.html").unwrap();
        let body = r#"
            <p class="work_download">
                <a href="https://www.dlsite.com/home/download/=/product_id/VJ123456.html">download</a>
            </p>
        "#;

        let request = parse_serial_download_link(&page_url, body).unwrap();

        assert_eq!(
            request.url.as_str(),
            "https://www.dlsite.com/home/download/=/product_id/VJ123456.html"
        );
    }

    #[test]
    fn parses_serial_numbers() {
        let body = r#"
            <h2>シリアル番号</h2>
            <table>
                <tr>
                    <th>シリアル番号</th>
                    <td><strong class="color_02">ABCD-1234-EFGH</strong></td>
                </tr>
            </table>
        "#;

        let serial_numbers = parse_serial_numbers(body);

        assert_eq!(
            serial_numbers,
            vec![SerialNumber {
                label: "シリアル番号".to_owned(),
                value: "ABCD-1234-EFGH".to_owned()
            }]
        );
    }

    #[test]
    fn builds_serial_page_url() {
        let url = serial_page_url(&WorkId::from("RJ123456")).unwrap();

        assert_eq!(
            url.as_str(),
            "https://www.dlsite.com/home/serial/=/product_id/RJ123456.html"
        );
    }

    #[test]
    fn detects_absent_optional_serial_page_statuses() {
        assert!(is_absent_optional_serial_page_status(302));
        assert!(is_absent_optional_serial_page_status(500));
        assert!(!is_absent_optional_serial_page_status(200));
    }

    #[test]
    fn exports_and_imports_empty_session_snapshot() {
        let client = DlsiteClient::new(DlsiteClientConfig::default()).unwrap();
        let snapshot = client.export_session().unwrap();

        let other = DlsiteClient::new(DlsiteClientConfig::default()).unwrap();
        other.import_session(&snapshot).unwrap();

        assert_eq!(
            other.export_session().unwrap().cookies_json,
            snapshot.cookies_json
        );
    }
}
