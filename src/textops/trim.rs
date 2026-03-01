fn split_line_and_eol(line: &str) -> (&str, &str) {
    if let Some(content) = line.strip_suffix("\r\n") {
        (content, "\r\n")
    } else if let Some(content) = line.strip_suffix('\n') {
        (content, "\n")
    } else {
        (line, "")
    }
}

pub fn trim_edges_spaces_tabs(s: &str) -> (usize, usize) {
    let bytes = s.as_bytes();
    let mut left = 0usize;
    while left < bytes.len() && (bytes[left] == b' ' || bytes[left] == b'\t') {
        left += 1;
    }

    let mut right = 0usize;
    while right < bytes.len().saturating_sub(left)
        && (bytes[bytes.len() - right - 1] == b' ' || bytes[bytes.len() - right - 1] == b'\t')
    {
        right += 1;
    }

    (left, right)
}

pub fn trim_line_preserve_eol(line: &str) -> String {
    let (content, eol) = split_line_and_eol(line);
    let (left, right) = trim_edges_spaces_tabs(content);
    let content_len = content.len();
    let start = left.min(content_len);
    let end = content_len.saturating_sub(right).max(start);
    let mut out = String::with_capacity(line.len());
    out.push_str(&content[start..end]);
    out.push_str(eol);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_edges_leading_spaces() {
        assert_eq!(trim_line_preserve_eol(" abc"), "abc");
    }

    #[test]
    fn trim_edges_trailing_spaces() {
        assert_eq!(trim_line_preserve_eol("abc "), "abc");
    }

    #[test]
    fn trim_edges_both_sides() {
        assert_eq!(trim_line_preserve_eol(" \tabc\t "), "abc");
    }

    #[test]
    fn trim_edges_whitespace_only() {
        assert_eq!(trim_line_preserve_eol(" \t "), "");
    }

    #[test]
    fn trim_edges_empty() {
        assert_eq!(trim_line_preserve_eol(""), "");
    }

    #[test]
    fn trim_preserves_lf() {
        assert_eq!(trim_line_preserve_eol(" abc \n"), "abc\n");
    }

    #[test]
    fn trim_preserves_crlf() {
        assert_eq!(trim_line_preserve_eol(" abc \r\n"), "abc\r\n");
    }

    #[test]
    fn trim_preserves_interior_whitespace() {
        assert_eq!(trim_line_preserve_eol(" a b \n"), "a b\n");
    }

    #[test]
    fn trim_edges_counts() {
        assert_eq!(trim_edges_spaces_tabs(" \tabc\t "), (2, 2));
        assert_eq!(trim_edges_spaces_tabs(""), (0, 0));
        assert_eq!(trim_edges_spaces_tabs(" \t "), (3, 0));
    }

    #[test]
    fn trim_preserves_eol_sanity_set() {
        let samples = [
            "", "x", " x", "x ", " x ", "\t x \t", "x\n", " x \n", "\t\t\r\n", "x y\r\n",
        ];
        for sample in samples {
            let (_, in_eol) = split_line_and_eol(sample);
            let output = trim_line_preserve_eol(sample);
            let (_, out_eol) = split_line_and_eol(&output);
            assert_eq!(in_eol, out_eol);
            let (content, _) = split_line_and_eol(&output);
            if !content.is_empty() {
                let first = content.as_bytes()[0];
                let last = content.as_bytes()[content.len() - 1];
                assert!(first != b' ' && first != b'\t');
                assert!(last != b' ' && last != b'\t');
            }
        }
    }
}
