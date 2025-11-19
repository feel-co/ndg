/// State tracking for code fence detection in markdown.
///
/// This tracks whether we're currently inside a fenced code block  and
/// maintains the fence character and count for proper closing detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FenceTracker {
  in_code_block:    bool,
  code_fence_char:  Option<char>,
  code_fence_count: usize,
}

impl FenceTracker {
  /// Create a new fence tracker.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      in_code_block:    false,
      code_fence_char:  None,
      code_fence_count: 0,
    }
  }

  /// Check if currently inside a code block.
  #[must_use]
  pub const fn in_code_block(&self) -> bool {
    self.in_code_block
  }

  /// Process a line and update fence state.
  ///
  /// Returns the updated state after processing the line.
  /// Call this for each line to maintain accurate fence tracking.
  #[must_use]
  pub fn process_line(&self, line: &str) -> Self {
    let trimmed = line.trim_start();

    // Check for code fences (``` or ~~~)
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
      // Get the first character to determine fence type
      let Some(fence_char) = trimmed.chars().next() else {
        // Empty string after trim - no state change
        return *self;
      };

      let fence_count =
        trimmed.chars().take_while(|&c| c == fence_char).count();

      if fence_count >= 3 {
        if !self.in_code_block {
          // Starting a code block
          return Self {
            in_code_block:    true,
            code_fence_char:  Some(fence_char),
            code_fence_count: fence_count,
          };
        } else if self.code_fence_char == Some(fence_char)
          && fence_count >= self.code_fence_count
        {
          // Ending a code block
          return Self {
            in_code_block:    false,
            code_fence_char:  None,
            code_fence_count: 0,
          };
        }
      }
    }

    // No state change
    *self
  }
}

/// State tracking for code fences AND inline code in markdown.
///
/// This extends `FenceTracker` to also track inline code spans (`code`).
/// This is needed for character-level processing where inline code must be
/// skipped along with fenced code blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InlineTracker {
  in_code_block:  bool,
  in_inline_code: bool,
  fence_char:     Option<char>,
  fence_count:    usize,
}

impl InlineTracker {
  /// Create a new inline code tracker.
  #[must_use]
  pub const fn new() -> Self {
    Self {
      in_code_block:  false,
      in_inline_code: false,
      fence_char:     None,
      fence_count:    0,
    }
  }

  /// Check if currently inside any kind of code (block or inline).
  #[must_use]
  pub const fn in_any_code(&self) -> bool {
    self.in_code_block || self.in_inline_code
  }

  /// Check if currently inside a code block.
  #[must_use]
  pub const fn in_code_block(&self) -> bool {
    self.in_code_block
  }

  /// Check if currently inside inline code.
  #[must_use]
  pub const fn in_inline_code(&self) -> bool {
    self.in_inline_code
  }

  /// Process backticks and update state.
  ///
  /// Returns (new_state, number_of_backticks_consumed).
  #[must_use]
  pub fn process_backticks<I>(&self, chars: &mut I) -> (Self, usize)
  where
    I: Iterator<Item = char> + Clone,
  {
    let mut tick_count = 1; // we've already seen the first backtick
    let mut temp_chars = chars.clone();

    // Count consecutive backticks
    while temp_chars.next() == Some('`') {
      tick_count += 1;
    }

    // Actually consume the backticks from the iterator
    for _ in 1..tick_count {
      chars.next();
    }

    if tick_count >= 3 {
      // This is a code fence
      if !self.in_code_block {
        // Starting a code block
        (
          Self {
            in_code_block:  true,
            in_inline_code: false, // clear inline code when entering block
            fence_char:     Some('`'),
            fence_count:    tick_count,
          },
          tick_count,
        )
      } else if self.fence_char == Some('`') && tick_count >= self.fence_count {
        // Ending a code block
        (
          Self {
            in_code_block:  false,
            in_inline_code: false,
            fence_char:     None,
            fence_count:    0,
          },
          tick_count,
        )
      } else {
        // Inside a different fence type, no state change
        (*self, tick_count)
      }
    } else if tick_count == 1 && !self.in_code_block {
      // Single backtick - inline code toggle
      (
        Self {
          in_inline_code: !self.in_inline_code,
          ..*self
        },
        tick_count,
      )
    } else {
      // Multiple backticks but less than 3, or inside code block
      (*self, tick_count)
    }
  }

  /// Process tildes and update state.
  ///
  /// Returns (new_state, number_of_tildes_consumed).
  #[must_use]
  pub fn process_tildes<I>(&self, chars: &mut I) -> (Self, usize)
  where
    I: Iterator<Item = char> + Clone,
  {
    let mut tilde_count = 1; // we've already seen the first tilde
    let mut temp_chars = chars.clone();

    // Count consecutive tildes
    while temp_chars.next() == Some('~') {
      tilde_count += 1;
    }

    // Actually consume the tildes from the iterator
    for _ in 1..tilde_count {
      chars.next();
    }

    if tilde_count >= 3 {
      if !self.in_code_block {
        // Starting a tilde code block
        (
          Self {
            in_code_block:  true,
            in_inline_code: false, // clear inline code when entering block
            fence_char:     Some('~'),
            fence_count:    tilde_count,
          },
          tilde_count,
        )
      } else if self.fence_char == Some('~') && tilde_count >= self.fence_count
      {
        // Ending a tilde code block
        (
          Self {
            in_code_block:  false,
            in_inline_code: false,
            fence_char:     None,
            fence_count:    0,
          },
          tilde_count,
        )
      } else {
        // Inside a different fence type, no state change
        (*self, tilde_count)
      }
    } else {
      // Less than 3 tildes, no state change
      (*self, tilde_count)
    }
  }

  /// Process a newline and update state.
  ///
  /// Newlines end inline code if not properly closed.
  #[must_use]
  pub const fn process_newline(&self) -> Self {
    Self {
      in_inline_code: false,
      ..*self
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_fence_tracker_basic() {
    let tracker = FenceTracker::new();
    assert!(!tracker.in_code_block());

    // Opening fence
    let tracker = tracker.process_line("```rust");
    assert!(tracker.in_code_block());

    // Inside code block
    let tracker = tracker.process_line("fn main() {}");
    assert!(tracker.in_code_block());

    // Closing fence
    let tracker = tracker.process_line("```");
    assert!(!tracker.in_code_block());
  }

  #[test]
  fn test_fence_tracker_tilde() {
    let tracker = FenceTracker::new();

    // Tilde fence
    let tracker = tracker.process_line("~~~");
    assert!(tracker.in_code_block());

    let tracker = tracker.process_line("code");
    assert!(tracker.in_code_block());

    let tracker = tracker.process_line("~~~");
    assert!(!tracker.in_code_block());
  }

  #[test]
  fn test_fence_tracker_mismatched() {
    let tracker = FenceTracker::new();

    // Backtick fence
    let tracker = tracker.process_line("```");
    assert!(tracker.in_code_block());

    // Tilde doesn't close backtick fence
    let tracker = tracker.process_line("~~~");
    assert!(tracker.in_code_block());

    // Backtick closes
    let tracker = tracker.process_line("```");
    assert!(!tracker.in_code_block());
  }

  #[test]
  fn test_fence_tracker_count() {
    let tracker = FenceTracker::new();

    // 4 backticks
    let tracker = tracker.process_line("````");
    assert!(tracker.in_code_block());

    // 3 backticks don't close 4-backtick fence
    let tracker = tracker.process_line("```");
    assert!(tracker.in_code_block());

    // 4+ backticks close
    let tracker = tracker.process_line("````");
    assert!(!tracker.in_code_block());
  }

  #[test]
  fn test_fence_tracker_indented() {
    let tracker = FenceTracker::new();

    // Indented fence (trim_start handles this)
    let tracker = tracker.process_line("    ```");
    assert!(tracker.in_code_block());

    let tracker = tracker.process_line("    ```");
    assert!(!tracker.in_code_block());
  }

  #[test]
  fn test_inline_code_tracker_basic() {
    let tracker = InlineTracker::new();
    assert!(!tracker.in_any_code());

    // Single backtick - start inline code
    let mut chars = "rest".chars();
    let (tracker, count) = tracker.process_backticks(&mut chars);
    assert_eq!(count, 1);
    assert!(tracker.in_inline_code());
    assert!(tracker.in_any_code());

    // Another single backtick - end inline code
    let mut chars = "rest".chars();
    let (tracker, count) = tracker.process_backticks(&mut chars);
    assert_eq!(count, 1);
    assert!(!tracker.in_inline_code());
    assert!(!tracker.in_any_code());
  }

  #[test]
  fn test_inline_code_tracker_fence() {
    let tracker = InlineTracker::new();

    // Three backticks - code fence
    let mut chars = "``rust".chars();
    let (tracker, count) = tracker.process_backticks(&mut chars);
    assert_eq!(count, 3);
    assert!(tracker.in_code_block());
    assert!(!tracker.in_inline_code());

    // Single backtick inside fence - no inline code
    let mut chars = "rest".chars();
    let (tracker, _) = tracker.process_backticks(&mut chars);
    assert!(tracker.in_code_block());
    assert!(!tracker.in_inline_code());

    // Three backticks - close fence
    let mut chars = "``".chars();
    let (tracker, count) = tracker.process_backticks(&mut chars);
    assert_eq!(count, 3);
    assert!(!tracker.in_code_block());
    assert!(!tracker.in_inline_code());
  }

  #[test]
  fn test_inline_code_tracker_tildes() {
    let tracker = InlineTracker::new();

    // Three tildes - code fence
    let mut chars = "~~".chars();
    let (tracker, count) = tracker.process_tildes(&mut chars);
    assert_eq!(count, 3);
    assert!(tracker.in_code_block());

    // Close with tildes
    let mut chars = "~~".chars();
    let (tracker, count) = tracker.process_tildes(&mut chars);
    assert_eq!(count, 3);
    assert!(!tracker.in_code_block());
  }

  #[test]
  fn test_inline_code_tracker_newline() {
    let tracker = InlineTracker::new();

    // Start inline code
    let mut chars = "rest".chars();
    let (tracker, _) = tracker.process_backticks(&mut chars);
    assert!(tracker.in_inline_code());

    // Newline ends inline code
    let tracker = tracker.process_newline();
    assert!(!tracker.in_inline_code());
  }
}
