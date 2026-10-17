//! Builds the page the golden fixtures are cut from, and prints it.
//!
//! Run with `cargo run --example marketing_page`, or
//! `cargo run --example marketing_page -- --compact`.

use molt::Document;
use molt::prelude::*;
use molt::seo::SeoBuilder;

fn main() {
    let compact = std::env::args().any(|a| a == "--compact");

    let page = Document::new(Some("pt-BR"))
        .head_children(
            SeoBuilder::new(
                "RideKeeper — track every service",
                "Motorcycle maintenance companion",
            )
            .image("https://ridekeeper.example/og.jpg")
            .url("https://ridekeeper.example")
            .keywords(["rust", "motorcycle"])
            .author("Michel Lutz")
            .twitter_site("@micheltlutz")
            .build(),
        )
        .head_children([stylesheet("/css/style.css")])
        .body_children([
            header().child(
                nav()
                    .set_role("navigation")
                    .child(link_to("/").add_class("logo").text("RideKeeper"))
                    .child(
                        ul().child(li().child(link_to("/").text("Home")))
                            .child(li().child(link_to("/pricing").text("Pricing & plans"))),
                    ),
            ),
            main_tag().child(
                section()
                    .set_id("hero")
                    .child(h1().text("Track every service"))
                    .child(p().text("Fuel, tyres & chain — all in one place."))
                    .child(
                        link_to("https://apps.example/app")
                            .add_class("button")
                            .text("Download"),
                    ),
            ),
            footer().child(p().text("© 2026 RideKeeper — built with Rust & molt")),
        ]);

    // The audit is a development aid; it costs nothing in a release build.
    let issues = molt::accessibility::audit(&page.root().into());
    for issue in &issues {
        eprintln!("a11y: <{}> — {}", issue.tag, issue.message);
    }

    println!(
        "{}",
        if compact {
            page.render_compact()
        } else {
            page.render()
        }
    );
}
