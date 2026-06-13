//! Markdown heading fold-level computation.
//!
//! Pure logic kept out of the Win32 layer so it can be unit tested. The
//! resulting levels are applied to Scintilla via `SCI_SETFOLDLEVEL`.

/// Scintilla `SC_FOLDLEVELBASE`.
pub const FOLD_LEVEL_BASE: u32 = 0x400;
/// Scintilla `SC_FOLDLEVELHEADERFLAG`.
pub const FOLD_HEADER_FLAG: u32 = 0x2000;

/// Computes one fold level per line of `text`.
///
/// ATX headings (`#` through `######`, at most 3 leading spaces, followed by
/// space/tab/end-of-line) open fold sections. Lines inside fenced code blocks
/// (``` or ~~~) and indented code (4+ spaces or a tab) are never headings.
pub fn compute_fold_levels(text: &str) -> Vec<u32> {
    let mut levels = Vec::new();
    let mut current_depth: u32 = 0;
    // (fence byte, opening run length) while inside a fenced code block.
    let mut fence: Option<(u8, usize)> = None;

    for raw_line in text.split('\n') {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let trimmed = line.trim_start_matches([' ', '\t']);
        let leading = &line[..line.len() - trimmed.len()];
        let code_indent = leading.contains('\t') || leading.len() > 3;

        if let Some((fence_byte, fence_len)) = fence {
            levels.push(FOLD_LEVEL_BASE + current_depth);
            if !code_indent {
                let run = trimmed.bytes().take_while(|&b| b == fence_byte).count();
                if run >= fence_len && trimmed[run..].trim().is_empty() {
                    fence = None;
                }
            }
            continue;
        }

        if !code_indent
            && let Some(&first) = trimmed.as_bytes().first()
            && (first == b'`' || first == b'~')
        {
            let run = trimmed.bytes().take_while(|&b| b == first).count();
            // A backtick fence's info string may not contain backticks.
            let info_ok = first != b'`' || !trimmed[run..].contains('`');
            if run >= 3 && info_ok {
                fence = Some((first, run));
                levels.push(FOLD_LEVEL_BASE + current_depth);
                continue;
            }
        }

        let heading_depth = if !code_indent && trimmed.starts_with('#') {
            let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
            let after = trimmed.as_bytes().get(hashes).copied();
            if (1..=6).contains(&hashes) && matches!(after, Some(b' ') | Some(b'\t') | None) {
                Some(hashes as u32)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(depth) = heading_depth {
            current_depth = depth;
            levels.push((FOLD_LEVEL_BASE + depth - 1) | FOLD_HEADER_FLAG);
        } else {
            levels.push(FOLD_LEVEL_BASE + current_depth);
        }
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: u32 = FOLD_LEVEL_BASE;
    const H: u32 = FOLD_HEADER_FLAG;

    #[test]
    fn heading_hierarchy() {
        let levels = compute_fold_levels("# A\nbody\n## B\nbody");
        assert_eq!(levels, vec![B | H, B + 1, (B + 1) | H, B + 2]);
    }

    #[test]
    fn hash_inside_backtick_fence_is_not_heading() {
        let levels = compute_fold_levels("# A\n```\n# not a heading\n```\nafter");
        assert_eq!(levels, vec![B | H, B + 1, B + 1, B + 1, B + 1]);
    }

    #[test]
    fn hash_inside_tilde_fence_is_not_heading() {
        let levels = compute_fold_levels("~~~\n# code\n~~~\n# B");
        assert_eq!(levels, vec![B, B, B, B | H]);
    }

    #[test]
    fn fence_closes_only_with_matching_run() {
        let levels = compute_fold_levels("````\n```\n# still code\n````\n# B");
        assert_eq!(levels, vec![B, B, B, B, B | H]);
    }

    #[test]
    fn fence_with_info_string() {
        let levels = compute_fold_levels("```rust\n# code\n```\n# B");
        assert_eq!(levels, vec![B, B, B, B | H]);
    }

    #[test]
    fn indented_code_is_not_heading() {
        assert_eq!(compute_fold_levels("    # code"), vec![B]);
        assert_eq!(compute_fold_levels("\t# code"), vec![B]);
    }

    #[test]
    fn up_to_three_leading_spaces_is_heading() {
        assert_eq!(compute_fold_levels("   # A"), vec![B | H]);
    }

    #[test]
    fn hash_without_space_is_not_heading() {
        assert_eq!(compute_fold_levels("#nope"), vec![B]);
    }

    #[test]
    fn seven_hashes_is_not_heading() {
        assert_eq!(compute_fold_levels("####### too deep"), vec![B]);
    }

    #[test]
    fn bare_heading_with_crlf() {
        let levels = compute_fold_levels("##\r\nbody");
        assert_eq!(levels, vec![(B + 1) | H, B + 2]);
    }

    #[test]
    fn unclosed_fence_runs_to_end() {
        let levels = compute_fold_levels("# A\n```\n# code\n# more code");
        assert_eq!(levels, vec![B | H, B + 1, B + 1, B + 1]);
    }

    #[test]
    fn empty_text_yields_single_base_level() {
        assert_eq!(compute_fold_levels(""), vec![B]);
    }
}
