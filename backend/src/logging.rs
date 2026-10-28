use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::OnceLock,
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, Request},
    http::header,
    middleware::Next,
    response::Response,
};
use chrono::{Local, NaiveDate, SecondsFormat};

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

const RETENTION_DAYS: i64 = 3;
const CLEANUP_INTERVAL_SECS: u64 = 72 * 60 * 60;

/// Initializes daily file logging under `<root>/backend` and starts the
/// cleanup task. Called once from `main`; tests fall back to stderr.
pub fn init(root: PathBuf) -> std::io::Result<()> {
    let backend_dir = root.join("backend");
    fs::create_dir_all(&backend_dir)?;
    let _ = LOG_DIR.set(backend_dir.clone());
    cleanup();
    info("app", format!("log directory: {}", backend_dir.display()));

    tokio::spawn(async {
        let mut interval = tokio::time::interval(Duration::from_secs(CLEANUP_INTERVAL_SECS));
        interval.tick().await;
        loop {
            interval.tick().await;
            cleanup();
            info("app", "log cleanup finished");
        }
    });

    Ok(())
}

pub fn info(target: &str, message: impl AsRef<str>) {
    write("INFO", target, message.as_ref());
}

pub fn warn(target: &str, message: impl AsRef<str>) {
    write("WARN", target, message.as_ref());
}

pub fn error(target: &str, message: impl AsRef<str>) {
    write("ERROR", target, message.as_ref());
}

/// Axum middleware that records method, path, status, latency, size and
/// user agent for every request. Body contents are never logged.
pub async fn request_log(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request
        .uri()
        .path_and_query()
        .map(|value| value.as_str().to_owned())
        .unwrap_or_else(|| "/".to_owned());
    let user_agent = header_value(&request, header::USER_AGENT);
    let content_length = header_value(&request, header::CONTENT_LENGTH);
    let remote = request
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ConnectInfo(address)| address.to_string())
        .unwrap_or_else(|| "-".to_owned());
    let started = Instant::now();

    let response = next.run(request).await;
    let status = response.status();
    let latency = started.elapsed().as_millis();
    let line = format!(
        "{method} {path} -> {} {latency}ms size={} ip={remote} ua=\"{user_agent}\"",
        status.as_u16(),
        content_length
    );

    if status.is_server_error() {
        error("http", line);
    } else if status.is_client_error() {
        warn("http", line);
    } else {
        info("http", line);
    }
    response
}

fn header_value(request: &Request, name: header::HeaderName) -> String {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("-")
        .chars()
        .take(300)
        .collect()
}

fn write(level: &str, target: &str, message: &str) {
    let timestamp = Local::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let line = format!("{timestamp} [{level}] {target}: {message}");

    let Some(path) = log_path() else {
        eprintln!("{line}");
        return;
    };
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    } else {
        eprintln!("{line}");
    }
}

fn log_path() -> Option<PathBuf> {
    let directory = LOG_DIR.get()?;
    let date = Local::now().format("%Y-%m-%d");
    Some(directory.join(format!("{date}.log")))
}

/// Keeps today and the previous two days. The task above runs every 72 hours,
/// and startup also runs a cleanup so short-lived sessions still prune files.
fn cleanup() {
    let Some(directory) = LOG_DIR.get() else {
        return;
    };
    let cutoff = Local::now().date_naive() - chrono::Duration::days(RETENTION_DAYS - 1);
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("log") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") else {
            continue;
        };
        if date < cutoff {
            let _ = fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_daily_log_names() {
        assert!(NaiveDate::parse_from_str("2026-10-28", "%Y-%m-%d").is_ok());
        assert!(NaiveDate::parse_from_str("requests", "%Y-%m-%d").is_err());
    }

    #[test]
    fn retains_three_days() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
        let cutoff = today - chrono::Duration::days(RETENTION_DAYS - 1);
        assert_eq!(cutoff, NaiveDate::from_ymd_opt(2026, 10, 26).unwrap());
        assert!(NaiveDate::from_ymd_opt(2026, 10, 25).unwrap() < cutoff);
        assert!(NaiveDate::from_ymd_opt(2026, 10, 26).unwrap() >= cutoff);
    }
}
