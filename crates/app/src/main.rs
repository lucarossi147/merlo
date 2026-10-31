//! Merlo: learn to read music, one note at a time.

mod components;
mod geometry;
mod glyphs;
mod platform;
mod screens;
mod store;
mod survivor;

use dioxus::prelude::*;

/// The logo as a data URL, so the favicon needs no extra file.
const FAVICON: &str = concat!("data:image/svg+xml;utf8,", include_str!("../assets/brand/favicon.svg"));

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Title { "Merlo" }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1, viewport-fit=cover" }
        document::Meta { name: "theme-color", content: "#fbf7ef" }
        document::Link { rel: "icon", r#type: "image/svg+xml", href: FAVICON }
        screens::Shell { saved: store::load(), screen: screens::Screen::Home }
    }
}

#[cfg(test)]
mod tests;
