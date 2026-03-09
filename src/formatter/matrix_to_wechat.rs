use once_cell::sync::Lazy;
use regex::Regex;

/// Result of converting Matrix HTML to WeChat text format.
#[derive(Debug, Clone)]
pub struct FormattedMessage {
    /// The plain text content for WeChat.
    pub text: String,
    /// List of mentioned Matrix user IDs (MXIDs) extracted from HTML pills.
    pub mentions: Vec<String>,
}

static MATRIX_PILL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<a\s+href="https://matrix\.to/#/(@[^"]+)"[^>]*>([^<]*)</a>"#).unwrap()
});

/// Convert Matrix HTML formatted message to WeChat plain text.
///
/// This handles:
/// - Bold (`<strong>`, `<b>`) -> `*text*`
/// - Italic (`<em>`, `<i>`) -> `_text_`
/// - Strikethrough (`<del>`, `<s>`) -> `~text~`
/// - Monospace (`<code>`) -> `` `text` ``
/// - Code blocks (`<pre>`) -> ` ```text``` `
/// - Mention pills (`<a href="https://matrix.to/#/@user:domain">`) -> extracted MXIDs
/// - Line breaks and paragraph endings -> newlines
/// - Emoji conversion (unicode -> wechat)
pub fn matrix_to_wechat(html: &str) -> FormattedMessage {
    let mut mentions = Vec::new();

    // Extract mentions from Matrix pills before stripping HTML
    let text = MATRIX_PILL_REGEX
        .replace_all(html, |caps: &regex::Captures| {
            let mxid = caps.get(1).map_or("", |m| m.as_str());
            let displayname = caps.get(2).map_or("", |m| m.as_str());
            if mxid.starts_with('@') {
                mentions.push(mxid.to_string());
            }
            displayname.to_string()
        })
        .to_string();

    // Convert formatting tags
    let text = convert_formatting(&text);

    // Convert line breaks
    let text = text
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n");

    // Strip remaining HTML tags
    let text = super::HTML_TAG_REGEX.replace_all(&text, "").to_string();

    // Apply unicode-to-wechat emoji conversion
    let text = super::emoji::unicode_to_wechat(&text);

    FormattedMessage { text, mentions }
}

/// Convert HTML formatting tags to WeChat-style text markers.
fn convert_formatting(html: &str) -> String {
    // Process code blocks first (<pre>) to avoid interfering with inner tags
    let result = replace_tag_pair(html, "pre", "```", "```");

    // Process inline code
    let result = replace_tag_pair(&result, "code", "`", "`");

    // Process bold
    let result = replace_tag_pair(&result, "strong", "*", "*");
    let result = replace_tag_pair(&result, "b", "*", "*");

    // Process italic
    let result = replace_tag_pair(&result, "em", "_", "_");
    let result = replace_tag_pair(&result, "i", "_", "_");

    // Process strikethrough
    let result = replace_tag_pair(&result, "del", "~", "~");
    let result = replace_tag_pair(&result, "s", "~", "~");

    result
}

/// Replace matched HTML tag pairs with prefix/suffix markers.
/// Handles tags with or without attributes.
fn replace_tag_pair(html: &str, tag: &str, prefix: &str, suffix: &str) -> String {
    // Build regex to match opening tag (with optional attributes) through closing tag
    let pattern = format!(r"(?si)<{tag}(?:\s[^>]*)?>(.+?)</{tag}>");
    let re = Regex::new(&pattern).unwrap();
    re.replace_all(html, |caps: &regex::Captures| {
        let content = caps.get(1).map_or("", |m| m.as_str());
        format!("{prefix}{content}{suffix}")
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_text() {
        let result = matrix_to_wechat("Hello world");
        assert_eq!(result.text, "Hello world");
        assert!(result.mentions.is_empty());
    }

    #[test]
    fn test_bold_conversion() {
        let result = matrix_to_wechat("<strong>bold text</strong>");
        assert_eq!(result.text, "*bold text*");

        let result = matrix_to_wechat("<b>bold text</b>");
        assert_eq!(result.text, "*bold text*");
    }

    #[test]
    fn test_italic_conversion() {
        let result = matrix_to_wechat("<em>italic text</em>");
        assert_eq!(result.text, "_italic text_");

        let result = matrix_to_wechat("<i>italic text</i>");
        assert_eq!(result.text, "_italic text_");
    }

    #[test]
    fn test_strikethrough_conversion() {
        let result = matrix_to_wechat("<del>deleted</del>");
        assert_eq!(result.text, "~deleted~");

        let result = matrix_to_wechat("<s>strikethrough</s>");
        assert_eq!(result.text, "~strikethrough~");
    }

    #[test]
    fn test_code_conversion() {
        let result = matrix_to_wechat("<code>inline code</code>");
        assert_eq!(result.text, "`inline code`");
    }

    #[test]
    fn test_pre_conversion() {
        let result = matrix_to_wechat("<pre>code block</pre>");
        assert_eq!(result.text, "```code block```");
    }

    #[test]
    fn test_mention_extraction() {
        let html = r#"Hello <a href="https://matrix.to/#/@alice:example.com">Alice</a>!"#;
        let result = matrix_to_wechat(html);
        assert_eq!(result.text, "Hello Alice!");
        assert_eq!(result.mentions, vec!["@alice:example.com"]);
    }

    #[test]
    fn test_multiple_mentions() {
        let html = r#"<a href="https://matrix.to/#/@alice:example.com">Alice</a> and <a href="https://matrix.to/#/@bob:example.com">Bob</a>"#;
        let result = matrix_to_wechat(html);
        assert_eq!(result.text, "Alice and Bob");
        assert_eq!(
            result.mentions,
            vec!["@alice:example.com", "@bob:example.com"]
        );
    }

    #[test]
    fn test_line_breaks() {
        let result = matrix_to_wechat("line1<br>line2<br/>line3");
        assert_eq!(result.text, "line1\nline2\nline3");
    }
}
