#![allow(clippy::expect_used, reason = "Expect is acceptable in tests")]

use std::fs;

use ndg_manpage::generate_manpage;
use serde_json::json;
use tempfile::tempdir;

#[test]
fn test_manpage_declared_by_formatting() {
  let temp_dir = tempdir().expect("tempdir");
  let options_path = temp_dir.path().join("options.json");
  let output_path = temp_dir.path().join("out.5");

  let options = json!({
      "services.test": {
          "type": "string",
          "description": "Test option",
          "declarations": "modules/test/module.nix",
          "declarationURL": "https://example.com/test"
      },
      "services.no_url": {
          "type": "string",
          "description": "Another option",
          "declarations": "modules/other/module.nix"
      }
  });

  fs::write(&options_path, options.to_string()).expect("write options");

  generate_manpage(
    &options_path,
    Some(&output_path),
    Some("Module Options"),
    None,
    None,
    5,
  )
  .expect("generate manpage");

  let output = fs::read_to_string(&output_path).expect("read manpage");

  assert!(output.contains("\\fIDeclared by:\\fP"));
  assert!(output.contains(
    "\\fB<modules/test/module\\&.nix>\\fP (\\fIhttps://example\\&.com/test\\fP)"
  ));
  assert!(output.contains("\\fB<modules/other/module\\&.nix>\\fP"));
}

#[test]
fn test_manpage_list_rendering() {
  let temp_dir = tempdir().expect("tempdir");
  let options_path = temp_dir.path().join("options.json");
  let output_path = temp_dir.path().join("out.5");

  let options = json!({
      "services.listed": {
          "type": "string",
          "description": "- first bullet\n- second bullet\n1. first number\n2. second number"
      }
  });

  fs::write(&options_path, options.to_string()).expect("write options");

  generate_manpage(
    &options_path,
    Some(&output_path),
    Some("Lists"),
    None,
    None,
    5,
  )
  .expect("generate manpage");

  let output = fs::read_to_string(&output_path).expect("read manpage");

  assert!(output.contains(".IP \"\\[u2022]\" 4\nfirst bullet"));
  assert!(output.contains(".IP \"\\[u2022]\" 4\nsecond bullet"));
  assert!(output.contains(".IP \"1.\" 4\nfirst number"));
  assert!(output.contains(".IP \"2.\" 4\nsecond number"));
}

#[test]
fn test_manpage_leading_dot_escape() {
  let temp_dir = tempdir().expect("tempdir");
  let options_path = temp_dir.path().join("options.json");
  let output_path = temp_dir.path().join("out.5");

  let options = json!({
      "services.dotted": {
          "type": "string",
          "description": ".starts with dot\n'another leading quote"
      }
  });

  fs::write(&options_path, options.to_string()).expect("write options");

  generate_manpage(
    &options_path,
    Some(&output_path),
    Some("Dots"),
    None,
    None,
    5,
  )
  .expect("generate manpage");

  let output = fs::read_to_string(&output_path).expect("read manpage");

  assert!(output.contains("\\&.starts with dot"));

  assert!(
    output.contains("\\(aqanother leading quote")
      || output.contains("\\&\\[aq]another leading quote")
      || output.contains("\\&'another leading quote")
  );
}
