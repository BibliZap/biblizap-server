use std::{env, error::Error, time::Duration};

use lettre::{
    Message, SmtpTransport, Transport, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use reqwest::{Client, StatusCode, Url};
use serde::Deserialize;

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
    fn from_env() -> Result<Self, Box<dyn Error>> {
        let healthcheck_url = required_env("HEALTHCHECK_URL")?.parse()?;
        let check_interval = duration_env("CHECK_INTERVAL_SECONDS", 30)?;
        let request_timeout = duration_env("REQUEST_TIMEOUT_SECONDS", 10)?;
        let failure_threshold = parse_env("FAILURE_THRESHOLD", 2)?;
        if failure_threshold == 0 {
            return Err("FAILURE_THRESHOLD must be at least 1".into());
        }

        let recipients = required_env("ALERT_TO")?
            .split(',')
            .map(str::trim)
            .filter(|address| !address.is_empty())
            .map(str::parse)
            .collect::<Result<Vec<Mailbox>, _>>()?;
        if recipients.is_empty() {
            return Err("ALERT_TO must contain at least one email address".into());
        }

        let tls_mode = match env::var("SMTP_TLS_MODE")
            .unwrap_or_else(|_| "starttls".to_owned())
            .to_ascii_lowercase()
            .as_str()
        {
            "starttls" => TlsMode::StartTls,
            "implicit" => TlsMode::Implicit,
            "none" => TlsMode::None,
            value => {
                return Err(format!(
                    "invalid SMTP_TLS_MODE '{value}'; use starttls, implicit, or none"
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
                host: required_env("SMTP_HOST")?,
                port: parse_env("SMTP_PORT", 587)?,
                username: required_env("SMTP_USERNAME")?,
                password: required_env("SMTP_PASSWORD")?,
                tls_mode,
                timeout: duration_env("SMTP_TIMEOUT_SECONDS", 10)?,
                from: required_env("ALERT_FROM")?.parse()?,
                recipients,
            },
        })
    }
}

fn required_env(name: &str) -> Result<String, Box<dyn Error>> {
    env::var(name).map_err(|_| format!("required environment variable {name} is not set").into())
}

fn parse_env<T>(name: &str, default: T) -> Result<T, Box<dyn Error>>
where
    T: std::str::FromStr,
    T::Err: Error + 'static,
{
    match env::var(name) {
        Ok(value) => Ok(value.parse()?),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(error) => Err(error.into()),
    }
}

fn duration_env(name: &str, default_seconds: u64) -> Result<Duration, Box<dyn Error>> {
    let seconds = parse_env(name, default_seconds)?;
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
    dotenvy::dotenv().ok();
    env_logger::init();

    let config = Config::from_env()?;
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
}
