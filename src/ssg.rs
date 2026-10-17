//! Static site generation.
//!
//! Ports `Winged-Swift/Sources/WingedSwift/static/StaticSiteGenerator.swift`.
//!
//! Compiled out on `wasm32` regardless of the `ssg` feature — there is no filesystem
//! there.

use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

use crate::core::{Node, Render, RenderOptions};
use crate::document::Document;

/// Writes rendered pages and assets into an output directory.
///
/// # Examples
/// ```no_run
/// use molt::prelude::*;
/// use molt::{Document, ssg::StaticSiteGenerator};
///
/// let site = StaticSiteGenerator::new("dist");
/// site.clean(true)?;
/// site.generate(&Document::new(Some("pt-BR")), "index.html", &RenderOptions::pretty())?;
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct StaticSiteGenerator {
    output_directory: PathBuf,
}

impl StaticSiteGenerator {
    /// Creates a generator writing into `output_directory`.
    pub fn new(output_directory: impl Into<PathBuf>) -> Self {
        Self {
            output_directory: output_directory.into(),
        }
    }

    /// The directory pages are written into.
    #[must_use]
    pub fn output_directory(&self) -> &Path {
        &self.output_directory
    }

    /// Renders a document to `path`, relative to the output directory.
    ///
    /// The page is written with [`Document::render_to`], section by section, into a
    /// temporary file that is then renamed. The bytes match [`Document::render_with`].
    ///
    /// # Errors
    /// Returns an error if the path escapes the output directory, or if the write fails.
    pub fn generate(
        &self,
        document: &Document,
        path: &str,
        options: &RenderOptions,
    ) -> io::Result<()> {
        self.write_atomic(path, |file| {
            let mut buffered = io::BufWriter::new(file);
            document.render_to(&mut buffered, options)?;
            buffered.flush()
        })
    }

    /// Renders several documents.
    ///
    /// With the `parallel` feature the pages are rendered concurrently — sound because the
    /// node tree is `Send + Sync`, which Winged-Swift's reference-typed tree is not.
    ///
    /// **Every** failure is reported, not just the first: a bulk build that names one of
    /// twelve broken pages is worse than useless.
    ///
    /// # Errors
    /// Returns an error naming every page that failed.
    pub fn generate_multiple(
        &self,
        documents: &[(Document, String)],
        options: &RenderOptions,
    ) -> io::Result<()> {
        #[cfg(feature = "parallel")]
        let results: Vec<(String, io::Result<()>)> = {
            use rayon::prelude::*;
            documents
                .par_iter()
                .map(|(doc, path)| (path.clone(), self.generate(doc, path, options)))
                .collect()
        };

        #[cfg(not(feature = "parallel"))]
        let results: Vec<(String, io::Result<()>)> = documents
            .iter()
            .map(|(doc, path)| (path.clone(), self.generate(doc, path, options)))
            .collect();

        collect_failures(results)
    }

    /// Renders a bare node to `path`, optionally prefixed by the doctype.
    ///
    /// # Errors
    /// Returns an error if the path is unsafe or the write fails.
    pub fn generate_page(
        &self,
        page: &Node,
        path: &str,
        options: &RenderOptions,
        doctype: bool,
    ) -> io::Result<()> {
        let mut content = String::with_capacity(1024);
        if doctype {
            content.push_str("<!DOCTYPE html>\n");
        }
        page.write_into(&mut content, options, 0);
        self.write_file(&content, path)
    }

    /// Copies a file into the output directory, replacing any existing destination.
    ///
    /// # Errors
    /// Returns an error if the destination is unsafe or the copy fails.
    pub fn copy_asset(&self, from: impl AsRef<Path>, to: &str) -> io::Result<()> {
        let destination = self.resolve(to)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        if destination.exists() {
            fs::remove_file(&destination)?;
        }
        fs::copy(from, destination)?;
        Ok(())
    }

    /// Removes the output directory, optionally recreating it empty.
    ///
    /// # Errors
    /// Returns an error if the output directory is unsafe to delete — empty, the
    /// filesystem root, or containing a `..` component. Winged-Swift's version has no such
    /// guard and will happily recurse through whatever it is pointed at.
    pub fn clean(&self, create_directory: bool) -> io::Result<()> {
        guard_destructive_path(&self.output_directory)?;

        if self.output_directory.exists() {
            fs::remove_dir_all(&self.output_directory)?;
        }
        if create_directory {
            fs::create_dir_all(&self.output_directory)?;
        }
        Ok(())
    }

    /// Writes UTF-8 `content` to `path`, creating parent directories.
    ///
    /// The write is atomic: content goes to a temporary file in the same directory and is
    /// then renamed, so a reader never sees a half-written page.
    ///
    /// # Errors
    /// Returns an error if the path escapes the output directory or the write fails.
    pub fn write_file(&self, content: &str, path: &str) -> io::Result<()> {
        self.write_atomic(path, |file| file.write_all(content.as_bytes()))
    }

    /// Writes through a temporary file in the destination directory, then renames.
    ///
    /// The closure receives the open temporary file. On any error the temporary file is
    /// removed, so a reader never observes a half-written page under the final name.
    fn write_atomic(
        &self,
        path: &str,
        write_contents: impl FnOnce(&mut fs::File) -> io::Result<()>,
    ) -> io::Result<()> {
        let destination = self.resolve(path)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let temporary = destination.with_extension("winged-tmp");
        let mut file = fs::File::create(&temporary)?;
        if let Err(error) = write_contents(&mut file) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        if let Err(error) = fs::rename(&temporary, &destination) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        Ok(())
    }

    /// Joins `path` onto the output directory, rejecting anything that escapes it.
    fn resolve(&self, path: &str) -> io::Result<PathBuf> {
        let candidate = Path::new(path);
        if candidate.is_absolute() || candidate.components().any(|c| c == Component::ParentDir) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("path {path:?} escapes the output directory"),
            ));
        }
        Ok(self.output_directory.join(candidate))
    }
}

/// Rejects output directories that are dangerous to delete recursively.
fn guard_destructive_path(path: &Path) -> io::Result<()> {
    let reject = |reason: &str| {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("refusing to clean {}: {reason}", path.display()),
        ))
    };

    if path.as_os_str().is_empty() {
        return reject("the output directory is empty");
    }
    if path.parent().is_none() {
        return reject("the output directory is a filesystem root");
    }
    if path.components().any(|c| c == Component::ParentDir) {
        return reject("the output directory contains a `..` component");
    }
    Ok(())
}

/// Turns per-page results into one error naming every failure.
fn collect_failures(results: Vec<(String, io::Result<()>)>) -> io::Result<()> {
    let failures: Vec<String> = results
        .into_iter()
        .filter_map(|(path, result)| result.err().map(|e| format!("{path}: {e}")))
        .collect();

    if failures.is_empty() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{} page(s) failed to generate:\n  {}",
            failures.len(),
            failures.join("\n  ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::{body, h1, head, html_tag, title};

    /// A throwaway directory that removes itself. Avoids a dev-dependency for six tests.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("molt-{name}"));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("temp dir is creatable");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn page(text: &str) -> Document {
        Document::new(Some("en"))
            .head_children([title().text(text)])
            .body_children([h1().text(text)])
    }

    /// Ports `StaticSiteGeneratorTests.generateWritesADocument`.
    #[test]
    fn generate_writes_a_rendered_document() {
        let dir = TempDir::new("generate");
        let site = StaticSiteGenerator::new(&dir.0);
        site.generate(&page("Home"), "index.html", &RenderOptions::pretty())
            .expect("written");

        let written = fs::read_to_string(dir.0.join("index.html")).expect("readable");
        assert!(written.starts_with("<!DOCTYPE html>"));
        assert!(written.contains("<h1>Home</h1>"));
    }

    /// Ports `ThreeZeroTests.generatorStreamsDocuments` and
    /// `ThreeZeroTests.replacesAnExistingFileAndReportsFailures`.
    #[test]
    fn generate_replaces_a_file_with_the_rendered_bytes() {
        let dir = TempDir::new("stream");
        let site = StaticSiteGenerator::new(&dir.0);
        let destination = dir.0.join("index.html");
        fs::write(&destination, "old").expect("seed");

        let page = page("Home");
        site.generate(&page, "index.html", &RenderOptions::pretty())
            .expect("written");

        let written = fs::read(destination).expect("readable");
        assert_eq!(
            written,
            page.render_with(&RenderOptions::pretty()).into_bytes()
        );
        assert!(!dir.0.join("index.winged-tmp").exists());
    }

    /// Ports `StaticSiteGeneratorTests.generateCreatesNestedDirectories`.
    #[test]
    fn nested_paths_create_their_parent_directories() {
        let dir = TempDir::new("nested");
        let site = StaticSiteGenerator::new(&dir.0);
        site.generate(
            &page("Post"),
            "blog/2026/post.html",
            &RenderOptions::compact(),
        )
        .expect("written");
        assert!(dir.0.join("blog/2026/post.html").exists());
    }

    /// Ports `StaticSiteGeneratorTests.generateCanSkipDoctype`.
    #[test]
    fn a_bare_page_can_be_written_without_a_doctype() {
        let dir = TempDir::new("doctype");
        let site = StaticSiteGenerator::new(&dir.0);
        let node = Node::from(h1().text("Fragment"));

        site.generate_page(&node, "with.html", &RenderOptions::compact(), true)
            .expect("written");
        site.generate_page(&node, "without.html", &RenderOptions::compact(), false)
            .expect("written");

        assert!(
            fs::read_to_string(dir.0.join("with.html"))
                .unwrap()
                .starts_with("<!DOCTYPE")
        );
        assert!(
            !fs::read_to_string(dir.0.join("without.html"))
                .unwrap()
                .contains("DOCTYPE")
        );
    }

    /// Ports `StaticSiteGeneratorTests.writeFileAndCopyAsset`.
    #[test]
    fn copy_asset_replaces_an_existing_destination() {
        let dir = TempDir::new("assets");
        let source = dir.0.join("source.css");
        fs::write(&source, "body{}").expect("written");

        let site = StaticSiteGenerator::new(dir.0.join("out"));
        site.copy_asset(&source, "css/style.css").expect("copied");
        fs::write(&source, "body{color:red}").expect("written");
        site.copy_asset(&source, "css/style.css").expect("recopied");

        let copied = fs::read_to_string(dir.0.join("out/css/style.css")).expect("readable");
        assert_eq!(copied, "body{color:red}");
    }

    /// Ports `StaticSiteGeneratorTests.cleanRemovesPreviousOutput`.
    #[test]
    fn clean_empties_the_output_directory() {
        let dir = TempDir::new("clean");
        let site = StaticSiteGenerator::new(dir.0.join("out"));
        site.write_file("x", "a.html").expect("written");

        site.clean(true).expect("cleaned");
        assert!(dir.0.join("out").exists());
        assert!(!dir.0.join("out/a.html").exists());
    }

    /// The guard Winged-Swift does not have.
    #[test]
    fn clean_refuses_an_unsafe_output_directory() {
        for unsafe_path in ["", "/"] {
            let site = StaticSiteGenerator::new(unsafe_path);
            assert!(
                site.clean(false).is_err(),
                "{unsafe_path:?} should be rejected"
            );
        }
        assert!(StaticSiteGenerator::new("dist/../..").clean(false).is_err());
    }

    #[test]
    fn a_page_path_cannot_escape_the_output_directory() {
        let dir = TempDir::new("escape");
        let site = StaticSiteGenerator::new(&dir.0);
        assert!(site.write_file("x", "../escaped.html").is_err());
        assert!(site.write_file("x", "/etc/escaped.html").is_err());
    }

    /// Ports `StaticSiteGeneratorTests.generateMultipleWritesEveryPage`.
    #[test]
    fn generate_multiple_writes_every_page() {
        let dir = TempDir::new("multiple");
        let site = StaticSiteGenerator::new(&dir.0);
        let documents = vec![
            (page("A"), "a.html".to_string()),
            (page("B"), "nested/b.html".to_string()),
        ];

        site.generate_multiple(&documents, &RenderOptions::pretty())
            .expect("written");
        assert!(dir.0.join("a.html").exists());
        assert!(dir.0.join("nested/b.html").exists());
    }

    #[test]
    fn generate_multiple_reports_every_failure_not_just_the_first() {
        let dir = TempDir::new("failures");
        let site = StaticSiteGenerator::new(&dir.0);
        let documents = vec![
            (page("ok"), "ok.html".to_string()),
            (page("bad"), "../one.html".to_string()),
            (page("bad"), "../two.html".to_string()),
        ];

        let error = site
            .generate_multiple(&documents, &RenderOptions::compact())
            .expect_err("two pages are unwritable");
        let message = error.to_string();
        assert!(message.contains("../one.html"), "{message}");
        assert!(message.contains("../two.html"), "{message}");
    }

    /// Ports `StaticSiteGeneratorTests.generateWritesDoctypeAndMarkup`.
    #[test]
    fn a_page_is_written_with_its_doctype() {
        let directory = TempDir::new("doctype-and-markup");
        let site = StaticSiteGenerator::new(&directory.0);

        let page = Node::from(
            html_tag()
                .child(head().child(title().text("Home")))
                .child(body().child(h1().text("Hello"))),
        );
        site.generate_page(&page, "index.html", &RenderOptions::pretty(), true)
            .expect("write");

        let written = fs::read_to_string(directory.0.join("index.html")).expect("read");
        assert!(written.starts_with("<!DOCTYPE html>\n"));
        assert!(written.contains("<title>Home</title>"));
        assert!(written.contains("<h1>Hello</h1>"));
    }

    /// Ports `StaticSiteGeneratorTests.generateMultipleDocuments`.
    #[test]
    fn generate_multiple_writes_each_document_to_its_own_path() {
        let directory = TempDir::new("multiple-documents");
        let site = StaticSiteGenerator::new(&directory.0);

        let pages = vec![
            (page("Home"), "index.html".to_string()),
            (page("About"), "about/index.html".to_string()),
        ];
        site.generate_multiple(&pages, &RenderOptions::compact())
            .expect("write");

        assert!(
            fs::read_to_string(directory.0.join("index.html"))
                .expect("read")
                .contains("<h1>Home</h1>")
        );
        assert!(
            fs::read_to_string(directory.0.join("about/index.html"))
                .expect("read")
                .contains("<h1>About</h1>")
        );
    }
}
