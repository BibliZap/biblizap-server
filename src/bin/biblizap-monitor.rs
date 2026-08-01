use std::{error::Error, path::PathBuf, time::Duration};

use clap::Parser;
use config as conf;
use lettre::{
    Message, SmtpTransport, Transport, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;

const DEFAULT_CONFIG_PATH: &str = "/etc/biblizap-monitor/biblizap-monitor.toml";

#[derive(Parser)]
#[command(version, about = "Monitor BibliZap and send SMTP outage alerts")]
struct Args {
    /// Path to the monitor TOML configuration file
    #[arg(long, default_value = DEFAULT_CONFIG_PATH)]
    config: PathBuf,
}

#[derive(Clone)]
struct Config {
    healthcheck_url: Url,
    check_interval: Duration,
    request_timeout: Duration,
    failure_threshold: u32,
    smtp: SmtpConfig,
}

#[derive(Clone)]
struct SmtpConfig {
    host: String,
    port: u16,
    username: String,
    password: String,
    tls_mode: TlsMode,
    timeout: Duration,
    from: Mailbox,
    recipients: Vec<Mailbox>,
}

#[derive(Deserialize)]
struct FileConfig {
    health: HealthFileConfig,
    smtp: SmtpFileConfig,
    alerts: AlertsFileConfig,
}

#[derive(Deserialize)]
struct HealthFileConfig {
    url: String,
    #[serde(default = "default_check_interval_seconds")]
    interval_seconds: u64,
    #[serde(default = "default_request_timeout_seconds")]
    timeout_seconds: u64,
    #[serde(default = "default_failure_threshold")]
    failure_threshold: u32,
}

#[derive(Deserialize)]
struct SmtpFileConfig {
    host: String,
    #[serde(default = "default_smtp_port")]
    port: u16,
    username: String,
    password: String,
    #[serde(default = "default_tls_mode")]
    tls_mode: String,
    #[serde(default = "default_smtp_timeout_seconds")]
    timeout_seconds: u64,
}

#[derive(Deserialize)]
struct AlertsFileConfig {
    from: String,
    to: Vec<String>,
}

#[derive(Clone, Copy)]
enum TlsMode {
    StartTls,
    Implicit,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Notification {
    Outage,
    Recovery,
}

#[derive(Default)]
struct MonitorState {
    consecutive_failures: u32,
    outage_alerted: bool,
}

#[derive(Deserialize)]
struct HealthResponse {
    status: String,
}

impl MonitorState {
    fn observe(&mut self, healthy: bool, threshold: u32) -> Option<Notification> {
        if healthy {
            self.consecutive_failures = 0;
            return self.outage_alerted.then_some(Notification::Recovery);
        }

        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        (self.consecutive_failures >= threshold && !self.outage_alerted)
            .then_some(Notification::Outage)
    }

    fn notification_sent(&mut self, notification: Notification) {
        self.outage_alerted = notification == Notification::Outage;
    }
}

impl Config {
    fn load(path: PathBuf) -> Result<Self, Box<dyn Error>> {
        let file_config: FileConfig = conf::Config::builder()
            .add_source(conf::File::from(path))
            .add_source(
                conf::Environment::with_prefix("BIBLIZAP_MONITOR")
                    .prefix_separator("_")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()?;

        Self::try_from(file_config)
    }

    fn try_from(file: FileConfig) -> Result<Self, Box<dyn Error>> {
        let healthcheck_url = file.health.url.parse()?;
        let check_interval =
            checked_duration("health.interval_seconds", file.health.interval_seconds)?;
        let request_timeout =
            checked_duration("health.timeout_seconds", file.health.timeout_seconds)?;
        let failure_threshold = file.health.failure_threshold;
        if failure_threshold == 0 {
            return Err("health.failure_threshold must be at least 1".into());
        }

        let recipients = file
            .alerts
            .to
            .into_iter()
            .map(|address| address.parse())
            .collect::<Result<Vec<Mailbox>, _>>()?;
        if recipients.is_empty() {
            return Err("alerts.to must contain at least one email address".into());
        }

        let tls_mode = match file.smtp.tls_mode.to_ascii_lowercase().as_str() {
            "starttls" => TlsMode::StartTls,
            "implicit" => TlsMode::Implicit,
            "none" => TlsMode::None,
            value => {
                return Err(format!(
                    "invalid smtp.tls_mode '{value}'; use starttls, implicit, or none"
                )
                .into());
            }
        };

        Ok(Self {
            healthcheck_url,
            check_interval,
            request_timeout,
            failure_threshold,
            smtp: SmtpConfig {
                host: file.smtp.host,
                port: file.smtp.port,
                username: file.smtp.username,
                password: file.smtp.password,
                tls_mode,
                timeout: checked_duration("smtp.timeout_seconds", file.smtp.timeout_seconds)?,
                from: file.alerts.from.parse()?,
                recipients,
            },
        })
    }
}

const fn default_check_interval_seconds() -> u64 {
    30
}

const fn default_request_timeout_seconds() -> u64 {
    10
}

const fn default_failure_threshold() -> u32 {
    2
}

const fn default_smtp_port() -> u16 {
    587
}

fn default_tls_mode() -> String {
    "starttls".to_owned()
}

const fn default_smtp_timeout_seconds() -> u64 {
    10
}

fn checked_duration(name: &str, seconds: u64) -> Result<Duration, Box<dyn Error>> {
    if seconds == 0 {
        return Err(format!("{name} must be at least 1").into());
    }
    Ok(Duration::from_secs(seconds))
}

async fn check_health(client: &Client, url: &Url) -> Result<(), String> {
    let response = client
        .get(url.clone())
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if response.status() != StatusCode::OK {
        return Err(format!(
            "health endpoint returned HTTP {}",
            response.status()
        ));
    }

    let health: HealthResponse = response
        .json()
        .await
        .map_err(|error| format!("invalid health response: {error}"))?;
    if health.status == "ok" {
        Ok(())
    } else {
        Err(format!(
            "health endpoint returned unexpected status '{}'",
            health.status
        ))
    }
}

async fn send_notification(config: &Config, notification: Notification) -> Result<(), String> {
    let smtp = config.smtp.clone();
    let healthcheck_url = config.healthcheck_url.to_string();

    tokio::task::spawn_blocking(move || {
        send_notification_blocking(&smtp, &healthcheck_url, notification)
    })
    .await
    .map_err(|error| format!("email task failed: {error}"))?
}

fn send_notification_blocking(
    smtp: &SmtpConfig,
    healthcheck_url: &str,
    notification: Notification,
) -> Result<(), String> {
    let (subject, body) = match notification {
        Notification::Outage => (
            "BibliZap health check failed",
            format!(
                "BibliZap is unavailable. The health check at {healthcheck_url} failed repeatedly."
            ),
        ),
        Notification::Recovery => (
            "BibliZap has recovered",
            format!(
                "BibliZap is available again. The health check at {healthcheck_url} succeeded."
            ),
        ),
    };

    let mut message = Message::builder().from(smtp.from.clone()).subject(subject);
    for recipient in &smtp.recipients {
        message = message.to(recipient.clone());
    }
    let message = message.body(body).map_err(|error| error.to_string())?;

    let builder = match smtp.tls_mode {
        TlsMode::StartTls => SmtpTransport::starttls_relay(&smtp.host),
        TlsMode::Implicit => SmtpTransport::relay(&smtp.host),
        TlsMode::None => Ok(SmtpTransport::builder_dangerous(&smtp.host)),
    };
    let credentials = Credentials::new(smtp.username.clone(), smtp.password.clone());
    let mailer = builder
        .map_err(|error| error.to_string())?
        .port(smtp.port)
        .credentials(credentials)
        .timeout(Some(smtp.timeout))
        .build();

    mailer.send(&message).map_err(|error| error.to_string())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    env_logger::init();

    let config = Config::load(args.config)?;
    let client = Client::builder().timeout(config.request_timeout).build()?;
    let mut state = MonitorState::default();
    let mut interval = tokio::time::interval(config.check_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    log::info!(
        "Monitoring {} every {} seconds",
        config.healthcheck_url,
        config.check_interval.as_secs()
    );

    loop {
        interval.tick().await;

        let result = check_health(&client, &config.healthcheck_url).await;
        let healthy = result.is_ok();
        match &result {
            Ok(()) => log::debug!("Health check succeeded"),
            Err(error) => log::warn!("Health check failed: {error}"),
        }

        if let Some(notification) = state.observe(healthy, config.failure_threshold) {
            match send_notification(&config, notification).await {
                Ok(()) => {
                    state.notification_sent(notification);
                    log::info!("Sent {notification:?} notification");
                }
                Err(error) => {
                    // Leave the state unchanged so the next check retries the email.
                    log::error!("Unable to send {notification:?} notification: {error}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alerts_after_threshold_and_then_on_recovery() {
        let mut state = MonitorState::default();

        assert_eq!(state.observe(false, 2), None);
        assert_eq!(state.observe(false, 2), Some(Notification::Outage));
        state.notification_sent(Notification::Outage);
        assert_eq!(state.observe(false, 2), None);
        assert_eq!(state.observe(true, 2), Some(Notification::Recovery));
        state.notification_sent(Notification::Recovery);
        assert_eq!(state.observe(true, 2), None);
    }

    #[test]
    fn retries_a_notification_that_was_not_sent() {
        let mut state = MonitorState::default();

        assert_eq!(state.observe(false, 1), Some(Notification::Outage));
        assert_eq!(state.observe(false, 1), Some(Notification::Outage));
    }

    #[test]
    fn parses_structured_configuration() {
        let file: FileConfig = conf::Config::builder()
            .add_source(conf::File::from_str(
                r#"
                [health]
                url = "https://example.com/health"

                [smtp]
                host = "smtp.example.com"
                username = "monitor"
                password = "secret"

                [alerts]
                from = "monitor@example.com"
                to = ["owner@example.com"]
                "#,
                conf::FileFormat::Toml,
            ))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();

        let config = Config::try_from(file).unwrap();
        assert_eq!(config.check_interval, Duration::from_secs(30));
        assert_eq!(config.request_timeout, Duration::from_secs(10));
        assert_eq!(config.failure_threshold, 2);
        assert_eq!(config.smtp.port, 587);
    }
}
