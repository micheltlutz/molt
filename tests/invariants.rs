//! Property tests: invariants that must hold for *any* input, checked on thousands of hostile
//! ones.
//!
//! These are the same invariants as the libFuzzer targets in `fuzz/`, with two differences
//! that matter: they run in every `cargo test` on every platform (libFuzzer needs nightly and
//! a separate job), and they are **deterministic**. The generator is a small PRNG with a fixed
//! seed per test, so a failure reproduces exactly, and the failure message prints the input.
//!
//! A passing run is evidence, not proof: the inputs are drawn from a fixed hostile alphabet,
//! not from everything. What it buys is that the properties are stated in one place, checked
//! against an independent reference (a tiny unescaper, a browser-like URL normaliser), and
//! cannot silently stop being true.

#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

use molt::core::escape::{
    escape_attribute, escape_json_script, escape_text, escape_text_with_slashes, escape_xml,
};
use molt::core::tags::is_valid_tag_name;
use molt::core::url_policy::is_blocked_in;
use molt::prelude::*;

/// xorshift64*: tiny, fast, good enough to pick pieces, and fully reproducible.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

/// Fragments that have broken escapers, parsers and sanitisers before: markup, quotes,
/// comment terminators, script closers, character references, controls, non-characters, and
/// multi-byte text that a byte-based slice could cut in half.
const HOSTILE: &[&str] = &[
    "a",
    "Z",
    "0",
    " ",
    "x y",
    "\t",
    "\n",
    "\r",
    "\0",
    "\u{1}",
    "\u{8}",
    "\u{b}",
    "\u{1f}",
    "<",
    ">",
    "&",
    "\"",
    "'",
    "/",
    "\\",
    "=",
    "-",
    "--",
    "---",
    "-->",
    "--!>",
    "<!--",
    "</script>",
    "<script>",
    "</noscript>",
    "<img src=x onerror=1>",
    "&amp;",
    "&lt;",
    "&#60;",
    "&#x3c;",
    "&#106;",
    "&#x6A;",
    "&colon;",
    "&Tab;",
    "&NewLine;",
    "&bogus;",
    "&#",
    "&#x",
    "javascript:",
    "JaVaScRiPt:",
    "vbscript:",
    "data:text/html,",
    "data:image/svg+xml,",
    "http://",
    "https://",
    "é",
    "ç",
    "日本",
    "🦀",
    "👩‍👩‍👧",
    "\u{2028}",
    "\u{2029}",
    "\u{feff}",
    "\u{fffe}",
    "\u{ffff}",
    "\u{fffd}",
    "\u{10000}",
    "\u{7f}",
    "\u{80}",
    "\u{85}",
];

fn hostile(rng: &mut Rng) -> String {
    let pieces = rng.below(10);
    (0..pieces).map(|_| *rng.pick(HOSTILE)).collect()
}

/// As [`hostile`], without line breaks: for checks about the *writer's* newlines, which a
/// newline inside the data would be confused with.
fn hostile_one_line(rng: &mut Rng) -> String {
    hostile(rng).replace(['\n', '\r'], " ")
}

/// Runs `check` on many generated strings, naming the input if it fails.
fn for_each_input(seed: u64, rounds: usize, mut check: impl FnMut(&str)) {
    let mut rng = Rng::new(seed);
    for _ in 0..rounds {
        let input = hostile(&mut rng);
        check(&input);
    }
}

// ---- A tiny reference unescaper, written separately from the implementation ----------------

/// Undoes the five-character escapes, `&#x27;` and `&#x2F;`. Returns `None` on any `&` that
/// does not start one of them, which would mean the escaper left a stray ampersand.
fn unescape(escaped: &str, extra: &[(&str, char)]) -> Option<String> {
    const BASE: &[(&str, char)] = &[
        ("&amp;", '&'),
        ("&lt;", '<'),
        ("&gt;", '>'),
        ("&quot;", '"'),
        ("&#x27;", '\''),
        ("&apos;", '\''),
    ];
    let mut out = String::new();
    let mut rest = escaped;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        let (entity, character) = BASE
            .iter()
            .chain(extra)
            .find(|(entity, _)| rest.starts_with(entity))?;
        out.push(*character);
        rest = &rest[entity.len()..];
    }
    out.push_str(rest);
    Some(out)
}

const XML_ILLEGAL: fn(char) -> bool = |c| matches!(c, '\u{0}'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{fffe}' | '\u{ffff}');

// ---- Escapers --------------------------------------------------------------------------

#[test]
fn escape_text_neutralises_markup_and_round_trips() {
    for_each_input(0xA11C_E001, 20_000, |input| {
        let escaped = escape_text(input);
        assert!(
            !escaped.contains(['<', '>', '"', '\'']),
            "markup survived in {escaped:?} (from {input:?})"
        );
        assert_eq!(unescape(&escaped, &[]).as_deref(), Some(input), "{input:?}");
    });
}

#[test]
fn escape_text_with_slashes_also_escapes_slashes() {
    for_each_input(0xA11C_E002, 10_000, |input| {
        let escaped = escape_text_with_slashes(input);
        assert!(
            !escaped.contains(['<', '>', '"', '\'', '/']),
            "{escaped:?} (from {input:?})"
        );
        assert_eq!(
            unescape(&escaped, &[("&#x2F;", '/')]).as_deref(),
            Some(input),
            "{input:?}"
        );
    });
}

#[test]
fn escape_attribute_cannot_close_its_quotes_and_round_trips() {
    for_each_input(0xA11C_E003, 20_000, |input| {
        let escaped = escape_attribute(input);
        assert!(
            !escaped.contains(['"', '\'']),
            "{escaped:?} (from {input:?})"
        );
        assert_eq!(unescape(&escaped, &[]).as_deref(), Some(input), "{input:?}");
    });
}

#[test]
fn escape_xml_leaves_well_formed_text() {
    for_each_input(0xA11C_E004, 20_000, |input| {
        let escaped = escape_xml(input);
        assert!(
            !escaped.contains(['<', '>', '"', '\'']),
            "{escaped:?} (from {input:?})"
        );
        assert!(
            !escaped.chars().any(XML_ILLEGAL),
            "illegal XML character in {escaped:?}"
        );
        let expected: String = input.chars().filter(|c| !XML_ILLEGAL(*c)).collect();
        assert_eq!(unescape(&escaped, &[]), Some(expected), "{input:?}");
    });
}

#[test]
fn escape_json_script_keeps_the_html_parser_out_and_the_data_intact() {
    for_each_input(0xA11C_E005, 20_000, |input| {
        let escaped = escape_json_script(input);
        assert!(
            !escaped.contains(['<', '>', '&', '\u{2028}', '\u{2029}']),
            "{escaped:?} (from {input:?})"
        );
        // A JSON parser reads each escape back as the same character.
        let decoded = escaped
            .replace("\\u003c", "<")
            .replace("\\u003e", ">")
            .replace("\\u0026", "&")
            .replace("\\u2028", "\u{2028}")
            .replace("\\u2029", "\u{2029}");
        // The input may already contain a literal backslash-u sequence; that is untouched
        // data, and decoding it again here is the only way this reference can disagree.
        if !input.contains("\\u") {
            assert_eq!(decoded, input);
        }
    });
}

// ---- Comments, tags, attributes --------------------------------------------------------

#[test]
fn a_comment_never_closes_early() {
    for_each_input(0xA11C_E006, 20_000, |input| {
        let rendered = Node::comment(input).render();
        let inner = rendered
            .strip_prefix("<!-- ")
            .and_then(|r| r.strip_suffix(" -->"))
            .unwrap_or_else(|| panic!("comment lost its delimiters: {rendered:?}"));
        assert!(!inner.contains("--"), "{rendered:?} (from {input:?})");
        assert!(!inner.ends_with('-'), "{rendered:?} (from {input:?})");
        assert_eq!(rendered.matches("-->").count(), 1, "{rendered:?}");
    });
}

#[test]
fn a_tag_name_either_renders_as_itself_or_not_at_all() {
    for_each_input(0xA11C_E007, 20_000, |input| {
        let rendered = Element::new(input).text("body").render();
        if is_valid_tag_name(input) {
            assert_eq!(rendered, format!("<{input}>body</{input}>"));
        } else {
            assert_eq!(rendered, "", "{input:?} must not reach the output");
        }
        // However odd the name, a rendered element has exactly two `<`.
        assert!(rendered.matches('<').count() == 0 || rendered.matches('<').count() == 2);
    });
}

#[test]
fn text_children_can_only_ever_add_text() {
    for_each_input(0xA11C_E008, 20_000, |input| {
        let rendered = div().child(Node::text(input)).render();
        assert_eq!(
            rendered.matches('<').count(),
            2,
            "{rendered:?} (from {input:?})"
        );
        assert_eq!(
            rendered.matches('>').count(),
            2,
            "{rendered:?} (from {input:?})"
        );
        let inner = &rendered["<div>".len()..rendered.len() - "</div>".len()];
        assert_eq!(unescape(inner, &[]).as_deref(), Some(input));
    });
}

/// What a browser makes of an attribute value before it reads the scheme: one pass of
/// character-reference decoding, then tabs and newlines removed anywhere and leading controls
/// and spaces trimmed. Narrower than the implementation's own rule on purpose: anything a
/// real browser would treat as `javascript:` must be caught.
fn browser_url(written: &str) -> String {
    let named: &[(&str, char)] = &[("&colon;", ':'), ("&Tab;", '\t'), ("&NewLine;", '\n')];
    let mut decoded = String::new();
    let mut rest = written;
    while let Some(index) = rest.find('&') {
        decoded.push_str(&rest[..index]);
        rest = &rest[index..];
        if let Some((entity, character)) = named.iter().find(|(entity, _)| rest.starts_with(entity))
        {
            decoded.push(*character);
            rest = &rest[entity.len()..];
        } else if let Some(body) = rest.strip_prefix("&#") {
            let (digits, radix, skip) = match body.strip_prefix(['x', 'X']) {
                Some(hex) => (hex, 16, 3),
                None => (body, 10, 2),
            };
            let length = digits.chars().take_while(|c| c.is_digit(radix)).count();
            match u32::from_str_radix(&digits[..length], radix)
                .ok()
                .and_then(char::from_u32)
            {
                Some(character) if length > 0 => {
                    decoded.push(character);
                    let semicolon = usize::from(digits[length..].starts_with(';'));
                    rest = &rest[skip + length + semicolon..];
                }
                _ => {
                    decoded.push('&');
                    rest = &rest[1..];
                }
            }
        } else if let Some(after) = rest.strip_prefix("&amp;") {
            decoded.push('&');
            rest = after;
        } else {
            decoded.push('&');
            rest = &rest[1..];
        }
    }
    decoded.push_str(rest);
    decoded
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect::<String>()
        .trim_start_matches(|c: char| c <= ' ')
        .to_ascii_lowercase()
}

/// Scheme pieces and the ways of spelling them that have beaten URL filters.
const URL_PIECES: &[&str] = &[
    "java",
    "script",
    "Java",
    "SCRIPT",
    ":",
    "&colon;",
    "&#58;",
    "&#x3a;",
    "&#x3A",
    "&#106",
    "&#x6a;",
    "&#97;",
    "&Tab;",
    "&NewLine;",
    "\t",
    "\n",
    "\r",
    " ",
    "\u{1}",
    "\u{0}",
    "vb",
    "data",
    "text/html",
    ",",
    "alert(1)",
    "//",
    "x",
    "&",
    "&amp;",
    "https://e.com/",
    "/",
    "?q=1",
    "#",
];

#[test]
fn no_url_attribute_writes_a_script_url_however_it_is_spelled() {
    const KEYS: &[&str] = &[
        "href",
        "src",
        "action",
        "formaction",
        "poster",
        "data",
        "HREF",
        "xlink:href",
        "ping",
    ];
    let mut rng = Rng::new(0xA11C_E009);
    for round in 0..30_000 {
        let pieces = 1 + rng.below(8);
        let value: String = (0..pieces).map(|_| *rng.pick(URL_PIECES)).collect();
        let key = *rng.pick(KEYS);
        // Both constructors: `new` escapes `&`, `raw` stores the value as given.
        let attribute = if round % 2 == 0 {
            Attribute::new(key, &value)
        } else {
            Attribute::raw(key, value.clone())
        };
        let written = attribute.to_string();
        let Some(start) = written.find("=\"") else {
            continue; // the attribute was omitted altogether
        };
        let written_value = &written[start + 2..written.len() - 1];
        let seen_by_browser = browser_url(written_value);
        for scheme in ["javascript:", "vbscript:", "data:text/html"] {
            assert!(
                !seen_by_browser.starts_with(scheme),
                "{key}={value:?} was written as {written:?}, which a browser reads as {seen_by_browser:?}"
            );
        }
        // And a value that is blocked is replaced, not merely flagged.
        assert!(
            !is_blocked_in("a", key, written_value),
            "{written:?} is written but the policy still blocks it"
        );
    }
}

#[test]
fn an_attribute_value_never_leaves_its_quotes() {
    let mut rng = Rng::new(0xA11C_E00A);
    for _ in 0..30_000 {
        let key = if rng.below(3) == 0 {
            hostile(&mut rng)
        } else {
            "title".to_string()
        };
        let value = hostile(&mut rng);
        let written = Attribute::new(key.clone(), &value).to_string();
        if written.is_empty() {
            continue; // an unsafe name omits the whole attribute
        }
        assert!(written.starts_with(' '), "{written:?}");
        let start = written
            .find("=\"")
            .unwrap_or_else(|| panic!("no value in {written:?}"));
        let name = &written[1..start];
        assert!(
            !name.is_empty() && !name.contains(|c: char| c.is_control() || " \"'=<>/".contains(c)),
            "unsafe attribute name {name:?} written from {key:?}"
        );
        let body = &written[start + 2..written.len() - 1];
        assert!(!body.contains('"'), "{written:?} (value {value:?})");
        assert!(written.ends_with('"'), "{written:?}");
    }
}

// ---- Whole documents -------------------------------------------------------------------

fn random_tree(rng: &mut Rng, depth: usize) -> Node {
    if depth == 0 || rng.below(4) == 0 {
        return match rng.below(4) {
            0 => Node::text(hostile_one_line(rng)),
            1 => Node::comment(hostile_one_line(rng)),
            2 => Node::raw(*rng.pick(&["<hr>", "<b>x</b>", ""])),
            _ => Node::fragment([]),
        };
    }
    let tags = [
        "div", "p", "noscript", "iframe", "pre", "script", "style", "img", "a", "my-card",
        "bad tag",
    ];
    let mut element = Element::new(*rng.pick(&tags));
    for _ in 0..rng.below(3) {
        element = element.attr(
            *rng.pick(&["href", "alt", "title", "srcdoc", "src"]),
            hostile_one_line(rng),
        );
    }
    for _ in 0..rng.below(4) {
        element = element.child(random_tree(rng, depth - 1));
    }
    Node::from(element)
}

#[test]
fn any_tree_renders_deterministically_in_every_mode_without_panicking() {
    let mut rng = Rng::new(0xA11C_E00B);
    for _ in 0..3_000 {
        let tree = random_tree(&mut rng, 5);
        for options in [
            RenderOptions::compact(),
            RenderOptions::pretty(),
            RenderOptions::pretty()
                .with_indent("\t")
                .with_xhtml_self_closing(true),
            RenderOptions::pretty().with_max_depth(2),
            RenderOptions::compact().with_max_depth(0),
        ] {
            let first = tree.render_with(&options);
            assert_eq!(
                first,
                tree.render_with(&options),
                "non-deterministic render"
            );
            // The tree's own clone and equality agree with the render.
            let copy = tree.clone();
            // `assert!(a == b)`, not `assert_eq!`: a failure would format both trees with `Debug`.
            #[allow(clippy::manual_assert_eq)]
            {
                assert!(copy == tree);
            }
            assert_eq!(first, copy.render_with(&options));
        }
        // No tag the writer dropped may leave a blank line behind in pretty mode.
        let pretty = tree.render_pretty();
        assert!(!pretty.contains("\n\n"), "blank line in {pretty:?}");
    }
}

// ---- Sitemap and feed ------------------------------------------------------------------

/// Checks the structural promise of the XML generators: every `<` opens one of *their* tags,
/// no text contains a stray `>`, `&` or illegal character.
fn assert_well_formed_enough(xml: &str, from: &str) {
    assert!(
        !xml.chars().any(|c| XML_ILLEGAL(c) && !matches!(c, '\n')),
        "illegal character ({from})"
    );
    for segment in xml.split('<').skip(1) {
        let (tag, text) = segment
            .split_once('>')
            .unwrap_or_else(|| panic!("unclosed tag in {segment:?} ({from})"));
        assert!(!text.contains('>'), "stray `>` in text {text:?} ({from})");
        for part in text.split('&').skip(1) {
            assert!(
                ["amp;", "lt;", "gt;", "quot;", "apos;"]
                    .iter()
                    .any(|e| part.starts_with(e)),
                "stray `&` in {text:?} ({from})"
            );
        }
        let attributes = tag.split_once(' ').map_or("", |(_, rest)| rest);
        assert_eq!(
            attributes.matches('"').count() % 2,
            0,
            "unbalanced quotes in <{tag}> ({from})"
        );
    }
}

#[test]
fn sitemap_and_feed_are_well_formed_whatever_the_fields_hold() {
    use molt::feed::{RssGenerator, RssItem};
    use molt::sitemap::{SitemapGenerator, SitemapUrl};

    let mut rng = Rng::new(0xA11C_E00C);
    for _ in 0..3_000 {
        let from = hostile(&mut rng);
        let url = |rng: &mut Rng| {
            if rng.below(2) == 0 {
                format!("https://e.com/{}", hostile(rng))
            } else {
                hostile(rng)
            }
        };
        let urls: Vec<SitemapUrl> = (0..rng.below(4))
            .map(|_| {
                SitemapUrl::new(url(&mut rng))
                    .lastmod(hostile(&mut rng))
                    .priority(f64::from(rng.below(300) as u32) / 100.0 - 1.0)
            })
            .collect();
        assert_well_formed_enough(&SitemapGenerator::generate(&urls), &from);
        assert_well_formed_enough(
            &SitemapGenerator::generate_index(&[(url(&mut rng), Some(hostile(&mut rng)))]),
            &from,
        );

        let items: Vec<RssItem> = (0..rng.below(4))
            .map(|_| {
                RssItem::new(
                    hostile(&mut rng),
                    url(&mut rng),
                    hostile(&mut rng),
                    hostile(&mut rng),
                )
                .guid(url(&mut rng))
                .author(hostile(&mut rng))
                .categories([hostile(&mut rng)])
            })
            .collect();
        let feed = RssGenerator::new(hostile(&mut rng), url(&mut rng), hostile(&mut rng))
            .language(hostile(&mut rng))
            .generate(&items);
        assert_well_formed_enough(&feed, &from);
        // A link that is not http(s) is never written as one.
        for link in feed.split("<link>").skip(1) {
            let link = link.split("</link>").next().unwrap_or("");
            assert!(
                link.starts_with("http://")
                    || link.starts_with("https://")
                    || link.starts_with("HTTP"),
                "{link:?}"
            );
        }
    }
}

// ---- The filesystem ---------------------------------------------------------------------

#[cfg(all(feature = "ssg", not(target_arch = "wasm32")))]
#[test]
fn no_generated_path_lands_outside_the_output_directory() {
    const PIECES: &[&str] = &[
        "a",
        "b",
        "index.html",
        ".",
        "..",
        "/",
        "//",
        "\\",
        "C:",
        ":",
        " ",
        "x.y",
        "%2e%2e",
        "\0",
        "é",
        "~",
        "-",
    ];

    use molt::ssg::StaticSiteGenerator;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn everything_under(root: &Path, found: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                found.push(entry.path());
                if entry.path().is_dir() {
                    everything_under(&entry.path(), found);
                }
            }
        }
    }

    // A private parent, so anything that escapes the output directory is visible beside it.
    let parent = std::env::temp_dir().join(format!("molt-invariants-{}", std::process::id()));
    let _ = fs::remove_dir_all(&parent);
    fs::create_dir_all(&parent).expect("temp dir");
    let output = parent.join("out");
    let site = StaticSiteGenerator::new(&output);

    let mut rng = Rng::new(0xA11C_E00D);
    for _ in 0..4_000 {
        let path: String = (0..=rng.below(6)).map(|_| *rng.pick(PIECES)).collect();
        let result = site.write_file("x", &path);
        // The contract, not just the safety property: a path with a `..`, a root or a drive
        // prefix is refused outright, rather than quietly rewritten into something safe.
        let components = Path::new(&path).components();
        if components.into_iter().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        }) {
            assert!(result.is_err(), "{path:?} should have been refused");
        }
    }

    let siblings: Vec<_> = fs::read_dir(&parent)
        .expect("readable")
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert!(
        siblings.iter().all(|name| name == "out"),
        "something was written beside the output directory: {siblings:?}"
    );
    let canonical_output = fs::canonicalize(&output).unwrap_or(output.clone());
    let mut found = Vec::new();
    everything_under(&output, &mut found);
    for path in found {
        let real = fs::canonicalize(&path).expect("exists");
        assert!(
            real.starts_with(&canonical_output),
            "{path:?} resolves outside the output directory"
        );
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // The writer's temporaries are `.<name>.molt-<pid>-<n>.tmp`; a user file whose name
        // merely starts with a dot is not one.
        assert!(
            !(name.contains(".molt-")
                && Path::new(&name)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("tmp"))),
            "temporary file left behind: {path:?}"
        );
    }
    let _ = fs::remove_dir_all(&parent);
}
