#![expect(clippy::unwrap_used, reason = "Fine in benchmarks")]

use std::{fs, hint::black_box, path::Path};

use criterion::{Criterion, criterion_group, criterion_main};
use ndg::{
  config::Config,
  utils::{
    create_processor,
    process_markdown_files,
    process_markdown_files_with_cache,
  },
};
use tempfile::TempDir;

const PAGE_COUNT: usize = 128;

fn fixture() -> TempDir {
  let dir = TempDir::new().unwrap();
  let input = dir.path().join("docs");
  fs::create_dir_all(input.join("includes/nested")).unwrap();
  fs::write(
    input.join("includes/part.md"),
    "## Included section\n\n{#included-anchor}\n\n```nix\n{ pkgs, ... }: \
     pkgs.hello\n```\n\n```{=include=}\nnested/detail.md\n```\n",
  )
  .unwrap();
  fs::write(
    input.join("includes/nested/detail.md"),
    "### Nested include\n\nA nested [reference](../page-000.md).\n",
  )
  .unwrap();
  for index in 0..PAGE_COUNT {
    fs::write(
      input.join(format!("page-{index:03}.md")),
      format!(
        r#"+++
title = "Benchmark page {index}"
tags = ["benchmark", "markdown"]
+++
# Benchmark page {index}

[Next page](page-{:03}.md)

```{{=include=}}
includes/part.md
```

## Rendering path

```nix
let value = {index}; in value + 1
```
"#,
        (index + 1) % PAGE_COUNT,
      ),
    )
    .unwrap();
  }
  dir
}

fn config(root: &Path, highlight_code: bool) -> Config {
  Config {
    input_dir: Some(root.join("docs")),
    highlight_code,
    ..Config::default()
  }
}

fn markdown(c: &mut Criterion) {
  let fixture = fixture();
  let cache = fixture.path().join("cache");
  let plain_processor = create_processor(&config(fixture.path(), false), None);
  let highlighted_processor =
    create_processor(&config(fixture.path(), true), None);
  let mut group = c.benchmark_group("markdown-site-128-pages");

  group.bench_function("plain", |b| {
    b.iter(|| {
      let mut config = config(fixture.path(), false);
      black_box(
        process_markdown_files(&mut config, Some(&plain_processor)).unwrap(),
      );
    });
  });

  {
    let mut config = config(fixture.path(), true);
    process_markdown_files_with_cache(
      &mut config,
      Some(&highlighted_processor),
      &cache,
    )
    .unwrap();
  }
  group.bench_function("warm-cache", |b| {
    b.iter(|| {
      let mut config = config(fixture.path(), true);
      black_box(
        process_markdown_files_with_cache(
          &mut config,
          Some(&highlighted_processor),
          &cache,
        )
        .unwrap(),
      );
    });
  });

  group.bench_function("highlighted", |b| {
    b.iter(|| {
      let mut config = config(fixture.path(), true);
      black_box(
        process_markdown_files(&mut config, Some(&highlighted_processor))
          .unwrap(),
      );
    });
  });
  group.finish();
}

criterion_group!(benches, markdown);
criterion_main!(benches);
