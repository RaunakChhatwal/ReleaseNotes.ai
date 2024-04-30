use chrono::Local;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::form::Form;
use crate::util::{Arguments, Ticket};

#[component]
pub fn RootApp() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        <Body class="bg-gray-300" />

        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/getinsured.css" />

        // sets the document title
        <Title text="ReleaseNotes.ai" />

        // content for this welcome page
        <Router fallback=|| {
            view! {
                <h1>"404 Not Found"</h1>
            }
            .into_view()
        }>
            <main class="p-[2vw] text-[1.1em]">
                <Routes>
                    <Route path="" view=HomePage />
                    <Route path="/test" view=TestPage />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let mut arguments = Arguments::default();
    arguments.release_date = Local::now().date_naive();
    arguments.tickets.push(Ticket::default());

    view! {
        <App default_arguments={arguments} />
    }
}

#[component]
fn TestPage() -> impl IntoView {
    let mut arguments: Arguments = serde_json::from_str(include_str!("./templates/test-arguments.json")).unwrap();
    arguments.release_date = Local::now().date_naive();

    view! {
        <App default_arguments={arguments} />
    }
}

#[component]
fn App(default_arguments: Arguments) -> impl IntoView {
    let (release_notes, set_release_notes) = create_signal("".to_string());

    view! {
        <h1 class="text-[1.5em]">"ReleaseNotes.ai"</h1>
        <div class="grid grid-cols-[50vw_40vw]">
            <Form default_arguments set_release_notes />
            <ReleaseNotes release_notes />
        </div>
    }
}

#[component]
fn ReleaseNotes(release_notes: ReadSignal<String>) -> impl IntoView {
    view! {
        <div
            style:display=move || release_notes().is_empty().then(|| "None")
        >
            <h1 class="text-[1.2em] underline">"Release Notes"</h1>
            <p class="my-[5vh] p-[1vw] w-[35vw] text-[0.9rem] border-2 border-black">{
                release_notes
            }</p>
        </div>
    }
}