use anyhow::{Result, ensure};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::json;
use std::{
    io::{self, Read, Write},
    path::PathBuf,
};
use uup_cli::{
    NoMatch,
    archive::{Index, Limits, scan},
    extract::{Extraction, extract},
    mutate,
    pack::{self, PackOptions},
    query::{self, Part, Selector},
    write,
};

#[derive(Parser)]
#[command(
    name = "uup-cli",
    version,
    about = "ununitypackage-cli: inspect, extract, edit and create Unity packages"
)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global=true, default_value_t=64_u64 << 30)]
    max_expanded_bytes: u64,
    #[arg(long, global=true, default_value_t=8_u64 << 30)]
    max_entry_bytes: u64,
    #[arg(long, global = true, default_value_t = 1_000_000)]
    max_entries: usize,
    #[command(subcommand)]
    command: Command,
}
#[derive(Args)]
#[group(id = "selector", required = true, multiple = false)]
struct Select {
    #[arg(long)]
    path: Option<String>,
    #[arg(long)]
    guid: Option<String>,
    #[arg(long)]
    entry: Option<String>,
}
impl Select {
    fn selector(&self, part: Part) -> Selector {
        Selector {
            path: self.path.clone(),
            guid: self.guid.clone(),
            entry: self.entry.clone(),
            part,
        }
    }
}
#[derive(Args)]
struct Destination {
    #[arg(short, long)]
    output: PathBuf,
    #[arg(long)]
    force: bool,
}
#[derive(ValueEnum, Clone)]
enum MetadataKind {
    Manifest,
    Icon,
    Cover,
}
impl MetadataKind {
    fn name(&self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::Icon => "icon",
            Self::Cover => "cover",
        }
    }
}
#[derive(Subcommand)]
enum MetadataCommand {
    Get {
        kind: MetadataKind,
    },
    Set {
        kind: MetadataKind,
        #[arg(long)]
        file: PathBuf,
        #[command(flatten)]
        destination: Destination,
    },
    Remove {
        kind: MetadataKind,
        #[command(flatten)]
        destination: Destination,
    },
}
#[derive(ValueEnum, Clone)]
enum Layout {
    Packages,
    Assets,
}
#[derive(Subcommand)]
enum Command {
    /// Show package totals and diagnostics.
    Info { package: PathBuf },
    /// List resources and special/unknown content.
    List {
        package: PathBuf,
        #[arg(long)]
        raw: bool,
    },
    /// Find paths by substring, glob or regular expression.
    Find {
        package: PathBuf,
        pattern: String,
        #[arg(long, conflicts_with = "regex")]
        glob: bool,
        #[arg(long)]
        regex: bool,
        #[arg(long)]
        raw: bool,
    },
    /// Stream one asset, meta, preview or raw entry to stdout.
    Cat {
        package: PathBuf,
        #[command(flatten)]
        select: Select,
        #[arg(long, value_enum, default_value = "asset")]
        part: Part,
    },
    /// Show entry details and a bounded content preview.
    Show {
        package: PathBuf,
        #[command(flatten)]
        select: Select,
        #[arg(long, value_enum, default_value = "asset")]
        part: Part,
        #[arg(long)]
        hex: bool,
        #[arg(long, default_value_t = 4096)]
        limit: usize,
    },
    /// Validate compression, tar records and resource consistency.
    Verify { package: PathBuf },
    /// Extract all content or selected resources/raw entries.
    Extract {
        package: PathBuf,
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
        #[arg(long)]
        path: Vec<String>,
        #[arg(long)]
        guid: Vec<String>,
        #[arg(long)]
        glob: Vec<String>,
        #[arg(long)]
        entry: Vec<String>,
        #[arg(long, conflicts_with = "guid")]
        raw: bool,
        #[arg(long)]
        no_meta: bool,
    },
    /// Build a Unity package from a directory.
    #[command(visible_alias = "build")]
    Pack {
        directory: PathBuf,
        output: PathBuf,
        #[arg(long, default_value = "Assets")]
        prefix: String,
        #[arg(long)]
        generate_meta: bool,
        #[arg(long)]
        exclude: Vec<String>,
        #[arg(long)]
        force: bool,
    },
    /// Recompress a package while preserving its contents.
    Repack {
        package: PathBuf,
        #[command(flatten)]
        destination: Destination,
    },
    /// Add a resource or raw entry at an explicit location.
    Add {
        package: PathBuf,
        file: PathBuf,
        #[arg(long, required_unless_present = "entry", conflicts_with = "entry")]
        path: Option<String>,
        #[arg(long)]
        entry: Option<String>,
        #[arg(long)]
        meta: Option<PathBuf>,
        #[arg(long)]
        generate_meta: bool,
        #[command(flatten)]
        destination: Destination,
    },
    /// Replace content, preserving resource identity by default.
    Replace {
        package: PathBuf,
        file: PathBuf,
        #[command(flatten)]
        select: Select,
        #[arg(long, value_enum, default_value = "asset")]
        part: Part,
        #[command(flatten)]
        destination: Destination,
    },
    /// Delete a resource and its siblings, or an explicit raw entry.
    Remove {
        package: PathBuf,
        #[command(flatten)]
        select: Select,
        #[arg(long)]
        recursive: bool,
        #[command(flatten)]
        destination: Destination,
    },
    /// Read, set or delete manifest/icon/cover metadata.
    Metadata {
        package: PathBuf,
        #[command(subcommand)]
        action: MetadataCommand,
    },
    /// Convert a package.json directory to a Unity package.
    FromUpm {
        directory: PathBuf,
        output: PathBuf,
        #[arg(long, value_enum, default_value = "packages")]
        layout: Layout,
        #[arg(long)]
        prefix: Option<String>,
        #[arg(long)]
        generate_meta: bool,
        #[arg(long)]
        exclude: Vec<String>,
        #[arg(long)]
        force: bool,
    },
}
fn emit(value: &impl serde::Serialize) -> Result<()> {
    let mut out = io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, value)?;
    writeln!(out)?;
    Ok(())
}
fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            let code = if e.is::<NoMatch>() { 3 } else { 1 };
            if cli.json {
                eprintln!("{}", json!({"error":format!("{e:#}"), "exit_code":code}));
            } else {
                eprintln!("error: {e:#}");
            }
            std::process::ExitCode::from(code)
        }
    }
}
fn run(cli: &Cli) -> Result<()> {
    let limits = Limits {
        entries: cli.max_entries,
        entry_bytes: cli.max_entry_bytes,
        expanded_bytes: cli.max_expanded_bytes,
    };
    let open = |path: &PathBuf| -> Result<Index> {
        let idx = Index::open(path, limits)?;
        if !cli.json {
            for d in &idx.diagnostics {
                eprintln!("{}: {}", d.severity, d.message);
            }
        }
        Ok(idx)
    };
    match &cli.command {
        Command::Info { package } => {
            let idx = open(package)?;
            let summary = json!({"package":package,"entries":idx.entries.len(),"resources":idx.resources.len(),
                "payload_bytes":idx.entries.iter().map(|e| e.size).sum::<u64>(),"diagnostics":idx.diagnostics});
            if cli.json {
                emit(&summary)?;
            } else {
                println!(
                    "{} resources, {} tar entries, {} payload bytes",
                    idx.resources.len(),
                    idx.entries.len(),
                    summary["payload_bytes"]
                );
            }
        }
        Command::List { package, raw } => {
            let idx = open(package)?;
            print_items(&query::items(&idx, *raw), cli.json)?;
        }
        Command::Find {
            package,
            pattern,
            glob,
            regex,
            raw,
        } => {
            let idx = open(package)?;
            let glob = if *glob {
                Some(globset::Glob::new(pattern)?.compile_matcher())
            } else {
                None
            };
            let regex = if *regex {
                Some(regex::Regex::new(pattern)?)
            } else {
                None
            };
            let items: Vec<_> = query::items(&idx, *raw)
                .into_iter()
                .filter(|item| {
                    if let Some(g) = &glob {
                        g.is_match(&item.path)
                    } else if let Some(r) = &regex {
                        r.is_match(&item.path)
                    } else {
                        item.path.contains(pattern)
                    }
                })
                .collect();
            print_items(&items, cli.json)?;
            if items.is_empty() {
                return Err(NoMatch("no matching files".into()).into());
            }
        }
        Command::Cat {
            package,
            select,
            part,
        } => {
            let idx = open(package)?;
            query::copy_entry(
                &idx,
                select.selector(*part).resolve(&idx)?,
                &mut io::stdout().lock(),
            )?;
        }
        Command::Show {
            package,
            select,
            part,
            hex,
            limit,
        } => {
            ensure!(*limit <= 16 << 20, "preview limit exceeds 16 MiB");
            let idx = open(package)?;
            let id = select.selector(*part).resolve(&idx)?;
            let mut bytes = Vec::new();
            idx.check_unchanged()?;
            scan(&idx.source, idx.limits, |current, entry| {
                if current == id {
                    entry.take(*limit as u64).read_to_end(&mut bytes)?;
                }
                Ok(())
            })?;
            let preview = if *hex {
                bytes
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                String::from_utf8_lossy(&bytes)
                    .chars()
                    .flat_map(|c| {
                        if c.is_control() && !matches!(c, '\n' | '\r' | '\t') {
                            c.escape_default().collect::<Vec<_>>()
                        } else {
                            vec![c]
                        }
                    })
                    .collect()
            };
            if cli.json {
                emit(
                    &json!({"entry":idx.entries[id],"preview":preview,"truncated":idx.entries[id].size>bytes.len() as u64}),
                )?;
            } else {
                println!(
                    "{} ({} bytes)\n{}",
                    idx.entries[id].path, idx.entries[id].size, preview
                );
            }
        }
        Command::Verify { package } => {
            let idx = open(package)?;
            idx.ensure_valid()?;
            if cli.json {
                emit(
                    &json!({"valid":true,"entries":idx.entries.len(),"resources":idx.resources.len(),"diagnostics":idx.diagnostics}),
                )?;
            } else {
                println!(
                    "valid: {} resources, {} entries",
                    idx.resources.len(),
                    idx.entries.len()
                );
            }
        }
        Command::Extract {
            package,
            output,
            path,
            guid,
            glob,
            entry,
            raw,
            no_meta,
        } => {
            let idx = open(package)?;
            let count = extract(
                &idx,
                output,
                &Extraction {
                    paths: path.clone(),
                    guids: guid.clone(),
                    globs: glob.clone(),
                    entries: entry.clone(),
                    raw: *raw,
                    no_meta: *no_meta,
                },
            )?;
            if cli.json {
                emit(&json!({"extracted":count,"output":output}))?;
            } else {
                eprintln!("extracted {count} entries to {}", output.display());
            }
        }
        Command::Pack {
            directory,
            output,
            prefix,
            generate_meta,
            exclude,
            force,
        } => {
            let count = pack::pack(
                directory,
                output,
                &PackOptions {
                    prefix: prefix.clone(),
                    generate_meta: *generate_meta,
                    exclude: exclude.clone(),
                    force: *force,
                },
                limits,
            )?;
            if cli.json {
                emit(&json!({"output":output,"resources":count}))?;
            } else {
                eprintln!("packed {count} resources to {}", output.display());
            }
        }
        Command::Repack {
            package,
            destination,
        } => {
            let idx = open(package)?;
            write::rewrite(
                &idx,
                &destination.output,
                destination.force,
                &Default::default(),
                &[],
            )?;
        }
        Command::Add {
            package,
            file,
            path,
            entry,
            meta,
            generate_meta,
            destination,
        } => {
            let idx = open(package)?;
            mutate::add(
                &idx,
                file,
                &Selector {
                    path: path.clone(),
                    entry: entry.clone(),
                    ..Default::default()
                },
                meta.as_deref(),
                *generate_meta,
                &destination.output,
                destination.force,
            )?;
        }
        Command::Replace {
            package,
            file,
            select,
            part,
            destination,
        } => {
            let idx = open(package)?;
            mutate::replace(
                &idx,
                file,
                &select.selector(*part),
                &destination.output,
                destination.force,
            )?;
        }
        Command::Remove {
            package,
            select,
            recursive,
            destination,
        } => {
            let idx = open(package)?;
            mutate::remove(
                &idx,
                &select.selector(Part::Asset),
                *recursive,
                &destination.output,
                destination.force,
            )?;
        }
        Command::Metadata { package, action } => {
            let idx = open(package)?;
            match action {
                MetadataCommand::Get { kind } => {
                    mutate::metadata_get(&idx, kind.name(), &mut io::stdout().lock())?
                }
                MetadataCommand::Set {
                    kind,
                    file,
                    destination,
                } => mutate::metadata_set(
                    &idx,
                    kind.name(),
                    file,
                    &destination.output,
                    destination.force,
                )?,
                MetadataCommand::Remove { kind, destination } => mutate::remove(
                    &idx,
                    &Selector {
                        entry: Some(mutate::metadata_path(kind.name())?.into()),
                        ..Default::default()
                    },
                    false,
                    &destination.output,
                    destination.force,
                )?,
            }
        }
        Command::FromUpm {
            directory,
            output,
            layout,
            prefix,
            generate_meta,
            exclude,
            force,
        } => {
            let (manifest, mut warnings) = pack::upm_manifest(directory)?;
            let prefix = prefix.clone().unwrap_or_else(|| {
                format!(
                    "{}/{}",
                    match layout {
                        Layout::Packages => "Packages",
                        Layout::Assets => "Assets",
                    },
                    manifest["name"].as_str().unwrap()
                )
            });
            if matches!(layout, Layout::Assets) {
                warnings.push("Assets mapping changes package semantics; no automatic source/reference rewriting is performed.".into());
            }
            let count = pack::pack(
                directory,
                output,
                &PackOptions {
                    prefix: prefix.clone(),
                    generate_meta: *generate_meta,
                    exclude: exclude.clone(),
                    force: *force,
                },
                limits,
            )?;
            if cli.json {
                emit(&pack::upm_report(&manifest, &prefix, count, &warnings))?;
            } else {
                for warning in warnings {
                    eprintln!("warning: {warning}");
                }
                eprintln!("converted {count} resources to {}", output.display());
            }
        }
    }
    let destination = match &cli.command {
        Command::Repack { destination, .. }
        | Command::Add { destination, .. }
        | Command::Replace { destination, .. }
        | Command::Remove { destination, .. } => Some(destination),
        Command::Metadata {
            action:
                MetadataCommand::Set { destination, .. } | MetadataCommand::Remove { destination, .. },
            ..
        } => Some(destination),
        _ => None,
    };
    if let Some(destination) = destination {
        if cli.json {
            emit(&json!({"output":destination.output,"success":true}))?;
        } else {
            eprintln!("wrote {}", destination.output.display());
        }
    }
    Ok(())
}
fn print_items(items: &[query::Item], json: bool) -> Result<()> {
    if json {
        emit(&items)?;
    } else {
        let mut out = io::stdout().lock();
        for item in items {
            writeln!(out, "{}\t{}\t{}", item.kind, item.size, item.path)?;
        }
    }
    Ok(())
}
