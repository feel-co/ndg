<!-- markdownlint-disable MD024 -->

# Sidebar Configuration

NDG supports flexible sidebar customization through the `sidebar` configuration
section, allowing you to control the appearance, ordering, and content of
navigation items in your generated documentation.

## Configuration Structure

Sidebar behavior is configured through the `[sidebar]` section in your
`ndg.toml` file or via the equivalent JSON structure.

```toml
[sidebar]
numbered = false
number_special_files = false
ordering = "alphabetical"

[[sidebar.matches]]
# Pattern-based matching rules

[sidebar.options]
# NixOS options-specific configuration
```

## Top-Level Options

### `numbered`

Controls whether sidebar items are numbered sequentially.

- **Type**: Boolean
- **Default**: `false`

When enabled, items display as "1. Title", "2. Title", etc. Special files
(`index.md`, `README.md`) are excluded from numbering by default unless
`number_special_files` is enabled.

**Example:**

```toml
[sidebar]
numbered = true
```

Result:

```plaintext
Getting Started
1. Installation
2. Configuration
3. Usage
```

### `number_special_files`

Controls whether special files (`index.md`, `README.md`) are included in the
numbering sequence. Only has effect when `numbered = true`.

- **Type**: Boolean
- **Default**: `false`

**Example with `number_special_files = false`:**

```plaintext
Getting Started  # index.md - not numbered
1. Installation
2. Configuration
```

**Example with `number_special_files = true`:**

```plaintext
1. Getting Started  # index.md - numbered
2. Installation
3. Configuration
```

### `ordering`

Determines how sidebar items are sorted.

- **Type**: String
- **Default**: `"alphabetical"`
- **Options**:
  - `"alphabetical"` - Sort items alphabetically by title
  - `"filesystem"` - Preserve filesystem order
  - `"custom"` - Sort by the `position` field from pattern matches

## Pattern Matching

The `matches` array contains pattern-based rules for customizing specific
sidebar items. Rules are evaluated in order, and the **first matching rule
wins**.

### Match Rule Structure

Each `[[sidebar.matches]]` entry supports:

#### Matching Fields

<!-- markdownlint-disable MD013 -->

| Field         | Type   | Description                                               |
| ------------- | ------ | --------------------------------------------------------- |
| `path`        | String | Exact path match (shorthand for `path.exact`)             |
| `path.exact`  | String | Exact path match (e.g., `"getting-started.md"`)           |
| `path.regex`  | String | Regex pattern for path matching (e.g., `"^api/.*\\.md$"`) |
| `title`       | String | Exact title match (shorthand for `title.exact`)           |
| `title.exact` | String | Exact title match (e.g., `"Getting Started"`)             |
| `title.regex` | String | Regex pattern for title matching (e.g., `"^API.*"`)       |

<!-- markdownlint-enable MD013 -->

#### Action Fields

| Field       | Type    | Description                                       |
| ----------- | ------- | ------------------------------------------------- |
| `new_title` | String  | Custom title to display in the sidebar            |
| `position`  | Integer | Custom position (used when `ordering = "custom"`) |

### Matching Logic

- All specified matching conditions must be satisfied (AND logic)
- If both `path` and `title` are specified, both must match
- If both `exact` and `regex` are specified for the same field, both must match
- Only the first matching rule is applied to each item

### Shorthand Syntax

For exact matches, shorthand syntax is available:

```toml
# Shorthand (recommended for exact matches)
[[sidebar.matches]]
path = "installation.md"
position = 1

# Full nested syntax (required for regex)
[[sidebar.matches]]
path.regex = "^api/.*\\.md$"
position = 2

# Mixed styles allowed
[[sidebar.matches]]
path = "getting-started.md"
title.regex = ".*Guide.*"
new_title = "Setup Guide"
```

## Examples

### Basic Numbering

```toml
[sidebar]
numbered = true
```

### Custom Ordering

```toml
[sidebar]
numbered = true
ordering = "custom"

[[sidebar.matches]]
path = "getting-started.md"
position = 1

[[sidebar.matches]]
path = "installation.md"
position = 2

[[sidebar.matches]]
path = "api-reference.md"
position = 3
```

### Custom Titles

```toml
[sidebar]
numbered = false

[[sidebar.matches]]
path = "getting-started.md"
new_title = "🚀 Quick Start"

[[sidebar.matches]]
path = "api-reference.md"
new_title = "📚 API Reference"
```

### Regex Pattern Matching

```toml
[sidebar]
ordering = "custom"

# Match all API documentation files
[[sidebar.matches]]
path.regex = "^api/.*\\.md$"
new_title = "API Documentation"
position = 50

# Match files with "Release" in the title
[[sidebar.matches]]
title.regex = "^Release.*"
new_title = "What's New"
position = 999
```

### Combined Conditions

```toml
[[sidebar.matches]]
path.regex = "^api/.*\\.md$"
title = "API Functions"
new_title = "API: Core Functions"
position = 50
```

This rule matches files that are both in the `api/` directory AND have the exact
title "API Functions".

## Options Sidebar

The `sidebar.options` section provides specialized configuration for NixOS
module options, controlling how options appear in the options table of contents.

### Structure

```toml
[sidebar.options]
depth = 2
ordering = "alphabetical"

[[sidebar.options.matches]]
# Option-specific matching rules
```

### Options Fields

#### `depth`

Controls the grouping depth for option categories.

- **Type**: Integer
- **Default**: `2`

A depth of 2 groups options by their first two components (e.g.,
`services.nginx` groups all `services.nginx.*` options).

**Example with `depth = 2`:**

```plaintext
- services.nginx (contains nginx.enable, nginx.package, nginx.virtualHosts.*)
- programs.git (contains git.enable, git.package, git.config.*)
```

**Example with `depth = 3`:**

```plaintext
- services.nginx.virtualHosts (contains only virtualHosts.* options)
- services.nginx (contains nginx.enable, nginx.package)
```

#### `ordering`

Determines how options are sorted in the table of contents.

- **Type**: String
- **Default**: `"alphabetical"`
- **Options**: `"alphabetical"`, `"custom"`, `"filesystem"`

### Option Pattern Matching

Each `[[sidebar.options.matches]]` entry supports:

#### Matching Fields

<!-- markdownlint-disable MD013 -->

| Field        | Type   | Description                                              |
| ------------ | ------ | -------------------------------------------------------- |
| `name`       | String | Exact option name match (shorthand for `name.exact`)     |
| `name.exact` | String | Exact option name (e.g., `"services.nginx.enable"`)      |
| `name.regex` | String | Regex pattern for option name (e.g., `"^internal\\..*"`) |

<!-- markdownlint-enable MD013 -->

#### Action Fields

| Field      | Type    | Description                                       |
| ---------- | ------- | ------------------------------------------------- |
| `new_name` | String  | Custom display name for the option or category    |
| `depth`    | Integer | Custom grouping depth for this specific option    |
| `position` | Integer | Custom position (used when `ordering = "custom"`) |
| `hidden`   | Boolean | Hide this option from the TOC (default: `false`)  |

### Shorthand Syntax

```toml
# Shorthand for exact matches
[[sidebar.options.matches]]
name = "services.nginx.enable"
position = 1

# Full nested syntax for regex
[[sidebar.options.matches]]
name.regex = "^internal\\..*"
hidden = true
```

### Options Examples

#### Hiding Internal Options

```toml
[sidebar.options]
depth = 2

# Hide all internal.* options
[[sidebar.options.matches]]
name.regex = "^internal\\..*"
hidden = true

# Hide module system internals
[[sidebar.options.matches]]
name = "_module.args"
hidden = true
```

#### Custom Display Names

```toml
[sidebar.options]
depth = 2

[[sidebar.options.matches]]
name = "programs.git"
new_name = "Git Configuration"

[[sidebar.options.matches]]
name = "services.nginx"
new_name = "NGINX Web Server"
```

#### Custom Ordering

```toml
[sidebar.options]
ordering = "custom"
depth = 2

# Prioritize important options
[[sidebar.options.matches]]
name = "networking.firewall"
new_name = "Firewall Settings"
position = 1

[[sidebar.options.matches]]
name = "services.openssh"
new_name = "SSH Server"
position = 2

# Group remaining services
[[sidebar.options.matches]]
name.regex = "^services\\..*"
position = 100
```

#### Per-Option Depth Override

```toml
[sidebar.options]
depth = 2

# Use deeper grouping for complex hierarchies
[[sidebar.options.matches]]
name.regex = "^services\\.nginx\\..*"
depth = 3

# Use shallower grouping for simple options
[[sidebar.options.matches]]
name.regex = "^programs\\..*"
depth = 1
```

Result:

```plaintext
# Default depth = 2:
- services.nginx (all nginx.* options)

# With depth = 3 override:
- services.nginx.virtualHosts (just virtualHosts.* options)
- services.nginx.upstreams (just upstreams.* options)
- services.nginx (remaining nginx.* options)

# With depth = 1 override:
- programs (all programs.* options together)
```

#### Combined Example

```toml
[sidebar.options]
depth = 2
ordering = "custom"

# Hide internals
[[sidebar.options.matches]]
name.regex = "^internal\\..*"
hidden = true

# Important options with custom names and positions
[[sidebar.options.matches]]
name = "networking.firewall"
new_name = "🔥 Firewall"
position = 1

[[sidebar.options.matches]]
name = "services.openssh"
new_name = "🔐 SSH Server"
position = 2

# Deeper grouping for nginx
[[sidebar.options.matches]]
name.regex = "^services\\.nginx\\..*"
new_name = "🌐 NGINX Web Server"
depth = 3
position = 10

# Group other services
[[sidebar.options.matches]]
name.regex = "^services\\..*"
position = 50
```

## JSON Configuration

The sidebar configuration can be specified in JSON format with either shorthand
or nested syntax:

**Shorthand syntax:**

```json
{
  "sidebar": {
    "numbered": true,
    "ordering": "custom",
    "matches": [
      {
        "path": "getting-started.md",
        "new_title": "🚀 Quick Start",
        "position": 1
      },
      {
        "path": "installation.md",
        "new_title": "📦 Installation",
        "position": 2
      }
    ]
  }
}
```

**Nested syntax (required for regex):**

```json
{
  "sidebar": {
    "numbered": true,
    "ordering": "custom",
    "matches": [
      {
        "path": {
          "exact": "getting-started.md"
        },
        "new_title": "🚀 Quick Start",
        "position": 1
      },
      {
        "path": {
          "regex": "^api/.*\\.md$"
        },
        "position": 2
      }
    ]
  }
}
```

**Options configuration:**

```json
{
  "sidebar": {
    "options": {
      "depth": 2,
      "ordering": "custom",
      "matches": [
        {
          "name": {
            "regex": "^internal\\..*"
          },
          "hidden": true
        },
        {
          "name": "programs.git",
          "new_name": "Git Configuration",
          "position": 5
        }
      ]
    }
  }
}
```
