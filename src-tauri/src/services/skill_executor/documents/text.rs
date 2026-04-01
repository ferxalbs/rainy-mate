pub(super) fn normalize_document_text(input: &str) -> String {
    let normalized = input
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace("â€¢", "-")
        .replace("â€”", "-")
        .replace("â€“", "-")
        .replace("â€™", "'")
        .replace("â€œ", "\"")
        .replace("â€\u{009d}", "\"")
        .replace("â€¦", "...")
        .replace('•', "-")
        .replace('—', "-")
        .replace('–', "-")
        .replace('’', "'")
        .replace('‘', "'")
        .replace('“', "\"")
        .replace('”', "\"")
        .replace('…', "...")
        .replace('→', "->")
        .replace('←', "<-")
        .replace("**", "")
        .replace("__", "")
        .replace('`', "");

    normalized
        .lines()
        .map(strip_markdown_line_prefix)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn strip_markdown_line_prefix(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let without_heading = trimmed.trim_start_matches('#').trim();
    let without_bullet = without_heading
        .strip_prefix("- ")
        .or_else(|| without_heading.strip_prefix("* "))
        .or_else(|| without_heading.strip_prefix("+ "))
        .unwrap_or(without_heading);

    without_bullet.to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize_document_text;

    #[test]
    fn strips_common_markdown_and_mojibake() {
        let normalized = normalize_document_text("## Title\n- **Bold** point â€” done");
        assert_eq!(normalized, "Title\nBold point - done");
    }
}
