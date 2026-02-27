use std::collections::HashMap;

pub const DEFAULT_TEMPLATE: &str = include_str!("../templates/default.html");
pub const OPTIONS_TEMPLATE: &str = include_str!("../templates/options.html");
pub const SEARCH_TEMPLATE: &str = include_str!("../templates/search.html");
pub const OPTIONS_TOC_TEMPLATE: &str =
  include_str!("../templates/options_toc.html");
pub const NAVBAR_TEMPLATE: &str = include_str!("../templates/navbar.html");
pub const FOOTER_TEMPLATE: &str = include_str!("../templates/footer.html");
pub const LIB_TEMPLATE: &str = include_str!("../templates/lib.html");

pub const DEFAULT_CSS: &str = include_str!("../templates/default.css");
pub const SEARCH_JS: &str = include_str!("../templates/search.js");
pub const SEARCH_WORKER_JS: &str =
  include_str!("../templates/search-worker.js");
pub const MAIN_JS: &str = include_str!("../templates/main.js");

#[must_use]
pub fn all_templates() -> HashMap<&'static str, &'static str> {
  let mut templates = HashMap::new();
  templates.insert("default.html", DEFAULT_TEMPLATE);
  templates.insert("options.html", OPTIONS_TEMPLATE);
  templates.insert("search.html", SEARCH_TEMPLATE);
  templates.insert("options_toc.html", OPTIONS_TOC_TEMPLATE);
  templates.insert("navbar.html", NAVBAR_TEMPLATE);
  templates.insert("footer.html", FOOTER_TEMPLATE);
  templates.insert("lib.html", LIB_TEMPLATE);
  templates.insert("default.css", DEFAULT_CSS);
  templates.insert("search.js", SEARCH_JS);
  templates.insert("search-worker.js", SEARCH_WORKER_JS);
  templates.insert("main.js", MAIN_JS);
  templates
}
