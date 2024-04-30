#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::{Router, routing};
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use releasenotes::app::*;
    use releasenotes::submit::submit;
    use releasenotes::fileserv::file_and_error_handler;

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).await.unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(RootApp);

    // build our application with a route
    let app = Router::new()
        .route("/submit", routing::get(submit))
        .leptos_routes(&leptos_options, routes, RootApp)
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
