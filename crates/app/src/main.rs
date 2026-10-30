//! Merlo: learn to read music, one note at a time.

mod components;
mod geometry;
mod glyphs;
mod platform;
mod screens;
mod store;
mod survivor;

use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Title { "Merlo" }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1, viewport-fit=cover" }
        screens::Shell { saved: store::load(), screen: screens::Screen::Home }
    }
}

#[cfg(test)]
mod tests;
