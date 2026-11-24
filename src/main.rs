use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use entscript_converter::{compile_to_json, compile_with_project_json};
use serde_json::{Map, Value};

const HELP: &str = r#"Entscript Compiler

Usage:
  entscript build [OPTIONS]
  entscript <SOURCE> [OPTIONS]

Build command (reads ./settings.json):
  -C, --project-dir <DIR>  Project directory (default: current directory)
      --settings <FILE>    Settings file (default: settings.json, relative to project dir)
  -i, --entry <FILE>       Override build.entry
  -o, --output <FILE>      Override build.output
      --name <NAME>        Override metadata.name
  -p, --project <FILE>     Existing Entry project JSON to update

Legacy direct compile:
  <SOURCE>                 Entscript source file, or - to read from stdin
  -o, --output <FILE>      Output Entry package path (default: source name with .ent)
                           Use - for a gzip-compressed tar archive on stdout
  -p, --project <FILE>     Existing Entry project JSON to update

Common options:
  -h, --help               Print this help message
  -V, --version            Print version information

Package format:
  <FILE>.ent is a gzip-compressed tar archive containing temp/project.json and assets

Examples:
  entscript build
  entscript build --name "My Game" -o dist/game.ent
  entscript build --project-dir ./project
  entscript hellos.ents
  entscript hellos.ents -o output.ent
  entscript hellos.ents --project project.json -o updated.ent
"#;

#[derive(Debug)]
struct CliArgs {
    source: PathBuf,
    output: Option<PathBuf>,
    project: Option<PathBuf>,
}

#[derive(Debug)]
struct BuildArgs {
    project_dir: PathBuf,
    settings: PathBuf,
    entry: Option<PathBuf>,
    output: Option<PathBuf>,
    name: Option<String>,
    project: Option<PathBuf>,
}

enum CommandKind {
    Help,
    Version,
    Build(BuildArgs),
    Compile(CliArgs),
}

#[derive(Debug, Clone)]
struct BuildSettings {
    metadata: MetadataSettings,
    entry: PathBuf,
    output: Option<PathBuf>,
}

#[derive(Debug, Clone, Default)]
struct MetadataSettings {
    name: Option<String>,
    description: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    // None = not supplied; Some(None) = explicitly set to null.
    thumbnail: Option<Option<String>>,
}

impl Default for BuildSettings {
    fn default() -> Self {
        Self {
            metadata: MetadataSettings::default(),
            entry: PathBuf::from("main.ents"),
            output: None,
        }
    }
}

fn main() {
    match parse_args(env::args_os().skip(1)) {
        Ok(CommandKind::Help) => {
            print!("{HELP}");
        }
        Ok(CommandKind::Version) => {
            println!("entscript {}", env!("CARGO_PKG_VERSION"));
        }
        Ok(CommandKind::Build(args)) => {
            if let Err(error) = build_project(args) {
                eprintln!("error: {error}");
                process::exit(1);
            }
        }
        Ok(CommandKind::Compile(args)) => {
            if let Err(error) = compile_file(args) {
                eprintln!("error: {error}");
                process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("error: {error}\n\nRun 'entscript --help' for usage.");
            process::exit(2);
        }
    }
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<CommandKind, String> {
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Err("missing command or Entscript source file".to_owned());
    };

    let first_text = first.to_string_lossy().into_owned();
    match first_text.as_str() {
        "-h" | "--help" => Ok(CommandKind::Help),
        "-V" | "--version" => Ok(CommandKind::Version),
        "build" => parse_build_args(args),
        _ => {
            let remaining = std::iter::once(first).chain(args);
            parse_compile_args(remaining)
        }
    }
}

fn parse_build_args(args: impl IntoIterator<Item = OsString>) -> Result<CommandKind, String> {
    let mut args = args.into_iter();
    let mut project_dir = PathBuf::from(".");
    let mut settings = PathBuf::from("settings.json");
    let mut entry = None;
    let mut output = None;
    let mut name = None;
    let mut project = None;

    while let Some(argument) = args.next() {
        let text = argument.to_string_lossy().into_owned();
        match text.as_str() {
            "-h" | "--help" => return Ok(CommandKind::Help),
            "-V" | "--version" => return Ok(CommandKind::Version),
            "-C" | "--project-dir" => {
                project_dir = PathBuf::from(next_value(&mut args, "--project-dir")?);
            }
            "--settings" => {
                settings = PathBuf::from(next_value(&mut args, "--settings")?);
            }
            "-i" | "--entry" => {
                entry = Some(PathBuf::from(next_value(&mut args, "--entry")?));
            }
            "-o" | "--output" => {
                output = Some(PathBuf::from(next_value(&mut args, "--output")?));
            }
            "--name" => {
                name = Some(next_value(&mut args, "--name")?.to_string_lossy().into_owned());
            }
            "-p" | "--project" => {
                project = Some(PathBuf::from(next_value(&mut args, "--project")?));
            }
            _ if text.starts_with('-') => return Err(format!("unknown build option '{text}'")),
            _ => return Err(format!("unexpected build argument '{text}'")),
        }
    }

    Ok(CommandKind::Build(BuildArgs {
        project_dir,
        settings,
        entry,
        output,
        name,
        project,
    }))
}

fn parse_compile_args(args: impl IntoIterator<Item = OsString>) -> Result<CommandKind, String> {
    let mut args = args.into_iter();
    let mut source: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut project: Option<PathBuf> = None;
    let mut positional_only = false;

    while let Some(argument) = args.next() {
        let argument_text = argument.to_string_lossy().into_owned();

        if !positional_only {
            match argument_text.as_str() {
                "--" => {
                    positional_only = true;
                    continue;
                }
                "-h" | "--help" => return Ok(CommandKind::Help),
                "-V" | "--version" => return Ok(CommandKind::Version),
                "-o" | "--output" => {
                    output = Some(PathBuf::from(next_value(&mut args, "--output")?));
                    continue;
                }
                "-p" | "--project" => {
                    project = Some(PathBuf::from(next_value(&mut args, "--project")?));
                    continue;
                }
                _ if argument_text.starts_with('-') && argument_text != "-" => {
                    return Err(format!("unknown option '{argument_text}'"));
                }
                _ => {}
            }
        }

        if source.is_some() {
            return Err(format!("unexpected extra argument '{}'", argument_text));
        }
        source = Some(PathBuf::from(argument));
    }

    let source = source.ok_or_else(|| "missing Entscript source file".to_owned())?;

    Ok(CommandKind::Compile(CliArgs {
        source,
        output,
        project,
    }))
}

fn next_value(
    args: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> Result<OsString, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {option}"))
}

/// Build from the project directory's settings.json. All relative paths in
/// settings.json and build-specific CLI options are resolved from project_dir.
fn build_project(args: BuildArgs) -> Result<(), Box<dyn Error>> {
    let project_dir = make_absolute(&args.project_dir)?;
    let settings_path = resolve_from(&project_dir, &args.settings);
    let mut settings = load_settings(&settings_path)?;

    if let Some(entry) = args.entry {
        settings.entry = entry;
    }
    if let Some(name) = args.name {
        settings.metadata.name = Some(name);
    }
    if settings.entry == PathBuf::from("-") {
        return Err(invalid_settings("build.entry must be a file path; use direct compile mode for stdin").into());
    }

    let source_path = resolve_from(&project_dir, &settings.entry);
    let source_text = fs::read_to_string(&source_path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to read source '{}': {error}", source_path.display()),
        )
    })?;

    let output_json = if let Some(project_path) = args.project {
        let project_path = resolve_from(&project_dir, &project_path);
        let project_json = fs::read_to_string(&project_path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to read Entry project '{}': {error}", project_path.display()),
            )
        })?;
        compile_with_project_json(&source_text, &project_json)?
    } else {
        compile_to_json(&source_text)?
    };

    let output_json = apply_metadata(&output_json, &settings.metadata)?;
    let output_path = args
        .output
        .or(settings.output)
        .map(|path| {
            if path == PathBuf::from("-") {
                path
            } else {
                resolve_from(&project_dir, &path)
            }
        })
        .unwrap_or_else(|| default_output_path(&source_path));

    let assets_dir = project_dir.join("assets");
    if output_path == PathBuf::from("-") {
        write_archive_to_stdout(&output_json, Some(&assets_dir))?;
    } else {
        write_archive_file(&output_path, &output_json, Some(&assets_dir))?;
        eprintln!("Created {}", output_path.display());
    }

    Ok(())
}

fn compile_file(args: CliArgs) -> Result<(), Box<dyn Error>> {
    let source_text = if args.source == PathBuf::from("-") {
        let mut source = String::new();
        io::stdin().read_to_string(&mut source)?;
        source
    } else {
        fs::read_to_string(&args.source).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to read '{}': {error}", args.source.display()),
            )
        })?
    };

    let output_json = if let Some(project_path) = &args.project {
        let project_json = fs::read_to_string(project_path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to read project '{}': {error}", project_path.display()),
            )
        })?;
        compile_with_project_json(&source_text, &project_json)?
    } else {
        compile_to_json(&source_text)?
    };

    let output_path = args.output.unwrap_or_else(|| {
        if args.source == PathBuf::from("-") {
            PathBuf::from("-")
        } else {
            default_output_path(&args.source)
        }
    });

    let assets_dir = env::current_dir()?.join("assets");
    if output_path == PathBuf::from("-") {
        write_archive_to_stdout(&output_json, Some(&assets_dir))?;
    } else {
        write_archive_file(&output_path, &output_json, Some(&assets_dir))?;
        eprintln!("Created {}", output_path.display());
    }

    Ok(())
}

fn load_settings(path: &Path) -> Result<BuildSettings, Box<dyn Error>> {
    match fs::read_to_string(path) {
        Ok(contents) => parse_settings(&contents).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("invalid settings file '{}': {error}", path.display()),
            )
            .into()
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            eprintln!(
                "warning: '{}' not found; using default build settings (entry: main.ents)",
                path.display()
            );
            Ok(BuildSettings::default())
        }
        Err(error) => Err(io::Error::new(
            error.kind(),
            format!("failed to read settings '{}': {error}", path.display()),
        )
        .into()),
    }
}

fn parse_settings(contents: &str) -> io::Result<BuildSettings> {
    let root: Value = serde_json::from_str(contents)
        .map_err(|error| invalid_settings(format!("JSON parse error: {error}")))?;
    let root = root
        .as_object()
        .ok_or_else(|| invalid_settings("root must be a JSON object"))?;

    warn_unknown_fields(root, &["version", "metadata", "build"], "settings");

    let version = root
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_settings("'version' must be an integer"))?;
    if version != 1 {
        return Err(invalid_settings(format!(
            "unsupported settings version {version}; this compiler supports version 1"
        )));
    }

    let mut settings = BuildSettings::default();

    if let Some(metadata) = root.get("metadata") {
        let metadata = metadata
            .as_object()
            .ok_or_else(|| invalid_settings("'metadata' must be an object"))?;
        warn_unknown_fields(
            metadata,
            &["name", "description", "category", "tags", "thumbnail"],
            "metadata",
        );

        settings.metadata.name = optional_string(metadata, "name", "metadata.name")?;
        settings.metadata.description =
            optional_string(metadata, "description", "metadata.description")?;
        settings.metadata.category =
            optional_string(metadata, "category", "metadata.category")?;

        if let Some(value) = metadata.get("tags") {
            let values = value
                .as_array()
                .ok_or_else(|| invalid_settings("'metadata.tags' must be an array of strings"))?;
            let mut tags = Vec::with_capacity(values.len());
            for (index, value) in values.iter().enumerate() {
                let tag = value.as_str().ok_or_else(|| {
                    invalid_settings(format!("'metadata.tags[{index}]' must be a string"))
                })?;
                tags.push(tag.to_owned());
            }
            settings.metadata.tags = Some(tags);
        }

        if let Some(value) = metadata.get("thumbnail") {
            settings.metadata.thumbnail = Some(match value {
                Value::Null => None,
                Value::String(path) => Some(path.clone()),
                _ => {
                    return Err(invalid_settings(
                        "'metadata.thumbnail' must be a string or null",
                    ));
                }
            });
        }
    }

    if let Some(build) = root.get("build") {
        let build = build
            .as_object()
            .ok_or_else(|| invalid_settings("'build' must be an object"))?;
        warn_unknown_fields(build, &["entry", "output"], "build");

        if let Some(entry) = optional_string(build, "entry", "build.entry")? {
            if entry.is_empty() {
                return Err(invalid_settings("'build.entry' cannot be empty"));
            }
            if entry == "-" {
                return Err(invalid_settings("'build.entry' must be a file path, not '-'"));
            }
            settings.entry = PathBuf::from(entry);
        }
        if let Some(output) = optional_string(build, "output", "build.output")? {
            if output.is_empty() {
                return Err(invalid_settings("'build.output' cannot be empty"));
            }
            settings.output = Some(PathBuf::from(output));
        }
    }

    Ok(settings)
}

fn optional_string(
    object: &Map<String, Value>,
    key: &str,
    full_name: &str,
) -> io::Result<Option<String>> {
    match object.get(key) {
        None => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(invalid_settings(format!("'{full_name}' must be a string"))),
    }
}

fn warn_unknown_fields(object: &Map<String, Value>, allowed: &[&str], parent: &str) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            eprintln!("warning: unknown {parent} field '{key}' (ignored)");
        }
    }
}

fn invalid_settings(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// Apply supplied metadata only. Unspecified keys keep the compiler/project defaults.
fn apply_metadata(project_json: &str, metadata: &MetadataSettings) -> Result<String, Box<dyn Error>> {
    let mut project: Value = serde_json::from_str(project_json)?;
    let fields = project.as_object_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "compiled Entry project JSON root must be an object",
        )
    })?;

    if let Some(name) = &metadata.name {
        fields.insert("name".to_owned(), Value::String(name.clone()));
    }
    if let Some(description) = &metadata.description {
        fields.insert("description".to_owned(), Value::String(description.clone()));
    }
    if let Some(category) = &metadata.category {
        fields.insert("category".to_owned(), Value::String(category.clone()));
    }
    if let Some(tags) = &metadata.tags {
        fields.insert(
            "tags".to_owned(),
            Value::Array(tags.iter().cloned().map(Value::String).collect()),
        );
    }
    if let Some(thumbnail) = &metadata.thumbnail {
        fields.insert(
            "thumbnail".to_owned(),
            thumbnail
                .as_ref()
                .map(|value| Value::String(value.clone()))
                .unwrap_or(Value::Null),
        );
    }

    Ok(serde_json::to_string(&project)?)
}

fn resolve_from(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn default_output_path(source: &Path) -> PathBuf {
    let mut output = source.to_path_buf();
    output.set_extension("ent");
    output
}

/// Creates a package whose archive root is `temp/` and whose project file is
/// exactly `temp/project.json`.
fn write_archive_file(
    output_path: &Path,
    project_json: &str,
    assets_dir: Option<&Path>,
) -> io::Result<()> {
    let output_path = make_absolute(output_path)?;
    let output_parent = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(output_parent)?;

    let stage = TempDirectory::create()?;
    write_staged_project(stage.path(), project_json, assets_dir)?;

    // Match the requested workflow: create <target>.tar, then rename it to
    // <target>.ent. The tar file is placed beside the destination so rename
    // stays on the same filesystem.
    let tar_path = intermediate_tar_path(&output_path);
    let _remove_tar_on_drop = RemoveFileOnDrop(tar_path.clone());

    run_tar(stage.path(), &tar_path)?;
    fs::rename(&tar_path, &output_path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "failed to rename '{}' to '{}': {error}",
                tar_path.display(),
                output_path.display()
            ),
        )
    })?;

    Ok(())
}

fn write_archive_to_stdout(project_json: &str, assets_dir: Option<&Path>) -> io::Result<()> {
    let stage = TempDirectory::create()?;
    write_staged_project(stage.path(), project_json, assets_dir)?;

    let status = Command::new("tar")
        .args([OsStr::new("-czf"), OsStr::new("-"), OsStr::new("temp")])
        .current_dir(stage.path())
        .status()
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to run 'tar'; install tar and ensure it is on PATH: {error}"),
            )
        })?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("tar failed with status {status}"),
        ));
    }

    Ok(())
}

fn write_staged_project(
    stage_root: &Path,
    project_json: &str,
    assets_dir: Option<&Path>,
) -> io::Result<()> {
    let package_dir = stage_root.join("temp");
    fs::create_dir(&package_dir)?;
    fs::write(package_dir.join("project.json"), project_json.as_bytes())?;

    if let Some(assets_dir) = assets_dir {
        copy_entry_assets(assets_dir, &package_dir, project_json)?;
    }

    Ok(())
}

/// Copy only the local Entry assets referenced by project JSON into `temp/`.
/// Unused files and unrelated files in `assets/` are deliberately omitted.
/// References are constrained to Entry's hashed asset directory layout.
fn copy_entry_assets(
    source_root: &Path,
    package_root: &Path,
    project_json: &str,
) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(source_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            validate_referenced_entry_assets(source_root, project_json)?;
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("assets path '{}' must be a real directory, not a file or symlink", source_root.display()),
        ));
    }

    let references = entry_asset_references(project_json)?;
    for relative_path in references {
        let source_path = validate_entry_asset_file(source_root, &relative_path)?;
        let destination = package_root.join(&relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source_path, destination)?;
    }

    Ok(())
}

/// Verify that local Entry-managed images, thumbnails, and sounds referenced
/// by project JSON exist before creating the package. Built-in runtime URLs and
/// other paths outside Entry's hashed asset layout are left untouched.
fn validate_referenced_entry_assets(source_root: &Path, project_json: &str) -> io::Result<()> {
    for relative_path in entry_asset_references(project_json)? {
        validate_entry_asset_file(source_root, &relative_path)?;
    }
    Ok(())
}

fn entry_asset_references(project_json: &str) -> io::Result<BTreeSet<PathBuf>> {
    let project: Value = serde_json::from_str(project_json).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("generated project JSON is invalid while checking assets: {error}"),
        )
    })?;
    let mut references = BTreeSet::new();
    collect_entry_asset_references(&project, &mut references);
    Ok(references)
}

fn validate_entry_asset_file(source_root: &Path, relative_path: &Path) -> io::Result<PathBuf> {
    let mut source_path = source_root.to_path_buf();
    let mut metadata = None;
    for component in relative_path.components() {
        source_path.push(component);
        let current_metadata = fs::symlink_metadata(&source_path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "project references asset '{}', but it is missing from '{}'",
                    relative_path.display(),
                    source_root.display()
                ),
            )
        })?;
        if current_metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "project asset '{}' contains a symbolic link",
                    relative_path.display()
                ),
            ));
        }
        metadata = Some(current_metadata);
    }

    let metadata = metadata.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "empty Entry asset path")
    })?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "project asset '{}' must be a regular file inside '{}'",
                relative_path.display(),
                source_root.display()
            ),
        ));
    }

    Ok(source_path)
}

fn collect_entry_asset_references(value: &Value, references: &mut BTreeSet<PathBuf>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_entry_asset_references(item, references);
            }
        }
        Value::Object(fields) => {
            for (key, item) in fields {
                if matches!(key.as_str(), "fileurl" | "thumbUrl") {
                    if let Some(reference) = item.as_str() {
                        if let Some(path) = entry_asset_relative_path(reference) {
                            references.insert(path);
                        }
                    }
                }
                collect_entry_asset_references(item, references);
            }
        }
        _ => {}
    }
}

/// Extract the four safe path components from an Entry asset URL/path:
/// `<2>/<2>/{image|thumb|sound}/<file-id>.<ext>`.
fn entry_asset_relative_path(reference: &str) -> Option<PathBuf> {
    let without_query = reference.split(|character| character == '?' || character == '#')
        .next().unwrap_or(reference);
    let normalized = without_query.replace('\\', "/");
    let parts = normalized.split('/').filter(|part| !part.is_empty()).collect::<Vec<_>>();

    for window in parts.windows(4) {
        let first = window[0];
        let second = window[1];
        let kind = window[2];
        let filename = window[3];
        if first.len() != 2 || second.len() != 2
            || !first.bytes().all(|byte| byte.is_ascii_alphanumeric())
            || !second.bytes().all(|byte| byte.is_ascii_alphanumeric())
            || !matches!(kind, "image" | "thumb" | "sound")
        {
            continue;
        }
        let Some((file_id, extension)) = filename.rsplit_once('.') else { continue };
        if extension.is_empty()
            || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
            || file_id.len() < 4
            || !file_id.bytes().all(|byte| byte.is_ascii_alphanumeric())
            || !file_id[..2].eq_ignore_ascii_case(first)
            || !file_id[2..4].eq_ignore_ascii_case(second)
        {
            continue;
        }
        return Some(PathBuf::from(first).join(second).join(kind).join(filename));
    }

    None
}

fn run_tar(stage_root: &Path, tar_path: &Path) -> io::Result<()> {
    let status = Command::new("tar")
        .arg("-czf")
        .arg(tar_path)
        .arg("temp")
        .current_dir(stage_root)
        .status()
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("failed to run 'tar'; install tar and ensure it is on PATH: {error}"),
            )
        })?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("tar failed with status {status}"),
        ));
    }

    Ok(())
}

fn make_absolute(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

fn intermediate_tar_path(output_path: &Path) -> PathBuf {
    let mut tar_path = output_path.to_path_buf();
    match output_path.extension().and_then(OsStr::to_str) {
        Some(extension) if extension.eq_ignore_ascii_case("ent") => {
            tar_path.set_extension("tar");
        }
        _ => {
            let file_name = output_path
                .file_name()
                .unwrap_or_else(|| OsStr::new("output"))
                .to_string_lossy();
            tar_path.set_file_name(format!("{file_name}.tar"));
        }
    }
    tar_path
}

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn create() -> io::Result<Self> {
        let root = env::temp_dir();

        for attempt in 0..100_u32 {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = root.join(format!(
                "entscript-{}-{timestamp}-{attempt}",
                process::id()
            ));

            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not create a unique Entscript temporary directory",
        ))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct RemoveFileOnDrop(PathBuf);

impl Drop for RemoveFileOnDrop {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_metadata, build_project, default_output_path, intermediate_tar_path, parse_args,
        parse_settings, write_archive_file, BuildArgs, BuildSettings, CommandKind,
        MetadataSettings, TempDirectory,
    };
    use serde_json::Value;
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    #[test]
    fn parses_source_and_output_arguments() {
        let command = parse_args([
            OsString::from("hellos.ents"),
            OsString::from("-o"),
            OsString::from("output.ent"),
        ])
        .unwrap();

        match command {
            CommandKind::Compile(args) => {
                assert_eq!(args.source, PathBuf::from("hellos.ents"));
                assert_eq!(args.output.unwrap(), PathBuf::from("output.ent"));
                assert!(args.project.is_none());
            }
            _ => panic!("expected compile command"),
        }
    }

    #[test]
    fn parses_project_argument() {
        let command = parse_args([
            OsString::from("hellos.ents"),
            OsString::from("--project"),
            OsString::from("project.json"),
        ])
        .unwrap();

        match command {
            CommandKind::Compile(args) => {
                assert_eq!(args.project.unwrap(), PathBuf::from("project.json"));
            }
            _ => panic!("expected compile command"),
        }
    }

    #[test]
    fn parses_build_options() {
        let command = parse_args([
            OsString::from("build"),
            OsString::from("--project-dir"),
            OsString::from("./project"),
            OsString::from("--name"),
            OsString::from("My Game"),
            OsString::from("-o"),
            OsString::from("dist/game.ent"),
        ])
        .unwrap();

        match command {
            CommandKind::Build(args) => {
                assert_eq!(args.project_dir, PathBuf::from("./project"));
                assert_eq!(args.name.as_deref(), Some("My Game"));
                assert_eq!(args.output, Some(PathBuf::from("dist/game.ent")));
            }
            _ => panic!("expected build command"),
        }
    }

    #[test]
    fn parses_settings_and_uses_project_defaults() {
        let settings = parse_settings(
            r#"{
                "version": 1,
                "metadata": { "name": "Hello", "tags": ["a", "b"] },
                "build": { "entry": "src/main.ents" }
            }"#,
        )
        .unwrap();

        assert_eq!(settings.metadata.name.as_deref(), Some("Hello"));
        assert_eq!(
            settings.metadata.tags.as_ref().unwrap(),
            &vec!["a".to_owned(), "b".to_owned()]
        );
        assert_eq!(settings.entry, PathBuf::from("src/main.ents"));
        assert!(settings.output.is_none());
    }

    #[test]
    fn rejects_unsupported_settings_version() {
        let error = parse_settings(r#"{"version": 2}"#).unwrap_err();
        assert!(error.to_string().contains("supports version 1"));
    }

    #[test]
    fn metadata_only_overwrites_values_that_were_supplied() {
        let metadata = MetadataSettings {
            name: Some("New name".to_owned()),
            description: Some("Description".to_owned()),
            category: Some("game".to_owned()),
            tags: Some(vec!["Entscript".to_owned()]),
            thumbnail: Some(None),
        };
        let output = apply_metadata(r#"{"name":"Old name","objects":[],"keep":true}"#, &metadata)
            .unwrap();
        let value: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(value["name"], "New name");
        assert_eq!(value["description"], "Description");
        assert_eq!(value["category"], "game");
        assert_eq!(value["tags"][0], "Entscript");
        assert!(value["thumbnail"].is_null());
        assert_eq!(value["keep"], true);
    }

    #[test]
    fn absent_metadata_preserves_project_json() {
        let input = r#"{"name":"Existing","objects":[]}"#;
        let output = apply_metadata(input, &MetadataSettings::default()).unwrap();
        let before: Value = serde_json::from_str(input).unwrap();
        let after: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn rejects_missing_source() {
        assert!(parse_args(Vec::<OsString>::new()).is_err());
    }

    #[test]
    fn default_output_replaces_ents_with_ent() {
        assert_eq!(
            default_output_path(Path::new("hello.ents")),
            PathBuf::from("hello.ent")
        );
    }

    #[test]
    fn intermediate_archive_uses_tar_extension() {
        assert_eq!(
            intermediate_tar_path(Path::new("hello.ent")),
            PathBuf::from("hello.tar")
        );
    }

    #[test]
    fn creates_ent_archive_with_temp_project_json() {
        let output_dir = TempDirectory::create().unwrap();
        let archive_path = output_dir.path().join("sample.ent");
        let expected_json = r#"{"objects":[]}"#;

        write_archive_file(&archive_path, expected_json, None).unwrap();

        let listing = Command::new("tar")
            .arg("-tzf")
            .arg(&archive_path)
            .output()
            .unwrap();
        assert!(listing.status.success());

        let listing = String::from_utf8(listing.stdout).unwrap();
        assert!(listing.lines().any(|entry| entry.trim_end_matches('/') == "temp"));
        assert!(listing.lines().any(|entry| entry == "temp/project.json"));

        let extract_dir = output_dir.path().join("extract");
        std::fs::create_dir(&extract_dir).unwrap();
        let extract_status = Command::new("tar")
            .arg("-xzf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&extract_dir)
            .status()
            .unwrap();
        assert!(extract_status.success());

        let actual_json = std::fs::read_to_string(extract_dir.join("temp/project.json")).unwrap();
        assert_eq!(actual_json, expected_json);
    }

    #[test]
    fn packages_entry_assets_in_the_expected_directory_layout() {
        let output_dir = TempDirectory::create().unwrap();
        let project_dir = output_dir.path().join("project");
        let assets_dir = project_dir.join("assets");
        let asset_relative = Path::new("ab/cd/image/abcd1234.png");
        let asset_source = assets_dir.join(asset_relative);
        std::fs::create_dir_all(asset_source.parent().unwrap()).unwrap();
        std::fs::write(&asset_source, b"test-image").unwrap();

        let unused_asset = assets_dir.join("ef/gh/sound/efgh5678.mp3");
        std::fs::create_dir_all(unused_asset.parent().unwrap()).unwrap();
        std::fs::write(&unused_asset, b"unused-sound").unwrap();
        std::fs::write(assets_dir.join("thumbnail.png"), b"not-an-entry-asset").unwrap();

        let project_json = r#"{
            "objects": [{
                "sprite": {
                    "pictures": [{
                        "fileurl": "https://cdn.example/ab/cd/image/abcd1234.png"
                    }]
                }
            }]
        }"#;
        let archive_path = output_dir.path().join("assets.ent");
        write_archive_file(&archive_path, project_json, Some(&assets_dir)).unwrap();

        let extract_dir = output_dir.path().join("extract-assets");
        std::fs::create_dir(&extract_dir).unwrap();
        let status = Command::new("tar")
            .arg("-xzf")
            .arg(&archive_path)
            .arg("-C")
            .arg(&extract_dir)
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(
            std::fs::read(extract_dir.join("temp").join(asset_relative)).unwrap(),
            b"test-image"
        );
        assert!(!extract_dir.join("temp/thumbnail.png").exists());
        assert!(!extract_dir.join("temp/ef/gh/sound/efgh5678.mp3").exists());
    }

    #[test]
    fn verifies_image_thumbnail_and_sound_assets_referenced_by_project_json() {
        let output_dir = TempDirectory::create().unwrap();
        let assets_dir = output_dir.path().join("assets");
        let image = Path::new("ab/cd/image/abcd1234.png");
        let thumbnail = Path::new("ef/gh/thumb/efgh5678.png");
        let sound = Path::new("wx/yz/sound/wxyz9012.mp3");

        for path in [image, thumbnail, sound] {
            let file = assets_dir.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, b"asset").unwrap();
        }

        let project_json = r#"{
            "objects": [{
                "sprite": {
                    "pictures": [{
                        "fileurl": "https://cdn.example/ab/cd/image/abcd1234.png",
                        "thumbUrl": "/ef/gh/thumb/efgh5678.png?cache=1"
                    }],
                    "sounds": [{
                        "fileurl": "./wx/yz/sound/wxyz9012.mp3"
                    }]
                }
            }]
        }"#;

        validate_referenced_entry_assets(&assets_dir, project_json).unwrap();
    }

    #[test]
    fn fails_when_a_referenced_entry_asset_is_missing() {
        let output_dir = TempDirectory::create().unwrap();
        let assets_dir = output_dir.path().join("assets");
        fs::create_dir_all(&assets_dir).unwrap();

        let project_json = r#"{
            "objects": [{
                "sprite": {
                    "pictures": [{
                        "fileurl": "/ab/cd/image/abcd1234.png"
                    }]
                }
            }]
        }"#;

        let error = validate_referenced_entry_assets(&assets_dir, project_json).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        assert!(error.to_string().contains("abcd1234.png"));
    }

    #[test]
    fn default_settings_use_main_ents() {
        let settings = BuildSettings::default();
        assert_eq!(settings.entry, PathBuf::from("main.ents"));
    }

    #[test]
    fn build_command_reads_settings_applies_metadata_and_creates_ent() {
        let project_dir = TempDirectory::create().unwrap();
        std::fs::write(
            project_dir.path().join("settings.json"),
            r#"{
                "version": 1,
                "metadata": {
                    "name": "Settings Build Test",
                    "description": "Metadata test",
                    "category": "game",
                    "tags": ["test"],
                    "thumbnail": null
                },
                "build": {
                    "entry": "main.ents",
                    "output": "build/test.ent"
                }
            }"#,
        )
        .unwrap();
        std::fs::write(
            project_dir.path().join("main.ents"),
            r#"@엔트리봇 { when start { say("hello") } }"#,
        )
        .unwrap();

        build_project(BuildArgs {
            project_dir: project_dir.path().to_path_buf(),
            settings: PathBuf::from("settings.json"),
            entry: None,
            output: None,
            name: None,
            project: None,
        })
        .unwrap();

        let archive = project_dir.path().join("build/test.ent");
        assert!(archive.exists());

        let extract_dir = project_dir.path().join("extract");
        std::fs::create_dir(&extract_dir).unwrap();
        let status = Command::new("tar")
            .arg("-xf")
            .arg(&archive)
            .arg("-C")
            .arg(&extract_dir)
            .status()
            .unwrap();
        assert!(status.success());

        let project_json = std::fs::read_to_string(extract_dir.join("temp/project.json")).unwrap();
        let project: Value = serde_json::from_str(&project_json).unwrap();
        assert_eq!(project["name"], "Settings Build Test");
        assert_eq!(project["description"], "Metadata test");
        assert_eq!(project["category"], "game");
        assert_eq!(project["tags"][0], "test");
        assert!(project["thumbnail"].is_null());
    }
}
