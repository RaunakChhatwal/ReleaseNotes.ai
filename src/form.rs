#[cfg(feature = "ssr")]
use std::hash::{DefaultHasher, Hash, Hasher};

use leptos::*;

#[derive(Clone)]
enum Progress {
    Cloning,
    Streaming
}

#[derive(Clone, Default)]
struct Ticket {
    summary: String,
    target_branch: String,
    feature_branch: String,
    description: String
}

#[cfg(feature = "ssr")]
fn pull_or_clone(link: String) -> Result<(), git2::Error> {
    // the link is hashed and the repo is cloned into ./repos/<hash> rather than ./repos/<repo-name>
    // this is because different repos may have the same name
    let mut hasher = DefaultHasher::new();
    link.hash(&mut hasher);
    let hash = hasher.finish();
    let repo_path = format!("./repos/{hash}");
    let repo_path = std::path::Path::new(&repo_path);

    if repo_path.exists() {
        // pull from origin
        let repo = git2::Repository::open(repo_path)?;
        let mut remote = repo.find_remote("origin")?;
        remote.fetch::<&str>(&[], None, None)?;
        let reference = repo.resolve_reference_from_short_name("origin/main")?;
        repo.merge(&[
            &repo.annotated_commit_from_fetchhead("main", &link, &reference.target().unwrap())?
        ], None, None)?;
    } else {
        git2::Repository::clone(&link, repo_path)?;
    }

    return Ok(());
}

#[server]
async fn clone_repo(link: String) -> Result<(), ServerFnError> {
    match pull_or_clone(link) {
        Ok(_) => Ok(()),
        Err(error) => Err(ServerFnError::ServerError(error.message().to_string()))
    }
}

#[component]
fn TicketForm(
    tickets: ReadSignal<Vec<(usize, (ReadSignal<Ticket>, WriteSignal<Ticket>))>>,
    set_tickets: WriteSignal<Vec<(usize, (ReadSignal<Ticket>, WriteSignal<Ticket>))>>,
    id: usize,
    set_ticket: WriteSignal<Ticket>
) -> impl IntoView {
    let delete_ticket = move |_| {
        set_tickets.update(|tickets| {
            tickets.retain(|&(ticket_id, (read_signal, write_signal))| {
                if ticket_id == id {
                    read_signal.dispose();
                    write_signal.dispose();
                }
                ticket_id != id
            });
        });
    };

    view! {
        <div class="relative p-4 border-2 border-black">
            <button
                class="absolute top-0 right-0 m-2 px-[0.3em] py-[0.15em] text-[0.9em] border-2 border-black hover:bg-gray-200"
                style:display=move || (tickets().len() <= 1).then(|| "None")
                on:click=delete_ticket
            >"Remove"</button>
            <div class="grid grid-cols-[repeat(2,max-content)] gap-4 p-[0.75vw] text-[1rem]">
                <div class="col-span-2">
                    <p>"Summary"</p>
                    <input
                        class="w-full px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                        type = "text"
                        on:input = move |event| set_ticket.update(|ticket|
                            ticket.summary = event_target_value(&event)
                        ) />
                </div>
                <div>
                    <p>"Feature branch (hash)"</p>
                    <input
                        class="w-[10em] px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                        type = "text"
                        on:input = move |event| set_ticket.update(|ticket|
                            ticket.feature_branch = event_target_value(&event)
                        ) />
                </div>
                <div>
                    <p>"Target branch (hash)"</p>
                    <input
                        class="w-[10em] px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                        type = "text"
                        on:input = move |event| set_ticket.update(|ticket|
                            ticket.target_branch = event_target_value(&event)
                        ) />
                </div>
                <div class="col-span-2">
                    <p>"Description"</p>
                    <textarea
                        class="w-full h-[8rem] px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                        type = "text"
                        on:input = move |event| set_ticket.update(|ticket|
                            ticket.description = event_target_value(&event)
                        ) />
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn Form() -> impl IntoView {
    let (repo_link, set_repo_link) = create_signal("".to_string());
    let (product_name, set_product_name) = create_signal("".to_string());
    let (version_number, set_version_number) = create_signal("".to_string());
    let mut counter = 0usize;
    let (tickets, set_tickets) = create_signal(vec![(counter, create_signal(Ticket::default()))]);
    let (progress, set_progress) = create_signal(None::<Progress>);
    let (error_message, set_error_message) = create_signal("".to_string());

    let on_submit = move |_| {
        set_error_message("".to_string());

        let repo_link = repo_link();
        let product_name = product_name();
        let version_number = version_number();
        let tickets = tickets()
            .iter()
            .map(|(_, (ticket, _))| ticket())
            .collect::<Vec<Ticket>>();

        for field in vec![&repo_link, &product_name, &version_number]
            .into_iter()
            .chain(tickets
                .iter()
                .flat_map(|Ticket {summary, feature_branch, target_branch, description}|
                    vec![summary, feature_branch, target_branch, description])
        ) {
            if field.is_empty() {
                set_error_message("A field has been left empty.".to_string());
                return;
            }
        }

        spawn_local(async move {
            set_progress(Some(Progress::Cloning));
            if let Err(error) = clone_repo(repo_link).await {
                if let ServerFnError::ServerError(error_message) = error {
                    set_error_message(format!("Error while cloning: {error_message}"));
                } else {
                    set_error_message(error.to_string());
                }
                set_progress(None);
                return;
            }
            set_progress(Some(Progress::Streaming));
        });
    };

    let add_ticket = move |_| {
        counter += 1;
        set_tickets.update(|tickets| tickets.push((counter, create_signal(Ticket::default()))));
    };

    view! {
        <div class="ml-[3vw]">
            <div class="grid grid-cols-[repeat(2,max-content)] gap-4 mb-[3vh]">
                <p>"HTTP clone link:"</p>
                <input
                    class="w-[25em] px-[3px] text-[1rem] placeholder-gray-500 bg-gray-200 border-2 border-black"
                    type = "text"
                    on:input = move |event| set_repo_link(event_target_value(&event))
                    placeholder = "https://github.com/example/example.git" />
                <p>"Product name:"</p>
                <input
                    class="w-[15em] px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                    type = "text"
                    on:input = move |event| set_product_name(event_target_value(&event)) />
                <p>"Version number:"</p>
                <input
                    class="w-[15em] px-[3px] text-[1rem] bg-gray-200 border-2 border-black"
                    type = "text"
                    on:input = move |event| set_version_number(event_target_value(&event)) />
            </div>
            <h1 class="text-[1.2em] underline">"Tickets"</h1>
            <div
                class="grid grid-cols-[repeat(2,max-content)] gap-8 m-[1vw]"
            >
                <For
                    each=tickets
                    key=|&counter| counter.0
                    children=move |(id, (_, set_ticket))| {
                        view! {
                            <TicketForm tickets set_tickets id set_ticket />
                        }
                    }
                />
            </div>
            <button
                class="px-[0.5em] py-[0.2em] mb-[3vh] border-2 border-black hover:bg-gray-200"
                on:click=add_ticket>"Add"</button>
            <div class="flex">
                <p
                    class="pr-[0.5em] py-[0.2em]"
                    style:display=move || (!matches!(progress(), Some(Progress::Cloning))).then(|| "None")
                >"Cloning"</p>
                <button
                    class="px-[0.5em] py-[0.2em] border-2 border-black hover:bg-gray-200"
                    style:display=move || progress().map(|_| "None")
                    on:click=on_submit
                >"Submit"</button>
                <button
                    class="px-[0.5em] py-[0.2em] border-2 border-black hover:bg-gray-200"
                    style:display=move || progress().is_none().then(|| "None")
                    on:click=move |_| set_progress(None)
                >"Cancel"</button>
            </div>
            <p
                class="text-red-600"
                style:display=move || error_message().is_empty().then(|| "None")
            >{error_message}</p>
        </div>
    }
}