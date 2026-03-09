/// Information about a mentioned user for WeChat-to-Matrix conversion.
#[derive(Debug, Clone)]
pub struct MentionInfo {
    /// The WeChat UID of the mentioned user.
    pub uid: String,
    /// The Matrix user ID (MXID) of the mentioned user.
    pub mxid: String,
    /// The display name to show in the mention pill.
    pub displayname: String,
}

/// Convert WeChat text to Matrix format.
///
/// Returns a tuple of `(plain_text, html_formatted_text)`.
///
/// - `plain_text`: The message with WeChat emoji codes converted to Unicode.
/// - `html_formatted_text`: The message with emoji conversion AND mention pills
///   (e.g., `<a href="https://matrix.to/#/@puppet:domain">Display Name</a>`).
///
/// When `mentions` is non-empty, any `@nickname` patterns in the text that match
/// a mention's UID-based group nickname will be replaced with Matrix HTML pills.
pub fn wechat_to_matrix(text: &str, mentions: &[MentionInfo]) -> (String, String) {
    // Apply emoji conversion for plain text
    let plain_text = super::emoji::wechat_to_unicode(text);

    if mentions.is_empty() {
        // No mentions to process; HTML is same as plain text
        return (plain_text.clone(), plain_text);
    }

    // Build HTML with mention pills
    let mut html_text = super::emoji::wechat_to_unicode(text);
    let mut prepended_mentions = String::new();

    for mention in mentions {
        let original = format!("@{}", mention.uid);
        let pill = format!(
            r#"<a href="https://matrix.to/#/{}">{}</a> "#,
            mention.mxid, mention.displayname
        );

        if html_text.contains(&original) {
            html_text = html_text.replace(&original, &pill);
        } else {
            // If the @mention text is not found in the message body,
            // prepend it as a pill at the beginning (matching Go behavior).
            prepended_mentions.push_str(&pill);
        }
    }

    if !prepended_mentions.is_empty() {
        prepended_mentions.push_str("<br> ");
        html_text = format!("{prepended_mentions}{html_text}");
    }

    (plain_text, html_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_mentions() {
        let (plain, html) = wechat_to_matrix("Hello world", &[]);
        assert_eq!(plain, "Hello world");
        assert_eq!(html, "Hello world");
    }

    #[test]
    fn test_emoji_conversion() {
        let (plain, html) = wechat_to_matrix("[微笑]", &[]);
        assert_eq!(plain, "😃");
        assert_eq!(html, "😃");
    }

    #[test]
    fn test_mention_in_text() {
        let mentions = vec![MentionInfo {
            uid: "wxid_abc123".to_string(),
            mxid: "@puppet_abc:example.com".to_string(),
            displayname: "Alice".to_string(),
        }];

        let (plain, html) = wechat_to_matrix("Hello @wxid_abc123 how are you?", &mentions);
        assert_eq!(plain, "Hello @wxid_abc123 how are you?");
        assert!(html.contains(r#"<a href="https://matrix.to/#/@puppet_abc:example.com">Alice</a>"#));
    }

    #[test]
    fn test_mention_not_in_text() {
        let mentions = vec![MentionInfo {
            uid: "wxid_abc123".to_string(),
            mxid: "@puppet_abc:example.com".to_string(),
            displayname: "Alice".to_string(),
        }];

        let (plain, html) = wechat_to_matrix("Hello everyone", &mentions);
        assert_eq!(plain, "Hello everyone");
        assert!(html.starts_with(r#"<a href="https://matrix.to/#/@puppet_abc:example.com">Alice</a>"#));
        assert!(html.contains("<br> "));
        assert!(html.contains("Hello everyone"));
    }

    #[test]
    fn test_multiple_mentions() {
        let mentions = vec![
            MentionInfo {
                uid: "wxid_alice".to_string(),
                mxid: "@puppet_alice:example.com".to_string(),
                displayname: "Alice".to_string(),
            },
            MentionInfo {
                uid: "wxid_bob".to_string(),
                mxid: "@puppet_bob:example.com".to_string(),
                displayname: "Bob".to_string(),
            },
        ];

        let (plain, html) =
            wechat_to_matrix("@wxid_alice and @wxid_bob hello!", &mentions);
        assert_eq!(plain, "@wxid_alice and @wxid_bob hello!");
        assert!(html.contains(r#"<a href="https://matrix.to/#/@puppet_alice:example.com">Alice</a>"#));
        assert!(html.contains(r#"<a href="https://matrix.to/#/@puppet_bob:example.com">Bob</a>"#));
    }
}
