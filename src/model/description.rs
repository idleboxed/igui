//! Shared text layout for model scroll bounds and the fixed-width renderer.
pub const COLUMNS: usize = 76;
pub const ROWS: usize = 6;

/// Preserve paragraphs and split long words at Unicode scalar boundaries.
pub fn wrap(value: &str, columns: usize) -> Vec<String> {
    if columns == 0 || value.trim().is_empty() {
        return vec![];
    }
    let mut lines = Vec::new();
    for paragraph in value.lines() {
        let mut line = String::new();
        let mut length = 0;
        for word in paragraph.split_whitespace() {
            let chars: Vec<_> = word.chars().collect();
            if length != 0 && length + 1 + chars.len() > columns {
                lines.push(std::mem::take(&mut line));
                length = 0;
            }
            if length != 0 {
                line.push(' ');
                length += 1;
            }
            for chunk in chars.chunks(columns) {
                if length == columns {
                    lines.push(std::mem::take(&mut line));
                    length = 0;
                }
                line.extend(chunk);
                length += chunk.len();
            }
        }
        lines.push(line);
    }
    lines
}
