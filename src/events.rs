use std::fmt::{Display, Formatter};
use mail_parser::Message;
use crate::state::State;
use anyhow::{anyhow, Result};
use serde::{Serialize, Deserialize};
use serde_xml_rs::from_str;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct AlarmEvent {
    input1: Option<String>,
    event_type: String,
    extra_text: String,
    date_time: String
}
impl Display for AlarmEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(input) = self.input1.as_ref().filter(|input| !input.trim().is_empty()) {
            write!(f, "#{input} | ")?
        }
        write!(f, "{} @ {}", self.event_type, self.date_time)?;
        Ok(())
    }
}

pub(crate) async fn handle_email<'a>(
    message: Message<'a>,
    state: &State
) -> Result<()> {

    // Ignore non alarm emails (tests, healthcheck pings).
    if !state.is_alarm_subject(message.subject().unwrap_or("")) {
        println!("Ignoring non-alarm message!");
        return Ok(());
    }

    let body = match message.body_text(0) {
        None => return Err(anyhow!("Failed to read parsed email body text!")),
        Some(body) => body.clone()
    };

    let event: AlarmEvent = from_str(body.as_ref().trim())?;
    match state.send_event_webhook(&event).await {
        Ok(_) => println!("Successfully sent '{event}' event!"),
        Err(e) => println!("Failed to send '{event}' with error: {e}")
    }

    Ok(())
}