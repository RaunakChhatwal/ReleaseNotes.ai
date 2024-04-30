use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse
};
use axum_extra::TypedHeader;
use itertools::Itertools;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::util::{serialize_err, serialize_ok, Arguments, DynResult, TargetAudience, Ticket};
use crate::fetch_tokens::fetch_tokens;
use crate::git::{read_commit_messages, fetch_or_clone};

fn generate_prompt(
    product_name: &str,
    release_version: &str,
    release_date: chrono::NaiveDate,
    target_audience: TargetAudience,
    tickets: Vec<Ticket>,
    commit_messages: Vec<String>
) -> String {
    let mut prompt = format!("Tickets:\n{}\n\n", &tickets
        .iter()
        .map(|ticket|
            serde_json::to_string(ticket).unwrap())
        .join("\n"));

    prompt += &format!("Commit messages:\n{}\n\n", &commit_messages.join("\n"));

    prompt += &format!("IMPORTANT: Your target audience is: {target_audience:?}. You must take this into account.\n\n");

    prompt += &format!("Template:\n{product_name} Release Notes - {release_version} - {release_date}\n\n");
    prompt += include_str!("./templates/template.md");

    return prompt;
}

async fn handle_request(arguments: Arguments, sender: mpsc::UnboundedSender<String>) -> DynResult<()> {
    let Arguments { repo_link, product_name, release_tag, prev_release_tag, release_date, target_audience, tickets } = arguments;
    if tickets.is_empty() {
        return Err("At least one ticket must be specified.".into());
    }

    // iterate through all string fields
    for field in vec![&repo_link, &product_name, &release_tag, &prev_release_tag]
        .into_iter()
        .chain(tickets
            .iter()
            .flat_map(|Ticket {summary, description}|
                vec![summary, description])
    ) {
        if field.is_empty() {
            return Err("A field has been left empty.".into());
        }
    }

    // fetch from origin if repo is already present, otherwise clone from link
    // tags don't need to be merged into local, so fetching is enough
    let mut repo = fetch_or_clone(repo_link)?;
    let commit_messages = read_commit_messages(&mut repo, &release_tag, &prev_release_tag)?;

    let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| "No OpenAI API Key")?;
    let prompt = generate_prompt(&product_name, &release_tag, release_date, target_audience, tickets, commit_messages);
    let mut token_stream = std::pin::pin!(fetch_tokens(&api_key, &prompt, include_str!("./templates/prompt.txt")));
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
                return Err(format!("Error fetching tokens: {error}").into());
            }
        }
    }

    return Ok(());
}

async fn parse_arguments(socket: &mut WebSocket) -> DynResult<Arguments>{
    match socket.recv().await {
        Some(Ok(Message::Text(message))) => {
            if let Ok(arguments) = serde_json::from_str::<Arguments>(&message) {
                return Ok(arguments);
            } else {
                return Err("Unable to parse request.".into());
            }
        },
        Some(Ok(_)) => {
            return Err("Unable to parse request.".into());
        },
        Some(Err(error)) => {
            return Err(error.into());
        }
        None => {
            return Err("Connection no longer alive".into());
        }
    }
}

async fn handle_socket(mut socket: WebSocket) {
    let arguments;
    match parse_arguments(&mut socket).await {
        Ok(args) => arguments = args,
        Err(error) => {
            let _ = socket.send(Message::Text(serialize_err(error))).await;
            return;
        }
    }

    let (sender, mut recv) = mpsc::unbounded_channel();
    let mut handle = futures::stream::once(tokio::spawn(handle_request(arguments, sender)));
    let (mut sink, mut stream) = socket.split();

    loop {
        tokio::select! {
            Some(token) = recv.recv() => {
                let _ = sink.send(Message::Text(serialize_ok(&token))).await;
            }
            Some(Ok(Message::Close(_))) = stream.next() => {
                break;
            }
            Some(result) = handle.next() => {
                match result {
                    Ok(Ok(())) => (),
                    Ok(Err(error)) => {
                        let _ = sink.send(Message::Text(serialize_err(error))).await;
                    },
                    Err(error) => {
                        let _ = sink.send(Message::Text(serialize_err(Box::new(error)))).await;
                    }
                }
                break;
            }
        }
    }
}

pub async fn submit(
    web_socket: WebSocketUpgrade,
    _: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    web_socket.on_upgrade(move |socket| async move {
        handle_socket(socket).await;
    })
}