fn safe_url(url: &str, image: bool) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    if lower.chars().any(char::is_control) {
        return false;
    }
    lower.starts_with("https://")
        || lower.starts_with("http://")
        || (image && !lower.contains(':') && !lower.starts_with("//") && !lower.contains(".."))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn performance_notice_has_a_safe_four_column_table() {
        let html = blocks(include_str!("../../NOTICE.md"), true, true);
        assert_eq!(html.matches("<th scope=").count(), 4);
        assert_eq!(html.matches("<td>").count(), 16);
        assert!(html.contains("342.0MiB"));
        assert!(!html.contains("| ---"));
        let hostile = blocks("| A | B |\n| --- | --- |\n| <script>x</script> | <img onerror=x> |", true, false);
        assert!(!hostile.contains("<script>"));
        assert!(!hostile.contains("<img "));
    }
    #[test]
    fn renders_docs_without_active_content() {
        let html = blocks(
            "# 제목\n\n- 첫 항목\n- 두 번째\n\n[공식](https://example.com)\n\n<script>alert(1)</script>\n\n[실행](javascript:alert%281%29)",
            false,
            false,
        );
        assert!(html.contains("<h1>제목</h1>"));
        assert!(html.contains("<li>첫 항목</li>"));
        assert!(html.contains("href=\"https://example.com\""));
        assert!(!html.contains("<script>"));
        assert!(!html.contains("href=\"javascript:"));
    }
}

// Preserve MarkdownBlocks.svelte's block-only DOM, including its CSS hooks.
pub fn blocks(source: &str, notice: bool, skip_title: bool) -> String {
    fn escape(text: &str) -> String {
        let mut decoded = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\'
                && chars
                    .peek()
                    .is_some_and(|c| "\\`*_[]{}()#+-.!".contains(*c))
            {
                decoded.push(chars.next().unwrap());
            } else {
                decoded.push(c);
            }
        }
        decoded
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }
    fn flush(out: &mut String, paragraph: &mut Vec<String>, list: &mut Vec<String>) {
        if !paragraph.is_empty() {
            out.push_str(&format!("<p>{}</p>", paragraph.join(" ")));
            paragraph.clear();
        }
        if !list.is_empty() {
            out.push_str("<ul>");
            for item in list.drain(..) {
                out.push_str(&format!("<li>{item}</li>"));
            }
            out.push_str("</ul>");
        }
    }
    let mut source = source.to_owned();
    if !notice {
        while let Some(start) = source.find("<!--") {
            if let Some(end) = source[start..].find("-->") {
                source.replace_range(start..start + end + 3, "");
            } else {
                break;
            }
        }
    }
    let mut out = String::new();
    let mut paragraph = Vec::new();
    let mut list = Vec::new();
    let mut skipped = false;
    let mut lines = source.lines().map(str::trim).peekable();
    while let Some(line) = lines.next() {
        if line.starts_with('|') && line.ends_with('|') {
            let cells = |s: &str| s.trim_matches('|').split('|').map(|c| c.trim().to_owned()).collect::<Vec<_>>();
            let headers = cells(line);
            let separator = lines.peek().map(|s| cells(s)).unwrap_or_default();
            if separator.len() == headers.len() && separator.iter().all(|s| {
                let dashes = s.trim_matches(':'); dashes.len() >= 3 && dashes.chars().all(|c| c == '-')
            }) {
                flush(&mut out, &mut paragraph, &mut list);
                lines.next();
                out.push_str("<div class=\"markdown-table-scroll\"><table class=\"markdown-table\"><thead><tr>");
                for h in &headers { out.push_str(&format!("<th scope=\"col\">{}</th>", escape(h))); }
                out.push_str("</tr></thead><tbody>");
                while lines.peek().is_some_and(|s| s.starts_with('|') && s.ends_with('|')) {
                    let row = cells(lines.next().unwrap());
                    out.push_str("<tr>");
                    for i in 0..headers.len() { out.push_str(&format!("<td>{}</td>", escape(row.get(i).map(String::as_str).unwrap_or("")))); }
                    out.push_str("</tr>");
                }
                out.push_str("</tbody></table></div>");
                continue;
            }
        }
        let level = line.chars().take_while(|c| *c == '#').count();
        if (1..=3).contains(&level) && line.as_bytes().get(level) == Some(&b' ') {
            flush(&mut out, &mut paragraph, &mut list);
            if skip_title && level == 1 && !skipped {
                skipped = true;
                continue;
            }
            out.push_str(&format!(
                "<h{level}>{}</h{level}>",
                escape(line[level + 1..].trim())
            ));
            continue;
        }
        if line.starts_with("- ") || line.starts_with("* ") {
            if !paragraph.is_empty() {
                out.push_str(&format!("<p>{}</p>", paragraph.join(" ")));
                paragraph.clear();
            }
            list.push(escape(line[2..].trim()));
            continue;
        }
        let image = line.starts_with("![");
        let prefix = if image { "![" } else { "[" };
        let link = line
            .strip_prefix(prefix)
            .and_then(|s| s.strip_suffix(')'))
            .and_then(|s| s.split_once("]("));
        if let Some((label, url)) = link {
            let allowed = if image {
                if notice {
                    url.starts_with("https://") && safe_url(url, true)
                } else {
                    url.starts_with("./doc_operation/") && safe_url(url, true)
                }
            } else {
                url.starts_with("https://") && safe_url(url, false)
            };
            if allowed {
                flush(&mut out, &mut paragraph, &mut list);
                if image {
                    let url = if notice {
                        url.to_owned()
                    } else {
                        format!("/public/{}", url.trim_start_matches("./"))
                    };
                    out.push_str(&format!(
                        "<img class=\"markdown-image\" src=\"{}\" alt=\"{}\" draggable=\"false\">",
                        escape(&url),
                        escape(label)
                    ));
                } else {
                    out.push_str(&format!(
                        "<a class=\"markdown-link\" href=\"{}\">{}</a>",
                        escape(url),
                        escape(label)
                    ));
                }
                continue;
            }
        }
        if line.is_empty() {
            flush(&mut out, &mut paragraph, &mut list);
        } else {
            if !list.is_empty() {
                flush(&mut out, &mut paragraph, &mut list);
            }
            paragraph.push(escape(line));
        }
    }
    flush(&mut out, &mut paragraph, &mut list);
    out
}

pub fn history(source: &str) -> Vec<(String, Vec<String>)> {
    let mut entries: Vec<(String, Vec<String>)> = Vec::new();
    for line in source.lines().map(str::trim) {
        if let Some(version) = line.strip_prefix("## ") {
            entries.push((version.into(), Vec::new()));
        } else if let Some(text) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            if let Some(entry) = entries.last_mut() {
                entry.1.push(text.into());
            }
        }
    }
    entries
}
