//! Bounded spelling scans and position conversion, independent of Win32.

use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use regex::Regex;

pub const CHUNK_BYTES: usize = 16 * 1024;
pub const MAX_DOCUMENT_BYTES: usize = 2 * 1024 * 1024;
pub const EDIT_DELAY: Duration = Duration::from_millis(450);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProseState {
    fence: Option<(u8, usize)>,
    continued_line: bool,
    closing_fence: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobKey {
    pub tab: isize,
    pub revision: u64,
    pub generation: u64,
    pub start: usize,
}

pub struct Job {
    pub key: JobKey,
    pub text: String,
    pub markdown: bool,
    pub context: ProseState,
    pub end_of_document: bool,
}

pub struct Checked {
    pub key: JobKey,
    pub next: usize,
    pub context: ProseState,
    pub ranges: Vec<Range<usize>>,
}

/// Checkpoints before an edit stay valid; the affected suffix is rescanned.
/// Only the active tab is scanned and only one worker job is outstanding.
pub struct Scan {
    pub mode: Option<bool>,
    pub next: usize,
    pub context: ProseState,
    pub generation: u64,
    pub edited_at: Instant,
    checkpoints: BTreeMap<usize, ProseState>,
}

impl Default for Scan {
    fn default() -> Self {
        Self {
            mode: None,
            next: 0,
            context: ProseState::default(),
            generation: 0,
            edited_at: Instant::now(),
            checkpoints: BTreeMap::new(),
        }
    }
}

impl Scan {
    pub fn set_mode(&mut self, mode: Option<bool>) -> bool {
        if self.mode == mode {
            return false;
        }
        self.mode = mode;
        self.invalidate(0);
        true
    }

    pub fn invalidate(&mut self, position: usize) -> usize {
        self.generation = self.generation.wrapping_add(1);
        self.edited_at = Instant::now();
        let limit = position.saturating_sub(1).min(self.next);
        let (&start, &context) = self
            .checkpoints
            .range(..=limit)
            .next_back()
            .unwrap_or((&0, &ProseState::default()));
        self.next = start;
        self.context = context;
        self.checkpoints.retain(|&offset, _| offset <= start);
        start
    }

    pub fn accepts(&self, result: &Checked, tab: isize, revision: u64) -> bool {
        self.mode.is_some()
            && result.key.tab == tab
            && result.key.revision == revision
            && result.key.generation == self.generation
            && result.key.start == self.next
    }

    pub fn advance(&mut self, result: &Checked) {
        self.next = result.next;
        self.context = result.context;
        self.checkpoints.insert(self.next, self.context);
    }
}

pub struct Prepared {
    pub text: String,
    pub context: ProseState,
    boundaries: Vec<Option<usize>>,
}

impl Prepared {
    pub fn byte_range(&self, start: u32, length: u32) -> Option<Range<usize>> {
        let end = start.checked_add(length)? as usize;
        let start = self.boundaries.get(start as usize).copied().flatten()?;
        let end = self.boundaries.get(end).copied().flatten()?;
        (start < end && self.text.get(start..end)?.chars().any(char::is_alphabetic))
            .then_some(start..end)
    }
}

/// Replace exclusions with spaces of the same byte length. Build the UTF-16
/// map *after* masking, so even excluded non-ASCII text preserves byte offsets.
pub fn prepare(job: &Job) -> Prepared {
    let mut bytes = job.text.as_bytes().to_vec();
    let mut context = job.context;
    let mut offset = 0;
    for line in job.text.split_inclusive('\n') {
        if !job.markdown {
            // Check long wrapped paragraphs in chunks, suppressing only split
            // tokens (including URLs) until whitespace in the next chunk.
            let complete = line.ends_with(char::is_whitespace) || job.end_of_document;
            if context.continued_line {
                let end = line.find(char::is_whitespace).unwrap_or(line.len());
                bytes[offset..offset + end].fill(b' ');
            }
            if !complete {
                let start = line.rfind(char::is_whitespace).unwrap_or(0);
                bytes[offset + start..offset + line.len()].fill(b' ');
            }
            context.continued_line = !complete;
            offset += line.len();
            continue;
        }
        let complete = line.ends_with('\n') || job.end_of_document;
        let trimmed = line.trim_start_matches([' ', '\t']);
        let indent = &line[..line.len() - trimmed.len()];
        let code_indent = indent.contains('\t') || indent.len() >= 4;
        let mut exclude = context.continued_line;
        if job.markdown {
            if context.continued_line {
                context.closing_fence &= line.trim().is_empty();
            } else if let Some((marker, count)) = context.fence {
                let run = trimmed.bytes().take_while(|&b| b == marker).count();
                context.closing_fence =
                    !code_indent && run >= count && trimmed[run..].trim().is_empty();
                exclude = true;
            } else if !code_indent
                && let Some(&marker) = trimmed.as_bytes().first()
                && matches!(marker, b'`' | b'~')
            {
                let run = trimmed.bytes().take_while(|&b| b == marker).count();
                if run >= 3 && (marker != b'`' || !trimmed[run..].contains('`')) {
                    context.fence = Some((marker, run));
                    exclude = true;
                }
            }
            exclude |= code_indent;
            if context.closing_fence && complete {
                context.fence = None;
                context.closing_fence = false;
            }
        }
        // Avoid checking partial words or inline constructs on a very long
        // line split by the chunk limit. Resume on its next physical line.
        exclude |= !complete;
        if exclude {
            bytes[offset..offset + line.len()].fill(b' ');
        } else if job.markdown {
            mask_markdown_inline(&mut bytes[offset..offset + line.len()]);
        }
        context.continued_line = !complete;
        offset += line.len();
    }

    static ADDRESSES: OnceLock<std::result::Result<Regex, regex::Error>> = OnceLock::new();
    if let Ok(pattern) = ADDRESSES.get_or_init(|| {
        Regex::new(r"(?i)\b(?:https?://|ftp://|www\.)[^\s<>]+|[\p{L}\p{N}._%+\-]+@[\p{L}\p{N}.\-]+\.[\p{L}]{2,}")
    }) {
        for matched in pattern.find_iter(&job.text) {
            bytes[matched.range()].fill(b' ');
        }
    }
    for byte in &mut bytes {
        if *byte == 0 {
            *byte = b' ';
        }
    }
    // Each masked span begins and ends at an ASCII delimiter or UTF-8 boundary.
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let mut boundaries = vec![Some(0)];
    for (start, ch) in text.char_indices() {
        if ch.len_utf16() == 2 {
            boundaries.push(None);
        }
        boundaries.push(Some(start + ch.len_utf8()));
    }
    Prepared {
        text,
        context,
        boundaries,
    }
}

fn mask_markdown_inline(bytes: &mut [u8]) {
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == b'`' {
            let start = i;
            let run = bytes[i..].iter().take_while(|&&b| b == b'`').count();
            i += run;
            let mut end = i;
            while end < bytes.len() {
                if bytes[end] == b'`' {
                    let closing = bytes[end..].iter().take_while(|&&b| b == b'`').count();
                    if closing == run {
                        end += run;
                        break;
                    }
                    end += closing;
                } else {
                    end += 1;
                }
            }
            bytes[start..end].fill(b' ');
            i = end;
        } else if bytes[i..].starts_with(b"](") {
            let start = i + 1;
            i += 2;
            let mut depth = 1;
            while i < bytes.len() && depth != 0 {
                match bytes[i] {
                    b'\\' => {
                        i = (i + 2).min(bytes.len());
                        continue;
                    }
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            bytes[start..i].fill(b' ');
        } else if bytes[i..].starts_with(b"]:") {
            bytes[i + 1..].fill(b' ');
            break;
        } else {
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn job(text: &str, markdown: bool) -> Job {
        Job {
            key: JobKey {
                tab: 7,
                revision: 1,
                generation: 0,
                start: 0,
            },
            text: text.into(),
            markdown,
            context: ProseState::default(),
            end_of_document: true,
        }
    }

    #[test]
    fn unicode_offsets_reject_half_surrogates_and_invalid_ranges() {
        let source = "é\u{1f642}e\u{301}\r\nwrng";
        let prepared = prepare(&job(source, false));
        assert_eq!(prepared.byte_range(7, 4), Some(11..15));
        assert_eq!(prepared.byte_range(2, 1), None);
        assert_eq!(prepared.byte_range(u32::MAX, 1), None);
        assert_eq!(prepared.byte_range(0, 99), None);
        assert_eq!(prepared.byte_range(0, 0), None);
    }

    #[test]
    fn exclusions_keep_offsets_after_non_ascii_code_and_nulls() {
        let source = "`é\u{1f642}`\0 wrng https://bad.example/typoo a@typoo.example";
        let prepared = prepare(&job(source, true));
        let start = source.find("wrng");
        assert_eq!(prepared.text.find("wrng"), start);
        assert_eq!(prepared.byte_range(10, 4), start.map(|s| s..s + 4));
        assert!(!prepared.text.contains("typoo"));
        assert!(!prepared.text.contains('\0'));
    }

    #[test]
    fn markdown_keeps_link_labels_but_excludes_code_and_destinations() {
        let prepared = prepare(&job(
            "[wrng](relative/typoo(a))\n    typoox\n``code ` typooy`` prose\n[label]: typoow\n",
            true,
        ));
        assert!(prepared.text.contains("wrng"));
        assert!(prepared.text.contains("prose"));
        assert!(!prepared.text.contains("typoo"));
    }

    #[test]
    fn fences_carry_across_chunks_and_ignore_short_closing_runs() {
        let first = prepare(&job("````rust\ntypoo\n", true));
        let mut second = job("```\ntypoo\n````\nwrng", true);
        second.context = first.context;
        let prepared = prepare(&second);
        assert!(!prepared.text.contains("typoo"));
        assert!(prepared.text.contains("wrng"));
        assert_eq!(prepared.context, ProseState::default());
    }

    #[test]
    fn oversized_lines_are_skipped_until_the_next_line() {
        let mut first = job("partialword", false);
        first.end_of_document = false;
        let first = prepare(&first);
        let mut second = job("continuation\nwrng", false);
        second.context = first.context;
        assert_eq!(prepare(&second).text.trim(), "wrng");
    }

    #[test]
    fn long_plain_paragraphs_check_complete_words_and_skip_split_urls() {
        let mut first = job("wrng https://very.long/partial", false);
        first.end_of_document = false;
        let prepared = prepare(&first);
        assert_eq!(prepared.text.trim(), "wrng");
        let mut second = job("urlpath wrng", false);
        second.context = prepared.context;
        assert_eq!(prepare(&second).text.trim(), "wrng");
    }

    #[test]
    fn edits_restore_prior_context_and_reject_stale_results() {
        let mut scan = Scan::default();
        scan.set_mode(Some(true));
        let mut result = Checked {
            key: JobKey {
                tab: 7,
                revision: 1,
                generation: scan.generation,
                start: 0,
            },
            next: 100,
            context: ProseState {
                fence: Some((b'`', 3)),
                ..ProseState::default()
            },
            ranges: vec![],
        };
        assert!(scan.accepts(&result, 7, 1));
        assert!(!scan.accepts(&result, 8, 1));
        assert!(!scan.accepts(&result, 7, 2));
        scan.advance(&result);
        result.key.start = 100;
        result.next = 200;
        scan.advance(&result);
        assert_eq!(scan.invalidate(150), 100);
        assert_eq!(scan.context.fence, Some((b'`', 3)));
        assert!(!scan.accepts(&result, 7, 1));
        assert_eq!(scan.invalidate(50), 0);
        assert_eq!(scan.context, ProseState::default());
    }
}
