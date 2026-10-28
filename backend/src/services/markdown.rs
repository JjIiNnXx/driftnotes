use std::collections::HashSet;

use ammonia::Builder;
use pulldown_cmark::{html, CowStr, Event, Options, Parser};

use crate::error::{ApiError, ApiResult};

pub fn render(markdown: &str) -> ApiResult<String> {
    if markdown.len() > 1024 * 1024 {
        return Err(ApiError::bad_request("markdown must not exceed 1 MiB"));
    }

    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(markdown, options).map(|event| match event {
        // Raw HTML becomes plain text and is escaped by pulldown-cmark.
        Event::Html(value) | Event::InlineHtml(value) => {
            Event::Text(CowStr::from(value.into_string()))
        }
        other => other,
    });
    let mut unsafe_html = String::new();
    html::push_html(&mut unsafe_html, parser);

    let clean = Builder::default()
        .url_schemes(HashSet::from(["http", "https", "mailto"]))
        .link_rel(Some("noopener noreferrer"))
        .clean(&unsafe_html)
        .to_string();
    Ok(clean)
}

pub fn excerpt(markdown: &str, maximum: usize) -> String {
    let mut text = String::new();
    let mut previous_space = false;
    for character in markdown.chars() {
        let skip = matches!(character, '#' | '`' | '*' | '>' | '[' | ']' | '(' | ')');
        if character.is_whitespace() || skip {
            if !previous_space && !text.is_empty() {
                text.push(' ');
            }
            previous_space = true;
        } else {
            text.push(character);
            previous_space = false;
        }
    }
    let trimmed = text.trim();
    if trimmed.chars().count() <= maximum {
        trimmed.to_owned()
    } else {
        let mut result = trimmed.chars().take(maximum - 1).collect::<String>();
        result.push('…');
        result
    }
}

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn escapes_raw_html_and_dangerous_links() {
        let html = render("<script>alert(1)</script>\n\n[bad](javascript:alert(1))").unwrap();
        assert!(!html.contains("<script"));
        assert!(!html.contains("javascript:"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn renders_markdown_code() {
        let html = render("```rust\nfn main() {}\n```").unwrap();
        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }
}
