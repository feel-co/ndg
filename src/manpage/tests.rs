use super::*;

#[test]
fn test_manpage_formatting() {
    let markdown = r"
# Test Document

## Roles
Link to a man page: {manpage}`nix.conf(5)`

Other literals:

* {command}`rm -rfi`
* {env}`XDG_DATA_DIRS`
* {file}`/etc/passwd`
* {option}`networking.useDHCP`
* {var}`/etc/passwd`

## Admonitions

::: {.warning}
This is a warning
:::

::: {.important}
This is important
:::

::: {.note}
This is a note

blah blah blah blah blah and blah
:::

::: {.tip}
This is a tip
:::

::: {.example}
Title for this example
:::
";

    let result = markdown_to_manpage(markdown, "Test", 1, "Test Manual").unwrap();

    // Print the result to debug
    println!("RESULT:\n{result}");

    // Test that list items are properly separated with correct formatting
    assert!(result.contains("\\fBrm -rfi\\fR") || result.contains("\\fBrm \\-rfi\\fR"));
    assert!(result.contains("\\fIXDG_DATA_DIRS\\fR"));

    // Make sure items don't run together
    assert!(!result.contains(".IP \\(bu 2\n\\fBrm -rfi\\fR.IP"));
    assert!(!result.contains(".IP \\(bu 2\n\\fIXDG_DATA_DIRS\\fR.IP"));

    // Check for proper admonition formatting - indented blocks with bold titles
    assert!(result.contains("\\fBWarning\\fR\n.br\nThis is a warning"));

    // Check for proper troff formatting codes (single backslashes)
    assert!(!result.contains("\\\\fB"));
    assert!(!result.contains("\\\\fI"));
    assert!(!result.contains("\\\\fR"));
}
