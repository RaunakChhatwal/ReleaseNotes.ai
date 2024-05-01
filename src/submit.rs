use anyhow::{anyhow, Result};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::fetch_tokens::fetch_tokens;
use crate::git::{read_commit_messages, fetch_or_clone};
use crate::util::{Arguments, TargetAudience, Ticket};

fn generate_prompt(
    product_name: &str,
    release_version: &str,
    release_date: chrono::NaiveDate,
    target_audience: TargetAudience,
    tickets: Vec<Ticket>,
    commit_messages: Vec<String>
) -> String {
    let directive = format!("IMPORTANT: Your target audience is: {target_audience:?}. You must take this into account.");

    let prompt = format!("Template:\n{product_name} Release Notes - {release_version} - {release_date}\n\n")
        + include_str!("./templates/template.md");

    return format!("Tickets:\n{}\n\nCommit messages:\n{}\n\n{directive}\n\n{prompt}",
        tickets
            .iter()
            .map(|ticket|
                format!("Summary:{}\nDescription:{}", ticket.summary, ticket.description))
            .collect::<Vec<_>>()
            .join("\n--------------------\n"),
        commit_messages.join("\n"));
}

async fn handle_request(arguments: Arguments, sender: mpsc::UnboundedSender<String>) -> Result<()> {
    if arguments.any_field_empty() {
        return Err(anyhow!("A field has been left empty."));
    }

    let Arguments { repo_link, product_name, release_tag, prev_release_tag, release_date, target_audience, tickets } = arguments;

    // fetch from origin if repo is already present, otherwise clone from link
    // tags don't need to be merged into local, so fetching is enough
    let mut repo = fetch_or_clone(repo_link)?;
    let commit_messages = read_commit_messages(&mut repo, &release_tag, &prev_release_tag)?;

    let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| anyhow!("No OpenAI API Key"))?;
    let prompt = generate_prompt(&product_name, &release_tag, release_date, target_audience, tickets, commit_messages);
    let mut token_stream = fetch_tokens(&api_key, &prompt, include_str!("./templates/prompt.txt"));
    sender.send("Streaming".to_string())?;
    while let Some(token) = token_stream.next().await {
        match token {
            Ok(Some(token)) => {
                sender.send(token)?;
            },
            Ok(None) => {
                break;
            },
            Err(error) => {
                return Err(anyhow!("Error fetching tokens: {error}"));
            }
        }
    }

    Ok(())
}

#[derive(thiserror::Error, Debug)]
enum ParseError {
    #[error("Close handshake initiated.")]
    CloseRequest,
    #[error("Connection error: {0}.")]
    ConnectionError(anyhow::Error),
    #[error("Unable to parse message.")]
    UnsupportedFormat,
    #[error("Unable to parse message.")]
    InvalidArguments
}

async fn parse_arguments(socket: &mut WebSocket) -> Result<Arguments, ParseError> {
    match socket.recv().await {
        Some(Ok(Message::Text(message))) => {
            if let Ok(arguments) = serde_json::from_str::<Arguments>(&message) {
                return Ok(arguments);
            } else {
                return Err(ParseError::InvalidArguments);
            }
        },
        Some(Ok(Message::Close(_))) => {
            return Err(ParseError::CloseRequest);
        },
        // messages in non-text format aren't supported
        Some(Ok(_)) => {
            return Err(ParseError::UnsupportedFormat);
        },
        Some(Err(error)) => {
            return Err(ParseError::ConnectionError(anyhow!("{error}")));
        }
        None => {
            return Err(ParseError::ConnectionError(anyhow!("Connection is dead.")));
        }
    }
}

async fn handle_socket(mut socket: WebSocket) {
    let arguments;
    match parse_arguments(&mut socket).await {
        Ok(args) => arguments = args,
        Err(ParseError::CloseRequest) => {
            return;
        }
        Err(error) => {
            let _ = socket.send(Message::Text(anyhow!("{error}").to_string())).await;
            return;
        }
    }

    let (sender, mut recv) = mpsc::unbounded_channel();         // for output tokens
    let mut handle = tokio::spawn(handle_request(arguments, sender));

    loop {
        // the server must respond to the events below
        tokio::select! {
            // when there is an output token, it should be immediately sent to the client
            Some(token) = recv.recv() => {
                let serialized_ok = serde_json::to_string(&Ok::<String, String>(token))
                    .expect("Serializing Result<String, String> should always succeed.");
                // purposefully ignore an error since the error message would need to reach the client with the broken socket
                let _ = socket.send(Message::Text(serialized_ok)).await;
            }
            // if it receives a close message from the client, the server must end the connection
            Some(Ok(Message::Close(_))) = socket.next() => {
                break;
            }
            // if the request has been handled or there is an error, close the connection
            result = &mut handle => {
                match result {
                    Ok(Ok(())) => {
                        // for some reason, handle sometimes enters the event queue before the last token does
                        // This is to flush all tokens out from the sender
                        while let Some(token) = recv.recv().await {
                            let serialized_ok = serde_json::to_string(&Ok::<String, String>(token))
                                .expect("Serializing Result<String, String> should always succeed.");
                            let _ = socket.send(Message::Text(serialized_ok)).await;
                        }
                    },
                    Ok(Err(error)) => {
                        let serialized_error = serde_json::to_string(&Err::<String, String>(error.to_string()))
                            .expect("Serializing Result<String, String> should always succeed.");
                        let _ = socket.send(Message::Text(serialized_error)).await;
                    },
                    Err(error) => {
                        let serialized_error = serde_json::to_string(&Err::<String, String>(error.to_string()))
                            .expect("Serializing Result<String, String> should always succeed.");
                        let _ = socket.send(Message::Text(serialized_error)).await;
                    }
                }
                break;
            }
        }
    }
}

pub async fn submit(
    web_socket: WebSocketUpgrade,
    _: Option<axum_extra::TypedHeader<headers::UserAgent>>,
) -> impl axum::response::IntoResponse {
    web_socket.on_upgrade(move |socket| async move {
        handle_socket(socket).await;
    })
}