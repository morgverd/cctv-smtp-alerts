mod config;
mod events;

use anyhow::{Context, Result};
use dotenv::dotenv;
use mail_parser::MessageParser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use crate::config::Config;
use crate::events::handle_email;

async fn send_response(writer: &mut OwnedWriteHalf, data: &[u8]) -> Result<()> {
    println!("<- {}", String::from_utf8_lossy(data).replace("\r", "").replace("\n", ""));
    writer.write_all(data).await.context("Failed to send response!")
}

async fn handle_message(
    message: &str,
    reader: &mut BufReader<OwnedReadHalf>,
    writer: &mut OwnedWriteHalf,
    authenticated: &mut bool,
    config: &Config
) -> Result<()> {

    println!("-> {}", message.replace("\r", "").replace("\n", ""));

    let parts: Vec<&str> = message.trim().splitn(2, " ").collect();
    let command = parts.get(0).unwrap_or(&"").to_uppercase();
    let argument = parts.get(1).unwrap_or(&"");

    match command.as_str() {
        "HELO" | "EHLO" => send_response(writer, b"250 Hello\r\n").await?,
        "QUIT" => send_response(writer, b"221 OK\r\n").await?, // TODO: Actually close connection.

        "MAIL" if argument.starts_with("FROM:") => send_response(writer, b"250 OK\r\n").await?,
        "RCPT" if argument.starts_with("TO:") => send_response(writer, b"250 OK\r\n").await?,
        "AUTH" if argument.eq_ignore_ascii_case("LOGIN") => {

            let mut username = String::new();
            let mut password = String::new();
            let prompts = vec![
                (b"334 VXNlcm5hbWU6\r\n", &mut username),
                (b"334 UGFzc3dvcmQ6\r\n", &mut password)
            ];

            // Prompt and read each part.
            for (prompt, storage) in prompts {
                writer.write_all(prompt).await?;
                writer.flush().await?;

                storage.clear();
                reader.read_line(storage).await?;
            }

            // Validate credentials.
            if config.creds_match(username, password) {
                *authenticated = true;
                send_response(writer, b"235 Authentication successful\r\n").await?;
            } else {
                send_response(writer, b"535 Authentication credentials invalid\r\n").await?;
            }
        },
        "DATA" if *authenticated => {

            send_response(writer, b"354 End data with <CR><LF>.<CR><LF>\r\n").await?;

            // Keep reading line by line until there is a single period terminator (or connection loss).
            // Read as bytes since the MessageParser takes it as bytes anyway.
            let mut data = String::new();
            let mut line = String::new();

            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 || line.trim() == "." {
                    break; // TODO: Handle connection close and message end separately.
                }
                data.push_str(&line);
                line.clear();
            }

            match MessageParser::default().parse(&data) {
                None => {
                    eprintln!("Failed to decode authenticated email!");
                    send_response(writer, b"550 Failed to parse email\r\n").await?;
                },
                Some(message) => {
                    handle_email(message).await;
                    send_response(writer, b"250 Message accepted\r\n").await?;
                }
            }
        },

        "." => send_response(writer, b"250 Message accepted\r\n").await?,
        _ => {
            if *authenticated {
                send_response(writer, b"500 Command unrecognised\r\n").await?;
            } else {
                send_response(writer, b"530 Authentication required\r\n").await?;
            }
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {

    dotenv().ok();

    let listener = TcpListener::bind("0.0.0.0:2525").await?;
    let config = Config::new();

    loop {
        let (socket, _) = listener.accept().await?;
        let config_clone = config.clone();

        tokio::spawn(async move {

            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            // Send fake greeting.
            if let Err(_) = writer.write_all(b"220 Welcome to the CCTV SMTP server\r\n").await {
                return;
            }

            let mut authenticated = false;
            while let Ok(n) = reader.read_line(&mut line).await {
                if n == 0 {
                    println!("Connection closed!");
                    break;
                }

                handle_message(&line, &mut reader, &mut writer, &mut authenticated, &config_clone)
                    .await
                    .expect("TODO: panic message");

                line.clear();
            }
        });
    }
}