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
    pub is_procedural: bool,
    pub seed: i64,
}

impl BackgroundPattern {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            width: 0,
            height: 0,
            is_procedural: false,
            seed: 0,
        }
    }

    pub fn set_seed(&mut self, seed: i64) {
        self.seed = seed;
    }

    pub fn enable_procedural(&mut self) {
        self.is_procedural = true;
        self.width = 1; // So it doesn't trigger checkerboard fallback in ui.rs
        self.height = 1;
    }

    pub fn load_from_embedded(&mut self, filename: &str) -> bool {
        self.is_procedural = false;
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
        self.is_procedural = false;
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
        if self.is_procedural {
            // Deterministic hash based on coordinates and the game's seed
            let mut hash = self.seed as u64;
            hash = hash.wrapping_add((x as u64).wrapping_mul(0x9E3779B185EBCA87));
            hash ^= hash >> 33;
            hash = hash.wrapping_mul(0xC2B2AE3D27D4EB4F);
            hash ^= hash >> 29;
            hash = hash.wrapping_add((y as u64).wrapping_mul(0x85EBCA77C2B2AE63));
            hash ^= hash >> 32;
            hash = hash.wrapping_mul(0x165667B19E3779F9);
            hash ^= hash >> 32;

            // Density threshold: 85% empty space, 15% crypto characters
            if hash % 100 > 15 {
                return ' ';
            }

            // Cryptography-themed character set
            let chars = ['0', '1', 'A', 'F', 'X', 'C', '4', '8', 'E', '.', '-', '+', '|', '/', '\\', '#', '@'];
            let idx = (hash % (chars.len() as u64)) as usize;
            return chars[idx];
        }

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
    let mut bgs = vec!["PROCEDURAL_CRYPTO".to_string()];
    bgs.extend(Asset::iter().map(|f| f.to_string()));
    bgs
}
