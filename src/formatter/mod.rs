pub mod emoji;
pub mod matrix_to_wechat;
pub mod wechat_to_matrix;

use once_cell::sync::Lazy;
use regex::Regex;

pub use matrix_to_wechat::FormattedMessage;
pub use wechat_to_matrix::MentionInfo;

pub static HTML_TAG_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]*>").unwrap());

pub fn strip_html(html: &str) -> String {
    HTML_TAG_REGEX.replace_all(html, "").to_string()
}

pub fn html_to_plain(html: &str) -> String {
    let text = html
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n");

    HTML_TAG_REGEX.replace_all(&text, "").to_string()
}

/// Convert Matrix HTML to WeChat text format.
///
/// Returns a `FormattedMessage` containing the plain text and any extracted mention MXIDs.
pub fn matrix_to_wechat(html: &str) -> FormattedMessage {
    matrix_to_wechat::matrix_to_wechat(html)
}

/// Convert WeChat text to Matrix format.
///
/// Returns `(plain_text, html_formatted_text)` with emoji conversion and mention pills.
pub fn wechat_to_matrix(text: &str, mentions: &[MentionInfo]) -> (String, String) {
    wechat_to_matrix::wechat_to_matrix(text, mentions)
}
