use std::{
  fs,
  path::{Path, PathBuf},
};

use color_eyre::eyre::{Context, Result};
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use log::{info, warn};
use ndg_commonmark::{
  Header,
  MarkdownOptionsBuilder,
  MarkdownProcessor,
  collect_markdown_files,
  processor::types::TabStyle,
};
use ndg_config::Config;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

/// TOML frontmatter parsed from the `+++` delimited block at the start of a
/// markdown document.
///
/// Frontmatter must appear at the very beginning of the file, wrapped in
/// `+++` delimiters on their own lines. Any unrecognised keys are silently
/// ignored by the TOML deserialiser.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PageFrontmatter {
  /// Overrides the page title extracted from the first H1 heading.
  pub title:       Option<String>,
  /// Per-page meta description. Injected as `<meta name="description">` and
  /// as `og:description` where OG tags are enabled.
  pub description: Option<String>,

  /// Per-page author. Injected as `<meta name="author">`.
  pub author: Option<String>,

  /// Name of the template file to use for this page (e.g. `"custom"` or
  /// `"custom.html"`). Falls back to the default template resolution if the
  /// named file cannot be found.
  pub template: Option<String>,

  /// When `false`, suppresses the table of contents for this page.
  pub toc: Option<bool>,

  /// Per-page tags. Injected as `<meta name="keywords">` and override the
  /// global `meta.tags.keywords` for this page.
  pub tags: Option<Vec<String>>,

  /// Arbitrary user-defined data. Accessible in templates as
  /// `{{ page.extra.key }}`.
  pub extra: Option<toml::Table>,
}

/// Strips TOML frontmatter from the start of a markdown document.
///
/// Frontmatter must begin at the very first byte of the file, delimited by
/// `+++` on its own line. The closing `+++` must also be on its own line.
/// Returns the parsed [`PageFrontmatter`] (if present and valid) and the
/// remaining markdown content with the frontmatter block removed.
///
/// Malformed TOML inside a valid `+++` block is logged as a warning and
/// treated as absent; the full original content is returned unchanged.
fn strip_frontmatter(content: &str) -> (Option<PageFrontmatter>, String) {
  const DELIM: &str = "+++";

  // Normalize CRLF to LF so that Windows-style line endings are handled.
  let lf_content;
  let content: &str = if content.contains("\r\n") {
    lf_content = content.replace("\r\n", "\n");
    &lf_content
  } else {
    content
  };

  // The opening delimiter must be the very first thing in the file.
  let Some(after_open) = content.strip_prefix(DELIM) else {
    return (None, content.to_owned());
  };
  // Must be immediately followed by a newline.
  let Some(after_open) = after_open.strip_prefix('\n') else {
    return (None, content.to_owned());
  };

  // Find the closing delimiter.
  let needle = format!("\n{DELIM}");
  let Some(end_idx) = after_open.find(needle.as_str()) else {
    return (None, content.to_owned());
  };

  let toml_str = &after_open[..end_idx];
  let after_close = &after_open[end_idx + needle.len()..];
  // Strip the single newline that follows the closing delimiter, if present.
  let remaining = after_close.strip_prefix('\n').unwrap_or(after_close);

  match toml::from_str::<PageFrontmatter>(toml_str) {
    Ok(fm) => (Some(fm), remaining.to_owned()),
    Err(e) => {
      warn!("Failed to parse frontmatter: {e}");
      (None, content.to_owned())
    },
  }
}

/// Output entry for processed markdown files
pub struct ProcessedMarkdown {
  pub html_content: String,
  pub headers:      Vec<Header>,
  pub title:        Option<String>,
  pub source_path:  PathBuf,
  pub output_path:  String,
  pub is_included:  bool,
  pub frontmatter:  Option<PageFrontmatter>,
}

const MARKDOWN_CACHE_SCHEMA: u8 = 2;

#[derive(Serialize, Deserialize)]
struct MarkdownCacheEntry {
  schema:           u8,
  source_digest:    String,
  dependencies:     Vec<(PathBuf, Option<String>)>,
  processor_digest: String,
  result:           ndg_commonmark::MarkdownResult,
  frontmatter:      Option<PageFrontmatter>,
}

/// Processes all markdown files in the input directory and returns processed
/// data.
///
/// This function renders all markdown files, handles custom output paths,
/// and returns structured data for each file. Files that are included in other
/// files are marked but still returned (caller decides whether to skip them).
///
/// As a side effect, this function populates `config.included_files` with the
/// mapping of included files to their parent files.
///
/// # Arguments
///
/// * `config` - The loaded configuration for documentation generation.
/// * `processor` - Optional pre-created processor to reuse. If None, creates a
///   new one.
///
/// # Returns
///
/// A vector of `ProcessedMarkdown` entries.
///
/// # Errors
///
/// Returns an error if any file cannot be read or rendered.
///
/// # Panics
///
/// Panics if `config.input_dir` is `None` when processing markdown files.
pub fn process_markdown_files(
  config: &mut Config,
  processor: Option<&MarkdownProcessor>,
) -> Result<Vec<ProcessedMarkdown>> {
  process_markdown_files_impl(config, processor, None)
}

/// Processes markdown files, reusing project-local render entries when valid.
///
/// # Errors
///
/// Returns an error when Markdown sources cannot be collected, read, or
/// processed.
pub fn process_markdown_files_with_cache(
  config: &mut Config,
  processor: Option<&MarkdownProcessor>,
  cache_dir: &Path,
) -> Result<Vec<ProcessedMarkdown>> {
  process_markdown_files_impl(config, processor, Some(cache_dir))
}

fn process_markdown_files_impl(
  config: &mut Config,
  processor: Option<&MarkdownProcessor>,
  cache_dir: Option<&Path>,
) -> Result<Vec<ProcessedMarkdown>> {
  if let Some(ref input_dir) = config.input_dir {
    info!("Input directory: {}", input_dir.display());
    let mut files = collect_markdown_files(input_dir);
    files.sort();
    info!("Found {} markdown files", files.len());

    // Map: output html path -> output entry
    #[expect(clippy::items_after_statements, reason = "Local type alias")]
    type OutputEntry = (
      String,
      Vec<Header>,
      Option<String>,
      PathBuf,
      bool,
      Option<PageFrontmatter>,
    );
    let mut output_map: FxHashMap<String, OutputEntry> = FxHashMap::default();

    // Track custom outputs keyed by included file path
    let mut pending_custom_outputs: FxHashMap<PathBuf, Vec<String>> =
      FxHashMap::default();

    // Track generated output paths for included files with html:into-file.
    let mut included_output_files: FxHashMap<PathBuf, PathBuf> =
      FxHashMap::default();

    // Track all included files across all markdown files
    let mut all_included_files: FxHashMap<PathBuf, PathBuf> =
      FxHashMap::default();

    // Use provided processor or create a new one
    let base_processor = processor
      .map_or_else(|| create_processor(config, None), std::clone::Clone::clone);

    // Store render results to avoid rendering twice
    #[expect(clippy::items_after_statements, reason = "Local type alias")]
    type RenderCache = FxHashMap<PathBuf, ndg_commonmark::MarkdownResult>;
    let mut render_cache: RenderCache = FxHashMap::default();

    // Store parsed frontmatter per source file
    let mut frontmatter_cache: FxHashMap<PathBuf, Option<PageFrontmatter>> =
      FxHashMap::default();

    let cache = cache_dir.map(|dir| (dir, processor_digest(&base_processor)));
    let progress = ProgressBar::new(files.len() as u64);
    #[expect(
      clippy::expect_used,
      reason = "template string is a compile-time constant"
    )]
    let style = ProgressStyle::default_bar()
      .template(
        "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
         {pos}/{len} ({eta}) {msg}",
      )
      .expect("valid progress bar template string")
      .progress_chars("#>-");
    progress.set_style(style);
    progress.set_message("Processing markdown");
    progress.enable_steady_tick(std::time::Duration::from_millis(100));

    let rendered: Result<Vec<_>> = files
      .par_iter()
      .progress_with(progress.clone())
      .map(|file_path| {
        let raw_content =
          fs::read_to_string(file_path).wrap_err_with(|| {
            format!("Failed to read markdown file: {}", file_path.display())
          })?;
        let base_dir = file_path.parent().unwrap_or(input_dir.as_path());
        let source_digest = digest(&raw_content);
        if let Some((dir, processor_digest)) = &cache
          && let Some(entry) = read_cache(
            dir,
            file_path,
            base_dir,
            &source_digest,
            processor_digest,
          )
        {
          return Ok((file_path.clone(), entry.frontmatter, entry.result));
        }

        let (frontmatter, content) = strip_frontmatter(&raw_content);
        let result = base_processor
          .clone()
          .with_base_dir(base_dir)
          .render(&content);
        if let Some((dir, processor_digest)) = &cache {
          write_cache(
            dir,
            file_path,
            base_dir,
            &source_digest,
            processor_digest,
            &result,
            frontmatter.as_ref(),
          );
        }
        Ok((file_path.clone(), frontmatter, result))
      })
      .collect();
    progress.finish_with_message("Markdown processing complete");

    for (file_path, frontmatter, result) in rendered? {
      frontmatter_cache.insert(file_path.clone(), frontmatter);
      let base_dir = file_path.parent().unwrap_or(input_dir.as_path());

      // Track custom outputs for included files
      for inc in &result.included_files {
        if let Some(ref custom_output) = inc.custom_output {
          let inc_path = base_dir.join(&inc.path);
          if let Ok(inc_rel) = inc_path.strip_prefix(input_dir) {
            let normalized_output = normalize_custom_output_path(custom_output);
            pending_custom_outputs
              .entry(inc_rel.to_path_buf())
              .or_default()
              .push(normalized_output.to_string_lossy().to_string());
            included_output_files
              .insert(inc_rel.to_path_buf(), normalized_output);
          }
        }
      }

      // Collect all included files for this markdown file
      for inc in &result.included_files {
        let inc_path = base_dir.join(&inc.path);
        if let Ok(inc_rel) = inc_path.strip_prefix(input_dir) {
          let Ok(includer_rel) = file_path.strip_prefix(input_dir) else {
            continue;
          };

          all_included_files
            .entry(inc_rel.to_path_buf())
            .and_modify(|old| {
              // For deterministic output
              if includer_rel < old.as_path() {
                *old = includer_rel.to_path_buf();
              }
            })
            .or_insert_with(|| includer_rel.to_owned());
        }
      }

      // Cache the render result
      render_cache.insert(file_path, result);
    }

    // Second pass: use cached results to build output map
    for file_path in &files {
      // Retrieve cached result instead of rendering again
      #[expect(clippy::expect_used, reason = "File is guaranteed to be cached")]
      let result = render_cache
        .get(file_path)
        .expect("File should have cached result");

      let rel_path = file_path.strip_prefix(input_dir).wrap_err_with(|| {
        format!(
          "Failed to determine relative path for {}",
          file_path.display()
        )
      })?;
      let mut output_path = rel_path.to_path_buf();
      output_path.set_extension("html");

      // If index.use_readme is enabled and this is README.md, output to
      // index.html
      if config.use_readme_as_homepage() {
        let file_name = rel_path
          .file_name()
          .and_then(|n| n.to_str())
          .unwrap_or("")
          .to_ascii_lowercase();
        if file_name == "readme.md" {
          output_path.set_file_name("index.html");
        }
      }

      let output_path_str = output_path.to_string_lossy().to_string();

      // Check if this file is included in another file
      let is_included = all_included_files.contains_key(rel_path);

      // Get any pending custom outputs for this file
      let custom_outputs =
        pending_custom_outputs.remove(rel_path).unwrap_or_default();

      let frontmatter = frontmatter_cache.remove(file_path).flatten();

      // If this file has custom outputs via html:into-file, write to those
      // locations
      if custom_outputs.is_empty() {
        output_map.insert(
          output_path_str,
          (
            result.html.clone(),
            result.headers.clone(),
            result.title.clone(),
            file_path.clone(),
            is_included,
            frontmatter,
          ),
        );
      } else {
        for custom in custom_outputs {
          output_map.insert(
            custom.clone(),
            (
              result.html.clone(),
              result.headers.clone(),
              result.title.clone(),
              file_path.clone(),
              false,
              frontmatter.clone(),
            ),
          );
        }
      }
    }

    // Populate config.included_files so caller can use it
    config.included_files.clone_from(&all_included_files);
    config
      .included_output_files
      .clone_from(&included_output_files);

    // Convert output_map to Vec<ProcessedMarkdown>
    let mut results: Vec<ProcessedMarkdown> = output_map
      .into_iter()
      .map(
        |(
          output_path,
          (html_content, headers, title, source_path, is_included, frontmatter),
        )| {
          ProcessedMarkdown {
            html_content,
            headers,
            title,
            source_path,
            output_path,
            is_included,
            frontmatter,
          }
        },
      )
      .collect();

    // Sort for deterministic output
    results.sort_by(|a, b| a.output_path.cmp(&b.output_path));

    Ok(results)
  } else {
    info!("No input directory provided, skipping markdown processing");
    Ok(Vec::new())
  }
}

fn normalize_custom_output_path(path: &str) -> PathBuf {
  PathBuf::from(path.trim_start_matches('/'))
}

fn digest(content: impl AsRef<[u8]>) -> String {
  blake3::hash(content.as_ref()).to_hex().to_string()
}

fn digest_file(path: &Path) -> Option<String> {
  fs::read(path).ok().map(digest)
}

fn digest_tree(path: Option<&str>) -> String {
  let Some(path) = path else {
    return String::new();
  };
  let path = Path::new(path);
  if path.is_file() {
    return digest_file(path).unwrap_or_default();
  }
  let mut files: Vec<_> = walkdir::WalkDir::new(path)
    .into_iter()
    .filter_map(Result::ok)
    .filter(|entry| !entry.file_type().is_dir())
    .map(walkdir::DirEntry::into_path)
    .collect();
  files.sort();
  let mut hasher = blake3::Hasher::new();
  for file in files {
    hasher.update(file.to_string_lossy().as_bytes());
    if let Ok(content) = fs::read(&file) {
      hasher.update(&content);
    }
  }
  hasher.finalize().to_hex().to_string()
}

fn processor_digest(processor: &MarkdownProcessor) -> String {
  processor_digest_for_version(processor, env!("CARGO_PKG_VERSION"))
}

fn processor_digest_for_version(
  processor: &MarkdownProcessor,
  version: &str,
) -> String {
  let options = processor.options();
  digest(format!(
    "{MARKDOWN_CACHE_SCHEMA}|{version}|{:?}|{}|{}|{}|{}|{}",
    options,
    digest_tree(options.manpage_urls_path.as_deref()),
    digest_tree(options.syntax_queries_path.as_deref()),
    cfg!(feature = "commonmark-ndg"),
    cfg!(feature = "commonmark-nixpkgs"),
    cfg!(feature = "commonmark-nixpkgs") || cfg!(feature = "commonmark-ndg"),
  ))
}

fn cache_path(cache_dir: &Path, source: &Path) -> PathBuf {
  cache_dir.join(format!(
    "{}.json",
    digest(source.to_string_lossy().as_bytes())
  ))
}

fn read_cache(
  cache_dir: &Path,
  source: &Path,
  base_dir: &Path,
  source_digest: &str,
  processor_digest: &str,
) -> Option<MarkdownCacheEntry> {
  let entry: MarkdownCacheEntry =
    serde_json::from_slice(&fs::read(cache_path(cache_dir, source)).ok()?)
      .ok()?;
  if entry.schema != MARKDOWN_CACHE_SCHEMA
    || entry.source_digest != source_digest
    || entry.processor_digest != processor_digest
  {
    return None;
  }
  entry
    .dependencies
    .iter()
    .all(|(path, expected)| digest_file(&base_dir.join(path)) == *expected)
    .then_some(entry)
}

fn write_cache(
  cache_dir: &Path,
  source: &Path,
  base_dir: &Path,
  source_digest: &str,
  processor_digest: &str,
  result: &ndg_commonmark::MarkdownResult,
  frontmatter: Option<&PageFrontmatter>,
) {
  let dependencies = result
    .included_files
    .iter()
    .map(|include| {
      let path = PathBuf::from(&include.path);
      let digest = digest_file(&base_dir.join(&path));
      (path, digest)
    })
    .collect();
  let entry = MarkdownCacheEntry {
    schema: MARKDOWN_CACHE_SCHEMA,
    source_digest: source_digest.to_owned(),
    dependencies,
    processor_digest: processor_digest.to_owned(),
    result: result.clone(),
    frontmatter: frontmatter.cloned(),
  };
  let Ok(data) = serde_json::to_vec(&entry) else {
    return;
  };
  if fs::create_dir_all(cache_dir).is_err() {
    return;
  }
  let path = cache_path(cache_dir, source);
  let temp = path.with_extension(format!("{}.tmp", std::process::id()));
  if fs::write(&temp, data).is_ok() {
    let _ = fs::rename(temp, path);
  }
}

/// Creates a markdown processor from the NDG configuration.
///
/// # Arguments
/// * `config` - The loaded configuration for documentation generation.
/// * `valid_options` - Optional set of valid option names for validation.
#[must_use]
#[expect(
  clippy::implicit_hasher,
  reason = "Standard FxHashSet sufficient for this use case"
)]
pub fn create_processor(
  config: &Config,
  valid_options: Option<FxHashSet<String>>,
) -> MarkdownProcessor {
  let tab_style = match config.tab_style.as_str() {
    "warn" => TabStyle::Warn,
    "normalize" => TabStyle::Normalize,
    _ => TabStyle::None,
  };

  let mut builder = MarkdownOptionsBuilder::new()
    .gfm(true)
    .highlight_code(config.highlight_code)
    .tab_style(tab_style);

  if let Some(mappings_path) = &config.manpage_urls_path {
    builder = builder
      .manpage_urls_path(Some(mappings_path.to_string_lossy().to_string()));
  }

  if let Some(queries_path) = &config.syntax_queries_path {
    builder = builder
      .syntax_queries_path(Some(queries_path.to_string_lossy().to_string()));
  }

  if let Some(options) = valid_options {
    builder = builder.valid_options(Some(options));
  }

  MarkdownProcessor::new(builder.build())
}

/// Extracts the page title from a markdown file.
///
/// This attempts to extract the first heading as the title, falling back to the
/// file name if no heading is found or the file cannot be read.
///
/// # Arguments
///
/// * `file_path` - The path to the markdown file.
/// * `html_path` - The corresponding HTML output path (used for fallback).
///
/// # Returns
///
/// The extracted title or the file stem as a fallback.
#[must_use]
pub fn extract_page_title(file_path: &Path, html_path: &Path) -> String {
  let default_title = html_path
    .file_stem()
    .unwrap_or_default()
    .to_string_lossy()
    .to_string();

  match fs::read_to_string(file_path) {
    Ok(content) => {
      ndg_commonmark::utils::extract_markdown_title_and_id(&content)
        .map_or(default_title, |(title, _)| title)
    },
    Err(e) => {
      log::warn!(
        "Failed to read {} for title extraction: {e}",
        file_path.display()
      );
      default_title
    },
  }
}

#[cfg(test)]
mod tests {
  #![allow(clippy::unwrap_used, reason = "Tests can unwrap")]

  use ndg_commonmark::{IncludedFile, MarkdownResult};
  use tempfile::TempDir;

  use super::*;

  #[test]
  fn cache_rejects_changed_inputs() {
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("docs");
    let cache = temp.path().join("cache");
    let source = base.join("page.md");
    let include = base.join("part.md");
    fs::create_dir_all(&base).unwrap();
    fs::write(&source, "# Page").unwrap();
    fs::write(&include, "included v1").unwrap();

    let result = MarkdownResult {
      html:           String::new(),
      headers:        Vec::new(),
      title:          None,
      included_files: vec![IncludedFile {
        path:          "part.md".to_string(),
        custom_output: None,
      }],
    };
    let source_digest = digest("# Page");
    write_cache(
      &cache,
      &source,
      &base,
      &source_digest,
      "processor-a",
      &result,
      None,
    );

    assert!(
      read_cache(&cache, &source, &base, &source_digest, "processor-a")
        .is_some(),
      "unchanged inputs should reuse the cache"
    );
    assert!(
      read_cache(&cache, &source, &base, &digest("# Changed"), "processor-a")
        .is_none(),
      "changed Markdown should invalidate the cache"
    );

    fs::write(&include, "included v2").unwrap();
    assert!(
      read_cache(&cache, &source, &base, &source_digest, "processor-a")
        .is_none(),
      "changed includes should invalidate the cache"
    );

    fs::write(&include, "included v1").unwrap();
    assert!(
      read_cache(&cache, &source, &base, &source_digest, "processor-b")
        .is_none(),
      "changed processor options should invalidate the cache"
    );
  }

  #[test]
  fn cache_rejects_a_dependency_that_appears() {
    let temp = TempDir::new().unwrap();
    let base = temp.path().join("docs");
    let cache = temp.path().join("cache");
    let source = base.join("page.md");
    fs::create_dir_all(&base).unwrap();

    let result = MarkdownResult {
      html:           String::new(),
      headers:        Vec::new(),
      title:          None,
      included_files: vec![IncludedFile {
        path:          "missing.md".to_string(),
        custom_output: None,
      }],
    };
    let source_digest = digest("# Page");
    write_cache(
      &cache,
      &source,
      &base,
      &source_digest,
      "processor",
      &result,
      None,
    );

    assert!(
      read_cache(&cache, &source, &base, &source_digest, "processor").is_some()
    );
    fs::write(base.join("missing.md"), "now available").unwrap();
    assert!(
      read_cache(&cache, &source, &base, &source_digest, "processor").is_none(),
      "creating a previously missing include should invalidate the cache"
    );
  }

  #[test]
  fn processor_digest_tracks_query_files() {
    let temp = TempDir::new().unwrap();
    let queries = temp.path().join("queries");
    fs::create_dir_all(&queries).unwrap();
    fs::write(queries.join("highlights.scm"), "query v1").unwrap();
    let config = Config {
      syntax_queries_path: Some(queries.clone()),
      ..Config::default()
    };
    let processor = create_processor(&config, None);
    let before = processor_digest(&processor);

    fs::write(queries.join("highlights.scm"), "query v2").unwrap();

    assert_ne!(
      before,
      processor_digest(&processor),
      "changed syntax queries should invalidate the cache"
    );
  }

  #[test]
  fn processor_digest_tracks_ndg_version() {
    let processor = create_processor(&Config::default(), None);

    assert_ne!(
      processor_digest_for_version(&processor, "2.9.0"),
      processor_digest_for_version(&processor, "2.10.0"),
      "upgrading NDG should invalidate the cache"
    );
  }
}
