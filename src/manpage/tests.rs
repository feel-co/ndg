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

    // Test role formatting with properly escaped dash (\-)
    assert!(result.contains("\\fBnix.conf\\fR(5)"));
    assert!(result.contains("\\fBrm \\-rfi\\fR"));
    assert!(result.contains("\\fIXDG_DATA_DIRS\\fR"));
    assert!(result.contains("\\fI/etc/passwd\\fR"));
    assert!(result.contains("\\fBnetworking.useDHCP\\fR"));

    // Test list items formatting
    assert!(result.contains(".IP \\(bu 2\n\\fBrm \\-rfi\\fR"));
    assert!(!result.contains(".IP \\(bu 2\n\\fBrm \\-rfi\\fR.IP"));

    // Check admonition formatting with proper paragraph formatting
    assert!(result.contains(".PP\n\\fBWarning:\\fR This is a warning"));
    assert!(result.contains(".PP\n\\fBImportant:\\fR This is important"));

    // Multi-paragraph admonition
    assert!(result.contains("\\fBNote:\\fR This is a note"));
    assert!(result.contains(".PP\nblah blah blah blah blah and blah"));

    assert!(result.contains(".PP\n\\fBTip:\\fR This is a tip"));
    assert!(result.contains(".PP\n\\fBExample:\\fR Title for this example"));

    // Validate no inline paragraph directives
    assert!(!result.contains("\\fBNote:\\fR This is a note .PP"));
    assert!(!result.contains("Example: .PP"));

    // Ensure proper troff formatting codes (single backslashes)
    assert!(!result.contains("\\\\fB"));
    assert!(!result.contains("\\\\fI"));
    assert!(!result.contains("\\\\fR"));

    // Make sure sections are properly formatted
    assert!(result.contains(".SH \"Test Document\""));
    assert!(result.contains(".SS \"Roles\""));
    assert!(result.contains(".SS \"Admonitions\""));
}
