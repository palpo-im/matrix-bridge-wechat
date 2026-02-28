pub mod emoji;
pub mod matrix_to_wechat;
pub mod wechat_to_matrix;

use once_cell::sync::Lazy;
use regex::Regex;

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

pub fn wechat_to_matrix(text: &str) -> String {
    let text = emoji::wechat_to_unicode(text);
    text
}

pub fn matrix_to_wechat(text: &str) -> String {
    let text = html_to_plain(text);
    let text = emoji::unicode_to_wechat(&text);
    text
}
