pub mod date_utils;
pub mod http_client;

pub fn extract_number(text: &str) -> Option<i32> {
    use once_cell::sync::Lazy;
    use regex::Regex;

    static DIGIT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\d+").unwrap());
    DIGIT_RE
        .find(text)
        .and_then(|m| m.as_str().parse::<i32>().ok())
}

/// 清理文本的示例
pub fn clean_text(s: &str) -> String {
    // 这里可以去掉多余空白、HTML 实体等
    s.trim().to_string()
}
