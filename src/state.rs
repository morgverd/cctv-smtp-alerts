use std::env::var;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use base64::{engine, Engine};
use reqwest::{Client, StatusCode};
use anyhow::{anyhow, Result};
use crate::events::AlarmEvent;

const WEBHOOK_TIMEOUT: Duration = Duration::from_secs(5);

fn base64_encode(v: String) -> String {
    engine::general_purpose::STANDARD.encode(v)
}

#[derive(Debug, Clone)]
pub(crate) struct State {
    username: String,
    password: String,
    webhook_url: String,
    webhook_key: String,
    alarm_subject: Option<String>,
    accepted_ip: Option<IpAddr>,
    http_client: Arc<Client>
}
impl State {
    pub fn new() -> (Self, SocketAddr) {
        (
            State {
                username: base64_encode(var("CCTV_USERNAME").expect("Missing CCTV_USERNAME")),
                password: base64_encode(var("CCTV_PASSWORD").expect("Missing CCTV_PASSWORD")),
                webhook_url: var("CCTV_WEBHOOK_URL").expect("Missing CCTV_WEBHOOK_URL"),
                webhook_key: var("CCTV_WEBHOOK_KEY").expect("Missing CCTV_WEBHOOK_KEY"),
                alarm_subject: var("CCTV_ALARM_SUBJECT").ok(),
                accepted_ip: var("CCTV_ALARM_IP").ok().and_then(|ip| IpAddr::from_str(&ip).ok()),
                http_client: Arc::new(Client::new())
            },
            var("CCTV_BIND_ADDR").expect("Missing CCTV_BIND_ADDR").parse::<SocketAddr>().expect("CCTV_BIND_ADDR is invalid")
        )
    }

    #[inline]
    pub fn creds_match(&self, username: String, password: String) -> bool {
        let filter = |v: String| v.chars().filter(|&c| !c.is_control()).collect::<String>();
        filter(username) == self.username && filter(password) == self.password
    }

    #[inline]
    pub fn is_alarm_subject(&self, subject: &str) -> bool {
        self.alarm_subject.as_ref().map_or(true, |v| v == subject)
    }

    #[inline]
    pub fn is_socketaddr_accepted(&self, ip: SocketAddr) -> bool {
        self.accepted_ip.as_ref().map_or(true, |v| v == &ip.ip())
    }

    #[inline]
    pub async fn send_event_webhook(&self, event: &AlarmEvent) -> Result<()> {
        let response = self.http_client
            .post(&self.webhook_url)
            .header("Authorization", &self.webhook_key)
            .json(event)
            .timeout(WEBHOOK_TIMEOUT)
            .send()
            .await?;

        let status = response.status();
        match status {
            StatusCode::OK => Ok(()),
            _ => Err(anyhow!("Webhook responded with non-ok status: {}", status.as_str())),
        }
    }
}