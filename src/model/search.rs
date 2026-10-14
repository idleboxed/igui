// SPDX-License-Identifier: BSD-3-Clause
//! Ephemeral ASCII editor and keyboard shared by navigation and rendering.

pub const KEY_ROWS: [&str; 4] = ["0123456789", "ABCDEFGHIJ", "KLMNOPQRST", "UVWXYZ "];
pub const COLUMNS: usize = 10;
pub const MAX_QUERY: usize = 128;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Search {
    query: String,
    cursor: usize,
    row: usize,
    column: usize,
}

impl Search {
    /// Borrow the current search query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Return the query's text cursor position.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Return the selected on-screen keyboard row and column.
    pub fn key_position(&self) -> (usize, usize) {
        (self.row, self.column)
    }

    /// Input is restricted to the ASCII keyboard; no per-title lowercase copy.
    pub fn matches(&self, title: &str) -> bool {
        self.query.is_empty()
            || title
                .as_bytes()
                .windows(self.query.len())
                .any(|part| part.eq_ignore_ascii_case(self.query.as_bytes()))
    }

    /// Move the text cursor one position right or left, staying inside the query.
    pub(crate) fn move_cursor(&mut self, right: bool) {
        self.cursor = if right {
            (self.cursor + 1).min(self.query.len())
        } else {
            self.cursor.saturating_sub(1)
        };
    }

    /// False means the caller should handle leaving the keyboard at this edge.
    pub(crate) fn move_key(&mut self, rows: isize, columns: isize) -> bool {
        let row = self.row.checked_add_signed(rows);
        let column = self.column.checked_add_signed(columns);
        let Some(row) = row.filter(|&row_item| row_item < KEY_ROWS.len()) else {
            return false;
        };
        let Some(column) = column else {
            return false;
        };
        if columns != 0 && column >= KEY_ROWS[row].len() {
            return false;
        }
        self.row = row;
        self.column = column.min(KEY_ROWS[row].len() - 1);
        true
    }

    /// Insert the currently selected on-screen key within the query length limit.
    pub(crate) fn insert_key(&mut self) -> bool {
        if self.query.len() == MAX_QUERY {
            return false;
        }
        self.query.insert(
            self.cursor,
            KEY_ROWS[self.row].as_bytes()[self.column] as char,
        );
        self.cursor += 1;
        true
    }

    /// Delete the query character before the text cursor, if one exists.
    pub(crate) fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.query.remove(self.cursor);
        true
    }

    /// Clear the query and reset its text cursor.
    pub(crate) fn clear(&mut self) -> bool {
        let changed = !self.query.is_empty();
        self.query.clear();
        self.cursor = 0;
        changed
    }
}
