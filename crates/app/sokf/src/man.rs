//! This module renders the man page.
//!
//! `clap_mangen` renders only the top-level command, which would leave the
//! flags of every verb undocumented. This module keeps the title, name,
//! synopsis, description, and global options sections that `clap_mangen`
//! renders, and then appends hand-written roff: a COMMANDS section with the
//! flags of each verb, an EXIT STATUS section, and a FILES section.

use std::fmt::Write as _;

use clap::CommandFactory;

use crate::cli::Cli;

/// `render` renders the complete man page as roff.
pub fn render() -> std::io::Result<Vec<u8>> {
    let man = clap_mangen::Man::new(Cli::command());
    let mut buf = Vec::new();
    man.render_title(&mut buf)?;
    man.render_name_section(&mut buf)?;
    man.render_synopsis_section(&mut buf)?;
    man.render_description_section(&mut buf)?;
    man.render_options_section(&mut buf)?;

    let mut extra = String::new();
    render_commands(&mut extra);
    render_exit_status(&mut extra);
    render_files(&mut extra);
    buf.extend_from_slice(extra.as_bytes());

    man.render_version_section(&mut buf)?;
    Ok(buf)
}

/// `render_commands` writes one `.SS` subsection for each visible verb, with
/// its positionals and flags.
///
/// It walks the subcommands of a command that has not been built.
/// `clap::Command::build` propagates the global flags and adds `--help` to
/// every subcommand. So a built command would repeat `--repo-root`,
/// `--no-cache`, and `-V` under each verb. A verb with subcommands of its own, such as `info`, gets
/// one subsection for each subcommand.
fn render_commands(out: &mut String) {
    out.push_str(".SH COMMANDS\n");
    for sub in Cli::command().get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        if sub.has_subcommands() {
            for inner in sub.get_subcommands().filter(|c| !c.is_hide_set()) {
                render_verb(out, &format!("sokf {}", sub.get_name()), inner);
            }
        } else {
            render_verb(out, "sokf", sub);
        }
    }
}

fn render_verb(out: &mut String, prefix: &str, sub: &clap::Command) {
    let _ = writeln!(out, ".SS \"{}\"", usage(prefix, sub));
    let about = sub.get_about().map(ToString::to_string).unwrap_or_default();
    out.push_str(&text(&about));
    for arg in sub.get_arguments() {
        if arg.get_id() == "help" || arg.is_global_set() {
            continue;
        }
        out.push_str(".TP\n");
        let _ = writeln!(out, "{}", arg_header(arg));
        let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
        out.push_str(&text(&help));
    }
}

/// `usage` returns the `.SS` heading of a verb: its name, each required
/// positional as `<NAME>`, each optional positional as `[NAME]`, and
/// `[OPTIONS]` when the verb has flags.
fn usage(prefix: &str, sub: &clap::Command) -> String {
    let mut s = format!("{prefix} {}", sub.get_name());
    for pos in sub.get_positionals() {
        if pos.is_required_set() {
            let _ = write!(s, " <{}>", value_name(pos));
        } else {
            let _ = write!(s, " [{}]", value_name(pos));
        }
    }
    if sub.get_arguments().any(|a| !a.is_positional()) {
        s.push_str(" [OPTIONS]");
    }
    s
}

/// `arg_header` returns the text that heads a `.TP` entry: the short and long
/// forms of a flag in bold, or a positional in italics, followed by the italic
/// value placeholder when the flag takes a value.
fn arg_header(arg: &clap::Arg) -> String {
    if arg.is_positional() {
        return format!("\\fI<{}>\\fR", value_name(arg));
    }
    let mut s = String::new();
    match (arg.get_short(), arg.get_long()) {
        (Some(short), Some(long)) => {
            let _ = write!(s, "\\fB\\-{short}\\fR, \\fB\\-\\-{}\\fR", opt(long));
        }
        (Some(short), None) => {
            let _ = write!(s, "\\fB\\-{short}\\fR");
        }
        (None, Some(long)) => {
            let _ = write!(s, "\\fB\\-\\-{}\\fR", opt(long));
        }
        (None, None) => {}
    }
    if arg.get_action().takes_values() {
        let _ = write!(s, " \\fI<{}>\\fR", value_name(arg));
    }
    s
}

/// `value_name` returns the `<VALUE>` placeholder of an argument: its first
/// declared value name, or else its id in upper case.
fn value_name(arg: &clap::Arg) -> String {
    arg.get_value_names()
        .and_then(|names| names.first())
        .map(ToString::to_string)
        .unwrap_or_else(|| arg.get_id().to_string().to_uppercase())
}

/// `render_exit_status` writes the exit codes that CLI-1_AC-9, CLI-1_AC-10,
/// and CLI-1_AC-11 define.
fn render_exit_status(out: &mut String) {
    out.push_str(
        ".SH EXIT STATUS\n\
         .TP\n.B 0\nThe command succeeded and found no errors.\n\
         .TP\n.B 1\nThe command found errors in the knowledge.\n\
         .TP\n.B 2\nA usage or internal error occurred. sokf reports it on stderr.\n",
    );
}

/// `render_files` writes the FILES section, which lists the files that sokf
/// reads and writes (contract-config, CLI-4_AC-18).
fn render_files(out: &mut String) {
    out.push_str(
        ".SH FILES\n\
         .TP\n.I /.sokf/config.toml\nThis optional file lists the knowledge roots and the paths that the repository excludes.\n\
         .TP\n.I /.sokf/formats.yaml\nThis optional file holds the line formats defined for this repository.\n\
         .TP\n.I /.sokf/cache/\nThis directory holds a cache entry for each file, and git ignores the directory. The \\-\\-no\\-cache flag skips the cache.\n\
         .TP\n.I <root>/manifest.sokf.yaml\nThis file is the manifest of a knowledge store.\n",
    );
}

/// `text` returns a line of roff text: it escapes backslashes, and it puts
/// `\&` before a control character at the start of the line. The `\&` makes
/// roff print the character instead of reading the line as a request.
fn text(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\");
    if escaped.starts_with('.') || escaped.starts_with('\'') {
        format!("\\&{escaped}\n")
    } else {
        format!("{escaped}\n")
    }
}

/// `opt` escapes the dashes inside a long option name: `dry-run` becomes `dry\-run`.
fn opt(long: &str) -> String {
    long.replace('-', "\\-")
}
