//! Generates a small static site into `dist/`, with a sitemap and an RSS feed.
//!
//! Run with `cargo run --example static_site`.

// The `ssg` module is compiled out on wasm32 — there is no filesystem there — but
// `cargo test --target wasm32` still builds every example, so the whole thing is gated and
// a stub `main` stands in.
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use molt::feed::{RssGenerator, RssItem};
    use molt::prelude::*;
    use molt::sitemap::{ChangeFreq, SitemapGenerator, SitemapUrl};
    use molt::ssg::StaticSiteGenerator;
    use molt::{Document, Layout};

    const SITE: &str = "https://ridekeeper.example";

    /// The shell every page shares.
    struct Shell {
        site_name: String,
    }

    impl Layout for Shell {
        fn wrap(&self, content: Node) -> Node {
            body()
                .child(
                    header().child(
                        nav()
                            .child(link_to("/").add_class("logo").text(&self.site_name))
                            .child(link_to("/blog/").text("Blog")),
                    ),
                )
                .child(main_tag().child(content))
                .child(footer().child(p().text("© 2026")))
                .into()
        }
    }

    fn page(shell: &Shell, title_text: &str, content: Node) -> Document {
        Document::with_parts(
            Some("pt-BR"),
            head()
                .child(meta().attr("charset", "UTF-8"))
                .child(title().text(title_text)),
            match shell.wrap(content) {
                Node::Element(element) => element,
                other => body().child(other),
            },
        )
    }

    pub fn run() -> std::io::Result<()> {
        let shell = Shell {
            site_name: "RideKeeper".to_string(),
        };
        let site = StaticSiteGenerator::new("dist");

        site.clean(true)?;

        let pages = vec![
            (
                page(
                    &shell,
                    "RideKeeper",
                    h1().text("Track every service").into(),
                ),
                "index.html".to_string(),
            ),
            (
                page(&shell, "Blog — RideKeeper", h1().text("Blog").into()),
                "blog/index.html".to_string(),
            ),
        ];

        site.generate_multiple(&pages, &RenderOptions::pretty())?;

        site.write_file(
            &SitemapGenerator::generate(&[
                SitemapUrl::new(format!("{SITE}/"))
                    .changefreq(ChangeFreq::Weekly)
                    .priority(1.0),
                SitemapUrl::new(format!("{SITE}/blog/"))
                    .changefreq(ChangeFreq::Daily)
                    .priority(0.8),
            ]),
            "sitemap.xml",
        )?;

        site.write_file(
            &RssGenerator::new("RideKeeper", SITE, "Release notes")
                .language("pt-BR")
                .generate(&[RssItem::new(
                    "1.2 — tyres & chain",
                    format!("{SITE}/blog/1-2"),
                    "Tyre pressure log and chain reminders",
                    "Tue, 11 Aug 2026 10:00:00 +0000",
                )]),
            "feed.xml",
        )?;

        site.write_file(&format!("Sitemap: {SITE}/sitemap.xml\n"), "robots.txt")?;

        println!("Generated {} pages into dist/", pages.len());
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::io::Result<()> {
    native::run()
}

#[cfg(target_arch = "wasm32")]
fn main() {}
