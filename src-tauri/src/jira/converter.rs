//! HTML → Markdown conversion for Jira issue descriptions.
//!
//! Jira Cloud returns the description as Atlassian Document Format (ADF).
//! Rather than parsing ADF ourselves, we request the rendered HTML via the
//! API's `renderedFields` and convert it here. This reuses Atlassian's own
//! renderer, preserving tables, code blocks, lists, and links.

/// Convert a Jira rendered-HTML description to GitHub-Flavored Markdown.
///
/// Returns an empty string for empty/blank input so callers can store
/// `None` downstream.
pub fn html_to_markdown(html: &str) -> Option<String> {
    let trimmed = html.trim();
    if trimmed.is_empty() {
        return None;
    }
    match htmd::convert(trimmed) {
        Ok(md) => {
            let md = md.trim();
            if md.is_empty() {
                None
            } else {
                Some(md.to_string())
            }
        }
        // If the converter fails, fall back to the raw HTML rather than
        // dropping the description entirely — the user can still read it.
        Err(_) => Some(html.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blanks_become_none() {
        assert_eq!(html_to_markdown(""), None);
        assert_eq!(html_to_markdown("   \n  "), None);
    }

    #[test]
    fn headings_convert() {
        let md = html_to_markdown("<h1>Title</h1><p>Body</p>").unwrap();
        assert!(md.contains("# Title"));
        assert!(md.contains("Body"));
    }

    #[test]
    fn preserves_code_block() {
        let md = html_to_markdown("<pre><code>let x = 1;</code></pre>").unwrap();
        assert!(md.contains("let x = 1;"));
    }

    #[test]
    fn preserves_list() {
        let md = html_to_markdown("<ul><li>a</li><li>b</li></ul>").unwrap();
        assert!(md.contains("a"));
        assert!(md.contains("b"));
    }

    #[test]
    fn preserves_table() {
        let html = "<table><thead><tr><th>A</th></tr></thead><tbody><tr><td>1</td></tr></tbody></table>";
        let md = html_to_markdown(html).unwrap();
        assert!(md.contains("A"));
        assert!(md.contains('1'));
    }
}
