use chrono::NaiveDate;
use leptos::*;

use wasm_bindgen::prelude::*;
use web_sys::{js_sys, ErrorEvent, MessageEvent, WebSocket};

use crate::util::{Arguments, DynResult, TargetAudience, Ticket};

#[derive(Clone, Debug)]
enum Progress {
    Cloning,
    Streaming
}

#[component]
fn TicketForm(
    ticket: ReadSignal<Ticket>,
    set_ticket: WriteSignal<Ticket>,
    tickets: ReadSignal<Vec<(usize, (ReadSignal<Ticket>, WriteSignal<Ticket>))>>,
    set_tickets: WriteSignal<Vec<(usize, (ReadSignal<Ticket>, WriteSignal<Ticket>))>>,
    id: usize
) -> impl IntoView {
    let delete_ticket = move |_| {
        set_tickets.update(|tickets| {
            tickets.retain(|&(ticket_id, (read_signal, write_signal))| {
                if ticket_id == id {
                    read_signal.dispose();
                    write_signal.dispose();
                    return false;
                } else {
                    return true;
                }
            });
        });
    };

    view! {
        <div class="relative p-4 text-[0.9rem] border-2 border-black">
            <button
                class="absolute top-0 right-0 m-2 px-[0.3em] py-[0.15em] border-2 border-black hover:bg-gray-200"
                style:display=move || (tickets().len() <= 1).then(|| "None")
                on:click=delete_ticket
            >"Remove"</button>
            <div class="grid grid-cols-full gap-4 p-[0.75vw]">
                <div>
                    <p class="text-[0.95rem]">"Summary"</p>
                    <input
                        class="w-full px-[3px] bg-gray-200 border-2 border-black"
                        type = "text"
                        value=move || ticket().summary
                        on:input = move |event| set_ticket.update(|ticket|
                            ticket.summary = event_target_value(&event)
                        ) />
                </div>
                <div>
                    <p class="text-[0.95rem]">"Description"</p>
                    <textarea
                        class="w-full h-[10rem] px-[3px] bg-gray-200 border-2 border-black"
                        type = "text"
                        defaultValue=move || ticket().description
                        on:input = move |event| set_ticket.update(|ticket|
                            ticket.description = event_target_value(&event))
                    >{ticket.get_untracked().description}</textarea>
                </div>
            </div>
        </div>
    }
}

fn setup_callbacks(
    web_socket: &mut WebSocket,
    arguments: Arguments,
    set_release_notes: WriteSignal<String>,
    progress: ReadSignal<Option<Progress>>,
    set_progress: WriteSignal<Option<Progress>>,
    set_error_message: WriteSignal<String>
) -> DynResult<()> {
    web_socket.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let ws = web_socket.clone();
    let on_open = Closure::<dyn FnMut()>::new(move || {
        if let Err(error) = ws.send_with_str(&serde_json::to_string(&arguments).unwrap()) {
            set_error_message(format!("{error:?}"));
            let _ = ws.close();
            set_progress(None);
        }
    });
    web_socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    on_open.forget();    // forget the callback to keep it alive


    let ws = web_socket.clone();
    let on_message = Closure::<dyn FnMut(_)>::new(move |event: MessageEvent| {
        if let Ok(message) = event.data().dyn_into::<js_sys::JsString>() {
            let message: String = message.into();

            let token;
            match serde_json::from_str::<Result<String, String>>(&message).unwrap() {
                Ok(new_token) => token = new_token,
                Err(error_message) => {
                    set_error_message(error_message);
                    let _ = ws.close();
                    set_progress(None);
                    return;
                }
            }
            match progress.get_untracked() {
                Some(Progress::Cloning) => set_progress(Some(Progress::Streaming)),
                Some(Progress::Streaming) => set_release_notes.update(|release_notes|
                    *release_notes += &token),
                None => {
                    let _ = ws.close();
                }
            }
        } else {
            set_error_message("Error parsing message.".to_string());
        }
    });
    web_socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    let ws = web_socket.clone();
    let on_error = Closure::<dyn FnMut(_)>::new(move |error: ErrorEvent| {
        set_error_message(error.message());
        let _ = ws.close();
        set_progress(None);
    });
    web_socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    on_error.forget();

    let on_close = Closure::<dyn FnMut()>::new(move || {
        set_progress(None);
    });
    web_socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    on_close.forget();

    return Ok(());
}

#[component]
pub fn Form(default_arguments: Arguments, set_release_notes: WriteSignal<String>) -> impl IntoView {
    let (repo_link, set_repo_link) = create_signal(default_arguments.repo_link);
    let (product_name, set_product_name) = create_signal(default_arguments.product_name);
    let (release_tag, set_release_tag) = create_signal(default_arguments.release_tag);
    let (prev_release_tag, set_prev_release_tag) = create_signal(default_arguments.prev_release_tag);
    let (release_date, set_release_date) = create_signal(default_arguments.release_date);
    let (target_audience, set_target_audience) = create_signal(default_arguments.target_audience);
    let mut counter = default_arguments.tickets.len();
    let (tickets, set_tickets) = create_signal(default_arguments
        .tickets
        .into_iter()
        .enumerate()
        .map(|(i, ticket)|
            return (i, create_signal(ticket)))
        .collect::<Vec<_>>()
    );
    let (progress, set_progress) = create_signal(None::<Progress>);
    let (web_socket, set_web_socket) = create_signal(None::<WebSocket>);
    let (error_message, set_error_message) = create_signal("".to_string());

    let on_submit = move |_| {
        set_error_message("".to_string());

        let arguments = Arguments {
            repo_link: repo_link(),
            product_name: product_name(),
            release_tag: release_tag(),
            prev_release_tag: prev_release_tag(),
            release_date: release_date(),
            target_audience: target_audience(),
            tickets: tickets()
                .iter()
                .map(|(_, (ticket, _))| ticket())
                .collect::<Vec<Ticket>>(),
        };

        for field in vec![&arguments.repo_link, &arguments.product_name, &arguments.release_tag, &arguments.prev_release_tag]
            .into_iter()
            .chain(arguments.tickets
                .iter()
                .flat_map(|Ticket {summary, description}|
                    vec![summary, description])
        ) {
            if field.is_empty() {
                set_error_message("A field has been left empty.".to_string());
            }
        }

        set_progress(Some(Progress::Cloning));
        set_release_notes("".to_string());

        let mut web_socket;
        match WebSocket::new(&format!("ws://{}/submit", window().location().host().unwrap())) {
            Ok(websocket) => web_socket = websocket,
            Err(error) => {
                set_error_message(format!("{error:?}"));
                set_progress(None);
                return;
            }
        }

        if let Err(error) = setup_callbacks(&mut web_socket, arguments, set_release_notes, progress, set_progress, set_error_message) {
            set_error_message(format!("{error:?}"));
            return;
        }

        set_web_socket(Some(web_socket));
    };

    let add_ticket = move |_| {
        counter += 1;
        set_tickets.update(|tickets| tickets.push((counter, create_signal(Ticket::default()))));
    };

    view! {
        <div class="px-[2vw] py-[6vh]">
            <div class="grid grid-cols-[repeat(2,max-content)] gap-4 mb-[3vh]">
                <p>"Repository link:"</p>
                <input
                    class="w-[25em] px-[3px] text-[1rem] placeholder-gray-500 bg-gray-200 border-2 border-black"
                    type="text"
                    value={repo_link}
                    on:input = move |event| set_repo_link(event_target_value(&event))
                    placeholder = "https://github.com/example/example.git" />
                <p>"Product name:"</p>
                <input
                    class="w-[10em] px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                    type="text"
                    value={product_name}
                    on:input = move |event| set_product_name(event_target_value(&event)) />
                <p>"Release tag:"</p>
                <input
                    class="w-[10em] px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                    type="text"
                    value={release_tag}
                    on:input = move |event| set_release_tag(event_target_value(&event)) />
                <p>"Previous release tag:"</p>
                <input
                    class="w-[10em] px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                    type="text"
                    value={prev_release_tag}
                    on:input = move |event| set_prev_release_tag(event_target_value(&event)) />
                <p>"Release date:"</p>
                <input
                    class="w-[10em] px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                    type="date"
                    value=move || release_date().format("%Y-%m-%d").to_string()
                    on:input={move |event| {
                        let value = event_target_value(&event);
                        set_release_date(
                            NaiveDate::parse_from_str(&value, "%Y-%m-%d").unwrap()
                        );
                    }} />
                <p>"Target Audience:"</p>
                <select
                    class="w-[10em] px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                    on:select = move |event| set_target_audience(
                        serde_json::from_str(&event_target_value(&event)).unwrap()
                    )
                >
                    <option
                        value="NonTechnical"
                        selected=move || target_audience() == TargetAudience::NonTechnical
                    >"Non-technical"</option>
                    <option
                        value="ProjectManager"
                        selected=move || target_audience() == TargetAudience::ProjectManager
                    >"Project manager"</option>
                    <option
                        value="Technical"
                        selected=move || target_audience() == TargetAudience::Technical
                    >"Technical"</option>
                </select>
            </div>
            <h1 class="text-[1.2em] underline">"Tickets"</h1>
            <div
                class="grid grid-cols-[repeat(2,20vw)] gap-8 m-[1vw]"
            >
                <For
                    each=tickets
                    key=|&counter| counter.0
                    children=move |(id, (ticket, set_ticket))| {
                        view! {
                            <TicketForm ticket set_ticket tickets set_tickets id />
                        }
                    }
                />
            </div>
            <button
                class="px-[0.5em] py-[0.2em] mb-[3vh] border-2 border-black hover:bg-gray-200"
                on:click=add_ticket>"Add"</button>
            <div class="flex mt-[2vh]">
                <p
                    class="pr-[0.5em] py-[0.2em]"
                    style:display=move || progress().is_none().then(|| "None")
                >{move || progress().map(|progress| format!("{progress:?}"))}</p>
                <button
                    class="px-[0.5em] py-[0.2em] border-2 border-black hover:bg-gray-200"
                    style:display=move || progress().map(|_| "None")
                    on:click=on_submit
                >"Submit"</button>
                <button
                    class="px-[0.5em] py-[0.2em] border-2 border-black hover:bg-gray-200"
                    style:display=move || progress().is_none().then(|| "None")
                    on:click=move |_| {
                        web_socket.get_untracked().map(|web_socket|
                            web_socket.close());
                        set_web_socket(None);
                    }
                >"Cancel"</button>
            </div>
            <p
                class="text-red-600"
                style:display=move || error_message().is_empty().then(|| "None")
            >{error_message}</p>
        </div>
    }
}