#[cfg(feature = "ssr")]
use std::hash::{DefaultHasher, Hash, Hasher};

use leptos::*;

#[derive(Clone)]
enum Progress {
    Cloning,
    Streaming
}

#[cfg(feature = "ssr")]
fn pull_or_clone(link: String) -> Result<(), git2::Error> {
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
        repo.merge(&[&repo.annotated_commit_from_fetchhead("main", &link, &reference.target().unwrap())?], None, None)?;
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
pub fn Form() -> impl IntoView {
    let (repo_link, set_repo_link) = create_signal("".to_string());
    let (progress, set_progress) = create_signal(None::<Progress>);
    let (error_message, set_error_message) = create_signal("".to_string());

    view! {
        <div class="ml-[3vw]">
            <label class="mr-[1em]">"HTTP clone link:"</label>
            <input
                class="w-[25em] px-[3px] text-[1rem] border-2 border-black"
                type = "text"
                // value = {repo_link}
                on:input = move |event| set_repo_link(event_target_value(&event))
                placeholder = "https://github.com/example/example.git" />
            <br />
            <label
                class="pr-[0.5em] py-[0.2em]"
                style:display=move || (!matches!(progress(), Some(Progress::Cloning))).then(|| "None")
            >"Cloning"</label>
            <button
                class="px-[0.5em] py-[0.2em] border-2 border-black hover:bg-gray-200"
                style:display=move || progress().map(|_| "None")
                on:click=move |_| {
                    set_error_message("".to_string());
                    set_progress(Some(Progress::Cloning));
                    spawn_local(async move {
                        if let Err(error) = clone_repo(repo_link()).await {
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
                }
            >"Clone"</button>
            <button
                class="px-[0.5em] py-[0.2em] border-2 border-black hover:bg-gray-200"
                style:display=move || progress().is_none().then(|| "None")
                on:click=move |_| set_progress(None)
            >"Cancel"</button>
            <p
                class="text-red-600"
                style:display=move || Some(error_message()).filter(|error_message| error_message == "").map(|_| "None")
            >{error_message}</p>
        </div>
    }
}