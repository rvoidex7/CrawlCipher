use rust_embed::RustEmbed;
use std::fs;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "assets/backgrounds/"]
struct Asset;

pub struct BackgroundPattern {
    pub rows: Vec<String>,
    pub width: usize,
    pub height: usize,
}

impl BackgroundPattern {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            width: 0,
            height: 0,
        }
    }

    pub fn load_from_embedded(&mut self, filename: &str) -> bool {
        if let Some(file) = Asset::get(filename) {
            let content = match file.data {
                Cow::Borrowed(bytes) => std::str::from_utf8(bytes).unwrap_or(""),
                Cow::Owned(ref bytes) => std::str::from_utf8(bytes).unwrap_or(""),
            };
            self.parse_content(content);
            true
        } else {
            false
        }
    }

    pub fn load_from_file(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        self.parse_content(&content);
        Ok(())
    }

    fn parse_content(&mut self, content: &str) {
        self.rows = content.lines().map(|s| s.to_string()).collect();
        self.height = self.rows.len();
        self.width = self.rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    }

    pub fn get_char(&self, x: i32, y: i32) -> char {
        if self.height == 0 {
            return ' ';
        }
        let row_idx = (y as usize) % self.height;
        let row = &self.rows[row_idx];

        if row.is_empty() { return ' '; }

        let char_idx = (x as usize) % row.chars().count();
        row.chars().nth(char_idx).unwrap_or(' ')
    }
}

pub fn list_embedded_backgrounds() -> Vec<String> {
    Asset::iter().map(|f| f.to_string()).collect()
}
