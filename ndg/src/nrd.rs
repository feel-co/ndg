use std::{
  cmp::Ordering,
  collections::HashMap,
  env,
  fs,
  path::PathBuf,
  sync::LazyLock,
};

use clap::{Args, Parser, Subcommand};
use color_eyre::eyre::{Context, Result, bail};
use comrak::{Arena, Options, nodes::NodeValue, parse_document};
use regex::Regex;
use serde_json::{Map, Value};

const NRD_BIN_NAMES: &[&str] = &["nrd", "nixos-render-docs"];
const ROLE_START: char = '\u{E000}';
const ROLE_SEP: char = '\u{E001}';
const ROLE_END: char = '\u{E002}';
const PROTECTED_SPACE: &str = "\0p";

static ROFF_UNICODE: LazyLock<Regex> = LazyLock::new(|| {
  #[expect(
    clippy::expect_used,
    reason = "the roff Unicode character-class regex is static and tested"
  )]
  Regex::new(r"[^\n !#$%&()*+,\-./0-9:;<=>?@A-Z\[\\\]_a-z{|}]")
    .expect("roff unicode regex should compile")
});

#[derive(Debug, Parser)]
#[command(
  name = "nixos-render-docs",
  about = "render nixos manual bits",
  disable_version_flag = true
)]
struct NrdCli {
  #[arg(short = 'j', long = "jobs")]
  jobs: Option<usize>,

  #[command(subcommand)]
  command: NrdCommand,
}

#[derive(Debug, Subcommand)]
enum NrdCommand {
  Options(OptionsCommand),
  Manual(ManualCommand),
}

#[derive(Debug, Args)]
struct OptionsCommand {
  #[command(subcommand)]
  format: OptionsFormat,
}

#[derive(Debug, Subcommand)]
enum OptionsFormat {
  Commonmark(CommonMarkArgs),
  Manpage(ManpageArgs),
  Asciidoc(AsciiDocArgs),
}

#[derive(Debug, Args)]
struct CommonMarkArgs {
  #[arg(long = "manpage-urls", required = true)]
  manpage_urls: PathBuf,

  #[arg(long, required = true)]
  revision: String,

  #[arg(long = "anchor-style", default_value = "none", value_parser = ["none", "legacy"])]
  anchor_style: String,

  #[arg(long = "anchor-prefix", default_value = "")]
  anchor_prefix: String,

  infile:  PathBuf,
  outfile: PathBuf,
}

#[derive(Debug, Args)]
struct ManpageArgs {
  #[arg(long, required = true)]
  revision: String,

  #[arg(long)]
  header: Option<PathBuf>,

  #[arg(long)]
  footer: Option<PathBuf>,

  infile:  PathBuf,
  outfile: PathBuf,
}

#[derive(Debug, Args)]
struct AsciiDocArgs {
  #[arg(long = "manpage-urls", required = true)]
  manpage_urls: PathBuf,

  #[arg(long, required = true)]
  revision: String,

  infile:  PathBuf,
  outfile: PathBuf,
}

#[derive(Debug, Args)]
struct ManualCommand {
  #[command(subcommand)]
  format: ManualFormat,
}

#[derive(Debug, Subcommand)]
enum ManualFormat {
  Html(ManualHtmlArgs),
}

#[derive(Debug, Args)]
struct ManualHtmlArgs {
  #[arg(long = "manpage-urls", required = true)]
  manpage_urls: PathBuf,

  #[arg(long, required = true)]
  revision: String,

  #[arg(long, default_value = "nixos-render-docs")]
  generator: String,

  #[arg(long = "stylesheet", action = clap::ArgAction::Append)]
  stylesheet: Vec<String>,

  #[arg(long = "script", action = clap::ArgAction::Append)]
  script: Vec<String>,

  #[arg(long = "toc-depth", default_value_t = 1)]
  toc_depth: usize,

  #[arg(long = "chunk-toc-depth", default_value_t = 1)]
  chunk_toc_depth: usize,

  #[arg(long = "section-toc-depth", default_value_t = 0)]
  section_toc_depth: usize,

  #[arg(long = "media-dir", default_value = "media")]
  media_dir: PathBuf,

  #[arg(long)]
  redirects: Option<PathBuf>,

  infile:  PathBuf,
  outfile: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AnchorStyle {
  None,
  Legacy,
}

#[derive(Debug)]
struct RenderedOption {
  loc:   Vec<String>,
  lines: Vec<String>,
}

#[derive(Debug)]
struct CommonMarkConverter {
  manpage_urls:  HashMap<String, String>,
  revision:      String,
  anchor_style:  AnchorStyle,
  anchor_prefix: String,
  options:       HashMap<String, RenderedOption>,
}

#[derive(Debug)]
struct CommonMarkRenderer {
  parstack:   Vec<ParagraphState>,
  list_stack: Vec<ListState>,
  link_stack: Vec<String>,
}

#[derive(Debug)]
struct ManpageConverter {
  revision:      String,
  header:        Option<Vec<String>>,
  footer:        Option<Vec<String>>,
  options_by_id: HashMap<String, String>,
  options:       HashMap<String, RenderedManpageOption>,
}

#[derive(Debug)]
struct RenderedManpageOption {
  loc:   Vec<String>,
  lines: Vec<String>,
  links: Vec<String>,
}

#[derive(Debug)]
struct ManpageRenderer {
  inline_code_is_quoted: bool,
  link_footnotes:        Option<Vec<String>>,
  href_targets:          HashMap<String, String>,
  link_stack:            Vec<String>,
  do_parbreak_stack:     Vec<bool>,
  list_stack:            Vec<ManpageListState>,
  font_stack:            Vec<&'static str>,
}

#[derive(Clone, Debug)]
struct ManpageListState {
  width:           usize,
  next_idx:        Option<usize>,
  compact:         bool,
  first_item_seen: bool,
}

#[derive(Clone, Debug)]
struct ParagraphState {
  indent:     String,
  continuing: bool,
}

#[derive(Clone, Debug)]
struct ListState {
  next_idx:        Option<usize>,
  compact:         bool,
  first_item_seen: bool,
}

/// Return true when this binary was invoked through the nixos-render-docs
/// compatibility names.
pub fn invoked_as_nrd() -> bool {
  let Some(arg0) = env::args_os().next() else {
    return false;
  };
  let path = PathBuf::from(arg0);
  path
    .file_name()
    .and_then(std::ffi::OsStr::to_str)
    .is_some_and(|name| NRD_BIN_NAMES.contains(&name))
}

pub fn run() -> Result<()> {
  let cli = NrdCli::parse();

  if let Some(jobs) = cli.jobs {
    rayon::ThreadPoolBuilder::new()
      .num_threads(jobs)
      .build_global()
      .wrap_err("failed to configure nrd worker pool")?;
  }

  match cli.command {
    NrdCommand::Options(command) => run_options(command),
    NrdCommand::Manual(command) => run_manual(command),
  }
}

fn run_options(command: OptionsCommand) -> Result<()> {
  match command.format {
    OptionsFormat::Commonmark(args) => run_options_commonmark(args),
    OptionsFormat::Manpage(args) => run_options_manpage(args),
    OptionsFormat::Asciidoc(args) => {
      let _ = (args.manpage_urls, args.revision, args.infile, args.outfile);
      bail!("nixos-render-docs options asciidoc is not implemented yet")
    },
  }
}

fn run_manual(command: ManualCommand) -> Result<()> {
  match command.format {
    ManualFormat::Html(args) => {
      let _ = (
        args.manpage_urls,
        args.revision,
        args.generator,
        args.stylesheet,
        args.script,
        args.toc_depth,
        args.chunk_toc_depth,
        args.section_toc_depth,
        args.media_dir,
        args.redirects,
        args.infile,
        args.outfile,
      );
      bail!("nixos-render-docs manual html is not implemented yet")
    },
  }
}

fn run_options_commonmark(args: CommonMarkArgs) -> Result<()> {
  let manpage_urls = read_string_map(&args.manpage_urls)?;
  let options = read_options(&args.infile)?;
  let anchor_style = parse_anchor_style(&args.anchor_style)?;

  let mut converter = CommonMarkConverter::new(
    manpage_urls,
    args.revision,
    anchor_style,
    args.anchor_prefix,
  );
  converter.add_options(&options)?;
  fs::write(&args.outfile, converter.finalize()).wrap_err_with(|| {
    format!(
      "failed to write commonmark output to {}",
      args.outfile.display()
    )
  })
}

fn run_options_manpage(args: ManpageArgs) -> Result<()> {
  let options = read_options(&args.infile)?;
  let mut converter = ManpageConverter::new(
    args.revision,
    read_optional_lines(args.header.as_ref())?,
    read_optional_lines(args.footer.as_ref())?,
  );
  converter.add_options(&options)?;
  fs::write(&args.outfile, converter.finalize()?).wrap_err_with(|| {
    format!(
      "failed to write manpage output to {}",
      args.outfile.display()
    )
  })
}

fn read_optional_text(path: Option<&PathBuf>) -> Result<Option<String>> {
  path
    .map(|path| {
      fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read {}", path.display()))
    })
    .transpose()
}

fn read_optional_lines(path: Option<&PathBuf>) -> Result<Option<Vec<String>>> {
  read_optional_text(path).map(|content| {
    content.map(|content| content.lines().map(str::to_string).collect())
  })
}

fn read_options(path: &PathBuf) -> Result<Map<String, Value>> {
  let content = fs::read_to_string(path)
    .wrap_err_with(|| format!("failed to read {}", path.display()))?;
  let value: Value = serde_json::from_str(&content)
    .wrap_err_with(|| format!("failed to parse {}", path.display()))?;
  match value {
    Value::Object(options) => Ok(options),
    _ => bail!("options input must be a JSON object"),
  }
}

fn read_string_map(path: &PathBuf) -> Result<HashMap<String, String>> {
  let content = fs::read_to_string(path)
    .wrap_err_with(|| format!("failed to read {}", path.display()))?;
  serde_json::from_str(&content)
    .wrap_err_with(|| format!("failed to parse {}", path.display()))
}

fn parse_anchor_style(value: &str) -> Result<AnchorStyle> {
  match value {
    "none" => Ok(AnchorStyle::None),
    "legacy" => Ok(AnchorStyle::Legacy),
    _ => bail!("invalid anchor style: {value}"),
  }
}

impl CommonMarkConverter {
  fn new(
    manpage_urls: HashMap<String, String>,
    revision: String,
    anchor_style: AnchorStyle,
    anchor_prefix: String,
  ) -> Self {
    Self {
      manpage_urls,
      revision,
      anchor_style,
      anchor_prefix,
      options: HashMap::new(),
    }
  }

  fn add_options(&mut self, options: &Map<String, Value>) -> Result<()> {
    for (name, option) in options {
      let rendered = self
        .render_option(name, option)
        .wrap_err_with(|| format!("failed to render option {name}"))?;
      self.options.insert(name.clone(), rendered);
    }
    Ok(())
  }

  fn render_option(
    &self,
    name: &str,
    option: &Value,
  ) -> Result<RenderedOption> {
    let Value::Object(option) = option else {
      bail!("option value must be a JSON object")
    };
    let mut renderer = CommonMarkRenderer::new();
    let loc = option_loc(name, option);
    Ok(RenderedOption {
      loc,
      lines: self.convert_one(option, &mut renderer)?,
    })
  }

  fn convert_one(
    &self,
    option: &Map<String, Value>,
    renderer: &mut CommonMarkRenderer,
  ) -> Result<Vec<String>> {
    let mut blocks = Vec::new();

    if let Some(description) = option.get("description") {
      let rendered = self.render_description(description, renderer)?;
      if !rendered.is_empty() {
        blocks.push(vec![rendered]);
      }
    }

    if let Some(Value::String(type_name)) = option.get("type") {
      let read_only = if option
        .get("readOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false)
      {
        " *(read only)*"
      } else {
        ""
      };
      blocks.push(vec![self.render_markdown(
        &format!("*Type:*\n{}{read_only}", md_escape(type_name)),
        renderer,
      )?]);
    }

    if option.get("default").is_some_and(json_truthy) {
      blocks.push(self.render_code(option, "default", renderer)?);
    }
    if option.get("example").is_some_and(json_truthy) {
      blocks.push(self.render_code(option, "example", renderer)?);
    }

    if let Some(Value::String(related)) = option.get("relatedPackages")
      && !related.is_empty()
    {
      blocks.push(vec![
        "*Related packages:*".to_string(),
        self.render_markdown(related, renderer)?,
      ]);
    }
    if let Some(Value::Array(declarations)) = option.get("declarations")
      && !declarations.is_empty()
    {
      blocks.push(self.render_decl_def("Declared by", declarations));
    }
    if let Some(Value::Array(definitions)) = option.get("definitions")
      && !definitions.is_empty()
    {
      blocks.push(self.render_decl_def("Defined by", definitions));
    }

    let last_non_empty = blocks.iter().rposition(|block| !block.is_empty());
    if let Some(last) = last_non_empty {
      for block in &mut blocks[..last] {
        if !block.is_empty() {
          block.push(String::new());
        }
      }
    }

    Ok(blocks.into_iter().flatten().collect())
  }

  fn render_description(
    &self,
    description: &Value,
    renderer: &mut CommonMarkRenderer,
  ) -> Result<String> {
    match description {
      Value::String(description) => self.render_markdown(description, renderer),
      Value::Object(object)
        if object.get("_type").and_then(Value::as_str) == Some("mdDoc") =>
      {
        object.get("text").and_then(Value::as_str).map_or_else(
          || Ok(String::new()),
          |text| self.render_markdown(text, renderer),
        )
      },
      _ => bail!("description has unrecognized type"),
    }
  }

  fn render_code(
    &self,
    option: &Map<String, Value>,
    key: &str,
    renderer: &mut CommonMarkRenderer,
  ) -> Result<Vec<String>> {
    match option.get(key) {
      Some(Value::Object(object))
        if object.get("_type").and_then(Value::as_str) == Some("literalMD") =>
      {
        let text = object.get("text").and_then(Value::as_str).unwrap_or("");
        Ok(vec![self.render_markdown(
          &format!("*{}:*\n{text}", capitalize(key)),
          renderer,
        )?])
      },
      Some(Value::Object(object))
        if object.get("_type").and_then(Value::as_str)
          == Some("literalExpression") =>
      {
        let text = object.get("text").and_then(Value::as_str).unwrap_or("");
        let code = md_make_code(text, "", None);
        Ok(vec![self.render_markdown(
          &format!("*{}:*\n{code}", capitalize(key)),
          renderer,
        )?])
      },
      Some(_) => bail!("{key} has unrecognized type"),
      None => Ok(Vec::new()),
    }
  }

  fn render_decl_def(&self, header: &str, locations: &[Value]) -> Vec<String> {
    let mut lines = vec![format!("*{header}:*")];
    for location in locations {
      let (href, name) = self.format_decl_def_loc(location);
      let escaped_name = md_escape(&name);
      if let Some(href) = href {
        lines.push(format!(" - [{escaped_name}]({href})"));
      } else {
        lines.push(format!(" - {escaped_name}"));
      }
    }
    lines
  }

  fn format_decl_def_loc(&self, location: &Value) -> (Option<String>, String) {
    match location {
      Value::String(location) => {
        if location.starts_with('/') {
          let name =
            if location.contains("nixops") && location.contains("/nix/") {
              let suffix = location
                .find("/nix/")
                .map_or(location.as_str(), |index| &location[index + 5..]);
              format!("<nixops/{suffix}>")
            } else {
              location.clone()
            };
          (Some(format!("file://{location}")), name)
        } else {
          let href = if self.revision == "local" {
            format!("https://github.com/NixOS/nixpkgs/blob/master/{location}")
          } else {
            format!(
              "https://github.com/NixOS/nixpkgs/blob/{}/{location}",
              self.revision
            )
          };
          (Some(href), format!("<nixpkgs/{location}>"))
        }
      },
      Value::Object(object) => {
        let href = object
          .get("url")
          .and_then(Value::as_str)
          .map(ToOwned::to_owned);
        let name = object
          .get("name")
          .and_then(Value::as_str)
          .unwrap_or("")
          .to_string();
        (href, name)
      },
      _ => (None, String::new()),
    }
  }

  fn sorted_options(&self) -> Vec<(&String, &RenderedOption)> {
    let mut options: Vec<_> = self.options.iter().collect();
    options.sort_by(|(name_a, option_a), (name_b, option_b)| {
      compare_locs(&option_a.loc, &option_b.loc)
        .then_with(|| name_a.cmp(name_b))
    });
    options
  }

  fn make_anchor_suffix(&self, loc: &[String]) -> String {
    match self.anchor_style {
      AnchorStyle::None => String::new(),
      AnchorStyle::Legacy => {
        let sanitized = loc
          .iter()
          .map(|part| make_xml_id(part))
          .collect::<Vec<_>>()
          .join(".");
        format!(" {{#{}{sanitized}}}", self.anchor_prefix)
      },
    }
  }

  fn finalize(&self) -> String {
    let mut result = Vec::new();
    for (name, option) in self.sorted_options() {
      let anchor_suffix = self.make_anchor_suffix(&option.loc);
      result.push(format!("## {}{anchor_suffix}\n", md_escape(name)));
      result.extend(option.lines.iter().cloned());
      result.push("\n\n".to_string());
    }
    result.join("\n")
  }

  fn render_markdown(
    &self,
    markdown: &str,
    renderer: &mut CommonMarkRenderer,
  ) -> Result<String> {
    let rewritten = rewrite_roles(markdown, &self.manpage_urls);
    let arena = Arena::new();
    let options = Options::default();
    let root = parse_document(&arena, &rewritten, &options);
    renderer.render(root)
  }
}

impl ManpageConverter {
  fn new(
    revision: String,
    header: Option<Vec<String>>,
    footer: Option<Vec<String>>,
  ) -> Self {
    Self {
      revision,
      header,
      footer,
      options_by_id: HashMap::new(),
      options: HashMap::new(),
    }
  }

  fn add_options(&mut self, options: &Map<String, Value>) -> Result<()> {
    for name in options.keys() {
      self.options_by_id.insert(
        format!("#{}", make_xml_id(&format!("opt-{name}"))),
        name.clone(),
      );
    }

    for (name, option) in options {
      let rendered = self
        .render_option(name, option)
        .wrap_err_with(|| format!("failed to render option {name}"))?;
      self.options.insert(name.clone(), rendered);
    }
    Ok(())
  }

  fn render_option(
    &self,
    name: &str,
    option: &Value,
  ) -> Result<RenderedManpageOption> {
    let Value::Object(option) = option else {
      bail!("option value must be a JSON object")
    };

    let mut renderer = ManpageRenderer::new(self.options_by_id.clone());
    renderer.link_footnotes = Some(Vec::new());
    let lines = self.convert_one(option, &mut renderer)?;
    Ok(RenderedManpageOption {
      loc: option_loc(name, option),
      lines,
      links: renderer.link_footnotes.take().unwrap_or_default(),
    })
  }

  fn convert_one(
    &self,
    option: &Map<String, Value>,
    renderer: &mut ManpageRenderer,
  ) -> Result<Vec<String>> {
    let mut blocks = Vec::new();

    if let Some(description) = option.get("description") {
      let rendered = Self::render_description(description, renderer)?;
      if !rendered.is_empty() {
        blocks.push(vec![rendered]);
      }
    }

    if let Some(Value::String(type_name)) = option.get("type") {
      let read_only = if option
        .get("readOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false)
      {
        " *(read only)*"
      } else {
        ""
      };
      blocks.push(vec![renderer.render_markdown(&format!(
        "*Type:*\n{}{read_only}",
        md_escape(type_name)
      ))?]);
    }

    if option.get("default").is_some_and(json_truthy) {
      blocks.push(Self::render_code(option, "default", renderer)?);
    }
    if option.get("example").is_some_and(json_truthy) {
      blocks.push(Self::render_code(option, "example", renderer)?);
    }

    if let Some(Value::String(related)) = option.get("relatedPackages")
      && !related.is_empty()
    {
      blocks.push(vec![
        "\\fIRelated packages:\\fP".to_string(),
        ".sp".to_string(),
        renderer.render_markdown(related)?,
      ]);
    }
    if let Some(Value::Array(declarations)) = option.get("declarations")
      && !declarations.is_empty()
    {
      blocks.push(self.render_decl_def("Declared by", declarations));
    }
    if let Some(Value::Array(definitions)) = option.get("definitions")
      && !definitions.is_empty()
    {
      blocks.push(self.render_decl_def("Defined by", definitions));
    }

    let last_non_empty = blocks.iter().rposition(|block| !block.is_empty());
    if let Some(last) = last_non_empty {
      for block in &mut blocks[..last] {
        if !block.is_empty() {
          block.push(".sp".to_string());
        }
      }
    }

    Ok(blocks.into_iter().flatten().collect())
  }

  fn render_description(
    description: &Value,
    renderer: &mut ManpageRenderer,
  ) -> Result<String> {
    match description {
      Value::String(description) => renderer.render_markdown(description),
      Value::Object(object)
        if object.get("_type").and_then(Value::as_str) == Some("mdDoc") =>
      {
        object.get("text").and_then(Value::as_str).map_or_else(
          || Ok(String::new()),
          |text| renderer.render_markdown(text),
        )
      },
      _ => bail!("description has unrecognized type"),
    }
  }

  fn render_code(
    option: &Map<String, Value>,
    key: &str,
    renderer: &mut ManpageRenderer,
  ) -> Result<Vec<String>> {
    let quoted = renderer.inline_code_is_quoted;
    renderer.inline_code_is_quoted = false;
    let result = match option.get(key) {
      Some(Value::Object(object))
        if object.get("_type").and_then(Value::as_str) == Some("literalMD") =>
      {
        let text = object.get("text").and_then(Value::as_str).unwrap_or("");
        Ok(vec![renderer.render_markdown(&format!(
          "*{}:*\n{text}",
          capitalize(key)
        ))?])
      },
      Some(Value::Object(object))
        if object.get("_type").and_then(Value::as_str)
          == Some("literalExpression") =>
      {
        let text = object.get("text").and_then(Value::as_str).unwrap_or("");
        let code = md_make_code(text, "", None);
        Ok(vec![renderer.render_markdown(&format!(
          "*{}:*\n{code}",
          capitalize(key)
        ))?])
      },
      Some(_) => bail!("{key} has unrecognized type"),
      None => Ok(Vec::new()),
    };
    renderer.inline_code_is_quoted = quoted;
    result
  }

  fn render_decl_def(&self, header: &str, locations: &[Value]) -> Vec<String> {
    let mut lines = vec![format!("\\fI{}:\\fP", man_escape(header))];
    for location in locations {
      let (_, name) = format_decl_def_loc(location, &self.revision);
      lines.push(".RS 4".to_string());
      lines.push(format!("\\fB{}\\fP", man_escape(&name)));
      lines.push(".RE".to_string());
    }
    lines
  }

  fn sorted_options(&self) -> Vec<(&String, &RenderedManpageOption)> {
    let mut options: Vec<_> = self.options.iter().collect();
    options.sort_by(|(name_a, option_a), (name_b, option_b)| {
      compare_locs(&option_a.loc, &option_b.loc)
        .then_with(|| name_a.cmp(name_b))
    });
    options
  }

  fn finalize(&self) -> Result<String> {
    let mut result = Vec::new();
    let mut header_renderer = ManpageRenderer::new(self.options_by_id.clone());

    if let Some(header) = &self.header {
      result.extend(header.iter().cloned());
    } else {
      result.extend([
        r#".TH "CONFIGURATION\&.NIX" "5" "01/01/1980" "NixOS" "NixOS Reference Pages""#.to_string(),
        r#".\" disable hyphenation"#.to_string(),
        ".nh".to_string(),
        r#".\" disable justification (adjust text to left margin only)"#.to_string(),
        ".ad l".to_string(),
        r#".\" enable line breaks after slashes"#.to_string(),
        ".cflags 4 /".to_string(),
        r#".SH "NAME""#.to_string(),
        header_renderer.render_markdown(
          "{file}`configuration.nix` - NixOS system configuration specification",
        )?,
        r#".SH "DESCRIPTION""#.to_string(),
        ".PP".to_string(),
        header_renderer.render_markdown(
          "The file {file}`/etc/nixos/configuration.nix` contains the declarative specification of your NixOS system configuration. The command {command}`nixos-rebuild` takes this file and realises the system configuration specified therein.",
        )?,
        r#".SH "OPTIONS""#.to_string(),
        ".PP".to_string(),
        header_renderer.render_markdown(
          "You can use the following options in {file}`configuration.nix`.",
        )?,
      ]);
    }

    for (name, option) in self.sorted_options() {
      result.extend([
        ".PP".to_string(),
        format!("\\fB{}\\fR", man_escape(name)),
        ".RS 4".to_string(),
      ]);
      result.extend(option.lines.iter().cloned());
      if !option.links.is_empty() {
        result.push(".sp".to_string());
        let mut links = String::new();
        for (index, link) in option.links.iter().enumerate() {
          if index > 0 {
            links.push('\n');
          }
          if link.starts_with("#opt-") {
            if let Some(option_name) = self.options_by_id.get(link) {
              links.push_str(&(index + 1).to_string());
              links.push_str(". see the ");
              links.push('{');
              links.push_str("option}`");
              links.push_str(option_name);
              links.push_str("` option");
            }
          } else {
            links.push_str(&(index + 1).to_string());
            links.push_str(". ");
            links.push_str(&md_escape(link));
          }
        }
        result.push(
          ManpageRenderer::new(self.options_by_id.clone())
            .render_markdown(&links)?,
        );
      }
      result.push(".RE".to_string());
    }

    if let Some(footer) = &self.footer {
      result.extend(footer.iter().cloned());
    } else {
      result.extend([
        r#".SH "AUTHORS""#.to_string(),
        ".PP".to_string(),
        "Eelco Dolstra and the Nixpkgs/NixOS contributors".to_string(),
      ]);
    }

    Ok(result.join("\n"))
  }
}

impl CommonMarkRenderer {
  fn new() -> Self {
    Self {
      parstack:   vec![ParagraphState {
        indent:     String::new(),
        continuing: false,
      }],
      list_stack: Vec::new(),
      link_stack: Vec::new(),
    }
  }

  fn render<'ast>(
    &mut self,
    node: &'ast comrak::nodes::AstNode<'ast>,
  ) -> Result<String> {
    let value = node.data.borrow().value.clone();
    match value {
      NodeValue::Document => self.render_children(node),
      NodeValue::Paragraph => {
        let paragraph_break = self.maybe_paragraph_break();
        Ok(format!("{paragraph_break}{}", self.render_children(node)?))
      },
      NodeValue::Text(text) => {
        self.current_paragraph().continuing = true;
        Ok(self.indent_raw(&md_escape(&text)))
      },
      NodeValue::SoftBreak => Ok(self.line_break()),
      NodeValue::LineBreak => Ok(format!("  {}", self.line_break())),
      NodeValue::Code(code) => {
        self.current_paragraph().continuing = true;
        Ok(md_make_code(&code.literal, "", None))
      },
      NodeValue::CodeBlock(code) => {
        let literal = code.literal.strip_suffix('\n').unwrap_or(&code.literal);
        let paragraph_break = self.maybe_paragraph_break();
        Ok(format!(
          "{paragraph_break}{}",
          self.indent_raw(&md_make_code(literal, &code.info, Some(true)))
        ))
      },
      NodeValue::Emph => Ok(format!("*{}*", self.render_children(node)?)),
      NodeValue::Strong => Ok(format!("**{}**", self.render_children(node)?)),
      NodeValue::Link(link) => {
        self.current_paragraph().continuing = true;
        self.link_stack.push(link.url);
        let label = self.render_children(node)?;
        let href = self.link_stack.pop().unwrap_or_default();
        Ok(format!("[{label}]({})", md_escape(&href)))
      },
      NodeValue::List(list) => {
        self.list_stack.push(ListState {
          next_idx:        if list.list_type == comrak::nodes::ListType::Ordered
          {
            Some(list.start)
          } else {
            None
          },
          compact:         list.tight,
          first_item_seen: false,
        });
        let paragraph_break = self.maybe_paragraph_break();
        let children = self.render_children(node)?;
        self.list_stack.pop();
        Ok(format!("{paragraph_break}{children}"))
      },
      NodeValue::Item(_) => {
        let (line_break_count, head) = {
          let list = self
            .list_stack
            .last_mut()
            .ok_or_else(|| color_eyre::eyre::eyre!("list item outside list"))?;
          let line_break_count = if list.first_item_seen {
            Some(if list.compact { 1 } else { 2 })
          } else {
            None
          };
          list.first_item_seen = true;

          let head = list.next_idx.as_mut().map_or_else(
            || " -".to_string(),
            |next_idx| {
              let head = format!(" {next_idx}.");
              *next_idx += 1;
              head
            },
          );
          (line_break_count, head)
        };
        let line_break = line_break_count
          .map(|count| self.line_break().repeat(count))
          .unwrap_or_default();

        self.enter_block(&" ".repeat(head.len() + 1));
        let children = self.render_children(node)?;
        self.leave_block();
        Ok(format!("{line_break}{head} {children}"))
      },
      NodeValue::BlockQuote => {
        let paragraph_break = self.maybe_paragraph_break();
        self.enter_block("> ");
        let children = self.render_children(node)?;
        self.leave_block();
        Ok(format!("{paragraph_break}> {children}"))
      },
      NodeValue::Heading(_) => bail!("md token not supported in options doc"),
      NodeValue::HtmlInline(html)
      | NodeValue::HtmlBlock(comrak::nodes::NodeHtmlBlock {
        literal: html,
        ..
      }) => {
        self.current_paragraph().continuing = true;
        Ok(self.indent_raw(&md_escape(&html)))
      },
      NodeValue::Raw(raw) => Ok(raw),
      NodeValue::ThematicBreak => {
        Ok(format!("{}---", self.maybe_paragraph_break()))
      },
      unsupported => {
        bail!("md token not supported in options doc: {unsupported:?}")
      },
    }
  }

  fn render_children<'ast>(
    &mut self,
    node: &'ast comrak::nodes::AstNode<'ast>,
  ) -> Result<String> {
    let mut rendered = String::new();
    for child in node.children() {
      rendered.push_str(&self.render(child)?);
    }
    Ok(rendered)
  }

  fn enter_block(&mut self, extra_indent: &str) {
    let mut indent = self.current_paragraph().indent.clone();
    indent.push_str(extra_indent);
    self.parstack.push(ParagraphState {
      indent,
      continuing: false,
    });
  }

  fn leave_block(&mut self) {
    self.parstack.pop();
    self.current_paragraph().continuing = true;
  }

  fn line_break(&mut self) -> String {
    let current = self.current_paragraph();
    current.continuing = true;
    format!("\n{}", current.indent)
  }

  fn maybe_paragraph_break(&mut self) -> String {
    let current = self.current_paragraph();
    let result = if current.continuing {
      format!("\n{}\n{}", current.indent, current.indent)
    } else {
      String::new()
    };
    current.continuing = true;
    result
  }

  fn indent_raw(&self, value: &str) -> String {
    if value.contains('\n') {
      value
        .split('\n')
        .collect::<Vec<_>>()
        .join(&format!("\n{}", self.current_indent()))
    } else {
      value.to_string()
    }
  }

  fn current_indent(&self) -> &str {
    self
      .parstack
      .last()
      .map_or("", |paragraph| paragraph.indent.as_str())
  }

  fn current_paragraph(&mut self) -> &mut ParagraphState {
    let last_index = self.parstack.len() - 1;
    &mut self.parstack[last_index]
  }
}

impl ManpageRenderer {
  const fn new(href_targets: HashMap<String, String>) -> Self {
    Self {
      inline_code_is_quoted: true,
      link_footnotes: None,
      href_targets,
      link_stack: Vec::new(),
      do_parbreak_stack: Vec::new(),
      list_stack: Vec::new(),
      font_stack: Vec::new(),
    }
  }

  fn render_markdown(&mut self, markdown: &str) -> Result<String> {
    let rewritten = rewrite_roles_as_markers(markdown);
    let arena = Arena::new();
    let mut options = Options::default();
    options.parse.smart = true;
    options.extension.table = true;
    options.extension.footnotes = true;
    options.extension.description_lists = true;
    let root = parse_document(&arena, &rewritten, &options);
    self.render(root)
  }

  fn render<'ast>(
    &mut self,
    node: &'ast comrak::nodes::AstNode<'ast>,
  ) -> Result<String> {
    self.do_parbreak_stack = vec![false];
    self.font_stack = vec!["\\fR"];
    self.render_node(node)
  }

  fn render_node<'ast>(
    &mut self,
    node: &'ast comrak::nodes::AstNode<'ast>,
  ) -> Result<String> {
    let value = node.data.borrow().value.clone();
    match value {
      NodeValue::Document | NodeValue::DescriptionItem(_) => {
        self.render_block_children(node)
      },
      NodeValue::Paragraph => {
        let paragraph_break = self.maybe_paragraph_break("");
        Ok(format!(
          "{paragraph_break}{}",
          self.render_inline_children(node)?
        ))
      },
      NodeValue::Text(text) => Self::render_text(&text),
      NodeValue::SoftBreak => Ok(" ".to_string()),
      NodeValue::LineBreak => Ok(".br".to_string()),
      NodeValue::Code(code) => {
        let escaped = protect_spaces(&man_escape(&code.literal));
        if self.inline_code_is_quoted {
          Ok(format!("\\fR\\(oq{escaped}\\(cq\\fP"))
        } else {
          Ok(escaped)
        }
      },
      NodeValue::CodeBlock(code) => {
        let literal = code.literal.trim_end_matches('\n');
        Ok(format!(
          ".sp\n.RS 4\n.nf\n{}\n.fi\n.RE",
          man_escape(literal)
        ))
      },
      NodeValue::Emph => {
        self.font_stack.push("\\fI");
        let rendered = self.render_inline_children(node)?;
        self.font_stack.pop();
        Ok(format!("\\fI{}{}", rendered, self.current_font()))
      },
      NodeValue::Strong => {
        self.font_stack.push("\\fB");
        let rendered = self.render_inline_children(node)?;
        self.font_stack.pop();
        Ok(format!("\\fB{}{}", rendered, self.current_font()))
      },
      NodeValue::Link(link) => {
        let href = link.url;
        let target_text = if node.first_child().is_none() {
          self.href_targets.get(&href).cloned().unwrap_or_default()
        } else {
          String::new()
        };
        self.link_stack.push(href);
        self.font_stack.push("\\fB");
        let rendered = self.render_inline_children(node)?;
        let href = self.link_stack.pop().unwrap_or_default();
        let footnote = self.link_footnotes.as_mut().map(|link_footnotes| {
          link_footnotes
            .iter()
            .position(|existing| existing == &href)
            .map_or_else(
              || {
                link_footnotes.push(href);
                link_footnotes.len()
              },
              |index| index + 1,
            )
        });
        let footnote_text = footnote
          .map(|footnote| {
            format!("\\fR{}", man_escape(&format!("[{footnote}]")))
          })
          .unwrap_or_default();
        self.font_stack.pop();
        Ok(format!(
          "\\fB{target_text}\0 <{rendered}>\0 {footnote_text}{}",
          self.current_font()
        ))
      },
      NodeValue::List(list) => {
        let item_count = node
          .children()
          .filter(|child| {
            matches!(child.data.borrow().value, NodeValue::Item(_))
          })
          .count();
        let width = if list.list_type == comrak::nodes::ListType::Ordered {
          3 + (list.start + item_count.saturating_sub(1))
            .to_string()
            .len()
        } else {
          4
        };
        self.list_stack.push(ManpageListState {
          width,
          next_idx: if list.list_type == comrak::nodes::ListType::Ordered {
            Some(list.start)
          } else {
            None
          },
          compact: list.tight,
          first_item_seen: false,
        });
        let paragraph_break = self.maybe_paragraph_break("");
        let children = self.render_block_children(node)?;
        self.list_stack.pop();
        Ok(format!("{paragraph_break}{children}"))
      },
      NodeValue::Item(_) => {
        self.enter_block();
        let (maybe_space, width, head) = {
          let list = self
            .list_stack
            .last_mut()
            .ok_or_else(|| color_eyre::eyre::eyre!("list item outside list"))?;
          let maybe_space = if list.compact || !list.first_item_seen {
            String::new()
          } else {
            ".sp\n".to_string()
          };
          list.first_item_seen = true;
          let head = list.next_idx.as_mut().map_or_else(
            || "•".to_string(),
            |next_idx| {
              let head = format!("{next_idx}.");
              *next_idx += 1;
              head
            },
          );
          (maybe_space, list.width, head)
        };
        let children = self.render_block_children(node)?;
        self.leave_block();
        Ok(format!(
          "{maybe_space}.RS \
           {width}\n\\h'-{}'\\fB{}\\fP\\h'1'\\c\n{children}\n.RE",
          head.len() + 1,
          man_escape(&head)
        ))
      },
      NodeValue::BlockQuote => {
        let maybe_paragraph = self.maybe_paragraph_break("\n");
        self.enter_block();
        let children = self.render_block_children(node)?;
        self.leave_block();
        Ok(format!(
          "{maybe_paragraph}.RS \
           4\n\\h'-3'\\fI\\(lq\\(rq\\fP\\h'1'\\c\n{children}\n.RE"
        ))
      },
      NodeValue::DescriptionList => {
        Ok(format!(".RS 4\n{}\n.RE", self.render_block_children(node)?))
      },
      NodeValue::DescriptionTerm => {
        Ok(format!(".PP\n{}", self.render_inline_children(node)?))
      },
      NodeValue::DescriptionDetails => {
        self.enter_block();
        let children = self.render_block_children(node)?;
        self.leave_block();
        Ok(format!(".RS 4\n{children}\n.RE"))
      },
      NodeValue::Heading(_) => bail!("md token not supported in manpages"),
      NodeValue::HtmlInline(html)
      | NodeValue::HtmlBlock(comrak::nodes::NodeHtmlBlock {
        literal: html,
        ..
      }) => Self::render_text(&html),
      NodeValue::Raw(raw) => Ok(raw),
      NodeValue::ThematicBreak => {
        Ok(format!("{}\n---", self.maybe_paragraph_break("")))
      },
      unsupported => {
        bail!("md node not supported in manpages: {unsupported:?}")
      },
    }
  }

  fn render_block_children<'ast>(
    &mut self,
    node: &'ast comrak::nodes::AstNode<'ast>,
  ) -> Result<String> {
    let mut rendered = Vec::new();
    for child in node.children() {
      let child = self.render_node(child)?;
      if !child.is_empty() {
        rendered.push(child);
      }
    }
    Ok(rendered.join("\n"))
  }

  fn render_inline_children<'ast>(
    &mut self,
    node: &'ast comrak::nodes::AstNode<'ast>,
  ) -> Result<String> {
    let mut rendered = String::new();
    for child in node.children() {
      rendered.push_str(&self.render_node(child)?);
    }
    Ok(normalize_space(&rendered))
  }

  fn render_text(text: &str) -> Result<String> {
    let mut rendered = String::with_capacity(text.len());
    let mut rest = text;

    while let Some(start) = rest.find(ROLE_START) {
      rendered.push_str(&man_escape(&rest[..start]));
      let after_start = &rest[start + ROLE_START.len_utf8()..];
      let Some(sep) = after_start.find(ROLE_SEP) else {
        rendered.push_str(&man_escape(after_start));
        return Ok(rendered);
      };
      let role = &after_start[..sep];
      let after_sep = &after_start[sep + ROLE_SEP.len_utf8()..];
      let Some(end) = after_sep.find(ROLE_END) else {
        rendered.push_str(&man_escape(after_sep));
        return Ok(rendered);
      };
      let content = &after_sep[..end];
      rendered.push_str(&render_manpage_role(role, content)?);
      rest = &after_sep[end + ROLE_END.len_utf8()..];
    }

    rendered.push_str(&man_escape(rest));
    Ok(rendered)
  }

  fn enter_block(&mut self) {
    self.do_parbreak_stack.push(false);
  }

  fn leave_block(&mut self) {
    self.do_parbreak_stack.pop();
    if let Some(parent) = self.do_parbreak_stack.last_mut() {
      *parent = true;
    }
  }

  fn maybe_paragraph_break(&mut self, suffix: &str) -> String {
    let do_break = self.do_parbreak_stack.last().copied().unwrap_or(false);
    if let Some(current) = self.do_parbreak_stack.last_mut() {
      *current = true;
    }
    if do_break {
      format!(".sp{suffix}")
    } else {
      String::new()
    }
  }

  fn current_font(&self) -> &'static str {
    self.font_stack.last().copied().unwrap_or("\\fR")
  }
}

fn format_decl_def_loc(
  location: &Value,
  revision: &str,
) -> (Option<String>, String) {
  match location {
    Value::String(location) => {
      if location.starts_with('/') {
        let name = if location.contains("nixops") && location.contains("/nix/")
        {
          let suffix = location
            .find("/nix/")
            .map_or(location.as_str(), |index| &location[index + 5..]);
          format!("<nixops/{suffix}>")
        } else {
          location.clone()
        };
        (Some(format!("file://{location}")), name)
      } else {
        let href = if revision == "local" {
          format!("https://github.com/NixOS/nixpkgs/blob/master/{location}")
        } else {
          format!("https://github.com/NixOS/nixpkgs/blob/{revision}/{location}")
        };
        (Some(href), format!("<nixpkgs/{location}>"))
      }
    },
    Value::Object(object) => {
      let href = object
        .get("url")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
      let name = object
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
      (href, name)
    },
    _ => (None, String::new()),
  }
}

fn render_manpage_role(role: &str, content: &str) -> Result<String> {
  match role {
    "command" | "env" | "option" => {
      Ok(format!("\\fB{}\\fP", man_escape(content)))
    },
    "file" | "var" => Ok(format!("\\fI{}\\fP", man_escape(content))),
    "manpage" => {
      let Some((page, section)) = content.rsplit_once('(') else {
        bail!("manpage role must contain a section: {content}")
      };
      let section = section.trim_end_matches(')').trim();
      Ok(format!(
        "\\fB{}\\fP\\fR({})\\fP",
        man_escape(page.trim()),
        man_escape(section)
      ))
    },
    _ => bail!("md node not supported in manpages: role {role}"),
  }
}

fn rewrite_roles_as_markers(markdown: &str) -> String {
  let mut rendered = String::with_capacity(markdown.len());
  let mut rest = markdown;

  while let Some(start) = rest.find('{') {
    let (before, role_start) = rest.split_at(start);
    rendered.push_str(before);

    let Some(role_end) = role_start.find('}') else {
      rendered.push_str(role_start);
      return rendered;
    };
    let role = &role_start[1..role_end];
    let after_role = &role_start[role_end + 1..];
    let Some(after_tick) = after_role.strip_prefix('`') else {
      rendered.push_str(&role_start[..=role_end]);
      rest = after_role;
      continue;
    };
    let Some(content_end) = after_tick.find('`') else {
      rendered.push_str(role_start);
      return rendered;
    };

    rendered.push(ROLE_START);
    rendered.push_str(role);
    rendered.push(ROLE_SEP);
    rendered.push_str(&after_tick[..content_end]);
    rendered.push(ROLE_END);
    rest = &after_tick[content_end + 1..];
  }

  rendered.push_str(rest);
  rendered
}

fn man_escape(value: &str) -> String {
  let mut escaped = String::with_capacity(value.len());
  for c in value.chars() {
    match c {
      '"' => escaped.push_str("\\(dq"),
      '\'' => escaped.push_str("\\(aq"),
      '-' => escaped.push_str("\\-"),
      '.' => escaped.push_str("\\&."),
      '\\' => escaped.push_str("\\e"),
      '^' => escaped.push_str("\\(ha"),
      '`' => escaped.push_str("\\(ga"),
      '~' => escaped.push_str("\\(ti"),
      _ => escaped.push(c),
    }
  }

  ROFF_UNICODE
    .replace_all(&escaped, |captures: &regex::Captures<'_>| {
      captures[0]
        .chars()
        .next()
        .map_or_else(String::new, |c| format!("\\[u{:04X}]", c as u32))
    })
    .into_owned()
}

fn protect_spaces(value: &str) -> String {
  value.replace(' ', PROTECTED_SPACE)
}

fn normalize_space(value: &str) -> String {
  let mut normalized = String::with_capacity(value.len());
  let mut rest = value;

  while !rest.is_empty() {
    if rest.starts_with("\0 <") {
      rest = &rest[3..];
      while rest.starts_with(' ') {
        rest = &rest[1..];
      }
      continue;
    }

    if rest.starts_with(">\0 ") {
      while normalized.ends_with(' ') {
        normalized.pop();
      }
      rest = &rest[3..];
      continue;
    }

    let Some(c) = rest.chars().next() else {
      break;
    };
    if c != ' ' || !normalized.ends_with(' ') {
      normalized.push(c);
    }
    rest = &rest[c.len_utf8()..];
  }

  normalized.replace(PROTECTED_SPACE, " ")
}

fn option_loc(name: &str, option: &Map<String, Value>) -> Vec<String> {
  option
    .get("loc")
    .and_then(Value::as_array)
    .map(|loc| {
      loc
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
    })
    .filter(|loc| !loc.is_empty())
    .unwrap_or_else(|| name.split('.').map(ToOwned::to_owned).collect())
}

fn compare_locs(left: &[String], right: &[String]) -> Ordering {
  left
    .iter()
    .map(|part| (loc_part_rank(part), part))
    .cmp(right.iter().map(|part| (loc_part_rank(part), part)))
}

fn loc_part_rank(part: &str) -> u8 {
  if part.starts_with("enable") {
    0
  } else if part.starts_with("package") {
    1
  } else {
    2
  }
}

fn json_truthy(value: &Value) -> bool {
  match value {
    Value::Null => false,
    Value::Bool(value) => *value,
    Value::Number(number) => number.as_i64() != Some(0),
    Value::String(value) => !value.is_empty(),
    Value::Array(value) => !value.is_empty(),
    Value::Object(value) => !value.is_empty(),
  }
}

fn capitalize(value: &str) -> String {
  let mut chars = value.chars();
  chars.next().map_or_else(String::new, |first| {
    first.to_uppercase().chain(chars).collect()
  })
}

fn rewrite_roles(
  markdown: &str,
  manpage_urls: &HashMap<String, String>,
) -> String {
  let mut rendered = String::with_capacity(markdown.len());
  let mut rest = markdown;

  while let Some(start) = rest.find('{') {
    let (before, role_start) = rest.split_at(start);
    rendered.push_str(before);

    let Some(role_end) = role_start.find('}') else {
      rendered.push_str(role_start);
      return rendered;
    };
    let role = &role_start[1..role_end];
    let after_role = &role_start[role_end + 1..];
    let Some(after_tick) = after_role.strip_prefix('`') else {
      rendered.push_str(&role_start[..=role_end]);
      rest = after_role;
      continue;
    };
    let Some(content_end) = after_tick.find('`') else {
      rendered.push_str(role_start);
      return rendered;
    };

    let content = &after_tick[..content_end];
    let code = md_make_code(content, "", None);
    if role == "manpage" {
      if let Some(url) = manpage_urls.get(content) {
        rendered.push('[');
        rendered.push_str(&code);
        rendered.push_str("](");
        rendered.push_str(url);
        rendered.push(')');
      } else {
        rendered.push_str(&code);
      }
    } else {
      rendered.push_str(&code);
    }
    rest = &after_tick[content_end + 1..];
  }

  rendered.push_str(rest);
  rendered
}

fn md_escape(value: &str) -> String {
  let mut escaped = String::with_capacity(value.len());
  for c in value.chars() {
    match c {
      '*' | '<' | '[' | '`' | '.' | '#' | '&' | '\\' => {
        escaped.push('\\');
        escaped.push(c);
      },
      _ => escaped.push(c),
    }
  }
  escaped
}

fn md_make_code(code: &str, info: &str, multiline: Option<bool>) -> String {
  let multiline =
    multiline.unwrap_or_else(|| !info.is_empty() || code.contains('\n'));
  let longest = longest_backtick_run(code);
  let ticks = "`".repeat(longest + if multiline { 3 } else { 1 });
  let separator = if multiline { "\n" } else { " " };
  format!("{ticks}{info}{separator}{code}{separator}{ticks}")
}

fn longest_backtick_run(value: &str) -> usize {
  let mut longest = 0;
  let mut current = 0;
  for c in value.chars() {
    if c == '`' {
      current += 1;
      longest = longest.max(current);
    } else {
      current = 0;
    }
  }
  longest
}

fn make_xml_id(value: &str) -> String {
  value
    .chars()
    .map(|c| {
      match c {
        '*' | '<' | ' ' | '>' | '[' | ']' | ':' | '"' => '_',
        _ => c,
      }
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  const SIMPLE_OPTIONS: &str = r#"{
  "services.frobnicator.types.<name>.enable": {
    "declarations": [
      "nixos/modules/services/frobnicator.nix"
    ],
    "description": "Whether to enable the frobnication of this (`<name>`) type.",
    "loc": [
      "services",
      "frobnicator",
      "types",
      "<name>",
      "enable"
    ],
    "readOnly": false,
    "type": "boolean"
  }
}"#;

  const SIMPLE_DEFAULT: &str = concat!(
    "## services\\.frobnicator\\.types\\.\\<name>\\.enable\n",
    "\n",
    "Whether to enable the frobnication of this (` <name> `) type\\.\n",
    "\n",
    "\n",
    "\n",
    "*Type:*\n",
    "boolean\n",
    "\n",
    "*Declared by:*\n",
    " - [\\<nixpkgs/nixos/modules/services/frobnicator\\.nix>](https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/services/frobnicator.nix)\n",
    "\n",
    "\n",
  );

  const SIMPLE_LEGACY: &str = concat!(
    "## services\\.frobnicator\\.types\\.\\<name>\\.enable {#opt-services.frobnicator.types._name_.enable}\n",
    "\n",
    "Whether to enable the frobnication of this (` <name> `) type\\.\n",
    "\n",
    "\n",
    "\n",
    "*Type:*\n",
    "boolean\n",
    "\n",
    "*Declared by:*\n",
    " - [\\<nixpkgs/nixos/modules/services/frobnicator\\.nix>](https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/services/frobnicator.nix)\n",
    "\n",
    "\n",
  );

  const SIMPLE_MANPAGE_CUSTOM: &str = concat!(
    ".TH TEST\n",
    ".PP\n",
    "\\fBservices\\&.frobnicator\\&.types\\&.<name>\\&.enable\\fR\n",
    ".RS 4\n",
    "Whether to enable the frobnication of this (\\fR\\(oq<name>\\(cq\\fP) \
     type\\&.\n",
    ".sp\n",
    "\\fIType:\\fR boolean\n",
    ".sp\n",
    "\\fIDeclared by:\\fP\n",
    ".RS 4\n",
    "\\fB<nixpkgs/nixos/modules/services/frobnicator\\&.nix>\\fP\n",
    ".RE\n",
    ".RE\n",
    ".SH END",
  );

  const COMPLEX_OPTIONS: &str = r#"{
  "services.foo.enable": {
    "declarations": [
      "nixos/modules/services/foo.nix"
    ],
    "definitions": [
      {
        "name": "<foo/modules/default.nix>",
        "url": "https://example.invalid/foo/modules/default.nix"
      }
    ],
    "default": {
      "_type": "literalExpression",
      "text": "false"
    },
    "description": "Whether to enable {command}`fooctl`. See [the manual](https://example.invalid/manual) and [](#opt-services.foo.mode).",
    "example": {
      "_type": "literalMD",
      "text": "`true`"
    },
    "loc": [
      "services",
      "foo",
      "enable"
    ],
    "readOnly": false,
    "relatedPackages": "{manpage}`foo(1)` and {file}`/nix/store/example`",
    "type": "boolean"
  },
  "services.foo.mode": {
    "declarations": [
      "/nix/store/abc-source/nixos/modules/services/foo.nix"
    ],
    "default": {
      "_type": "literalExpression",
      "text": "\"auto\""
    },
    "description": {
      "_type": "mdDoc",
      "text": "Mode for {option}`services.foo.enable` using {env}`FOO_MODE`, {var}`mode`, and {file}`/etc/foo.conf`."
    },
    "loc": [
      "services",
      "foo",
      "mode"
    ],
    "readOnly": true,
    "type": "one of \"auto\", \"manual\""
  }
}"#;

  const COMPLEX_MANPAGE_CUSTOM: &str = concat!(
    ".TH TEST 5\n",
    ".SH CUSTOM\n",
    ".PP\n",
    "\\fBservices\\&.foo\\&.enable\\fR\n",
    ".RS 4\n",
    "Whether to enable \\fBfooctl\\fP\\&. See \\fBthe manual\\fR[1]\\fR and \
     \\fBservices.foo.mode\\fR[2]\\fR\\&.\n",
    ".sp\n",
    "\\fIType:\\fR boolean\n",
    ".sp\n",
    "\\fIDefault:\\fR false\n",
    ".sp\n",
    "\\fIExample:\\fR true\n",
    ".sp\n",
    "\\fIRelated packages:\\fP\n",
    ".sp\n",
    "\\fBfoo\\fP\\fR(1)\\fP and \\fI/nix/store/example\\fP\n",
    ".sp\n",
    "\\fIDeclared by:\\fP\n",
    ".RS 4\n",
    "\\fB<nixpkgs/nixos/modules/services/foo\\&.nix>\\fP\n",
    ".RE\n",
    ".sp\n",
    "\\fIDefined by:\\fP\n",
    ".RS 4\n",
    "\\fB<foo/modules/default\\&.nix>\\fP\n",
    ".RE\n",
    ".sp\n",
    ".RS 4\n",
    "\\h'-3'\\fB1\\&.\\fP\\h'1'\\c\n",
    "https://example\\&.invalid/manual\n",
    ".RE\n",
    ".RS 4\n",
    "\\h'-3'\\fB2\\&.\\fP\\h'1'\\c\n",
    "see the \\fBservices\\&.foo\\&.mode\\fP option\n",
    ".RE\n",
    ".RE\n",
    ".PP\n",
    "\\fBservices\\&.foo\\&.mode\\fR\n",
    ".RS 4\n",
    "Mode for \\fBservices\\&.foo\\&.enable\\fP using \\fBFOO_MODE\\fP, \
     \\fImode\\fP, and \\fI/etc/foo\\&.conf\\fP\\&.\n",
    ".sp\n",
    "\\fIType:\\fR one of \\[u201C]auto\\[u201D], \\[u201C]manual\\[u201D] \
     \\fI(read only)\\fR\n",
    ".sp\n",
    "\\fIDefault:\\fR \\(dqauto\\(dq\n",
    ".sp\n",
    "\\fIDeclared by:\\fP\n",
    ".RS 4\n",
    "\\fB/nix/store/abc\\-source/nixos/modules/services/foo\\&.nix\\fP\n",
    ".RE\n",
    ".RE\n",
    ".SH END\n",
    "custom footer",
  );

  fn render_simple(anchor_style: AnchorStyle, anchor_prefix: &str) -> String {
    let options = serde_json::from_str(SIMPLE_OPTIONS).unwrap();
    let mut converter = CommonMarkConverter::new(
      HashMap::new(),
      "local".to_string(),
      anchor_style,
      anchor_prefix.to_string(),
    );
    converter.add_options(&options).unwrap();
    converter.finalize()
  }

  fn render_simple_manpage() -> String {
    let options = serde_json::from_str(SIMPLE_OPTIONS).unwrap();
    let mut converter = ManpageConverter::new(
      "local".to_string(),
      Some(vec![".TH TEST".to_string()]),
      Some(vec![".SH END".to_string()]),
    );
    converter.add_options(&options).unwrap();
    converter.finalize().unwrap()
  }

  fn render_complex_manpage() -> String {
    let options = serde_json::from_str(COMPLEX_OPTIONS).unwrap();
    let mut converter = ManpageConverter::new(
      "local".to_string(),
      Some(vec![".TH TEST 5".to_string(), ".SH CUSTOM".to_string()]),
      Some(vec![".SH END".to_string(), "custom footer".to_string()]),
    );
    converter.add_options(&options).unwrap();
    converter.finalize().unwrap()
  }

  fn render_manpage(markdown: &str) -> String {
    let mut renderer = ManpageRenderer::new(HashMap::new());
    renderer.render_markdown(markdown).unwrap()
  }

  #[test]
  fn options_commonmark_matches_simple_fixture() {
    assert_eq!(render_simple(AnchorStyle::None, ""), SIMPLE_DEFAULT);
  }

  #[test]
  fn options_commonmark_matches_legacy_anchor_fixture() {
    assert_eq!(render_simple(AnchorStyle::Legacy, "opt-"), SIMPLE_LEGACY);
  }

  #[test]
  fn options_manpage_matches_simple_fixture() {
    assert_eq!(render_simple_manpage(), SIMPLE_MANPAGE_CUSTOM);
  }

  #[test]
  fn options_manpage_matches_complex_fixture() {
    assert_eq!(render_complex_manpage(), COMPLEX_MANPAGE_CUSTOM);
  }

  #[test]
  fn manpage_renderer_matches_inline_code_fixture() {
    assert_eq!(
      render_manpage("1  `x  a  x`  2"),
      "1 \\fR\\(oqx  a  x\\(cq\\fP 2"
    );
  }

  #[test]
  fn manpage_renderer_matches_font_fixture() {
    assert_eq!(render_manpage("*a **b** c*"), "\\fIa \\fBb\\fI c\\fR");
    assert_eq!(
      render_manpage("*a [1 `2`](3) c*"),
      "\\fIa \\fB1 \\fR\\(oq2\\(cq\\fP\\fI c\\fR"
    );
  }

  #[test]
  fn manpage_renderer_expands_empty_link_targets() {
    let mut renderer = ManpageRenderer::new(HashMap::from([
      ("#foo1".to_string(), "bar".to_string()),
      ("#foo2".to_string(), "bar".to_string()),
    ]));

    assert_eq!(
      renderer
        .render_markdown("[a](#foo1) [](#foo2) [b](#bar1) [](#bar2)")
        .unwrap(),
      "\\fBa\\fR \\fBbar\\fR \\fBb\\fR \\fB\\fR"
    );
  }

  #[test]
  fn manpage_renderer_collects_links() {
    let mut renderer = ManpageRenderer::new(HashMap::new());
    renderer.link_footnotes = Some(Vec::new());

    assert_eq!(
      renderer.render_markdown("[a](link1) [b](link2)").unwrap(),
      "\\fBa\\fR[1]\\fR \\fBb\\fR[2]\\fR"
    );
    assert_eq!(renderer.link_footnotes.unwrap(), vec![
      "link1".to_string(),
      "link2".to_string()
    ]);
  }

  #[test]
  fn manpage_renderer_deduplicates_links() {
    let mut renderer = ManpageRenderer::new(HashMap::new());
    renderer.link_footnotes = Some(Vec::new());

    assert_eq!(
      renderer.render_markdown("[a](link) [b](link)").unwrap(),
      "\\fBa\\fR[1]\\fR \\fBb\\fR[1]\\fR"
    );
    assert_eq!(renderer.link_footnotes.unwrap(), vec!["link".to_string()]);
  }
}
