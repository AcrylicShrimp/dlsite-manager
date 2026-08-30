//! Sanitized live probe for the DLsite login flow, used to capture the two-factor
//! (TOTP) challenge contract.
//!
//! DLsite does not document its two-factor form, so the challenge page URL, the form
//! `action`, its hidden fields, and the name of the verification-code input all have to be
//! observed against a real two-factor-enabled account before they can be implemented.
//!
//! This probe performs the same credential POST as `DlsiteClient::login`, then follows the
//! redirect chain and reports, for every page it lands on, the forms and form controls it
//! contains. Field *values* are only ever printed as short previews, passwords are never
//! printed, and no verification code is submitted.
//!
//! Usage:
//!
//! ```text
//! DMSITE_API_TEST_USERNAME=... DMSITE_API_TEST_PASSWORD=... \
//!   cargo run -p dm-api --example probe_login
//! ```
//!
//! `DMSITE_API_PROBE_LOGIN_BODY_LIMIT` caps how much of each page is saved, and
//! `DMSITE_API_PROBE_LOGIN_OUT_DIR` overrides the `.dlsite-probe/` output directory. Both
//! are optional.

use dm_api::{raw::RawResponse, Credentials, DlsiteClient, DlsiteClientConfig};
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};
use url::Url;

const MAX_REDIRECTS: usize = 8;

/// Keywords that suggest the page is a two-factor challenge rather than a normal login or
/// post-login page.
const CHALLENGE_KEYWORDS: &[&str] = &[
    "2段階",
    "二段階",
    "認証コード",
    "確認コード",
    "ワンタイム",
    "認証アプリ",
    "バックアップコード",
    "two-step",
    "two_step",
    "two-factor",
    "two_factor",
    "2fa",
    "otp",
    "totp",
    "one-time",
    "one_time",
    "authenticator",
    "verification code",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    load_dotenv();

    let username = env::var("DMSITE_API_TEST_USERNAME")?;
    let password = env::var("DMSITE_API_TEST_PASSWORD")?;
    let body_limit = body_limit();
    let out_dir = out_dir();

    fs::create_dir_all(&out_dir)?;
    println!("probe output dir: {}", out_dir.display());

    let client = DlsiteClient::new(DlsiteClientConfig::default())?;
    let credentials = Credentials::new(username, password);

    let mut step = 0;
    let mut raw = client.raw_login_probe(&credentials, body_limit).await?;
    report_step(step, "credential POST", &raw, &out_dir)?;

    while let Some(location) = raw.location.clone() {
        if step >= MAX_REDIRECTS {
            println!("stopped after {MAX_REDIRECTS} redirects");
            break;
        }

        step += 1;
        raw = client.raw_get_with_body_limit(location, body_limit).await?;
        report_step(step, "redirect follow", &raw, &out_dir)?;
    }

    println!();
    println!("Next step: share the form `action`, the hidden field names, and the name of the");
    println!("verification-code input from the challenge page reported above.");

    Ok(())
}

fn report_step(
    step: usize,
    label: &str,
    raw: &RawResponse,
    out_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    println!();
    println!("== step {step}: {label} ==");
    println!("  status={}", raw.status);
    println!("  url={}", raw.url);
    println!("  location={:?}", raw.location.as_ref().map(Url::as_str));
    println!("  content_type={:?}", raw.content_type);

    let Some(body) = raw.body_snippet.as_deref() else {
        println!("  body=none (redirect)");
        return Ok(());
    };

    let path = out_dir.join(format!("login-step-{step:02}.html"));
    fs::write(&path, body)?;
    println!("  body_chars={}", body.chars().count());
    println!("  body_saved={}", path.display());

    let keywords = keyword_hits(body);
    if keywords.is_empty() {
        println!("  challenge_keywords=none");
    } else {
        println!("  challenge_keywords={}", keywords.join(","));
    }

    println!("  visible_text={}", visible_text_preview(body, 400));
    print_forms(body);

    Ok(())
}

fn keyword_hits(body: &str) -> Vec<&'static str> {
    let lower_body = body.to_ascii_lowercase();

    CHALLENGE_KEYWORDS
        .iter()
        .copied()
        .filter(|needle| {
            if needle.is_ascii() {
                lower_body.contains(*needle)
            } else {
                body.contains(*needle)
            }
        })
        .collect()
}

fn print_forms(body: &str) {
    let mut rest = body;
    let mut index = 0;

    while let Some(start) = rest.find("<form") {
        rest = &rest[start..];

        let Some(open_end) = rest.find('>') else {
            break;
        };
        let attributes = parse_attributes(&rest[..=open_end]);
        let Some(close) = rest.find("</form>") else {
            break;
        };
        let inner = &rest[open_end + 1..close];

        println!(
            "  form[{index}] method={:?} action={:?} id={:?}",
            attribute(&attributes, "method"),
            attribute(&attributes, "action"),
            attribute(&attributes, "id")
        );

        for tag in ["input", "select", "textarea", "button"] {
            for control in controls(inner, tag) {
                println!("    {control}");
            }
        }

        index += 1;
        rest = &rest[close + "</form>".len()..];
    }

    if index == 0 {
        println!("  forms=none");
    }
}

fn controls(inner: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}");
    let mut controls = Vec::new();
    let mut rest = inner;

    while let Some(start) = rest.find(&open) {
        rest = &rest[start..];

        let Some(open_end) = rest.find('>') else {
            break;
        };
        let attributes = parse_attributes(&rest[..=open_end]);
        let kind = attribute(&attributes, "type").unwrap_or_default();
        let mut described = format!(
            "{tag} name={:?} type={:?} id={:?}",
            attribute(&attributes, "name"),
            attribute(&attributes, "type"),
            attribute(&attributes, "id")
        );

        for extra in [
            "autocomplete",
            "inputmode",
            "maxlength",
            "placeholder",
            "pattern",
        ] {
            if let Some(value) = attribute(&attributes, extra) {
                described.push_str(&format!(" {extra}={value:?}"));
            }
        }

        if let Some(value) = attribute(&attributes, "value") {
            let name = attribute(&attributes, "name").unwrap_or_default();
            described.push_str(&format!(" value={}", masked_value(&name, &kind, &value)));
        }

        controls.push(described);
        rest = &rest[open_end + 1..];
    }

    controls
}

/// Field values are never needed to implement the flow, only field names are, so values are
/// reduced to a length-tagged preview and account-identifying values are dropped entirely.
/// The probe output is meant to be pasteable into an issue or a chat.
fn masked_value(name: &str, kind: &str, value: &str) -> String {
    const REDACTED_NAMES: &[&str] = &["login_id", "email", "username", "user", "tel", "phone"];

    if kind.eq_ignore_ascii_case("password")
        || REDACTED_NAMES
            .iter()
            .any(|redacted| name.eq_ignore_ascii_case(redacted))
    {
        return "<redacted>".to_owned();
    }

    let chars = value.chars().count();

    if chars <= 6 {
        return format!("{value:?}");
    }

    let preview = value.chars().take(6).collect::<String>();
    format!("{preview:?}…(len={chars})")
}

fn attribute(attributes: &[(String, String)], name: &str) -> Option<String> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
}

fn parse_attributes(tag: &str) -> Vec<(String, String)> {
    let chars = tag.chars().collect::<Vec<_>>();
    let mut attributes = Vec::new();
    let mut index = 0;

    while index < chars.len() && !chars[index].is_whitespace() {
        index += 1;
    }

    while index < chars.len() {
        while index < chars.len() && chars[index].is_whitespace() {
            index += 1;
        }

        if index >= chars.len() || chars[index] == '>' || chars[index] == '/' {
            break;
        }

        let name_start = index;
        while index < chars.len()
            && !chars[index].is_whitespace()
            && chars[index] != '='
            && chars[index] != '>'
        {
            index += 1;
        }

        let name = chars[name_start..index]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();
        let mut value = String::new();

        if index < chars.len() && chars[index] == '=' {
            index += 1;

            if index < chars.len() && (chars[index] == '"' || chars[index] == '\'') {
                let quote = chars[index];
                index += 1;
                let value_start = index;

                while index < chars.len() && chars[index] != quote {
                    index += 1;
                }

                value = chars[value_start..index].iter().collect();

                if index < chars.len() {
                    index += 1;
                }
            } else {
                let value_start = index;

                while index < chars.len() && !chars[index].is_whitespace() && chars[index] != '>' {
                    index += 1;
                }

                value = chars[value_start..index].iter().collect();
            }
        }

        if !name.is_empty() {
            attributes.push((name, value));
        }
    }

    attributes
}

fn visible_text_preview(body: &str, max_chars: usize) -> String {
    let without_blocks = strip_blocks(body, "script");
    let without_blocks = strip_blocks(&without_blocks, "style");
    let mut text = String::new();
    let mut in_tag = false;

    for value in without_blocks.chars() {
        match value {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(value),
            _ => {}
        }
    }

    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let preview = text.chars().take(max_chars).collect::<String>();

    format!("{preview:?}")
}

fn strip_blocks(body: &str, tag: &str) -> String {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut output = String::new();
    let mut rest = body;

    while let Some(start) = rest.find(&open) {
        output.push_str(&rest[..start]);
        rest = &rest[start..];

        match rest.find(&close) {
            Some(end) => rest = &rest[end + close.len()..],
            None => return output,
        }
    }

    output.push_str(rest);
    output
}

fn body_limit() -> usize {
    env::var("DMSITE_API_PROBE_LOGIN_BODY_LIMIT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2_000_000)
}

fn out_dir() -> PathBuf {
    if let Some(value) = env::var("DMSITE_API_PROBE_LOGIN_OUT_DIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return PathBuf::from(value);
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(|repo_root| repo_root.join(".dlsite-probe"))
        .unwrap_or_else(|| PathBuf::from(".dlsite-probe"))
}

fn load_dotenv() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [manifest_dir.join(".env")];

    for candidate in candidates {
        if candidate.exists() {
            let _ = dotenvy::from_path(candidate);
        }
    }
}
