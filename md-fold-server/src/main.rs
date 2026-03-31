use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::*;
use std::collections::HashMap;

fn main() {
    let (connection, io_threads) = Connection::stdio();

    let capabilities = ServerCapabilities {
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::FULL,
        )),
        ..Default::default()
    };

    // connection.initialize() wraps the value in {"capabilities": ...},
    // so pass just ServerCapabilities, not the full InitializeResult.
    let (id, _params) = connection.initialize_start().unwrap();
    let init_result = serde_json::to_value(InitializeResult {
        capabilities,
        server_info: Some(ServerInfo {
            name: "md-fold-server".into(),
            version: Some(env!("CARGO_PKG_VERSION").into()),
        }),
    })
    .unwrap();
    connection.initialize_finish(id, init_result).unwrap();
    eprintln!("[md-fold-server] initialized (version {}), advertising folding_range_provider: true", env!("CARGO_PKG_VERSION"));

    let mut documents: HashMap<Uri, String> = HashMap::new();
    let mut next_request_id: i32 = 1;
    let mut dynamic_registered = false;

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                eprintln!("[md-fold-server] request: {}", req.method);
                if connection.handle_shutdown(&req).unwrap() {
                    return;
                }
                handle_request(&req, &documents, &connection);
            }
            Message::Notification(notif) => {
                eprintln!("[md-fold-server] notification: {}", notif.method);
                handle_notification(&notif, &mut documents);

                // After receiving `initialized` or `didOpen`, send dynamic
                // registration for foldingRange. This works around a Zed bug
                // where the client checks capabilities before the remote
                // server's static capabilities have been propagated.
                if !dynamic_registered
                    && (notif.method == "initialized"
                        || notif.method == "textDocument/didOpen")
                {
                    dynamic_registered = true;
                    let registration = Registration {
                        id: "md-fold-folding-range".into(),
                        method: "textDocument/foldingRange".into(),
                        register_options: Some(serde_json::to_value(
                            TextDocumentRegistrationOptions {
                                document_selector: Some(vec![DocumentFilter {
                                    language: Some("markdown".into()),
                                    scheme: None,
                                    pattern: None,
                                }]),
                            },
                        ).unwrap()),
                    };
                    let params = RegistrationParams {
                        registrations: vec![registration],
                    };
                    let req = Request {
                        id: RequestId::from(next_request_id),
                        method: "client/registerCapability".into(),
                        params: serde_json::to_value(params).unwrap(),
                    };
                    next_request_id += 1;
                    eprintln!("[md-fold-server] sending dynamic registration for textDocument/foldingRange");
                    let _ = connection.sender.send(Message::Request(req));
                }
            }
            Message::Response(_) => {}
        }
    }

    io_threads.join().unwrap();
}

fn handle_request(req: &Request, documents: &HashMap<Uri, String>, conn: &Connection) {
    if req.method == "textDocument/foldingRange" {
        let params: FoldingRangeParams = serde_json::from_value(req.params.clone()).unwrap();
        let ranges = documents
            .get(&params.text_document.uri)
            .map(|text| compute_folding_ranges(text))
            .unwrap_or_default();
        eprintln!("[md-fold-server] foldingRange for {:?}: {} ranges", params.text_document.uri, ranges.len());

        let resp = Response {
            id: req.id.clone(),
            result: Some(serde_json::to_value(ranges).unwrap()),
            error: None,
        };
        conn.sender.send(Message::Response(resp)).unwrap();
    }
}

fn handle_notification(notif: &Notification, documents: &mut HashMap<Uri, String>) {
    match notif.method.as_str() {
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams =
                serde_json::from_value(notif.params.clone()).unwrap();
            documents.insert(params.text_document.uri, params.text_document.text);
        }
        "textDocument/didChange" => {
            let params: DidChangeTextDocumentParams =
                serde_json::from_value(notif.params.clone()).unwrap();
            if let Some(change) = params.content_changes.into_iter().last() {
                documents.insert(params.text_document.uri, change.text);
            }
        }
        "textDocument/didClose" => {
            let params: DidCloseTextDocumentParams =
                serde_json::from_value(notif.params.clone()).unwrap();
            documents.remove(&params.text_document.uri);
        }
        _ => {}
    }
}

fn compute_folding_ranges(text: &str) -> Vec<FoldingRange> {
    let lines: Vec<&str> = text.lines().collect();
    let mut ranges = Vec::new();

    // Front matter detection: must start at line 0 with `---`
    let loop_start = if lines.first().map(|l| l.trim()) == Some("---") {
        if let Some(j) = lines.iter().enumerate().skip(1).find_map(|(idx, l)| {
            if l.trim() == "---" { Some(idx) } else { None }
        }) {
            ranges.push(FoldingRange {
                start_line: 0,
                start_character: None,
                end_line: j as u32,
                end_character: None,
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: Some("---".into()),
            });
            j + 1
        } else {
            0
        }
    } else {
        0
    };

    // First pass: identify headings and code blocks
    let mut headings: Vec<(u32, usize)> = Vec::new(); // (line_number, level)
    let mut in_code_block = false;
    let mut code_block_start: u32 = 0;
    let mut blockquote_start: Option<u32> = None;
    let mut table_start: Option<u32> = None;
    let mut indented_code_start: Option<u32> = None;

    for (i, line) in lines.iter().enumerate().skip(loop_start) {
        let line_num = i as u32;
        let trimmed = line.trim();

        // Code fence detection
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            if in_code_block {
                // Closing fence
                if line_num > code_block_start {
                    ranges.push(FoldingRange {
                        start_line: code_block_start,
                        start_character: None,
                        end_line: line_num,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("...".into()),
                    });
                }
                in_code_block = false;
            } else {
                in_code_block = true;
                code_block_start = line_num;
            }
            continue;
        }

        if in_code_block {
            continue;
        }

        // Heading detection
        if let Some(level) = heading_level(trimmed) {
            // Close any open blockquote before this heading
            if let Some(bq_start) = blockquote_start.take() {
                let end = last_non_blank(bq_start as usize + 1, i, &lines);
                if end > bq_start {
                    ranges.push(FoldingRange {
                        start_line: bq_start,
                        start_character: None,
                        end_line: end,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("> ...".into()),
                    });
                }
            }
            // Close any open table before this heading
            if let Some(tbl_start) = table_start.take() {
                if line_num - tbl_start >= 2 {
                    let end = last_non_blank(tbl_start as usize + 1, i, &lines);
                    ranges.push(FoldingRange {
                        start_line: tbl_start,
                        start_character: None,
                        end_line: end,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("| ...".into()),
                    });
                }
            }
            // Close any open indented code block before this heading
            if let Some(ic_start) = indented_code_start.take() {
                let end = last_non_blank(ic_start as usize + 1, i, &lines);
                if end > ic_start {
                    ranges.push(FoldingRange {
                        start_line: ic_start,
                        start_character: None,
                        end_line: end,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("    ...".into()),
                    });
                }
            }
            headings.push((line_num, level));
            continue;
        }

        // Table detection: consecutive lines starting with '|'
        if trimmed.starts_with('|') {
            if table_start.is_none() {
                table_start = Some(line_num);
            }
            continue;
        } else if let Some(tbl_start) = table_start.take() {
            if line_num - tbl_start >= 2 {
                let end = last_non_blank(tbl_start as usize + 1, i, &lines);
                ranges.push(FoldingRange {
                    start_line: tbl_start,
                    start_character: None,
                    end_line: end,
                    end_character: None,
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: Some("| ...".into()),
                });
            }
        }

        // Blockquote detection
        if trimmed.starts_with('>') {
            if blockquote_start.is_none() {
                blockquote_start = Some(line_num);
            }
        } else if !trimmed.is_empty() {
            // Non-blank, non-blockquote line ends a blockquote
            if let Some(bq_start) = blockquote_start.take() {
                let end = last_non_blank(bq_start as usize + 1, i, &lines);
                if end > bq_start {
                    ranges.push(FoldingRange {
                        start_line: bq_start,
                        start_character: None,
                        end_line: end,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("> ...".into()),
                    });
                }
            }
        }

        // Indented code block detection (4+ spaces or tab)
        let is_indented = line.starts_with("    ") || line.starts_with('\t');
        if is_indented {
            if indented_code_start.is_none() {
                indented_code_start = Some(line_num);
            }
        } else if !trimmed.is_empty() {
            // Non-blank, non-indented line ends an indented code block
            if let Some(ic_start) = indented_code_start.take() {
                let end = last_non_blank(ic_start as usize + 1, i, &lines);
                if end > ic_start {
                    ranges.push(FoldingRange {
                        start_line: ic_start,
                        start_character: None,
                        end_line: end,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: Some("    ...".into()),
                    });
                }
            }
        }
    }

    // Close remaining blockquote at end of file
    if let Some(bq_start) = blockquote_start {
        let end = last_non_blank(bq_start as usize + 1, lines.len(), &lines);
        if end > bq_start {
            ranges.push(FoldingRange {
                start_line: bq_start,
                start_character: None,
                end_line: end,
                end_character: None,
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: Some("> ...".into()),
            });
        }
    }

    // Close remaining table at end of file
    if let Some(tbl_start) = table_start {
        let end = last_non_blank(tbl_start as usize + 1, lines.len(), &lines);
        if end > tbl_start && end - tbl_start >= 2 {
            ranges.push(FoldingRange {
                start_line: tbl_start,
                start_character: None,
                end_line: end,
                end_character: None,
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: Some("| ...".into()),
            });
        }
    }

    // Close remaining indented code block at end of file
    if let Some(ic_start) = indented_code_start {
        let end = last_non_blank(ic_start as usize + 1, lines.len(), &lines);
        if end > ic_start {
            ranges.push(FoldingRange {
                start_line: ic_start,
                start_character: None,
                end_line: end,
                end_character: None,
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: Some("    ...".into()),
            });
        }
    }

    // Second pass: compute heading section fold ranges
    // Each heading folds to just before the next heading of same or higher level (or EOF)
    for (idx, &(start_line, level)) in headings.iter().enumerate() {
        let section_end = headings[idx + 1..]
            .iter()
            .find(|&&(_, l)| l <= level)
            .map(|&(line, _)| line as usize)
            .unwrap_or(lines.len());

        let end = last_non_blank(start_line as usize + 1, section_end, &lines);
        if end > start_line {
            ranges.push(FoldingRange {
                start_line,
                start_character: None,
                end_line: end,
                end_character: None,
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: None, // Zed shows the heading text by default
            });
        }
    }

    ranges
}

/// Returns the heading level (1-6) if the line is an ATX heading, None otherwise.
fn heading_level(line: &str) -> Option<usize> {
    if !line.starts_with('#') {
        return None;
    }
    let level = line.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    // Must be followed by a space or be just `#` characters
    if line.len() == level || line.as_bytes()[level] == b' ' {
        Some(level)
    } else {
        None
    }
}

/// Find the last non-blank line in range [start, end), returning its line number.
/// Returns `start - 1` if no non-blank lines found (meaning the range is empty/blank).
fn last_non_blank(start: usize, end: usize, lines: &[&str]) -> u32 {
    for i in (start..end).rev() {
        if !lines[i].trim().is_empty() {
            return i as u32;
        }
    }
    (start.saturating_sub(1)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_level() {
        assert_eq!(heading_level("# H1"), Some(1));
        assert_eq!(heading_level("## H2"), Some(2));
        assert_eq!(heading_level("### H3"), Some(3));
        assert_eq!(heading_level("#### H4"), Some(4));
        assert_eq!(heading_level("#Not a heading"), None);
        assert_eq!(heading_level("####### Too many"), None);
        assert_eq!(heading_level("Regular text"), None);
        assert_eq!(heading_level(""), None);
        assert_eq!(heading_level("#"), Some(1));
    }

    #[test]
    fn test_heading_folding() {
        let text = "\
# Title

Some intro text

## Section A

Content A

## Section B

Content B

### Subsection B1

Sub content

# Another Top Level

Final content";

        let ranges = compute_folding_ranges(text);
        let heading_ranges: Vec<_> = ranges
            .iter()
            .filter(|r| r.collapsed_text.is_none())
            .map(|r| (r.start_line, r.end_line))
            .collect();

        // # Title (line 0) -> folds to line 14 (last non-blank before # Another Top Level)
        // ## Section A (line 4) -> folds to line 6 (last non-blank before ## Section B)
        // ## Section B (line 8) -> folds to line 14 (last non-blank before # Another Top Level)
        // ### Subsection B1 (line 12) -> folds to line 14
        // # Another Top Level (line 16) -> folds to line 18
        assert!(heading_ranges.contains(&(0, 14)));
        assert!(heading_ranges.contains(&(4, 6)));
        assert!(heading_ranges.contains(&(8, 14)));
        assert!(heading_ranges.contains(&(12, 14)));
        assert!(heading_ranges.contains(&(16, 18)));
    }

    #[test]
    fn test_code_block_folding() {
        let text = "\
# Heading

```python
def hello():
    print('hi')
```

More text";

        let ranges = compute_folding_ranges(text);
        let code_ranges: Vec<_> = ranges
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("..."))
            .map(|r| (r.start_line, r.end_line))
            .collect();

        // Code block from line 2 to line 5
        assert!(code_ranges.contains(&(2, 5)));
    }

    #[test]
    fn test_blockquote_folding() {
        let text = "\
# Heading

> This is a blockquote
> that spans multiple
> lines

Regular text";

        let ranges = compute_folding_ranges(text);
        let bq_ranges: Vec<_> = ranges
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("> ..."))
            .map(|r| (r.start_line, r.end_line))
            .collect();

        // Blockquote from line 2 to line 4
        assert!(bq_ranges.contains(&(2, 4)));
    }

    #[test]
    fn test_tilde_code_block() {
        let text = "\
~~~
some code
~~~";

        let ranges = compute_folding_ranges(text);
        let code_ranges: Vec<_> = ranges
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("..."))
            .map(|r| (r.start_line, r.end_line))
            .collect();

        assert!(code_ranges.contains(&(0, 2)));
    }

    #[test]
    fn test_front_matter_folding() {
        // Front matter at start of file should fold
        let text = "\
---
title: My Doc
date: 2024-01-01
---

# Heading

Some content";

        let ranges = compute_folding_ranges(text);
        let fm_ranges: Vec<_> = ranges
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("---"))
            .map(|r| (r.start_line, r.end_line))
            .collect();

        // Front matter from line 0 to line 3
        assert_eq!(fm_ranges, vec![(0, 3)]);

        // Heading should still fold correctly
        let heading_ranges: Vec<_> = ranges
            .iter()
            .filter(|r| r.collapsed_text.is_none())
            .map(|r| (r.start_line, r.end_line))
            .collect();
        assert!(heading_ranges.contains(&(5, 7)));

        // --- NOT at line 0 should NOT create a front matter fold
        let text_no_fm = "\
# Heading

---

Some content";

        let ranges_no_fm = compute_folding_ranges(text_no_fm);
        let fm_ranges_no_fm: Vec<_> = ranges_no_fm
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("---"))
            .collect();
        assert!(fm_ranges_no_fm.is_empty());
    }

    #[test]
    fn test_table_folding() {
        // A 5-line table (header + separator + 3 rows) should create a fold
        let text = "\
# Heading

| Column A | Column B |
|----------|----------|
| a1       | b1       |
| a2       | b2       |
| a3       | b3       |

Some text after";

        let ranges = compute_folding_ranges(text);
        let table_ranges: Vec<_> = ranges
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("| ..."))
            .map(|r| (r.start_line, r.end_line))
            .collect();

        // Table from line 2 to line 6
        assert_eq!(table_ranges, vec![(2, 6)]);

        // A single pipe line should NOT create a fold
        let text_single = "\
# Heading

| just one line

More text";

        let ranges_single = compute_folding_ranges(text_single);
        let table_ranges_single: Vec<_> = ranges_single
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("| ..."))
            .collect();
        assert!(table_ranges_single.is_empty());
    }

    #[test]
    fn test_indented_code_block_folding() {
        let text = "# Heading\n\n    first line of code\n    second line of code\n    third line of code\n\nSome text after";

        let ranges = compute_folding_ranges(text);
        let ic_ranges: Vec<_> = ranges
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("    ..."))
            .map(|r| (r.start_line, r.end_line))
            .collect();

        // Indented code block from line 2 to line 4
        assert_eq!(ic_ranges, vec![(2, 4)]);

        // Indented code with blank lines in the middle should stay as one block
        let text_with_blanks = "    line one\n\n    line three\n    line four";

        let ranges_blanks = compute_folding_ranges(text_with_blanks);
        let ic_ranges_blanks: Vec<_> = ranges_blanks
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("    ..."))
            .map(|r| (r.start_line, r.end_line))
            .collect();

        // Should be a single fold from line 0 to line 3
        assert_eq!(ic_ranges_blanks, vec![(0, 3)]);

        // Tab-indented code should also work
        let text_tabs = "\tfirst line\n\tsecond line\n\tthird line";

        let ranges_tabs = compute_folding_ranges(text_tabs);
        let ic_ranges_tabs: Vec<_> = ranges_tabs
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("    ..."))
            .map(|r| (r.start_line, r.end_line))
            .collect();

        assert_eq!(ic_ranges_tabs, vec![(0, 2)]);

        // A single indented line should NOT create a fold
        let text_single = "# Heading\n\n    just one line\n\nMore text";

        let ranges_single = compute_folding_ranges(text_single);
        let ic_ranges_single: Vec<_> = ranges_single
            .iter()
            .filter(|r| r.collapsed_text.as_deref() == Some("    ..."))
            .collect();
        assert!(ic_ranges_single.is_empty());
    }
}
