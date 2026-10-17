//! Static site generation.
//!
//!
//! Compiled out on `wasm32` regardless of the `ssg` feature — there is no filesystem
//! there.

use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::{Node, Render, RenderOptions};
use crate::document::Document;

/// Why a static site operation failed.
///
/// A real error type rather than `io::Error::other(String)`, so a caller can tell an
/// unsafe path from a full disk and can read which page of a bulk build failed. It
/// converts into [`io::Error`] (`?` works in a function returning `io::Result`), keeping
/// `InvalidInput` for the two refusals.
///
/// # Examples
/// ```
/// use molt::ssg::{SsgError, StaticSiteGenerator};
///
/// let site = StaticSiteGenerator::new("dist");
/// let error = site.write_file("x", "../escape.html").expect_err("refused");
/// assert!(matches!(error, SsgError::InvalidPath { .. }));
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum SsgError {
    /// A page or asset path that cannot be written under the output directory: a `..`, a
    /// root or drive prefix, no file name, or a symbolic link on the way.
    InvalidPath {
        /// The path as given.
        path: String,
        /// What is wrong with it.
        reason: &'static str,
    },
    /// `clean` refused to delete the output directory.
    UnsafeCleanTarget {
        /// The directory as given.
        path: PathBuf,
        /// Why it is too dangerous to remove.
        reason: &'static str,
    },
    /// The filesystem failed.
    Io {
        /// The file or directory the operation was working on.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// Several pages of a bulk build failed. Every failure is here, not just the first.
    Pages(Vec<(String, SsgError)>),
}

impl std::fmt::Display for SsgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath { path, reason } => write!(f, "path {path:?} {reason}"),
            Self::UnsafeCleanTarget { path, reason } => {
                write!(f, "refusing to clean {}: {reason}", path.display())
            }
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Pages(failures) => {
                write!(f, "{} page(s) failed to generate:", failures.len())?;
                for (path, error) in failures {
                    write!(f, "\n  {path}: {error}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for SsgError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<SsgError> for io::Error {
    fn from(error: SsgError) -> Self {
        match error {
            SsgError::InvalidPath { .. } | SsgError::UnsafeCleanTarget { .. } => {
                Self::new(io::ErrorKind::InvalidInput, error)
            }
            SsgError::Io { source, .. } => source,
            SsgError::Pages(_) => Self::other(error),
        }
    }
}

/// Result alias for this module's operations.
type Result<T> = std::result::Result<T, SsgError>;

/// Attaches the path an I/O operation was working on.
fn io_error(path: &Path) -> impl FnOnce(io::Error) -> SsgError + '_ {
    move |source| SsgError::Io {
        path: path.to_path_buf(),
        source,
    }
}

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
/// # Ok::<(), molt::ssg::SsgError>(())
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
    pub fn generate(&self, document: &Document, path: &str, options: &RenderOptions) -> Result<()> {
        self.write_atomic(path, |file| {
            let mut buffered = io::BufWriter::new(file);
            document.render_to(&mut buffered, options)?;
            buffered.flush()
        })
    }

    /// Renders several documents.
    ///
    /// With the `parallel` feature the pages are rendered concurrently — sound because the
    /// node tree is `Send + Sync`.
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
    ) -> Result<()> {
        #[cfg(feature = "parallel")]
        let results: Vec<(String, Result<()>)> = {
            use rayon::prelude::*;
            documents
                .par_iter()
                .map(|(doc, path)| (path.clone(), self.generate(doc, path, options)))
                .collect()
        };

        #[cfg(not(feature = "parallel"))]
        let results: Vec<(String, Result<()>)> = documents
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
    ) -> Result<()> {
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
    pub fn copy_asset(&self, from: impl AsRef<Path>, to: &str) -> Result<()> {
        let destination = self.resolve(to)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(io_error(parent))?;
        }
        // Copy beside the destination and rename over it, so a failed copy leaves the old
        // file in place instead of the delete-then-copy gap this used to have.
        let temporary = temporary_path(&destination);
        if let Err(error) = fs::copy(from.as_ref(), &temporary) {
            let _ = fs::remove_file(&temporary);
            return Err(io_error(from.as_ref())(error));
        }
        if let Err(error) = fs::rename(&temporary, &destination) {
            let _ = fs::remove_file(&temporary);
            return Err(io_error(&destination)(error));
        }
        Ok(())
    }

    /// Removes the output directory, optionally recreating it empty.
    ///
    /// # Errors
    /// Returns an error if the output directory is unsafe to delete — empty, a filesystem
    /// root, a top-level directory, the home directory, the working directory or one of its
    /// ancestors, or containing a `..` component, because removing it recursively would delete
    /// whatever it points at.
    pub fn clean(&self, create_directory: bool) -> Result<()> {
        guard_destructive_path(&self.output_directory)?;

        let directory = &self.output_directory;
        if directory.exists() {
            fs::remove_dir_all(directory).map_err(io_error(directory))?;
        }
        if create_directory {
            fs::create_dir_all(directory).map_err(io_error(directory))?;
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
    pub fn write_file(&self, content: &str, path: &str) -> Result<()> {
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
    ) -> Result<()> {
        let destination = self.resolve(path)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(io_error(parent))?;
        }

        let temporary = temporary_path(&destination);
        // `create_new` refuses an existing path, symlinks included, so the temporary file
        // can never be a pre-planted link that redirects the write.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(io_error(&temporary))?;
        if let Err(error) = write_contents(&mut file) {
            let _ = fs::remove_file(&temporary);
            return Err(io_error(&destination)(error));
        }
        if let Err(error) = fs::rename(&temporary, &destination) {
            let _ = fs::remove_file(&temporary);
            return Err(io_error(&destination)(error));
        }
        Ok(())
    }

    /// Joins `path` onto the output directory, rejecting anything that escapes it.
    ///
    /// Only plain names are accepted: `..`, a root and a Windows drive prefix (`C:foo`
    /// replaces the base on `join`) are refused, and so is a path with no file name
    /// (`""`, `"."`, `"a/"`), which would name the output directory itself. A component
    /// that already exists as a symlink is refused too, because writing through it lands
    /// wherever the link points.
    ///
    /// The symlink check is not atomic with the write: someone who can create links inside
    /// the output directory *while it is being generated* can still race it.
    fn resolve(&self, path: &str) -> Result<PathBuf> {
        let reject = |reason: &'static str| {
            Err(SsgError::InvalidPath {
                path: path.to_owned(),
                reason,
            })
        };

        let candidate = Path::new(path);
        let mut names = 0;
        for component in candidate.components() {
            match component {
                Component::Normal(_) => names += 1,
                Component::CurDir => {}
                _ => return reject("escapes the output directory"),
            }
        }
        if names == 0 || path.ends_with(['/', '\\']) {
            return reject("does not name a file");
        }

        let mut resolved = self.output_directory.clone();
        for component in candidate.components() {
            if let Component::Normal(name) = component {
                resolved.push(name);
                // Absent components cannot be links; only existing ones are inspected.
                if fs::symlink_metadata(&resolved).is_ok_and(|m| m.file_type().is_symlink()) {
                    return reject("passes through a symbolic link");
                }
            }
        }
        Ok(resolved)
    }
}

/// A sibling of `destination` that no other writer will pick.
///
/// `with_extension` was the wrong tool: it *replaces* the extension, so `index.html` and
/// `index.xml` both became `index.tmp` and two pages written in parallel shared one
/// file. The full file name, the process id and a counter make the name unique.
fn temporary_path(destination: &Path) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let name = destination
        .file_name()
        .map_or_else(|| "page".into(), |n| n.to_string_lossy().into_owned());
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    destination.with_file_name(format!(".{name}.molt-{}-{unique}.tmp", std::process::id()))
}

/// Rejects output directories that are dangerous to delete recursively.
///
/// Beyond the empty path, a filesystem root and `..`, this refuses the directory the
/// process is running in or any ancestor of it (`"."` has a non-empty parent, which is how
/// the old check let `clean(".")` through), the home directory, and anything fewer than two
/// levels below the root (`/usr`, `/home`). The guard only reads the filesystem; it never
/// removes anything, so it can be tested directly.
fn guard_destructive_path(path: &Path) -> Result<()> {
    let reject = |reason: &'static str| {
        Err(SsgError::UnsafeCleanTarget {
            path: path.to_path_buf(),
            reason,
        })
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

    let absolute = absolute_path(path).map_err(io_error(path))?;
    let depth = absolute
        .components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count();
    if depth < 2 {
        return reject("the output directory is a top-level directory");
    }
    if let Ok(cwd) = std::env::current_dir().and_then(fs::canonicalize) {
        if cwd.starts_with(&absolute) {
            return reject("the output directory is, or contains, the working directory");
        }
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    if let Some(home) = home.and_then(|h| fs::canonicalize(h).ok()) {
        if home == absolute {
            return reject("the output directory is the home directory");
        }
    }
    Ok(())
}

/// `path` made absolute and, when it exists, with symlinks resolved.
fn absolute_path(path: &Path) -> io::Result<PathBuf> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Ok(canonical);
    }
    let mut absolute = fs::canonicalize(std::env::current_dir()?)?;
    for component in path.components() {
        match component {
            Component::Normal(name) => absolute.push(name),
            Component::RootDir | Component::Prefix(_) => {
                absolute = PathBuf::from(component.as_os_str());
            }
            _ => {}
        }
    }
    Ok(absolute)
}

/// Turns per-page results into one error carrying every failure.
fn collect_failures(results: Vec<(String, Result<()>)>) -> Result<()> {
    let failures: Vec<(String, SsgError)> = results
        .into_iter()
        .filter_map(|(path, result)| result.err().map(|error| (path, error)))
        .collect();

    if failures.is_empty() {
        Ok(())
    } else {
        Err(SsgError::Pages(failures))
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
            // The process id keeps two test runs (or two checkouts) off the same directory.
            let path = std::env::temp_dir().join(format!("molt-{name}-{}", std::process::id()));
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

    /// Whether `dir` holds exactly these entries, i.e. no temporary file was left behind.
    fn only_files_named(dir: &Path, names: &[&str]) -> bool {
        let mut found: Vec<String> = fs::read_dir(dir)
            .expect("readable")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        found == names
    }

    fn page(text: &str) -> Document {
        Document::new(Some("en"))
            .head_children([title().text(text)])
            .body_children([h1().text(text)])
    }

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

    /// A generated file is the streamed render, replaces an existing one, and reports failures.
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
        assert!(only_files_named(&dir.0, &["index.html"]));
    }

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

    #[test]
    fn clean_empties_the_output_directory() {
        let dir = TempDir::new("clean");
        let site = StaticSiteGenerator::new(dir.0.join("out"));
        site.write_file("x", "a.html").expect("written");

        site.clean(true).expect("cleaned");
        assert!(dir.0.join("out").exists());
        assert!(!dir.0.join("out/a.html").exists());
    }

    /// `clean` refuses a directory that would be dangerous to remove.
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

    #[test]
    fn a_path_that_does_not_name_a_file_is_rejected() {
        let dir = TempDir::new("nameless");
        let site = StaticSiteGenerator::new(&dir.0);
        for path in ["", ".", "./", "a/", "a\\", "a/../../x.html", "C:\\x.html"] {
            // `C:\x.html` is an ordinary file name on Unix, so it is only rejected on Windows.
            if path.starts_with("C:") && !cfg!(windows) {
                continue;
            }
            let outcome = site.write_file("x", path);
            assert!(outcome.is_err(), "{path:?} should be rejected");
        }
        // Nothing was created beside the output directory by `with_extension`.
        let parent = dir.0.parent().expect("temp dir has a parent");
        let name = dir
            .0
            .file_name()
            .expect("named")
            .to_string_lossy()
            .into_owned();
        assert!(!parent.join(format!("{name}.tmp")).exists());
        assert!(only_files_named(&dir.0, &[]));
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_inside_the_output_directory_is_not_followed() {
        let dir = TempDir::new("symlink");
        let outside = dir.0.join("outside");
        fs::create_dir_all(&outside).expect("created");
        let out = dir.0.join("out");
        fs::create_dir_all(&out).expect("created");
        std::os::unix::fs::symlink(&outside, out.join("link")).expect("linked");

        let site = StaticSiteGenerator::new(&out);
        assert!(site.write_file("x", "link/x.html").is_err());
        let asset = dir.0.join("a.css");
        fs::write(&asset, "a{}").expect("written");
        assert!(site.copy_asset(&asset, "link/a.css").is_err());
        assert!(only_files_named(&outside, &[]));
    }

    /// `with_extension` made `index.html` and `index.xml` share `index.tmp`.
    #[test]
    fn pages_with_the_same_stem_do_not_share_a_temporary_file() {
        let dir = TempDir::new("stem");
        let site = StaticSiteGenerator::new(&dir.0);
        std::thread::scope(|scope| {
            for round in 0..8 {
                for name in ["index.html", "index.xml", "index.htm"] {
                    let site = &site;
                    scope.spawn(move || {
                        site.write_file(&format!("{name}:{round}"), name)
                            .expect("written");
                    });
                }
            }
        });
        assert!(only_files_named(
            &dir.0,
            &["index.htm", "index.html", "index.xml"]
        ));
        for name in ["index.html", "index.xml", "index.htm"] {
            let content = fs::read_to_string(dir.0.join(name)).expect("readable");
            assert!(
                content.starts_with(&format!("{name}:")),
                "{name}: {content}"
            );
        }
    }

    #[test]
    fn a_failed_asset_copy_keeps_the_previous_file() {
        let dir = TempDir::new("asset-atomic");
        let site = StaticSiteGenerator::new(dir.0.join("out"));
        let source = dir.0.join("source.css");
        fs::write(&source, "old").expect("written");
        site.copy_asset(&source, "style.css").expect("copied");

        assert!(
            site.copy_asset(dir.0.join("missing.css"), "style.css")
                .is_err()
        );
        assert_eq!(
            fs::read_to_string(dir.0.join("out/style.css")).expect("readable"),
            "old"
        );
        assert!(only_files_named(&dir.0.join("out"), &["style.css"]));
    }

    /// `Path::new(".").parent()` is `Some("")`, so the old guard let `clean(".")` through.
    #[test]
    fn the_clean_guard_refuses_the_working_directory_and_what_contains_it() {
        let cwd = std::env::current_dir().expect("cwd");
        let mut unsafe_paths = vec![PathBuf::from("."), PathBuf::from("./"), cwd.clone()];
        unsafe_paths.extend(cwd.ancestors().skip(1).map(Path::to_path_buf));
        unsafe_paths.push(PathBuf::from("/usr"));
        if let Some(home) = std::env::var_os("HOME") {
            unsafe_paths.push(PathBuf::from(home));
        }
        for path in unsafe_paths {
            assert!(
                guard_destructive_path(&path).is_err(),
                "{path:?} should be refused"
            );
        }
        // A directory of its own, even one that does not exist yet, is fine.
        assert!(guard_destructive_path(&cwd.join("dist")).is_ok());
        assert!(guard_destructive_path(Path::new("dist")).is_ok());
    }

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

        // The failures are data, not only text.
        let SsgError::Pages(failures) = error else {
            panic!("a bulk build reports Pages");
        };
        let paths: Vec<&str> = failures.iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(paths, ["../one.html", "../two.html"]);
        assert!(
            failures
                .iter()
                .all(|(_, e)| matches!(e, SsgError::InvalidPath { .. }))
        );
    }

    #[test]
    fn each_failure_has_its_own_variant_and_converts_to_an_io_error() {
        use std::error::Error;

        let dir = TempDir::new("variants");
        let site = StaticSiteGenerator::new(&dir.0);

        let refused = site.write_file("x", "../out.html").expect_err("path");
        assert!(matches!(refused, SsgError::InvalidPath { ref path, .. } if path == "../out.html"));
        assert!(refused.source().is_none());
        assert_eq!(io::Error::from(refused).kind(), io::ErrorKind::InvalidInput);

        let unsafe_clean = StaticSiteGenerator::new(".")
            .clean(false)
            .expect_err("guard");
        assert!(matches!(unsafe_clean, SsgError::UnsafeCleanTarget { .. }));
        assert_eq!(
            io::Error::from(unsafe_clean).kind(),
            io::ErrorKind::InvalidInput
        );

        // A directory sits where the page should go, so the rename fails.
        fs::create_dir_all(dir.0.join("page.html")).expect("created");
        let failed = site
            .write_file("x", "page.html")
            .expect_err("rename onto a directory");
        assert!(matches!(failed, SsgError::Io { .. }));
        assert!(failed.source().is_some());
        assert!(failed.to_string().contains("page.html"));
        assert!(
            only_files_named(&dir.0, &["page.html"]),
            "no temp left behind"
        );
    }

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
