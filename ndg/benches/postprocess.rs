#![allow(
  clippy::expect_used,
  clippy::unwrap_used,
  reason = "Fine in benchmarks"
)]
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ndg::{
  config::postprocess::PostprocessConfig,
  utils::postprocess::{process_css, process_html, process_js},
};

// XXX: While we *could* dump some sample files somewhere and bench with them, I
// find it nicer (or alternatively, "cleaner") to provide a self-contained
// module with all the test documents already included. This is not *nice*, but
// *nicer* than having a directory where I have to put my HTML code, which I
// then have tp track in `.gitattributes`. Instead I copy pasted some of my
// template code back here. It works, but we could *probably* test on the
// templates instead. Maybe a future consideration.
//
// tl;dr: don't complain.
const HTML_SMALL: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <title>Test Page</title>
  </head>
  <body>
    <h1>Welcome</h1>
    <p>This is a test paragraph. You can read this. Yes.</p>
  </body>
</html>"#;

const HTML_LARGE: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Documentation</title>
    <link rel="stylesheet" href="style.css" />
    <script defer src="main.js"></script>
  </head>
  <body>
    <div class="container">
      <header>
        <div class="header-left">
          <h1 class="site-title">
            <a href="/">Documentation</a>
          </h1>
          <nav>
            <a href="/guide">Guide</a>
            <a href="/reference">Reference</a>
            <a href="/about">About</a>
          </nav>
        </div>
        <div class="search-container">
          <input type="text" id="search-input" placeholder="Search..." />
          <div id="search-results" class="search-results"></div>
        </div>
      </header>
      <div class="layout">
        <aside class="sidebar">
          <ul>
            <li><a href="/intro">Introduction</a></li>
            <li><a href="/installation">Installation</a></li>
            <li><a href="/quickstart">Quick Start</a></li>
            <li><a href="/configuration">Configuration</a></li>
          </ul>
        </aside>
        <main class="content">
          <h1>Getting Started</h1>
          <p>If you can read this, chances are you can also use this tool properly</p>
          <h2>Prerequisites</h2>
          <ul>
            <li>Computer</li>
            <li>Basic knowledge of command line</li>
            <li>Text editor</li>
            <li>Internet connection</li>
          </ul>
          <h2>Installation</h2>
          <pre><code>cargo install ndg</code></pre>
          <p>After installation, you can verify it works:</p>
          <pre><code>ndg --version</code></pre>
          <h2>Configuration</h2>
          <p>Create a configuration file:</p>
          <pre><code>[build]
input_dir = "docs"
output_dir = "build"</code></pre>
        </main>
      </div>
      <footer>
        <p>&copy; 2025 NDG Document</p>
      </footer>
    </div>
  </body>
</html>"#;

const CSS_SMALL: &str = r"
body {
  margin: 0;
  padding: 0;
  color: #333333;
}

.container {
  max-width: 1200px;
}
";

const CSS_LARGE: &str = r"
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  line-height: 1.6;
  color: #333333;
  background-color: #ffffff;
}

// Container
.container {
  max-width: 1400px;
  margin: 0 auto;
  padding: 20px;
}

.layout {
  display: flex;
  gap: 2rem;
}

// Headers
header {
  background-color: #f5f5f5;
  padding: 1rem 2rem;
  border-bottom: 1px solid #e0e0e0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 2rem;
}

.site-title {
  font-size: 1.5rem;
  font-weight: 600;
}

// Nav
nav a {
  color: #0066cc;
  text-decoration: none;
  padding: 0.5rem 1rem;
  border-radius: 4px;
  transition: background-color 0.2s;
}

nav a:hover {
  background-color: #e6f2ff;
}

// Sidebar
.sidebar {
  width: 250px;
  flex-shrink: 0;
}

.sidebar ul {
  list-style: none;
}

.sidebar li {
  margin-bottom: 0.5rem;
}

// Content
.content {
  flex: 1;
  min-width: 0;
}

.content h1 {
  font-size: 2rem;
  margin-bottom: 1rem;
  color: #222222;
}

.content h2 {
  font-size: 1.5rem;
  margin-top: 2rem;
  margin-bottom: 1rem;
  color: #333333;
}

.content p {
  margin-bottom: 1rem;
}

// Code Blocks
pre {
  background-color: #f8f8f8;
  border: 1px solid #dddddd;
  border-radius: 4px;
  padding: 1rem;
  overflow-x: auto;
}

code {
  font-family: 'Monaco', 'Menlo', 'Consolas', monospace;
  font-size: 0.9em;
}

// Footer
footer {
  margin-top: 4rem;
  padding: 2rem;
  text-align: center;
  border-top: 1px solid #e0e0e0;
  color: #666666;
}

// Media
@media (max-width: 768px) {
  .layout {
    flex-direction: column;
  }

  .sidebar {
    width: 100%;
  }
}
";

const JS_SMALL: &str = r"
function init() {
  console.log('App initialized');
}

window.addEventListener('load', init);
";

const JS_LARGE: &str = r#"
// Sidebar toggle
function initSidebar() {
  const toggle = document.querySelector('.sidebar-toggle');
  const sidebar = document.querySelector('.sidebar');

  if (toggle && sidebar) {
    toggle.addEventListener('click', function() {
      sidebar.classList.toggle('collapsed');
      localStorage.setItem('sidebar-collapsed', sidebar.classList.contains('collapsed'));
    });

    // Restore state
    const collapsed = localStorage.getItem('sidebar-collapsed');
    if (collapsed === 'true') {
      sidebar.classList.add('collapsed');
    }
  }
}

// Search
function initSearch() {
  const searchInput = document.getElementById('search-input');
  const searchResults = document.getElementById('search-results');

  if (!searchInput || !searchResults) {
    return;
  }

  let searchIndex = null;
  let debounceTimer = null;

  // Load search index
  fetch('/search-index.json')
    .then(response => response.json())
    .then(data => {
      searchIndex = data;
    })
    .catch(error => {
      console.error('Failed to load search index:', error);
    });

  searchInput.addEventListener('input', function(e) {
    clearTimeout(debounceTimer);

    const query = e.target.value.trim().toLowerCase();

    if (query.length < 2) {
      searchResults.innerHTML = '';
      searchResults.style.display = 'none';
      return;
    }

    debounceTimer = setTimeout(function() {
      if (!searchIndex) {
        return;
      }

      const results = searchIndex.filter(function(item) {
        return item.title.toLowerCase().includes(query) ||
               item.content.toLowerCase().includes(query);
      }).slice(0, 10);

      if (results.length > 0) {
        searchResults.innerHTML = results.map(function(result) {
          return '<div class="search-result">' +
                 '<a href="' + result.url + '">' + result.title + '</a>' +
                 '</div>';
        }).join('');
        searchResults.style.display = 'block';
      } else {
        searchResults.innerHTML = '<div class="no-results">No results found</div>';
        searchResults.style.display = 'block';
      }
    }, 300);
  });

  // Close search results when clicking outside
  document.addEventListener('click', function(e) {
    if (!searchInput.contains(e.target) && !searchResults.contains(e.target)) {
      searchResults.style.display = 'none';
    }
  });
}

// Copy code button
function initCodeCopy() {
  const codeBlocks = document.querySelectorAll('pre code');

  codeBlocks.forEach(function(block) {
    const button = document.createElement('button');
    button.className = 'copy-button';
    button.textContent = 'Copy';

    button.addEventListener('click', function() {
      const code = block.textContent;
      navigator.clipboard.writeText(code).then(function() {
        button.textContent = 'Copied!';
        setTimeout(function() {
          button.textContent = 'Copy';
        }, 2000);
      });
    });

    block.parentElement.appendChild(button);
  });
}

// Initialize everything
window.addEventListener('DOMContentLoaded', function() {
  initSidebar();
  initSearch();
  initCodeCopy();
});
"#;

fn bench_html_minification(c: &mut Criterion) {
  let mut group = c.benchmark_group("html_minification");

  let config_disabled = PostprocessConfig {
    minify_html: false,
    ..Default::default()
  };

  let config_enabled = PostprocessConfig {
    minify_html: true,
    ..Default::default()
  };

  // Small HTML benchmarks
  group.bench_with_input(
    BenchmarkId::new("disabled", "small"),
    &HTML_SMALL,
    |b, html| {
      b.iter(|| {
        process_html(black_box(html), black_box(&config_disabled)).unwrap()
      });
    },
  );

  group.bench_with_input(
    BenchmarkId::new("enabled", "small"),
    &HTML_SMALL,
    |b, html| {
      b.iter(|| {
        process_html(black_box(html), black_box(&config_enabled)).unwrap()
      });
    },
  );

  // Large HTML benchmarks
  group.bench_with_input(
    BenchmarkId::new("disabled", "large"),
    &HTML_LARGE,
    |b, html| {
      b.iter(|| {
        process_html(black_box(html), black_box(&config_disabled)).unwrap()
      });
    },
  );

  group.bench_with_input(
    BenchmarkId::new("enabled", "large"),
    &HTML_LARGE,
    |b, html| {
      b.iter(|| {
        process_html(black_box(html), black_box(&config_enabled)).unwrap()
      });
    },
  );

  group.finish();
}

fn bench_css_minification(c: &mut Criterion) {
  let mut group = c.benchmark_group("css_minification");

  let config_disabled = PostprocessConfig {
    minify_css: false,
    ..Default::default()
  };

  let config_enabled = PostprocessConfig {
    minify_css: true,
    ..Default::default()
  };

  // Small CSS benchmarks
  group.bench_with_input(
    BenchmarkId::new("disabled", "small"),
    &CSS_SMALL,
    |b, css| {
      b.iter(|| {
        process_css(black_box(css), black_box(&config_disabled)).unwrap()
      });
    },
  );

  group.bench_with_input(
    BenchmarkId::new("enabled", "small"),
    &CSS_SMALL,
    |b, css| {
      b.iter(|| {
        process_css(black_box(css), black_box(&config_enabled)).unwrap()
      });
    },
  );

  // Large CSS benchmarks
  group.bench_with_input(
    BenchmarkId::new("disabled", "large"),
    &CSS_LARGE,
    |b, css| {
      b.iter(|| {
        process_css(black_box(css), black_box(&config_disabled)).unwrap()
      });
    },
  );

  group.bench_with_input(
    BenchmarkId::new("enabled", "large"),
    &CSS_LARGE,
    |b, css| {
      b.iter(|| {
        process_css(black_box(css), black_box(&config_enabled)).unwrap()
      });
    },
  );

  group.finish();
}

fn bench_js_minification(c: &mut Criterion) {
  let mut group = c.benchmark_group("js_minification");

  let config_disabled = PostprocessConfig {
    minify_js: false,
    ..Default::default()
  };

  let config_enabled = PostprocessConfig {
    minify_js: true,
    ..Default::default()
  };

  // Small JS benchmarks
  group.bench_with_input(
    BenchmarkId::new("disabled", "small"),
    &JS_SMALL,
    |b, js| {
      b.iter(|| {
        process_js(black_box(js), black_box(&config_disabled)).unwrap()
      });
    },
  );

  group.bench_with_input(
    BenchmarkId::new("enabled", "small"),
    &JS_SMALL,
    |b, js| {
      b.iter(|| process_js(black_box(js), black_box(&config_enabled)).unwrap());
    },
  );

  // Large JS benchmarks
  group.bench_with_input(
    BenchmarkId::new("disabled", "large"),
    &JS_LARGE,
    |b, js| {
      b.iter(|| {
        process_js(black_box(js), black_box(&config_disabled)).unwrap()
      });
    },
  );

  group.bench_with_input(
    BenchmarkId::new("enabled", "large"),
    &JS_LARGE,
    |b, js| {
      b.iter(|| process_js(black_box(js), black_box(&config_enabled)).unwrap());
    },
  );

  group.finish();
}

criterion_group!(
  benches,
  bench_html_minification,
  bench_css_minification,
  bench_js_minification
);
criterion_main!(benches);
