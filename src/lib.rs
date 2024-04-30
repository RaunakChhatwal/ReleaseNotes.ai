pub mod util;

#[cfg(feature = "ssr")]
pub mod git;

#[cfg(feature = "ssr")]
pub mod fetch_tokens;

#[cfg(feature = "ssr")]
pub mod submit;

pub mod ticket_form;
pub mod form;
pub mod app;
#[cfg(feature = "ssr")]
pub mod fileserv;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(RootApp);
}
