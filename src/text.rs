// Byte-level helpers that follow C's `isspace` (which includes \v and \f, unlike
// `u8::is_ascii_whitespace`). Everything we split on is ASCII, so slicing `&str`
// at these boundaries is always safe.

pub fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

pub fn is_space_except_newline(b: u8) -> bool {
    is_space(b) && b != b'\n' && b != b'\r'
}

pub fn trim_bytes(s: &[u8]) -> &[u8] {
    let start = s.iter().position(|&b| !is_space(b)).unwrap_or(s.len());
    let end = s.iter().rposition(|&b| !is_space(b)).map_or(start, |i| i + 1);
    &s[start..end]
}

pub fn trim(s: &str) -> &str {
    let bytes = trim_bytes(s.as_bytes());
    if bytes.is_empty() {
        return "";
    }
    let start = bytes.as_ptr() as usize - s.as_ptr() as usize;
    &s[start..start + bytes.len()]
}

pub fn trim_end(s: &str) -> &str {
    let end = s.bytes().rposition(|b| !is_space(b)).map_or(0, |i| i + 1);
    &s[..end]
}

pub fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Splits on runs of whitespace and commas, yielding `(byte_offset, word)`.
/// Used both for the `TAGS` property and for the update language.
pub fn split_list(s: &str) -> impl Iterator<Item = (usize, &str)> {
    let is_sep = |b: u8| is_space(b) || b == b',';
    let bytes = s.as_bytes();
    let mut pos = 0;
    std::iter::from_fn(move || {
        while pos < bytes.len() && is_sep(bytes[pos]) {
            pos += 1;
        }
        if pos >= bytes.len() {
            return None;
        }
        let start = pos;
        while pos < bytes.len() && !is_sep(bytes[pos]) {
            pos += 1;
        }
        Some((start, &s[start..pos]))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_c_whitespace() {
        assert_eq!(trim(" \t\x0Bhello\x0C\r\n"), "hello");
        assert_eq!(trim("   "), "");
        assert_eq!(trim_end("abc \r"), "abc");
    }

    #[test]
    fn splits_tags_like_the_spec_says() {
        let tags: Vec<_> = split_list("foo,,, hello  world").map(|(_, t)| t).collect();
        assert_eq!(tags, ["foo", "hello", "world"]);
        assert_eq!(split_list(" ,, ").count(), 0);
    }
}
