pub(super) fn tokenize_svg_tags(svg_text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut in_tag = false;
    let mut current_tag = String::new();
    let mut in_quote: Option<char> = None;
    let mut in_comment = false;
    // Index of the currently open <text> element so nested markup content
    // (<tspan>, <tref>, …) is accumulated into it instead of being dropped.
    // Text goes to a side buffer (flushed into the tag on </text>) so that
    // <tspan>/<br> boundaries can inject explicit line breaks.
    let mut open_text_idx: Option<usize> = None;
    let mut open_text_buf = String::new();
    // Presentational hints inside the open <text>: <b>/<strong> and
    // <i>/<em> upgrade the text style when no explicit font-weight /
    // font-style attribute is present.
    let mut text_had_bold = false;
    let mut text_had_italic = false;
    // Whether the innermost open <tspan> carries positioning (x/y/dx/dy):
    // only those (and <br>) start a new line. Plain adjacent tspans are
    // same-line continuations and must not inject breaks.
    let mut tspan_break_pending = false;

    let chars: Vec<char> = svg_text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        // Check for XML comments <!-- ... -->
        if in_quote.is_none()
            && i + 3 < chars.len()
            && chars[i] == '<'
            && chars[i + 1] == '!'
            && chars[i + 2] == '-'
            && chars[i + 3] == '-'
        {
            in_comment = true;
            i += 4;
            continue;
        }
        if in_comment {
            if i + 2 < chars.len() && chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>'
            {
                in_comment = false;
                i += 3;
            } else {
                i += 1;
            }
            continue;
        }

        if let Some(q) = in_quote {
            current_tag.push(c);
            if c == q {
                in_quote = None;
            }
            i += 1;
        } else if c == '"' || c == '\'' {
            if in_tag {
                in_quote = Some(c);
            }
            current_tag.push(c);
            i += 1;
        } else if c == '<' {
            in_tag = true;
            current_tag.clear();
            current_tag.push(c);
            i += 1;
        } else if c == '>' && in_tag {
            in_tag = false;
            current_tag.push(c);
            tags.push(current_tag.clone());
            let pushed = tags.len() - 1;
            let tag_head = tags[pushed]
                .split(|ch: char| ch.is_whitespace() || ch == '>')
                .next()
                .unwrap_or("");
            if tag_head == "<text" {
                open_text_idx = Some(pushed);
                open_text_buf.clear();
                text_had_bold = false;
                text_had_italic = false;
            } else if tag_head == "</text" {
                // Flush accumulated text (with tspan/br line breaks) into
                // the <text> tag so content extraction below just works.
                // Presentational hints ride along as marker attributes.
                if let (Some(idx), buf) = (open_text_idx, std::mem::take(&mut open_text_buf))
                {
                    if let Some(tag) = tags.get_mut(idx) {
                        tag.push_str(&buf);
                        if text_had_bold {
                            tag.push_str(" data-tbold=\"1\"");
                        }
                        if text_had_italic {
                            tag.push_str(" data-titalic=\"1\"");
                        }
                    }
                }
                open_text_idx = None;
            } else if open_text_idx.is_some() {
                if tag_head == "<b" || tag_head == "<strong" {
                    text_had_bold = true;
                } else if tag_head == "<i" || tag_head == "<em" {
                    text_had_italic = true;
                }
                if tag_head == "<tspan" {
                    let tag = &tags[pushed];
                    tspan_break_pending = tag.contains("x=")
                        || tag.contains("y=")
                        || tag.contains("dx=")
                        || tag.contains("dy=");
                } else if tag_head == "</tspan" {
                    if tspan_break_pending
                        && !open_text_buf.is_empty()
                        && !open_text_buf.ends_with('\n')
                    {
                        open_text_buf.push('\n');
                    }
                    tspan_break_pending = false;
                } else if (tag_head.starts_with("<br") || tag_head.starts_with("</br"))
                    && !open_text_buf.is_empty() && !open_text_buf.ends_with('\n') {
                        open_text_buf.push('\n');
                    }
            }
            current_tag.clear();
            i += 1;
        } else if in_tag {
            if c == '\n' || c == '\r' || c == '\t' {
                current_tag.push(' ');
            } else {
                current_tag.push(c);
            }
            i += 1;
        } else {
            // Text content outside tags (for <text>...</text>, including
            // nested <tspan> content which belongs to the open <text>)
            if open_text_idx.is_some() {
                open_text_buf.push(c);
            } else if let Some(last) = tags.last_mut() {
                if last.starts_with("<text") && !last.contains("</text>") {
                    last.push(c);
                }
            }
            i += 1;
        }
    }
    tags
}

