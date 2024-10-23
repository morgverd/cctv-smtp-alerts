mod state;
mod events;

use anyhow::{Context, Result};
use dotenv::dotenv;
use mail_parser::MessageParser;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use crate::state::State;
use crate::events::handle_email;

pub(crate) async fn send_response(writer: &mut OwnedWriteHalf, data: &[u8]) -> Result<()> {

    #[cfg(feature = "debug")]
    println!("<- {}", String::from_utf8_lossy(data).replace("\r", "").replace("\n", ""));

    writer.write_all(data).await.context("Failed to send response!")
}

async fn handle_message(
    message: &str,
    reader: &mut BufReader<OwnedReadHalf>,
    writer: &mut OwnedWriteHalf,
    authenticated: &mut bool,
    state: &State
) -> Result<bool> {

    #[cfg(feature = "debug")]
    println!("-> {}", message.replace("\r", "").replace("\n", ""));

    let parts: Vec<&str> = message.trim().splitn(2, " ").collect();
    let command = parts.get(0).unwrap_or(&"").to_uppercase();
    let argument = parts.get(1).unwrap_or(&"");

    match command.as_str() {
        "HELO" | "EHLO" => send_response(writer, b"250 Hello\r\n").await?,
        "QUIT" => {
            send_response(writer, b"221 OK\r\n").await?;
            return Ok(true);
        },
        "MAIL" => send_response(writer, b"250 OK\r\n").await?,
        "RCPT" => send_response(writer, b"250 OK\r\n").await?,
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
            if state.creds_match(username, password) {
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
                if n == 0 {
                    return Ok(true);
                }
                if line.trim() == "." {
                    break;
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
                    handle_email(message, state).await?;
                    send_response(writer, b"250 Message accepted\r\n").await?;
                }
            }
        },
        _ => {
            if *authenticated {
                send_response(writer, b"500 Command unrecognised\r\n").await?;
            } else {
                send_response(writer, b"530 Authentication required\r\n").await?;
            }
        }
    };

    Ok(false)
}

async fn handle_connection(
    socket: TcpStream,
    state: State
) -> Result<()> {

    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);

    // Send fake greeting.
    writer.write_all(b"220 Welcome to the CCTV SMTP server\r\n").await?;

    let mut authenticated = false;
    let mut line = String::new();

    while let Ok(n) = reader.read_line(&mut line).await {
        if n == 0 {
            return Ok(());
        }
        if handle_message(&line, &mut reader, &mut writer, &mut authenticated, &state).await? {
            break;
        }
        line.clear();
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {

    dotenv().ok();

    let (state, bind_addr) = State::new();
    let listener = TcpListener::bind(&bind_addr).await?;
    println!("Started listening to: {bind_addr}");

    loop {
        let (socket, addr) = listener.accept().await?;
        if !state.is_socketaddr_accepted(addr) {
            eprintln!("Connection attempted from disallowed IP: {}", addr.ip().to_string());
            continue;
        }

        let state_clone = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, state_clone).await {
                eprintln!("Error: {e:#?}");
            }
        });
    }
}