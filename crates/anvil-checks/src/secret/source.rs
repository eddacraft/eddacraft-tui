//! SDT-007: bounded line sources for the secret scan.
//!
//! Both scan passes are *window-local*: the pattern pass needs the matched
//! line plus a radius-2 context window, and the entropy pass reaches only
//! [`context_window`]`(lines, index, 2)` and `lines.get(index)`. Nothing
//! reaches further, so nothing has to be resident.
//!
//! There is one exception, and it is the reason this module exists rather
//! than a naive five-line array: `is_inside_rust_cfg_test_module` in
//! `scanner.rs` is a *prefix* fold — whether line `n` sits inside a
//! `#[cfg(test)] mod { … }` body depends on every line before it. It is a
//! forward fold with O(1) state, so it streams exactly; the driver below
//! advances it once per line **in read order** and carries the answer for
//! each line alongside the line itself. See `RustCfgTestTracker`.
//!
//! [`context_window`]: crate::secret::context::context_window

use std::borrow::Cow;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::ops::ControlFlow;
use std::path::Path;

use crate::secret::context::context_window;

/// The widest context either pass reaches, in lines either side of the
/// current one. Both `pattern_skip_reason` and `is_benign_entropy_fixture`
/// call `context_window(.., 2)`; nothing calls it with a larger radius.
pub(crate) const CONTEXT_RADIUS: usize = 2;

/// Lines resident at once: the current line, `CONTEXT_RADIUS` behind it and
/// `CONTEXT_RADIUS` ahead of it.
const WINDOW_LEN: usize = CONTEXT_RADIUS * 2 + 1;

/// Where a scan reads its lines from.
///
/// [`LineSource::Memory`] is the buffer path (save-time intercept, the
/// calibration corpus, every existing `&str` entry point) and borrows its
/// lines — it allocates nothing and splits with `str::lines`, so it is the
/// same iteration the pre-SDT-007 scanner performed.
/// [`LineSource::File`] is the on-disk path and never materialises the file.
pub(crate) enum LineSource<'a> {
    Memory(&'a str),
    File(&'a Path),
}

impl<'a> LineSource<'a> {
    /// Open a fresh read of this source. Called once per pass, so a source
    /// must be re-readable: the pattern pass and the entropy pass each walk
    /// the lines from the start, exactly as they did when both borrowed the
    /// same in-memory `content`.
    fn open(&self) -> io::Result<SourceReader<'a>> {
        match *self {
            LineSource::Memory(content) => Ok(SourceReader::Memory(content.lines())),
            LineSource::File(path) => Ok(SourceReader::File(BufReader::new(File::open(path)?))),
        }
    }
}

enum SourceReader<'a> {
    Memory(std::str::Lines<'a>),
    File(BufReader<File>),
}

impl<'a> SourceReader<'a> {
    fn next_line(&mut self) -> io::Result<Option<Cow<'a, str>>> {
        match self {
            SourceReader::Memory(lines) => Ok(lines.next().map(Cow::Borrowed)),
            SourceReader::File(reader) => {
                let mut buffer = String::new();
                if reader.read_line(&mut buffer)? == 0 {
                    return Ok(None);
                }
                // Reproduce `str::lines` exactly: drop the `\n`, and drop a
                // `\r` only when it immediately preceded it. A `\r` anywhere
                // else is content, and a file that does not end in a newline
                // still yields its final partial line.
                if buffer.ends_with('\n') {
                    buffer.pop();
                    if buffer.ends_with('\r') {
                        buffer.pop();
                    }
                }
                Ok(Some(Cow::Owned(buffer)))
            }
        }
    }
}

/// One line plus everything the scan is allowed to know about its
/// surroundings.
///
/// `lines` is the clipped `[index - CONTEXT_RADIUS ..= index + CONTEXT_RADIUS]`
/// slice of the file and `index` is the current line's position *within that
/// slice*. `context_window` saturates at both ends, so calling it on this
/// slice returns byte-for-byte what calling it on the whole file would.
pub(crate) struct LineWindow<'a> {
    lines: &'a [&'a str],
    index: usize,
    line_number: usize,
    in_rust_cfg_test: bool,
}

impl<'a> LineWindow<'a> {
    /// The line being scanned.
    pub(crate) fn line(&self) -> &'a str {
        self.lines[self.index]
    }

    /// 1-based line number in the file — what a finding reports.
    pub(crate) fn line_number(&self) -> usize {
        self.line_number
    }

    /// Does this line sit inside a `#[cfg(test)] mod … { … }` body?
    /// Computed by the caller's prefix fold as the line was read.
    pub(crate) fn in_rust_cfg_test(&self) -> bool {
        self.in_rust_cfg_test
    }

    /// The lowercased radius-2 context window around this line.
    pub(crate) fn context(&self) -> String {
        context_window(self.lines, self.index, CONTEXT_RADIUS)
    }

    /// Line immediately above this one, when the window still holds it.
    pub(crate) fn previous_line(&self) -> Option<&'a str> {
        self.index
            .checked_sub(1)
            .and_then(|index| self.lines.get(index).copied())
    }
}

/// Walk `source` one line at a time, handing `visit` a radius-2 window.
///
/// `line_flag` is called **exactly once per line, in read order**, before
/// that line is ever visited. That ordering is load-bearing: it is where the
/// `#[cfg(test)]` prefix fold advances, and a fold that saw lines out of
/// order or twice would answer differently from the whole-file scan it
/// replaces.
///
/// `visit` lags the read head by [`CONTEXT_RADIUS`] lines so the two lines
/// after the current one are already buffered. At most [`WINDOW_LEN`] lines
/// are resident.
pub(crate) fn for_each_windowed_line<A, V>(
    source: &LineSource<'_>,
    mut line_flag: A,
    mut visit: V,
) -> io::Result<()>
where
    A: FnMut(&str) -> bool,
    V: FnMut(&LineWindow<'_>) -> ControlFlow<()>,
{
    let mut reader = source.open()?;
    let mut buffered: VecDeque<(Cow<'_, str>, bool)> = VecDeque::with_capacity(WINDOW_LEN);
    // Absolute index of `buffered.front()`.
    let mut first_index = 0usize;
    // Absolute count of lines pulled from the reader.
    let mut read_count = 0usize;
    // Absolute index of the line being visited.
    let mut cursor = 0usize;
    let mut exhausted = false;

    loop {
        while !exhausted && read_count < cursor + CONTEXT_RADIUS + 1 {
            match reader.next_line()? {
                Some(line) => {
                    let flag = line_flag(line.as_ref());
                    buffered.push_back((line, flag));
                    read_count += 1;
                }
                None => exhausted = true,
            }
        }
        if cursor >= read_count {
            return Ok(());
        }

        let flow = {
            let mut window: [&str; WINDOW_LEN] = [""; WINDOW_LEN];
            let mut len = 0usize;
            for (line, _) in &buffered {
                window[len] = line.as_ref();
                len += 1;
            }
            let index = cursor - first_index;
            let view = LineWindow {
                lines: &window[..len],
                index,
                line_number: cursor + 1,
                in_rust_cfg_test: buffered[index].1,
            };
            visit(&view)
        };

        cursor += 1;
        while first_index + CONTEXT_RADIUS < cursor {
            buffered.pop_front();
            first_index += 1;
        }
        if flow.is_break() {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;

    use super::{LineSource, for_each_windowed_line};

    /// Collect `(line_number, line, context)` for every line of `content`.
    fn walk(source: &LineSource<'_>) -> Vec<(usize, String, String)> {
        let mut seen = Vec::new();
        for_each_windowed_line(
            source,
            |_| false,
            |window| {
                seen.push((
                    window.line_number(),
                    window.line().to_string(),
                    window.context(),
                ));
                ControlFlow::Continue(())
            },
        )
        .expect("in-memory walk cannot fail");
        seen
    }

    /// The window is the only thing the passes may reach, so it must equal
    /// what the whole-file slice would have produced.
    #[test]
    fn context_window_matches_the_whole_file_window() {
        let content = "one\ntwo\nthree\nfour\nfive\nsix\nseven\n";
        let lines: Vec<&str> = content.lines().collect();
        let seen = walk(&LineSource::Memory(content));

        assert_eq!(seen.len(), lines.len());
        for (index, (line_number, line, context)) in seen.iter().enumerate() {
            assert_eq!(*line_number, index + 1);
            assert_eq!(line, lines[index]);
            assert_eq!(
                context,
                &crate::secret::context::context_window(&lines, index, 2),
                "line {index} window drifted from the whole-file window"
            );
        }
    }

    /// `str::lines` semantics, reproduced by the file reader: no phantom
    /// final line after a trailing newline, a final partial line without
    /// one, `\r\n` stripped, and a bare `\r` kept as content.
    #[test]
    fn file_lines_split_exactly_like_str_lines() {
        let cases = [
            "",
            "\n",
            "a\n",
            "a",
            "a\nb\n",
            "a\nb",
            "a\r\nb\r\n",
            "a\r\rb\n",
            "\n\n\n",
            "trailing\nno newline",
        ];
        let dir = std::env::temp_dir().join(format!(
            "anvil-sdt007-lines-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");

        for (index, content) in cases.iter().enumerate() {
            let path = dir.join(format!("case-{index}.txt"));
            std::fs::write(&path, content).expect("write case");

            let from_memory = walk(&LineSource::Memory(content));
            let from_file = walk(&LineSource::File(&path));

            assert_eq!(
                from_memory, from_file,
                "file and memory line splitting diverged on {content:?}"
            );
            let expected: Vec<&str> = content.lines().collect();
            assert_eq!(
                from_file
                    .iter()
                    .map(|(_, l, _)| l.as_str())
                    .collect::<Vec<_>>(),
                expected,
                "file splitting diverged from `str::lines` on {content:?}"
            );
        }

        let _ = std::fs::remove_dir_all(dir);
    }

    /// A `Break` stops the walk where the whole-file loop's early `return`
    /// did — the scan's finding limit depends on it.
    #[test]
    fn breaking_stops_the_walk() {
        let content = "a\nb\nc\nd\ne\nf\n";
        let mut seen = Vec::new();
        for_each_windowed_line(
            &LineSource::Memory(content),
            |_| false,
            |window| {
                seen.push(window.line_number());
                if window.line_number() == 3 {
                    return ControlFlow::Break(());
                }
                ControlFlow::Continue(())
            },
        )
        .expect("in-memory walk cannot fail");
        assert_eq!(seen, vec![1, 2, 3]);
    }

    /// The prefix fold must see each line once, in order, before that line
    /// is visited — the `#[cfg(test)]` tracker is not order-independent.
    #[test]
    fn line_flags_are_computed_once_in_read_order() {
        let content = "a\nb\nc\nd\ne\nf\ng\n";
        let mut flagged = Vec::new();
        let mut visited = Vec::new();
        for_each_windowed_line(
            &LineSource::Memory(content),
            |line| {
                flagged.push(line.to_string());
                line == "c"
            },
            |window| {
                visited.push((window.line().to_string(), window.in_rust_cfg_test()));
                ControlFlow::Continue(())
            },
        )
        .expect("in-memory walk cannot fail");

        assert_eq!(flagged, content.lines().collect::<Vec<_>>());
        assert_eq!(
            visited,
            content
                .lines()
                .map(|line| (line.to_string(), line == "c"))
                .collect::<Vec<_>>(),
            "each line must carry its own flag, not a neighbour's"
        );
    }
}
