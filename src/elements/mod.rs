//! Constructors for every HTML element Molt supports.
//!
//! All 93 reduce to a handful of shapes, so they are generated from one table here
//! (`define_elements!`) rather than written one by one. `<main>` keeps a suffix — see
//! [`main_tag`] for why.

use crate::core::Element;

/// Generates a zero-argument constructor per tag.
macro_rules! define_elements {
    ($($(#[$meta:meta])* $name:ident => $tag:literal),* $(,)?) => {
        $(
            $(#[$meta])*
            ///
            /// # Examples
            /// ```
            /// use molt::prelude::*;
            #[doc = concat!("let el = ", stringify!($name), "();")]
            #[doc = concat!("assert_eq!(el.tag(), ", stringify!($tag), ");")]
            /// ```
            #[must_use]
            pub fn $name() -> Element {
                Element::from_static($tag)
            }
        )*

        /// Every tag this crate can build, as `(constructor name, tag name)`.
        ///
        /// Used by `scripts/generate-tag-catalog.sh` and by the catalog freshness test.
        pub const ALL_TAGS: &[(&str, &str)] = &[$((stringify!($name), $tag)),*];
    };
}

define_elements! {
    // MARK: - Inline semantics
    /// An abbreviation or acronym: `<abbr>`.
    abbr => "abbr",
    /// A cited creative work: `<cite>`.
    cite => "cite",
    /// Deleted text: `<del>`.
    del => "del",
    /// Stress emphasis: `<em>`.
    em => "em",
    /// Text in an alternate voice: `<i>`.
    i => "i",
    /// Inserted text: `<ins>`.
    ins => "ins",
    /// Keyboard input: `<kbd>`.
    kbd => "kbd",
    /// Marked or highlighted text: `<mark>`.
    mark => "mark",
    /// A short inline quotation: `<q>`.
    q => "q",
    /// Sample program output: `<samp>`.
    samp => "samp",
    /// Side comments and fine print: `<small>`.
    small => "small",
    /// A generic inline container: `<span>`.
    span => "span",
    /// Strong importance: `<strong>`.
    strong => "strong",
    /// Subscript: `<sub>`.
    sub => "sub",
    /// Superscript: `<sup>`.
    sup => "sup",
    /// A machine-readable date or time: `<time>`.
    time => "time",
    /// A variable name: `<var>`.
    var => "var",
    /// A line-break opportunity: `<wbr>`.
    wbr => "wbr",

    // MARK: - Void and leaf elements
    /// A line break: `<br>`.
    br => "br",
    /// A thematic break: `<hr>`.
    hr => "hr",
    /// An image: `<img>`.
    img => "img",
    /// An external resource link: `<link>`.
    link => "link",
    /// The document base URL: `<base>`.
    base => "base",
    /// Document metadata: `<meta>`. See [`crate::seo`] for the typed constructors.
    meta => "meta",
    /// The document title: `<title>`.
    title => "title",
    /// A script: `<script>`. Content is **not** escaped by default — use
    /// [`Element::raw_text`](crate::core::Element::raw_text).
    script => "script",
    /// Embedded CSS: `<style>`. Content is **not** escaped by default.
    style => "style",
    /// External content: `<embed>`.
    embed => "embed",

    // MARK: - Headings
    /// A top-level heading: `<h1>`.
    h1 => "h1",
    /// A second-level heading: `<h2>`.
    h2 => "h2",
    /// A third-level heading: `<h3>`.
    h3 => "h3",
    /// A fourth-level heading: `<h4>`.
    h4 => "h4",
    /// A fifth-level heading: `<h5>`.
    h5 => "h5",
    /// A sixth-level heading: `<h6>`.
    h6 => "h6",

    // MARK: - Sectioning and flow
    /// Contact information: `<address>`.
    address => "address",
    /// A self-contained composition: `<article>`.
    article => "article",
    /// Tangential content: `<aside>`.
    aside => "aside",
    /// An extended quotation: `<blockquote>`.
    blockquote => "blockquote",
    /// The document body: `<body>`.
    body => "body",
    /// A drawing surface: `<canvas>`.
    canvas => "canvas",
    /// A dialog box: `<dialog>`.
    dialog => "dialog",
    /// A generic block container: `<div>`.
    div => "div",
    /// A figure caption: `<figcaption>`.
    figcaption => "figcaption",
    /// Self-contained content with an optional caption: `<figure>`.
    figure => "figure",
    /// A footer: `<footer>`.
    footer => "footer",
    /// Document metadata container: `<head>`.
    head => "head",
    /// Introductory content: `<header>`.
    header => "header",
    /// The dominant content of the body: `<main>`.
    ///
    /// Named `main_tag` rather than `main` because a binary crate's own `fn main` shadows a
    /// glob-imported `main()`, so `main().child(…)` fails to compile in exactly the place
    /// people write it first.
    main_tag => "main",
    /// Navigation links: `<nav>`.
    nav => "nav",
    /// Fallback for disabled scripting: `<noscript>`.
    noscript => "noscript",
    /// A paragraph: `<p>`.
    p => "p",
    /// A responsive image container: `<picture>`.
    picture => "picture",
    /// A thematic grouping: `<section>`.
    section => "section",
    /// The document root: `<html>`.
    html_tag => "html",

    // MARK: - Lists
    /// An unordered list: `<ul>`.
    ul => "ul",
    /// An ordered list: `<ol>`.
    ol => "ol",
    /// A list item: `<li>`.
    li => "li",
    /// A description list: `<dl>`.
    dl => "dl",
    /// A description term: `<dt>`.
    dt => "dt",
    /// A description detail: `<dd>`.
    dd => "dd",

    // MARK: - Interactive
    /// A hyperlink: `<a>`.
    a => "a",
    /// A button: `<button>`.
    button => "button",
    /// A disclosure widget: `<details>`.
    details => "details",
    /// A disclosure summary: `<summary>`.
    summary => "summary",

    // MARK: - Tables
    /// A table: `<table>`.
    table => "table",
    /// A table row: `<tr>`.
    tr => "tr",
    /// A table cell: `<td>`.
    td => "td",
    /// A table header cell: `<th>`.
    th => "th",
    /// A table caption: `<caption>`.
    caption => "caption",
    /// A column group: `<colgroup>`.
    colgroup => "colgroup",
    /// A table column: `<col>`.
    col => "col",
    /// The table body: `<tbody>`.
    tbody => "tbody",
    /// The table footer: `<tfoot>`.
    tfoot => "tfoot",
    /// The table header: `<thead>`.
    thead => "thead",

    // MARK: - Forms
    /// A form: `<form>`.
    form => "form",
    /// A form control: `<input>`.
    input => "input",
    /// A multi-line text control: `<textarea>`.
    textarea => "textarea",
    /// A caption for a form control: `<label>`.
    label => "label",
    /// A group of form controls: `<fieldset>`.
    fieldset => "fieldset",
    /// A caption for a fieldset: `<legend>`.
    legend => "legend",
    /// A drop-down list: `<select>`.
    select => "select",
    /// An option in a select: `<option>`.
    option => "option",
    /// A group of options: `<optgroup>`.
    optgroup => "optgroup",
    /// A list of predefined options: `<datalist>`.
    datalist => "datalist",
    /// The result of a calculation: `<output>`.
    output => "output",
    /// A scalar measurement within a range: `<meter>`.
    meter => "meter",
    /// Task completion progress: `<progress>`.
    progress => "progress",

    // MARK: - Media
    /// Embedded sound: `<audio>`.
    audio => "audio",
    /// Embedded video: `<video>`.
    video => "video",
    /// A media resource: `<source>`.
    source => "source",
    /// A timed text track: `<track>`.
    track => "track",
    /// A nested browsing context: `<iframe>`. Prefer [`iframe_titled`], which cannot be
    /// built without the `title` an assistive technology needs.
    iframe => "iframe",

    // MARK: - Code
    /// Inline code: `<code>`. Whitespace-sensitive.
    code => "code",
    /// Preformatted text: `<pre>`. Whitespace-sensitive.
    pre => "pre",
}

// MARK: - Typed constructors
//
// The shapes that carry required or defaulted arguments and do not fit the table above. Rust has no default arguments, so a defaulted parameter becomes either an
// `Option` or a separate constructor — never a silently different default.

/// A hyperlink with its `href`: `<a href="…">`.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// assert_eq!(link_to("/pricing").text("Pricing").render(), r#"<a href="/pricing">Pricing</a>"#);
/// ```
#[must_use]
pub fn link_to(href: impl AsRef<str>) -> Element {
    a().attr("href", href)
}

/// An image with its `src` and `alt`: `<img src="…" alt="…">`.
///
/// `alt` is required rather than optional. An image without an alternative text is the
/// single most common accessibility defect in generated HTML. Requiring it here is cheaper
/// than linting for it later (though [`crate::accessibility::audit`] does lint for it). Pass `""` deliberately for a decorative image.
#[must_use]
pub fn image(src: impl AsRef<str>, alt: impl AsRef<str>) -> Element {
    img().attr("src", src).attr("alt", alt)
}

/// A `<script src="…">`.
#[must_use]
pub fn script_src(src: impl AsRef<str>) -> Element {
    script().attr("src", src)
}

/// A stylesheet link: `<link href="…" rel="stylesheet">`.
#[must_use]
pub fn stylesheet(href: impl AsRef<str>) -> Element {
    link().attr("href", href).attr("rel", "stylesheet")
}

/// A `<button type="…">`.
#[must_use]
pub fn button_typed(button_type: impl AsRef<str>) -> Element {
    button().attr("type", button_type)
}

/// An `<input type="…" name="…">`.
#[must_use]
pub fn input_named(input_type: impl AsRef<str>, name: impl AsRef<str>) -> Element {
    input().attr("type", input_type).attr("name", name)
}

/// A `<label for="…">`.
///
/// Named `for_id` because `for` is a Rust keyword; the rendered attribute is still `for`.
#[must_use]
pub fn label_for(for_id: impl AsRef<str>) -> Element {
    label().attr("for", for_id)
}

/// An `<iframe>` that cannot be built without a `title`.
///
/// A frame with no title is unusable with a screen reader, so `title` is a required
/// parameter. `loading="lazy"` is applied.
#[must_use]
pub fn iframe_titled(src: impl AsRef<str>, title_text: impl AsRef<str>) -> Element {
    iframe()
        .attr("src", src)
        .attr("title", title_text)
        .attr("loading", "lazy")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Render, RenderOptions};

    #[test]
    fn the_catalog_covers_every_generated_tag() {
        assert!(
            ALL_TAGS.len() >= 90,
            "expected ~93 tags, found {}",
            ALL_TAGS.len()
        );
    }

    /// A catalog pattern that rejects digits would silently drop `h1`–`h6`. Guard against that
    /// gap.
    #[test]
    fn the_catalog_includes_the_headings() {
        for heading in ["h1", "h2", "h3", "h4", "h5", "h6"] {
            assert!(
                ALL_TAGS.iter().any(|(_, tag)| *tag == heading),
                "{heading} missing from ALL_TAGS"
            );
        }
    }

    #[test]
    fn no_tag_is_listed_twice() {
        let mut tags: Vec<&str> = ALL_TAGS.iter().map(|(_, tag)| *tag).collect();
        tags.sort_unstable();
        let before = tags.len();
        tags.dedup();
        assert_eq!(before, tags.len(), "duplicate tag in ALL_TAGS");
    }

    /// `var` needs no suffix in Rust; `<main>` does, because `fn main` shadows it.
    #[test]
    fn the_renamed_tags_still_emit_their_html_names() {
        assert_eq!(main_tag().render(), "<main></main>");
        assert_eq!(var().render(), "<var></var>");
    }

    #[test]
    fn typed_constructors_set_their_attributes() {
        assert_eq!(link_to("/x").render(), r#"<a href="/x"></a>"#);
        assert_eq!(
            image("/a.png", "A cat").render(),
            r#"<img src="/a.png" alt="A cat">"#
        );
        assert_eq!(
            label_for("email").text("E-mail").render(),
            r#"<label for="email">E-mail</label>"#
        );
        assert_eq!(
            stylesheet("/s.css").render(),
            r#"<link href="/s.css" rel="stylesheet">"#
        );
    }

    #[test]
    fn an_iframe_built_through_the_typed_constructor_always_has_a_title() {
        let rendered = iframe_titled("/embed", "A map").render();
        assert!(rendered.contains(r#"title="A map""#));
        assert!(rendered.contains(r#"loading="lazy""#));
    }

    // Media, progress and column shapes go through the generic builder, so each test asserts
    // the rendered markup rather than a typed initialiser.

    #[test]
    fn a_table_renders_every_section_in_order() {
        let table = table()
            .child(caption().text("Sales"))
            .child(colgroup().child(col().attr("span", "2")))
            .child(thead().child(tr().child(th().text("Month")).child(th().text("Total"))))
            .child(tbody().child(tr().child(td().text("Jan")).child(td().text("10"))))
            .child(tfoot().child(tr().child(td().text("Sum")).child(td().text("10"))));

        assert_eq!(
            table.render(),
            concat!(
                "<table><caption>Sales</caption>",
                r#"<colgroup><col span="2"></colgroup>"#,
                "<thead><tr><th>Month</th><th>Total</th></tr></thead>",
                "<tbody><tr><td>Jan</td><td>10</td></tr></tbody>",
                "<tfoot><tr><td>Sum</td><td>10</td></tr></tfoot></table>",
            )
        );
    }

    #[test]
    fn void_elements_render_without_a_closing_tag() {
        assert_eq!(col().render(), "<col>");
        assert_eq!(wbr().render(), "<wbr>");
        assert_eq!(
            base().attr("href", "https://example.com/").render(),
            r#"<base href="https://example.com/">"#
        );
        assert_eq!(
            source()
                .attr("srcset", "a.webp")
                .attr("type", "image/webp")
                .render(),
            r#"<source srcset="a.webp" type="image/webp">"#
        );
        assert_eq!(
            track()
                .attr("src", "cc.vtt")
                .attr("kind", "subtitles")
                .attr("srclang", "en")
                .bool_attr("default")
                .render(),
            r#"<track src="cc.vtt" kind="subtitles" srclang="en" default>"#
        );
    }

    #[test]
    fn void_elements_honour_xhtml_mode() {
        let options = RenderOptions::compact().with_xhtml_self_closing(true);

        assert_eq!(col().render_with(&options), "<col />");
        assert_eq!(wbr().render_with(&options), "<wbr />");
    }

    #[test]
    fn a_picture_wraps_its_sources_and_fallback_image() {
        let picture = picture()
            .child(
                source()
                    .attr("srcset", "/img/hero.avif")
                    .attr("type", "image/avif"),
            )
            .child(image("/img/hero.jpg", "Hero"));

        assert_eq!(
            picture.render(),
            concat!(
                r#"<picture><source srcset="/img/hero.avif" type="image/avif">"#,
                r#"<img src="/img/hero.jpg" alt="Hero"></picture>"#,
            )
        );
    }

    #[test]
    fn video_boolean_attributes_render_bare() {
        let rendered = video()
            .attr("src", "/demo.mp4")
            .attr("poster", "/poster.jpg")
            .bool_attr("controls")
            .bool_attr("muted")
            .render();

        assert_eq!(
            rendered,
            r#"<video src="/demo.mp4" poster="/poster.jpg" controls muted></video>"#
        );
        assert!(!rendered.contains("autoplay"));
    }

    #[test]
    fn audio_without_controls_has_no_boolean_attribute() {
        assert_eq!(
            audio().attr("src", "/song.mp3").render(),
            r#"<audio src="/song.mp3"></audio>"#
        );
    }

    #[test]
    fn an_iframe_carries_its_title_and_lazy_loading() {
        assert_eq!(
            iframe_titled("https://example.com", "Example").render(),
            r#"<iframe src="https://example.com" title="Example" loading="lazy"></iframe>"#
        );
    }

    #[test]
    fn details_renders_open_with_a_summary() {
        let details = details()
            .bool_attr("open")
            .child(summary().text("More"))
            .child(p().text("Hidden text"));

        assert_eq!(
            details.render(),
            "<details open><summary>More</summary><p>Hidden text</p></details>"
        );
    }

    #[test]
    fn details_is_closed_unless_open_is_set() {
        assert_eq!(
            details().child(summary().text("More")).render(),
            "<details><summary>More</summary></details>"
        );
    }

    #[test]
    fn a_definition_list_pairs_terms_with_descriptions() {
        let list = dl()
            .child(dt().text("Molt"))
            .child(dd().text("An HTML library for Rust"));

        assert_eq!(
            list.render(),
            "<dl><dt>Molt</dt><dd>An HTML library for Rust</dd></dl>"
        );
    }

    #[test]
    fn the_text_semantic_tags_render_their_own_names() {
        assert_eq!(
            blockquote().text("Quoted").render(),
            "<blockquote>Quoted</blockquote>"
        );
        assert_eq!(
            q().attr("cite", "https://example.com")
                .text("Short")
                .render(),
            r#"<q cite="https://example.com">Short</q>"#
        );
        assert_eq!(cite().text("Moby Dick").render(), "<cite>Moby Dick</cite>");
        assert_eq!(
            abbr()
                .attr("title", "HyperText Markup Language")
                .text("HTML")
                .render(),
            r#"<abbr title="HyperText Markup Language">HTML</abbr>"#
        );
        assert_eq!(address().text("Rua 1").render(), "<address>Rua 1</address>");
        assert_eq!(sub().text("2").render(), "<sub>2</sub>");
        assert_eq!(sup().text("2").render(), "<sup>2</sup>");
        assert_eq!(del().text("old").render(), "<del>old</del>");
        assert_eq!(ins().text("new").render(), "<ins>new</ins>");
        assert_eq!(kbd().text("\u{2318}S").render(), "<kbd>\u{2318}S</kbd>");
        assert_eq!(samp().text("ok").render(), "<samp>ok</samp>");
        assert_eq!(var().text("x").render(), "<var>x</var>");
        assert_eq!(dialog().text("Hi").render(), "<dialog>Hi</dialog>");
        assert_eq!(
            noscript().text("Enable JS").render(),
            "<noscript>Enable JS</noscript>"
        );
        assert_eq!(
            canvas().attr("width", "300").render(),
            r#"<canvas width="300"></canvas>"#
        );
    }

    /// `text()` escapes everything, `<style>` included, so CSS goes in through `raw_text`,
    /// which is the documented escape hatch and greppable on purpose. Escaping here would turn a child
    /// selector into `a &gt; b` and break the sheet.
    #[test]
    fn a_stylesheet_is_not_escaped() {
        assert_eq!(
            style()
                .attr("media", "screen")
                .raw_text("a > b { color: red; }")
                .render(),
            r#"<style media="screen">a > b { color: red; }</style>"#
        );
    }

    /// There is nothing to omit: an attribute exists when you call `attr` for it.
    #[test]
    fn media_elements_render_only_the_attributes_they_are_given() {
        assert_eq!(
            source()
                .attr("src", "a.mp4")
                .attr("media", "(min-width: 40em)")
                .render(),
            r#"<source src="a.mp4" media="(min-width: 40em)">"#
        );
        assert_eq!(
            track()
                .attr("src", "t.vtt")
                .attr("kind", "captions")
                .attr("label", "PT")
                .render(),
            r#"<track src="t.vtt" kind="captions" label="PT">"#
        );
        assert_eq!(
            video().bool_attr("controls").render(),
            "<video controls></video>"
        );
        assert_eq!(
            audio().bool_attr("autoplay").bool_attr("loop").render(),
            "<audio autoplay loop></audio>"
        );
        assert_eq!(
            iframe()
                .attr("src", "/e")
                .attr("title", "E")
                .bool_attr("allowfullscreen")
                .render(),
            r#"<iframe src="/e" title="E" allowfullscreen></iframe>"#
        );
        assert_eq!(
            base().attr("href", "/").attr("target", "_blank").render(),
            r#"<base href="/" target="_blank">"#
        );
        assert_eq!(col().render(), "<col>");
        assert_eq!(q().text("quoted").render(), "<q>quoted</q>");
    }

    #[test]
    fn meta_renders_whichever_attributes_it_is_given() {
        assert_eq!(
            meta()
                .attr("http-equiv", "refresh")
                .attr("content", "5")
                .render(),
            r#"<meta http-equiv="refresh" content="5">"#
        );
        assert_eq!(
            meta().attr("itemprop", "name").render(),
            r#"<meta itemprop="name">"#
        );
    }

    #[test]
    fn flow_containers_accept_text_as_well_as_children() {
        assert_eq!(aside().text("Note").render(), "<aside>Note</aside>");
        assert_eq!(nav().text("Menu").render(), "<nav>Menu</nav>");
        assert_eq!(header().text("Top").render(), "<header>Top</header>");
        assert_eq!(footer().text("Bottom").render(), "<footer>Bottom</footer>");
        assert_eq!(main_tag().text("Body").render(), "<main>Body</main>");
        assert_eq!(article().text("Post").render(), "<article>Post</article>");
        assert_eq!(figure().text("Fig").render(), "<figure>Fig</figure>");
        assert_eq!(form().text("F").render(), "<form>F</form>");
        assert_eq!(fieldset().text("Set").render(), "<fieldset>Set</fieldset>");
    }

    #[test]
    fn flow_container_content_is_escaped() {
        assert_eq!(
            aside().text("<b>x</b>").render(),
            "<aside>&lt;b&gt;x&lt;/b&gt;</aside>"
        );
    }

    #[test]
    fn a_fieldset_carries_a_legend_and_its_fields() {
        let fieldset = fieldset()
            .child(legend().text("Account"))
            .child(input_named("text", "email"));

        assert_eq!(
            fieldset.render(),
            r#"<fieldset><legend>Account</legend><input type="text" name="email"></fieldset>"#
        );
    }

    #[test]
    fn a_select_groups_its_options() {
        let select = select().attr("name", "city").child(
            optgroup()
                .attr("label", "Brazil")
                .child(option().attr("value", "sp").text("S\u{e3}o Paulo")),
        );

        assert_eq!(
            select.render(),
            concat!(
                r#"<select name="city"><optgroup label="Brazil">"#,
                "<option value=\"sp\">S\u{e3}o Paulo</option></optgroup></select>",
            )
        );
    }

    #[test]
    fn the_form_display_elements_render() {
        assert_eq!(
            datalist().child(option().attr("value", "rust")).render(),
            r#"<datalist><option value="rust"></option></datalist>"#
        );
        assert_eq!(
            progress().attr("value", "0.7").attr("max", "1.0").render(),
            r#"<progress value="0.7" max="1.0"></progress>"#
        );
        assert_eq!(
            progress().attr("max", "1.0").render(),
            r#"<progress max="1.0"></progress>"#
        );
        assert_eq!(
            meter()
                .attr("value", "6.0")
                .attr("min", "0.0")
                .attr("max", "10.0")
                .render(),
            r#"<meter value="6.0" min="0.0" max="10.0"></meter>"#
        );
        assert_eq!(output().text("42").render(), "<output>42</output>");
    }

    #[test]
    fn boolean_attributes_carry_no_value() {
        let rendered = input_named("checkbox", "agree")
            .bool_attr("checked")
            .bool_attr("required")
            .render();

        assert_eq!(
            rendered,
            r#"<input type="checkbox" name="agree" checked required>"#
        );
        assert!(!rendered.contains("checked="));
        assert!(!rendered.contains("required="));
    }

    #[test]
    fn void_elements_do_not_self_close_by_default() {
        let rendered = image("a.png", "A").render();

        assert_eq!(rendered, r#"<img src="a.png" alt="A">"#);
        assert!(!rendered.contains("/>"));
    }

    #[test]
    fn xhtml_mode_self_closes_void_elements() {
        let options = RenderOptions::compact().with_xhtml_self_closing(true);

        assert_eq!(
            image("a.png", "A").render_with(&options),
            r#"<img src="a.png" alt="A" />"#
        );
    }

    #[test]
    fn anchors_and_headings_take_element_children() {
        assert_eq!(
            link_to("/news").child(image("thumb.jpg", "Thumb")).render(),
            r#"<a href="/news"><img src="thumb.jpg" alt="Thumb"></a>"#
        );
        assert_eq!(
            h3().child(link_to("/news").text("Headline")).render(),
            r#"<h3><a href="/news">Headline</a></h3>"#
        );
        assert_eq!(
            i().add_class("fas fa-search").render(),
            r#"<i class="fas fa-search"></i>"#
        );
    }

    #[test]
    fn a_submit_button_carries_its_type() {
        assert_eq!(
            button_typed("submit").text("Send").render(),
            r#"<button type="submit">Send</button>"#
        );
    }

    #[test]
    fn a_label_without_a_target_has_no_for_attribute() {
        let rendered = label().text("Accept cookies").render();

        assert_eq!(rendered, "<label>Accept cookies</label>");
        assert!(!rendered.contains("for="));
    }

    #[test]
    fn an_input_without_a_name_has_no_name_attribute() {
        let rendered = input()
            .attr("type", "search")
            .attr("placeholder", "Search")
            .render();

        assert_eq!(rendered, r#"<input type="search" placeholder="Search">"#);
        assert!(!rendered.contains("name="));
    }

    #[test]
    fn a_section_renders_its_class_and_text() {
        assert_eq!(
            section().add_class("hero").text("Hello").render(),
            r#"<section class="hero">Hello</section>"#
        );
    }

    #[test]
    fn the_inline_semantic_tags_render() {
        assert_eq!(strong().text("bold").render(), "<strong>bold</strong>");
        assert_eq!(em().text("emph").render(), "<em>emph</em>");
        assert_eq!(small().text("fine").render(), "<small>fine</small>");
        assert_eq!(br().render(), "<br>");
        assert_eq!(hr().render(), "<hr>");
    }

    #[test]
    fn a_pre_block_keeps_its_line_breaks() {
        let block =
            pre().text("This is preformatted text.\nIt preserves whitespace and line breaks.");

        assert_eq!(
            block.render(),
            "<pre>This is preformatted text.\nIt preserves whitespace and line breaks.</pre>"
        );
    }

    #[test]
    fn a_code_block_keeps_its_line_breaks() {
        let block = code().text("let x = 10\nprint(x)");

        assert_eq!(block.render(), "<code>let x = 10\nprint(x)</code>");
    }

    #[test]
    fn an_embed_is_a_void_element() {
        assert_eq!(
            embed()
                .attr("src", "video.mp4")
                .attr("type", "video/mp4")
                .render(),
            r#"<embed src="video.mp4" type="video/mp4">"#
        );
    }

    #[test]
    fn an_article_wraps_its_children() {
        assert_eq!(
            article().child(h1().text("Title")).render(),
            "<article><h1>Title</h1></article>"
        );
    }

    #[test]
    fn an_aside_wraps_its_children() {
        assert_eq!(
            aside().child(p().text("Sidebar")).render(),
            "<aside><p>Sidebar</p></aside>"
        );
    }

    #[test]
    fn a_figure_pairs_an_image_with_its_caption() {
        let block = figure()
            .child(image("image.jpg", "Test"))
            .child(figcaption().text("Image caption"));

        assert_eq!(
            block.render(),
            concat!(
                r#"<figure><img src="image.jpg" alt="Test">"#,
                "<figcaption>Image caption</figcaption></figure>",
            )
        );
    }

    #[test]
    fn a_time_element_carries_its_datetime() {
        assert_eq!(
            time()
                .attr("datetime", "2024-01-15")
                .text("January 15, 2024")
                .render(),
            r#"<time datetime="2024-01-15">January 15, 2024</time>"#
        );
    }

    #[test]
    fn a_mark_element_renders_its_text() {
        assert_eq!(
            mark().text("highlighted").render(),
            "<mark>highlighted</mark>"
        );
    }

    #[test]
    fn every_heading_level_renders_its_own_tag() {
        let headings = [
            h1().text("H1"),
            h2().text("H2"),
            h3().text("H3"),
            h4().text("H4"),
            h5().text("H5"),
            h6().text("H6"),
        ];

        for (index, heading) in headings.into_iter().enumerate() {
            let level = index + 1;
            assert_eq!(heading.render(), format!("<h{level}>H{level}</h{level}>"));
        }
    }

    #[test]
    fn a_form_renders_its_fieldsets_and_submit() {
        let form = form()
            .attr("action", "/submit")
            .child(
                fieldset()
                    .child(label_for("name").text("Name"))
                    .child(input_named("text", "name")),
            )
            .child(
                fieldset()
                    .child(label_for("message").text("Message"))
                    .child(textarea().attr("name", "message")),
            )
            .child(input_named("submit", "submit").attr("value", "Send"));

        assert_eq!(
            form.render(),
            concat!(
                r#"<form action="/submit"><fieldset><label for="name">Name</label>"#,
                r#"<input type="text" name="name"></fieldset><fieldset>"#,
                r#"<label for="message">Message</label>"#,
                r#"<textarea name="message"></textarea></fieldset>"#,
                r#"<input type="submit" name="submit" value="Send"></form>"#,
            )
        );
    }

    #[test]
    fn a_select_renders_each_option() {
        let select = select()
            .attr("name", "options")
            .children_from((1..=3).map(|i| {
                option()
                    .attr("value", i.to_string())
                    .text(format!("Option {i}"))
            }));

        assert_eq!(
            select.render(),
            concat!(
                r#"<select name="options"><option value="1">Option 1</option>"#,
                r#"<option value="2">Option 2</option>"#,
                r#"<option value="3">Option 3</option></select>"#,
            )
        );
    }

    #[test]
    fn a_label_points_at_its_field() {
        assert_eq!(
            label_for("username").text("Username").render(),
            r#"<label for="username">Username</label>"#
        );
    }

    #[test]
    fn an_input_renders_its_type_name_and_value() {
        assert_eq!(
            input_named("text", "username")
                .attr("value", "JohnDoe")
                .render(),
            r#"<input type="text" name="username" value="JohnDoe">"#
        );
    }

    #[test]
    fn a_textarea_renders_its_content_between_the_tags() {
        assert_eq!(
            textarea()
                .attr("name", "message")
                .text("Hello, World!")
                .render(),
            r#"<textarea name="message">Hello, World!</textarea>"#
        );
    }

    #[test]
    fn a_fieldset_groups_a_label_and_its_field() {
        let block = fieldset()
            .child(label_for("name").text("Name"))
            .child(input_named("text", "name"));

        assert_eq!(
            block.render(),
            concat!(
                r#"<fieldset><label for="name">Name</label>"#,
                r#"<input type="text" name="name"></fieldset>"#,
            )
        );
    }

    #[test]
    fn a_section_wraps_its_children() {
        assert_eq!(
            section().child(p().text("This is a section.")).render(),
            "<section><p>This is a section.</p></section>"
        );
    }
}
