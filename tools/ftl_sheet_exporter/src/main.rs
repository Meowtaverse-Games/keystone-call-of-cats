use std::{env, fs, path::{Path, PathBuf}};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use reqwest::Client;
use serde::Deserialize;
use yup_oauth2::{
    read_application_secret, InstalledFlowAuthenticator, InstalledFlowReturnMethod,
};

#[derive(Parser, Debug)]
#[command(name = "ftl_sheet_exporter", about = "Export Google Sheets rows into a Fluent FTL file")]
struct Args {
    /// Spreadsheet ID portion of the Google Sheets URL
    #[arg(long, env = "GOOGLE_SHEETS_SPREADSHEET_ID")]
    spreadsheet_id: String,

    /// A1 notation range to read, e.g. "main!A:C"
    #[arg(long, env = "GOOGLE_SHEETS_RANGE", default_value = "main!A:C")]
    range: String,

    /// Output path for the generated FTL file
    #[arg(long, env = "OUTPUT_FTL_PATH", default_value = "assets/locales/ja-JP/main.ftl")]
    output_path: PathBuf,

    /// Path to the OAuth clientsecret.json (falls back to GOOGLE_CLIENT_SECRET_JSON or GOOGLE_APPLICATION_CREDENTIALS)
    #[arg(long, env = "GOOGLE_CLIENT_SECRET_JSON")]
    client_secret: Option<PathBuf>,

    /// Where to cache OAuth tokens (reuse to avoid re-auth prompts)
    #[arg(long, env = "GOOGLE_OAUTH_TOKEN_STORE", default_value = "tools/ftl_sheet_exporter/.oauth_tokens.json")]
    token_store: PathBuf,

    /// Skip the first row when it contains column headers
    #[arg(long, env = "SHEETS_SKIP_HEADER", default_value_t = true)]
    skip_header: bool,
}

#[derive(Debug, Deserialize)]
struct ValuesResponse {
    values: Option<Vec<Vec<String>>>,
}

#[derive(Debug)]
struct FluentEntry {
    key: String,
    value: String,
    comment: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let client_secret_path = resolve_client_secret_path(args.client_secret)?;
    let token = fetch_access_token(&client_secret_path, &args.token_store).await?;
    let client = Client::builder().build().context("failed to build HTTP client")?;

    let rows = fetch_sheet_rows(&client, &token, &args.spreadsheet_id, &args.range).await?;
    let entries = rows_to_entries(rows, args.skip_header);
    let rendered = render_entries(&entries);

    if let Some(parent) = args.output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory for {:?}", args.output_path))?;
    }

    fs::write(&args.output_path, rendered)
        .with_context(|| format!("failed to write output file at {:?}", args.output_path))?;

    println!("Wrote {} entries to {}", entries.len(), args.output_path.display());
    Ok(())
}

fn resolve_client_secret_path(cli_value: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = cli_value {
        return Ok(path);
    }

    for var in ["GOOGLE_CLIENT_SECRET_JSON", "GOOGLE_APPLICATION_CREDENTIALS"] {
        if let Ok(path) = env::var(var) {
            return Ok(PathBuf::from(path));
        }
    }

    Err(anyhow!(
        "clientsecret.json not provided. Pass --client-secret or set GOOGLE_CLIENT_SECRET_JSON/GOOGLE_APPLICATION_CREDENTIALS"
    ))
}

async fn fetch_access_token(client_secret_path: &Path, token_store: &Path) -> Result<String> {
    if let Some(parent) = token_store.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create token store directory at {:?}", parent))?;
    }

    let secret = read_application_secret(client_secret_path)
        .await
        .with_context(|| format!("failed to read client secret file at {:?}", client_secret_path))?;

    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk(token_store)
        .build()
        .await
        .context("failed to build Google authenticator")?;

    let token = auth
        .token(&["https://www.googleapis.com/auth/spreadsheets.readonly"])
        .await
        .context("failed to fetch access token")?;

    Ok(token.token().to_owned())
}

async fn fetch_sheet_rows(
    client: &Client,
    token: &str,
    spreadsheet_id: &str,
    range: &str,
) -> Result<Vec<Vec<String>>> {
    let url = format!(
        "https://sheets.googleapis.com/v4/spreadsheets/{}/values/{}",
        spreadsheet_id,
        urlencoding::encode(range)
    );

    let response = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context("request to Sheets API failed")?
        .error_for_status()
        .context("Sheets API returned an error status")?;

    let parsed: ValuesResponse = response
        .json()
        .await
        .context("failed to parse Sheets API response")?;

    Ok(parsed.values.unwrap_or_default())
}

fn rows_to_entries(rows: Vec<Vec<String>>, skip_header: bool) -> Vec<FluentEntry> {
    let mut data = rows;

    if skip_header && data.first().map(|row| is_header_row(row)).unwrap_or(false) {
        data.remove(0);
    }

    let mut entries = Vec::new();

    for (index, row) in data.into_iter().enumerate() {
        let Some(key_raw) = row.get(0) else {
            eprintln!("Row {} missing key column, skipping", index + 1);
            continue;
        };

        let key = key_raw.trim();
        if key.is_empty() {
            eprintln!("Row {} has empty key, skipping", index + 1);
            continue;
        }

        let Some(value_raw) = row.get(1) else {
            eprintln!("Row {} missing translation column, skipping", index + 1);
            continue;
        };

        let value = value_raw.replace("\r\n", "\n");
        let comment = row
            .get(2)
            .map(|c| c.trim().to_owned())
            .filter(|c| !c.is_empty());

        entries.push(FluentEntry {
            key: key.to_owned(),
            value,
            comment,
        });
    }

    entries
}

fn is_header_row(row: &[String]) -> bool {
    row.get(0)
        .map(|cell| {
            let lowered = cell.to_ascii_lowercase();
            lowered == "key" || lowered == "id"
        })
        .unwrap_or(false)
}

fn render_entries(entries: &[FluentEntry]) -> String {
    let mut out = String::new();
    out.push_str("# Generated by tools/ftl_sheet_exporter.\n");
    out.push_str("# Source of truth: Google Sheets\n\n");

    for entry in entries {
        if let Some(comment) = &entry.comment {
            for line in comment.lines() {
                out.push_str("# ");
                out.push_str(line.trim_end());
                out.push('\n');
            }
        }

        out.push_str(&format_entry(&entry.key, &entry.value));
        out.push_str("\n\n");
    }

    out
}

fn format_entry(key: &str, value: &str) -> String {
    let mut lines = value.lines();

    match lines.next() {
        Some(first_line) if lines.clone().next().is_some() => {
            let mut out = String::new();
            out.push_str(&format!("{key} = {first_line}\n"));
            for line in lines {
                out.push_str("    ");
                out.push_str(line);
                out.push('\n');
            }
            // Remove trailing newline for consistency with single-line branch
            while out.ends_with('\n') {
                out.pop();
            }
            out
        }
        Some(single_line) => format!("{key} = {single_line}"),
        None => format!("{key} ="),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_single_line_entry() {
        let entry = format_entry("greeting", "hello world");
        assert_eq!(entry, "greeting = hello world");
    }

    #[test]
    fn formats_multi_line_entry() {
        let entry = format_entry("intro", "line1\nline2\nline3");
        assert_eq!(entry, "intro = line1\n    line2\n    line3");
    }
}
