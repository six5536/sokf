//! This module holds the `Session`, which keeps same-same's comparer for
//! the runs of one command, builds it from `[format]`, and settles the pairs
//! of a pass in one batch.

use std::{
    cell::OnceCell,
    path::{Path, PathBuf},
};

use same_same::{
    Cache, Command, Comparer, ConfiguredFormatter, Detail, Detect, Format, Input, Method, Note,
    Verdict,
};

use super::{Layout, Pair, words};
use crate::{
    RepoPath, RepoRoot,
    config::{FormatConfig, FormatRule},
    source,
};

/// `CACHE_FOLDER` is the folder of same-same's cache inside sokf's cache
/// directory (CACHE-2_AC-10).
pub const CACHE_FOLDER: &str = "same-same";

/// A `Session` holds what the runs of one command share of the comparison:
/// same-same's comparer, which it builds once, when the cache asks for the
/// identity of the formatter or at the first pair, with its cache.
pub struct Session {
    /// The repository root.
    root: RepoRoot,
    /// Whether the session keeps same-same's cache in its folder.
    on: bool,
    /// The comparer, once the session built it.
    built: OnceCell<Built>,
}

/// A `Built` comparer, with its cache and the file at which a warning that
/// its formatter failed lies.
struct Built {
    comparer: Comparer,
    cache: Cache,
    origin: RepoPath,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("on", &self.on)
            .finish_non_exhaustive()
    }
}

impl Drop for Session {
    /// The session writes the entries that same-same's cache gained to its
    /// folder. A failure costs the next run only the time of the
    /// comparisons again.
    fn drop(&mut self) {
        if let Some(built) = self.built.get() {
            let _ = built.cache.flush();
        }
    }
}

impl Session {
    /// `new` returns a session for the repository at `root`, which keeps
    /// same-same's cache in `/.sokf/cache/same-same/` when `on` holds, and
    /// in memory otherwise (CACHE-2_AC-8, CACHE-2_AC-10).
    // sokf-implements: CACHE-2_AC-8
    pub fn new(root: &RepoRoot, on: bool) -> Session {
        Session {
            root: root.clone(),
            on,
            built: OnceCell::new(),
        }
    }

    /// `identity` returns the identity of each formatter or command through
    /// which same-same compares the formats that it doesn't know, in
    /// same-same's text form, such as `n:prettier@3.3.3#1a2b3c4d`, or `none`,
    /// which the version of the store of the cache records, so that another
    /// formatter, another version of it, or other settings rebuild the store
    /// (CACHE-1_AC-6, CACHE-2_AC-11). It builds the comparer
    /// of the session from the `[format]` table of `/.sokf/config.toml`, and
    /// starts no process, so it misses the version of a command's program
    /// and of a formatter outside npm that only a run tells.
    // sokf-implements: CACHE-2_AC-11
    pub fn identity(&self) -> String {
        let built = self
            .built
            .get_or_init(|| self.build(&crate::config::format_config(&self.root)));
        let identities: Vec<String> = built
            .comparer
            .normalisers()
            .iter()
            .map(|(id, _)| id.to_string())
            .collect();
        if identities.is_empty() {
            "none".to_string()
        } else {
            identities.join(",")
        }
    }

    /// `settle` asks same-same about the pairs of `pairs` in one batch,
    /// through the formatter or the command that `config` names, and the
    /// formatter that same-same detects only where `config.detect` turns
    /// detection on, with the comparer that the session built already where
    /// it did, and returns the layout of the pairs. A pass with no pair
    /// starts no process (CLI-4_AC-44, CACHE-3_AC-13).
    // sokf-implements: CACHE-3_AC-13
    // sokf-implements: CLI-4_AC-49
    // sokf-implements: CLI-4_AC-111
    // sokf-implements: CLI-4_AC-113
    pub fn settle(&self, config: &FormatConfig, pairs: &[Pair]) -> Layout {
        let mut layout = Layout::default();
        if pairs.is_empty() {
            return layout;
        }
        let built = self.built.get_or_init(|| self.build(config));
        // The whole text of each included file whose expected text is a
        // range of its lines.
        let wholes: Vec<Option<String>> = pairs
            .iter()
            .map(|p| {
                let source = p.source.as_ref()?;
                let range = source.range.as_ref()?;
                let text = source::load_text(&self.root, &source.path).ok()?;
                // A file that changed since the expansion read it no longer
                // holds the expected text in that range.
                (lines_of(&text, range) == p.expected).then_some(text)
            })
            .collect();
        let inputs: Vec<(Input<'_>, Input<'_>)> = pairs
            .iter()
            .zip(&wholes)
            .map(|(p, whole)| inputs(p, whole.as_deref()))
            .collect();
        // A cached fingerprint answers a verdict without a parse, so only a
        // pair that differs is compared again for the place of its first
        // difference.
        let mut answers = built.comparer.compare_all(&inputs, Detail::Verdict);
        // An answer by the bytes is `Unknown`, and gives its place too, but
        // a pair that went through a normaliser, or tried one, isn't compared
        // again, which would run the normaliser again.
        let different: Vec<usize> = (0..answers.len())
            .filter(|&i| {
                let answer = &answers[i];
                let normalised = matches!(answer.method, Method::Normaliser { .. })
                    || answer.notes.iter().any(|note| {
                        matches!(
                            note,
                            Note::NormaliserUnavailable { .. } | Note::NormaliserCannot { .. }
                        )
                    });
                !normalised && matches!(answer.verdict, Verdict::Different | Verdict::Unknown)
            })
            .collect();
        if !different.is_empty() {
            let again: Vec<(Input<'_>, Input<'_>)> =
                different.iter().map(|&i| inputs[i].clone()).collect();
            for (i, answer) in different
                .into_iter()
                .zip(built.comparer.compare_all(&again, Detail::FirstPlace))
            {
                answers[i] = answer;
            }
        }
        for ((pair, answer), whole) in pairs.iter().zip(&answers).zip(&wholes) {
            let reading = words::reading(pair, answer, whole.is_some(), config.same, &built.origin);
            layout.insert(pair.clone(), reading);
        }
        layout
    }

    /// `build` returns same-same's comparer for every format that it
    /// compares by meaning, with the formatter or the command of `config`,
    /// the detection of the formatter at the repository root only where
    /// `config.detect` turns it on, and the cache in its folder or in
    /// memory.
    // sokf-implements: CLI-4_AC-48
    // sokf-implements: CONF-10_AC-1
    // sokf-implements: CONF-10_AC-2
    // sokf-implements: CONF-10_AC-3
    // sokf-implements: CONF-10_AC-8
    // sokf-implements: CONF-10_AC-26
    // sokf-implements: CACHE-2_AC-10
    fn build(&self, config: &FormatConfig) -> Built {
        let root = self.root.as_path().to_path_buf();
        let cache = if self.on {
            crate::cache::ensure_dir(&self.root).map_or_else(
                |_| Cache::memory(),
                |dir| Cache::folder(&dir.join(CACHE_FOLDER)),
            )
        } else {
            Cache::memory()
        };
        let builder = Comparer::builder()
            .formats(Format::ALL.iter().copied())
            .cache(cache.clone());
        let config_file = crate::config::config_path();
        let builder = match &config.rule {
            Some(FormatRule::Formatter {
                formatter,
                config: file,
                settings,
                plugins,
            }) => builder.formatter(ConfiguredFormatter {
                formatter: *formatter,
                files: vec!["*".into()],
                overrides: false,
                root: root.clone(),
                program: None,
                config: file.as_ref().map(PathBuf::from),
                settings: *settings,
                plugins: plugins.clone(),
                timeout: config.timeout,
            }),
            Some(FormatRule::Command { argv, config: file }) => builder.command(Command {
                argv: command_line(&root, argv),
                files: vec!["*".into()],
                overrides: false,
                timeout: config.timeout,
                version_argv: None,
                settings: file.as_deref().and_then(|f| settings_file(&root, f)),
            }),
            None => builder,
        };
        // A clone's program runs only by the project's choice, so sokf looks
        // for no formatter unless `format.detect` turns detection on.
        let (builder, origin) = if config.detect {
            let origin = match &config.rule {
                Some(_) => config_file,
                None => detected_origin(&root).unwrap_or(config_file),
            };
            let builder = builder.detect(Detect {
                root: root.clone(),
                timeout: config.timeout,
            });
            (builder, origin)
        } else {
            (builder, config_file)
        };
        // Every format that the build compiles is turned on, so the builder
        // can't fail.
        let comparer = builder
            .build()
            .expect("sokf compiles every format of same-same");
        Built {
            comparer,
            cache,
            origin,
        }
    }
}

/// `detected_origin` returns the configuration file at `root` that reveals
/// the formatter that same-same detects, as a path of the repository.
fn detected_origin(root: &Path) -> Option<RepoPath> {
    let found = same_same::detected(root)?;
    let root = std::path::absolute(root).ok()?;
    let parts: Vec<String> = found
        .found_by
        .strip_prefix(&root)
        .ok()?
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    RepoPath::parse(&format!("/{}", parts.join("/")))
}

/// `inputs` returns the two sides of `pair`: the expected text as the range
/// of its lines in `whole`, the whole included file, where it is one, else
/// as a fragment or a whole text, and the text that the file holds, marked
/// as the expected text is (CLI-4_AC-111).
fn inputs<'a>(pair: &'a Pair, whole: Option<&'a str>) -> (Input<'a>, Input<'a>) {
    let expected = match (whole, pair.source.as_ref().and_then(|s| s.range.clone())) {
        (Some(whole), Some(range)) => Input::bytes(whole.as_bytes(), &pair.name).lines(range),
        _ => mark(
            Input::bytes(pair.expected.as_bytes(), &pair.name),
            pair.fragment,
        ),
    };
    let actual = mark(
        Input::bytes(pair.actual.as_bytes(), &pair.name),
        pair.fragment,
    );
    (expected, actual)
}

/// `mark` marks `input` as a fragment where `fragment` holds.
fn mark(input: Input<'_>, fragment: bool) -> Input<'_> {
    if fragment { input.fragment() } else { input }
}

/// `lines_of` returns the lines of `text` in `range`, each with its newline.
fn lines_of(text: &str, range: &std::ops::Range<u32>) -> String {
    let start = usize::try_from(range.start).unwrap_or(usize::MAX);
    let end = usize::try_from(range.end).unwrap_or(usize::MAX);
    text.lines()
        .enumerate()
        .filter(|(i, _)| (start..end).contains(&(i + 1)))
        .fold(String::new(), |mut out, (_, line)| {
            out.push_str(line);
            out.push('\n');
            out
        })
}

/// `command_line` returns the command of `[format]` with a program that it
/// names by a relative path, such as `./fmt.sh`, as a path from `root`, and
/// a program that it names by its name alone as the one of
/// `node_modules/.bin` at `root` where a program of that name lies there,
/// in the repository, as
/// sokf 0.8.0 ran it, because same-same runs the command in a temporary
/// folder, and looks for a command's program on the path alone.
fn command_line(root: &Path, argv: &[String]) -> Vec<String> {
    let mut argv = argv.to_vec();
    if let Some(program) = argv.first_mut() {
        if program.contains(['/', '\\']) {
            if !program.starts_with(['/', '\\']) && Path::new(program.as_str()).is_relative() {
                *program = root.join(&*program).to_string_lossy().into_owned();
            }
        } else if let Some(found) = program_in(&root.join("node_modules").join(".bin"), program)
            .filter(|found| inside(root, found))
        {
            *program = found.to_string_lossy().into_owned();
        }
    }
    argv
}

/// `inside` returns whether the canonical form of `path` lies in the
/// canonical `root`, so that a program of `node_modules/.bin` that links out
/// of the repository is no program of the repository, as a program of
/// `[format]` named by a relative path isn't (CONF-10_AC-25).
fn inside(root: &Path, path: &Path) -> bool {
    std::fs::canonicalize(root)
        .ok()
        .zip(std::fs::canonicalize(path).ok())
        .is_some_and(|(root, path)| path.starts_with(root))
}

/// `program_in` returns the program `name` in `folder`, by same-same's rule
/// for a program in a folder of its own (`command/program.rs` of same-same
/// 0.1.0, `in_folder` and `candidates`, which it doesn't export): on Windows
/// the name with `.exe`, `.cmd`, and `.bat`, where it has none of them, and
/// the name itself only where it has an ending; elsewhere the name; each a
/// program only where it is a file, executable on Unix.
fn program_in(folder: &Path, name: &str) -> Option<PathBuf> {
    const ENDINGS: [&str; 3] = ["exe", "cmd", "bat"];
    let ending = Path::new(name).extension();
    let ended =
        ending.is_some_and(|ending| ENDINGS.iter().any(|of| ending.eq_ignore_ascii_case(of)));
    let mut candidates: Vec<PathBuf> = Vec::new();
    if cfg!(windows) && !ended {
        candidates.extend(ENDINGS.iter().map(|of| folder.join(format!("{name}.{of}"))));
    }
    if !cfg!(windows) || ending.is_some() {
        candidates.push(folder.join(name));
    }
    candidates.into_iter().find(|path| is_program(path))
}

/// `is_program` returns whether the file at `path` can be a program: a
/// file, executable on Unix.
fn is_program(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::metadata(path)
            .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        path.is_file()
    }
}

/// `settings_file` returns the name and the text of the configuration
/// file `file`, from `root`, which a command names as `{settings}`, or
/// `None` where it doesn't read.
fn settings_file(root: &Path, file: &str) -> Option<(String, String)> {
    let text = std::fs::read_to_string(root.join(file)).ok()?;
    let name = Path::new(file).file_name()?.to_string_lossy().into_owned();
    Some((name, text))
}

// The one test sets the mode bits of a file, which only Unix has.
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::test_support::{fresh_dir, write};

    // A bare program of a command runs from `node_modules/.bin` only where
    // the file there is a program by same-same's rule, and else from the
    // path, and a program named by a relative path runs from the root.
    #[test]
    fn a_bare_program_runs_from_node_modules_only_where_it_is_a_program() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = fresh_dir("command-line");
        let argv = |program: &str| vec![program.to_string(), "{files}".to_string()];
        write(&dir, "node_modules/.bin/fmt", "#!/bin/sh\n");
        assert_eq!(
            command_line(&dir, &argv("fmt"))[0],
            "fmt",
            "a file that isn't a program"
        );
        let bin = dir.join("node_modules/.bin/fmt");
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(command_line(&dir, &argv("fmt"))[0], bin.to_string_lossy());
        let outside = fresh_dir("command-line-outside");
        write(&outside, "fmt", "#!/bin/sh\n");
        std::fs::set_permissions(outside.join("fmt"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        std::os::unix::fs::symlink(outside.join("fmt"), dir.join("node_modules/.bin/out")).unwrap();
        assert_eq!(
            command_line(&dir, &argv("out"))[0],
            "out",
            "a link out of the repository"
        );
        std::fs::remove_dir_all(&outside).unwrap();
        std::fs::create_dir_all(dir.join("node_modules/.bin/dir")).unwrap();
        assert_eq!(command_line(&dir, &argv("dir"))[0], "dir", "a folder");
        assert_eq!(
            command_line(&dir, &argv("./tools/fmt.sh"))[0],
            dir.join("./tools/fmt.sh").to_string_lossy()
        );
        assert_eq!(command_line(&dir, &argv("/usr/bin/fmt"))[0], "/usr/bin/fmt");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
