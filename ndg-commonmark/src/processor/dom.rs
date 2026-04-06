/// Safely select DOM elements with graceful error handling.
pub(super) fn safe_select(
  document: &kuchikikiki::NodeRef,
  selector: &str,
) -> Vec<kuchikikiki::NodeRef> {
  match document.select(selector) {
    Ok(selections) => selections.map(|sel| sel.as_node().clone()).collect(),
    Err(e) => {
      log::warn!("DOM selector '{selector}' failed: {e:?}");
      Vec::new()
    },
  }
}
