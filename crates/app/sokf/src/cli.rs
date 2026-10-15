//! The command-line interface, declared with clap's derive macros.
//!
//! This module declares the commands and options that contract-cli
//! (`knowledge/contracts/contract-cli.md`) specifies in CLI-1 to CLI-8, and two
//! more commands: `completions` and the hidden `man`.

use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use sokf_core::Output;

// sokf-implements: CLI-1_AC-1
// sokf-implements: CLI-1_AC-2
// sokf:begin cli
/// Validate and map SOKF knowledge stores, which are directories of Markdown
/// documents with YAML frontmatter in a git repository: check the documents
/// against the SOKF specification, and print them as a graph.
#[derive(Debug, Parser)]
// The `version` attribute puts the version in the man page header and in
// `--help`. The built-in `-V` flag is disabled because `main::run` prints the
// version itself. The built-in `-h` flag is disabled so that `help` below can
// describe `--help` in a full sentence.
#[command(
    name = "sokf",
    about,
    long_about = None,
    version,
    disable_version_flag = true,
    disable_help_flag = true
)]
#[command(
    after_help = "Exit codes: 0 if there are no errors, 1 if there are errors, 2 on a usage or internal error.\n\
    Docs: https://github.com/six5536/sokf"
)]
pub struct Cli {
    /// `--repo-root` sets the repository root, and sokf otherwise uses the
    /// root of the Git repository that contains the current directory.
    #[arg(long, global = true, value_name = "DIR")]
    pub repo_root: Option<PathBuf>,

    /// `--no-cache` makes sokf read every file from disk and leave the cache
    /// unchanged.
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// The command to run.
    #[command(subcommand)]
    pub command: Option<Command>,

    /// `--version` prints the version and exits.
    #[arg(short = 'V', long = "version", global = true)]
    pub version: bool,

    /// `--help` prints the help of the command and exits.
    #[arg(short = 'h', long = "help", global = true, action = ArgAction::Help)]
    pub help: Option<bool>,
}

/// `Command` lists the top-level commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a knowledge store, and add its directory to the knowledge roots
    /// in `/.sokf/config.toml`.
    Init(InitArgs),
    /// Install the integration of an agent harness, or show its state.
    Harness(HarnessArgs),
    /// Check the knowledge stores and the other files that their schemas govern, and repair them with --fix.
    Validate(ValidateArgs),
    /// Print the schema language or the format registry that the validator uses.
    Info(InfoArgs),
    /// Create a document of a type from its schema's scaffold.
    New(NewArgs),
    /// Print the knowledge as a graph, starting from any document, file, key, or directory.
    Graph(GraphArgs),
    /// Print the path of the document that each id identifies.
    Locate(LocateArgs),
    /// Print a shell completion script to stdout.
    Completions(CompletionsArgs),
    /// Print a roff man page to stdout. The command is hidden because the
    /// release build runs it when it packages the tool, not a user in daily
    /// use: the release build writes its page into the release archives.
    #[command(hide = true)]
    Man,
}
// sokf:end cli

impl Command {
    /// `json_schema` returns the output whose schema the command prints, when
    /// it runs with `--json-schema`.
    pub fn json_schema(&self) -> Option<Output> {
        match self {
            Command::Init(a) => a.json_schema.then_some(Output::Init),
            Command::Harness(a) => match &a.what {
                HarnessCommand::Install(a) => a.json_schema.then_some(Output::Harness),
                HarnessCommand::Status(a) => a.json_schema.then_some(Output::Harness),
                HarnessCommand::Hook(_) => None,
            },
            Command::Validate(a) => a.json_schema.then_some(Output::Validate),
            Command::New(a) => a.json_schema.then_some(Output::New),
            Command::Graph(a) => a.json_schema.then_some(Output::Graph),
            Command::Locate(a) => a.json_schema.then_some(Output::Locate),
            Command::Info(_) | Command::Completions(_) | Command::Man => None,
        }
    }
}

// The help of each `--json` flag names `--json-schema`, which prints the
// schema of the output.
// sokf-implements: CLI-1_AC-17
// sokf:begin init
/// `InitArgs` holds the arguments of `init` (CLI-2).
#[derive(Debug, Args)]
pub struct InitArgs {
    /// The store's directory, as a filesystem path or a path from the
    /// repository root. It must be absent or empty. Default: `/knowledge`.
    #[arg(value_name = "DIR")]
    pub dir: Option<PathBuf>,

    /// The manifest `name`, a slug: lowercase words of letters and digits,
    /// joined by `-`. Default: the last segment of DIR.
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,

    /// The manifest `scope`, the slug of the store's owner. Default: the
    /// owner in the URL of the git remote `origin`.
    #[arg(long, value_name = "SCOPE")]
    pub scope: Option<String>,

    /// The manifest `description`. Default: the manifest has no `description`.
    #[arg(long, value_name = "TEXT")]
    pub description: Option<String>,

    /// Print one JSON object to stdout instead of the text lines.
    /// `--json-schema` prints its schema.
    #[arg(long)]
    pub json: bool,

    /// Print the JSON Schema of the `--json` output, and exit.
    #[arg(long, exclusive = true)]
    pub json_schema: bool,
}
// sokf:end init

// sokf:begin harness
/// `HarnessArgs` holds the arguments of `harness` (CLI-3).
#[derive(Debug, Args)]
pub struct HarnessArgs {
    /// What to do.
    #[command(subcommand)]
    pub what: HarnessCommand,
}

/// `HarnessCommand` lists the subcommands of `harness`.
#[derive(Debug, Subcommand)]
pub enum HarnessCommand {
    /// Make each harness's files match sokf's integration: write every absent
    /// or stale part, but keep a part that you edited unless you pass `--force`.
    Install(HarnessInstallArgs),
    /// Print the state of each part of each harness, and write nothing.
    Status(HarnessStatusArgs),
    /// `sokf harness hook` answers a hook of a harness. The command is hidden
    /// because the harness calls it, not a person.
    // sokf-implements: CLI-3_AC-50
    #[command(hide = true)]
    Hook(HarnessHookArgs),
}

/// `HarnessScopeArg` is the scope of `--scope`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum HarnessScopeArg {
    // The variants have no doc comments, so that clap keeps the help of
    // `--scope` on one line, as the help of every other option.
    // The repository's files, which you commit.
    #[default]
    Project,
    // The files in your home directory, for every repository.
    User,
    // The repository's files that stay out of git, for Claude Code and GitHub
    // Copilot.
    Local,
}

/// `HarnessInstallArgs` holds the arguments of `harness install`.
#[derive(Debug, Args)]
pub struct HarnessInstallArgs {
    /// The harnesses: `claude`, `codex`, `factory`, `gemini`, `copilot`,
    /// `cursor`, `pi`, `opencode`, `kilo`, `qwen`, `devin`, `agents` (any agent
    /// that reads `AGENTS.md`), or `all`.
    #[arg(value_name = "HARNESS", required = true)]
    pub names: Vec<String>,

    /// Where to install: `project` (the repository's files, which you
    /// commit), `user` (your home directory, for every repository), or `local`
    /// (the repository's files that stay out of git; Claude Code and GitHub
    /// Copilot only).
    #[arg(long, value_enum, value_name = "SCOPE", default_value_t)]
    pub scope: HarnessScopeArg,

    /// Skip a part of each harness. You can repeat the option, and the parts
    /// you list replace the list of skipped parts of each harness named in
    /// `/.sokf/config.toml`.
    #[arg(long, value_name = "PART", action = ArgAction::Append)]
    pub without: Vec<String>,

    /// Write a part even if you edited it.
    #[arg(long)]
    pub force: bool,

    /// Print one JSON object to stdout instead of the text lines.
    /// `--json-schema` prints its schema.
    #[arg(long)]
    pub json: bool,

    /// Print the JSON Schema of the `--json` output, and exit.
    #[arg(long, exclusive = true)]
    pub json_schema: bool,
}

/// `HarnessStatusArgs` holds the arguments of `harness status`.
#[derive(Debug, Args)]
pub struct HarnessStatusArgs {
    /// The harnesses, as for `harness install`, or `all`.
    #[arg(value_name = "HARNESS", required = true)]
    pub names: Vec<String>,

    /// Where to look: `project`, `user`, or `local`.
    #[arg(long, value_enum, value_name = "SCOPE", default_value_t)]
    pub scope: HarnessScopeArg,

    /// Print one JSON object to stdout instead of the text lines.
    /// `--json-schema` prints its schema.
    #[arg(long)]
    pub json: bool,

    /// Print the JSON Schema of the `--json` output, and exit.
    #[arg(long, exclusive = true)]
    pub json_schema: bool,
}

/// `HarnessHookArgs` holds the arguments of `harness hook`.
#[derive(Debug, Args)]
pub struct HarnessHookArgs {
    /// The harness that runs the hook, for example `claude`.
    #[arg(value_name = "HARNESS")]
    pub name: String,

    /// The event, for example `stop`.
    #[arg(value_name = "EVENT")]
    pub hook: String,
}
// sokf:end harness

// `--query` requires `--json`: clap refuses `--query` alone, and the tool exits
// with code 2.
// sokf-implements: CLI-4_AC-16
// sokf:begin validate
/// `ValidateArgs` holds the arguments of `validate` (CLI-4).
#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// The files or directories to report on, as filesystem paths or paths
    /// from the repository root.
    #[arg(value_name = "PATHS")]
    pub paths: Vec<PathBuf>,

    /// Repair the problems that have a mechanical fix, then check.
    #[arg(long)]
    pub fix: bool,

    /// Print one JSON object to stdout instead of text. `--json-schema`
    /// prints its schema.
    #[arg(long)]
    pub json: bool,

    /// Print the JSON Schema of the `--json` output, and exit.
    #[arg(long, exclusive = true)]
    pub json_schema: bool,

    /// Apply a JMESPath expression to the JSON object. Requires --json.
    #[arg(long, value_name = "EXPR", requires = "json")]
    pub query: Option<String>,

    /// List each warning. Default: print only the number of warnings.
    #[arg(long)]
    pub warnings: bool,

    /// Treat PATH as changed, and every other file as unchanged since the
    /// cache recorded it. You can repeat the option. The next run without the option finds
    /// any change to a path that no --changed option gives.
    #[arg(long, value_name = "PATH", conflicts_with = "no_cache")]
    pub changed: Vec<PathBuf>,
}
// sokf:end validate

// sokf-implements: CLI-7_AC-23
// sokf-implements: CLI-7_AC-24
// sokf:begin graph
/// `GraphArgs` holds the arguments of `graph` (CLI-7).
#[derive(Debug, Args)]
pub struct GraphArgs {
    /// Document ids, in any form that a reference takes (`glossary`,
    /// `sokf-spec:glossary`, `#key`), or the paths of files or directories,
    /// as filesystem paths or paths from the repository root. For a directory
    /// in a knowledge store, graph prints its layout, and for a document, a
    /// file, or a key, its edges.
    /// Default: the layout of every knowledge root.
    #[arg(value_name = "START")]
    pub start: Vec<String>,

    /// How many edges deep to print under each document, file, or key. A
    /// depth of 0 prints the node alone. Default: 1 for a START that is a
    /// document, a file, or a key, and 0 for a layout.
    #[arg(long, value_name = "N")]
    pub depth: Option<usize>,

    /// Print every entry of a group, past the limit of 20, every region of a
    /// folded include, and every marker of a folded marker edge. A group is
    /// the entries of a directory or the edges of one relationship of a node.
    /// A folded include is one edge that stands for the includes of two or
    /// more regions of one file. A folded marker edge is one edge that stands
    /// for the markers of one file that cite a document.
    #[arg(long)]
    pub all: bool,

    /// Print the archived documents and their edges too.
    #[arg(long)]
    pub archived: bool,

    /// Print only the rows whose document matches the predicate. You can
    /// repeat the option, and graph prints a row if any predicate matches.
    #[arg(short = 'l', long = "selector", value_name = "EXPR", action = ArgAction::Append)]
    pub selector: Vec<String>,

    /// Print only the edges that match the predicate. You can repeat the
    /// option, and graph prints an edge if any predicate matches.
    #[arg(long, value_name = "EXPR", action = ArgAction::Append)]
    pub edge: Vec<String>,

    /// Print only the edges whose target document matches the predicate. You
    /// can repeat the option, and graph prints an edge if any predicate matches.
    #[arg(long, value_name = "EXPR", action = ArgAction::Append)]
    pub to: Vec<String>,

    /// The template of a document's text line. Default: `{id}`. It
    /// applies only to text output.
    #[arg(long, value_name = "TPL", conflicts_with_all = ["json", "mermaid"])]
    pub format: Option<String>,

    /// Print one JSON object to stdout instead of text. `--json-schema`
    /// prints its schema.
    #[arg(long, conflicts_with = "mermaid")]
    pub json: bool,

    /// Print the JSON Schema of the `--json` output, and exit.
    #[arg(long, exclusive = true)]
    pub json_schema: bool,

    /// Apply a JMESPath expression to the JSON object. Requires --json.
    // Clap doesn't report a required argument as missing when it conflicts
    // with an argument that is present, so `--query` declares the conflicts of
    // `--json` as well.
    #[arg(long, value_name = "EXPR", requires = "json", conflicts_with_all = ["format", "mermaid"])]
    pub query: Option<String>,

    /// Print one Mermaid flowchart to stdout instead of text.
    #[arg(long)]
    pub mermaid: bool,
}
// sokf:end graph

// sokf-implements: CLI-8_AC-12
// sokf:begin locate
/// `LocateArgs` holds the arguments of `locate` (CLI-8).
#[derive(Debug, Args)]
pub struct LocateArgs {
    /// Document ids, in any form that a reference takes: `glossary`,
    /// `sokf-spec:glossary`, `scope/name:glossary`, or a key alone, `#key`.
    #[arg(value_name = "ID", required = true)]
    pub ids: Vec<String>,

    /// Each id must identify a sidecar, the document that describes an asset,
    /// such as an image. For each id, print the path of the asset.
    #[arg(long, conflicts_with = "json")]
    pub asset: bool,

    /// Print one JSON array of `{id, path, asset}` objects to stdout.
    /// `--json-schema` prints its schema.
    #[arg(long)]
    pub json: bool,

    /// Print the JSON Schema of the `--json` output, and exit.
    #[arg(long, exclusive = true)]
    pub json_schema: bool,
}
// sokf:end locate

// `--layer` conflicts with `--schema`: clap refuses the two together, and the
// tool exits with code 2.
// sokf-implements: CLI-5_AC-9
// sokf:begin info
/// `InfoArgs` holds the arguments of `info` (CLI-5).
#[derive(Debug, Args)]
pub struct InfoArgs {
    /// What to print.
    #[command(subcommand)]
    pub what: InfoCommand,
}

/// `InfoCommand` lists the subcommands of `info`.
#[derive(Debug, Subcommand)]
pub enum InfoCommand {
    /// `sokf info grammar` prints the schema language verbatim: the tramlines
    /// core grammar, then the tramlines-md grammar, and then the schema
    /// extension of the Superdev Open Knowledge Format (SOKF). Tramlines is a
    /// language that describes the form of a document, and tramlines-md
    /// applies it to Markdown.
    Grammar,
    /// `sokf info formats` prints a format registry as YAML, with the source
    /// of each entry.
    Formats(InfoFormatsArgs),
}

/// `InfoFormatsArgs` holds the arguments of `info formats`.
#[derive(Debug, Args)]
pub struct InfoFormatsArgs {
    /// The layer to print. Default: effective.
    #[arg(long, value_enum, value_name = "LAYER", conflicts_with = "schema")]
    pub layer: Option<Layer>,

    /// Print the effective registry, with the formats and types of the schema
    /// ID taking precedence over its entries.
    #[arg(long, value_name = "ID")]
    pub schema: Option<String>,
}

/// `Layer` lists the values of `--layer` of `info formats` (CLI-5_AC-4,
/// CLI-5_AC-5, CLI-5_AC-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Layer {
    /// Only the formats built into the tool. sokf builds this layer from the
    /// tool alone, without reading the repository.
    Embedded,
    /// Only the formats in `/.sokf/formats.yaml`.
    Repo,
    /// The two layers merged, with the repository's entries taking
    /// precedence.
    Effective,
}
// sokf:end info

// sokf-implements: CLI-6_AC-51
// sokf-implements: CLI-6_AC-57
// sokf:begin new
/// `NewArgs` holds the arguments of `new` (CLI-6).
#[derive(Debug, Args)]
pub struct NewArgs {
    /// The document type. When more than one store declares the type, put the store before it, as in `sokf-spec:Grammar`, or give `--dir` or `--for` in that store. The scaffold of `Schema` comes from the grammar of the schema language, the set of rules that says what a schema can contain. `sokf info grammar` prints that grammar.
    #[arg(value_name = "TYPE", required = true)]
    pub type_name: Option<String>,

    /// The value of the `slug` placeholder in the id template. It is required when the template has that placeholder.
    #[arg(long, value_name = "SLUG", conflicts_with = "id")]
    pub slug: Option<String>,

    /// The id, for a type whose schema declares no id template.
    #[arg(long, value_name = "ID")]
    pub id: Option<String>,

    /// The title. Default: the words of the slug, or of the id.
    #[arg(long, value_name = "TEXT")]
    pub title: Option<String>,

    /// Set a lifecycle key to a state, as KEY=STATE. Default: the first state of each lifecycle key.
    #[arg(long, value_name = "KEY=STATE", action = ArgAction::Append, value_parser = parse_state)]
    pub state: Vec<(String, String)>,

    /// The directory, for a type whose lifecycle states do not set the document's directory. Default: the knowledge root.
    #[arg(long, value_name = "DIR", conflicts_with = "for_asset")]
    pub dir: Option<PathBuf>,

    /// The asset that the new sidecar describes. sokf writes the sidecar beside the asset, at PATH.md.
    #[arg(long = "for", value_name = "PATH")]
    pub for_asset: Option<PathBuf>,

    /// Write the file at its derived path and report `created PATH`, where PATH is that path.
    #[arg(long, conflicts_with_all = ["json", "id_only"])]
    pub write: bool,

    /// Print one JSON object with the id, the path, and the document, instead of the document alone. `--json-schema` prints its schema.
    #[arg(long, conflicts_with = "id_only")]
    pub json: bool,

    /// Print the JSON Schema of the `--json` output, and exit.
    #[arg(long, exclusive = true)]
    pub json_schema: bool,

    /// Print the id and nothing else.
    #[arg(long = "id-only")]
    pub id_only: bool,
}
// sokf:end new

// sokf:begin parse-state
/// `parse_state` splits `KEY=STATE` at the first `=`, and refuses an empty key or state.
fn parse_state(s: &str) -> Result<(String, String), String> {
    match s.split_once('=') {
        Some((k, v)) if !k.is_empty() && !v.is_empty() => Ok((k.to_string(), v.to_string())),
        _ => Err(format!("`{s}` is not in the form KEY=STATE")),
    }
}
// sokf:end parse-state

/// `CompletionsArgs` holds the arguments of `completions`.
#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// The shell to print a completion script for.
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

#[cfg(test)]
mod tests;
