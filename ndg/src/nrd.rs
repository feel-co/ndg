use std::{cmp::Ordering, collections::HashMap, env, fs, path::PathBuf};

use clap::{Args, Parser, Subcommand};
use color_eyre::eyre::{Context, Result, bail};
use comrak::{Arena, Options, nodes::NodeValue, parse_document};
use serde_json::{Map, Value};

const NRD_BIN_NAMES: &[&str] = &["nrd", "nixos-render-docs"];

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
    OptionsFormat::Manpage(args) => {
      let _ = args.revision;
      let header = read_optional_text(args.header.as_ref())?;
      let footer = read_optional_text(args.footer.as_ref())?;
      ndg_manpage::generate_manpage(
        &args.infile,
        Some(&args.outfile),
        Some("configuration.nix"),
        header.as_deref(),
        footer.as_deref(),
        5,
      )
    },
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

fn read_optional_text(path: Option<&PathBuf>) -> Result<Option<String>> {
  path
    .map(|path| {
      fs::read_to_string(path)
        .wrap_err_with(|| format!("failed to read {}", path.display()))
    })
    .transpose()
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

  #[test]
  fn options_commonmark_matches_simple_fixture() {
    assert_eq!(render_simple(AnchorStyle::None, ""), SIMPLE_DEFAULT);
  }

  #[test]
  fn options_commonmark_matches_legacy_anchor_fixture() {
    assert_eq!(render_simple(AnchorStyle::Legacy, "opt-"), SIMPLE_LEGACY);
  }
}
