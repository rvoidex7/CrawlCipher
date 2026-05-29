//! # Ratatui Splash Screen (Combined Masterpiece)
//!
//! A clean, cinematic splash screen.
//! Only the mascot animation and RATATUI text. No debug controls.

use std::time::Instant;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Modifier};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{backend::Backend, Frame, Terminal};

const LOGO_LINES: [&str; 5] = [
    "██████      ████    ██████    ████    ██████  ██    ██  ██",
    "██    ██  ██    ██    ██    ██    ██    ██    ██    ██  ██",
    "██████    ████████    ██    ████████    ██    ██    ██  ██",
    "██  ██    ██    ██    ██    ██    ██    ██    ██    ██  ██",
    "██    ██  ██    ██    ██    ██    ██    ██      ████    ██",
];

const STEPS: [usize; 8] = [0, 10, 20, 28, 38, 46, 56, 58];

const RATATUI_MASCOT: &str = "               hhh
             hhhhhh
            hhhhhhh
           hhhhhhhh
          hhhhhhhhh
         hhhhhhhhhh
        hhhhhhhhhhhh
        hhhhhhhhhhhhh
        hhhhhhhhhhhhh     ██████
         hhhhhhhhhhh    ████████
              hhhhh ███████████
               hhh ██ee████████
                h █████████████
            ████ █████████████
           █████████████████
           ████████████████
           ████████████████
            ███ ██████████
          ▒▒    █████████
         ▒░░▒   █████████
        ▒░░░░▒ ██████████
       ▒░░▓░░░▒ █████████
      ▒░░▓▓░░░░▒ ████████
     ▒░░░░░░░░░░▒ ██████████
    ▒░░░░░░░░░░░░▒ ██████████
   ▒░░░░░░░▓▓░░░░░▒ █████████
  ▒░░░░░░░░░▓▓░░░░░▒ ████  ███
  ▒░░░░░░░░░░░░░░░░░░▒ ██   ███
 ▒░░░░░░░░░░░░░░░░░░░░▒ █   ███
 ▒░░░░░░░░░░░░░░░░░░░░░▒   ███
  ▒░░░░░░░░░░░░░░░░░░░░░▒ ███
   ▒░░░░░░░░░░░░░░░░░░░░░▒ █";

const EMPTY: char = ' ';
const SCREEN: char = '░';
const BEZEL: char = '▒';
const CONTENT: char = '▓';
const RAT: char = '█';
const HAT: char = 'h';
const EYE: char = 'e';

// --- MASCOT ENGINE ---

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MascotEyeColor {
    #[default]
    Default,
    Red,
}

#[derive(Debug, Clone, Copy)]
struct RatatuiMascot {
    eye_state: MascotEyeColor,
    rat_color: Color,
    rat_eye_color: Color,
    rat_eye_blink: Color,
    hat_color: Color,
    term_color: Color,
    term_border_color: Color,
    term_cursor_color: Color,
    hide_monitor: bool,
}

impl Default for RatatuiMascot {
    fn default() -> Self {
        Self {
            rat_color: Color::Indexed(252),
            hat_color: Color::Indexed(231),
            rat_eye_color: Color::Indexed(236),
            rat_eye_blink: Color::Indexed(196),
            term_color: Color::Indexed(232),
            term_border_color: Color::Indexed(237),
            term_cursor_color: Color::Indexed(248),
            eye_state: MascotEyeColor::Default,
            hide_monitor: false,
        }
    }
}

impl RatatuiMascot {
    const fn color_for(&self, c: char) -> Option<Color> {
        match c {
            RAT => Some(self.rat_color),
            HAT => Some(self.hat_color),
            EYE => Some(match self.eye_state {
                MascotEyeColor::Default => self.rat_eye_color,
                MascotEyeColor::Red => self.rat_eye_blink,
            }),
            SCREEN => Some(self.term_color),
            BEZEL => Some(self.term_border_color),
            CONTENT => Some(self.term_cursor_color),
            _ => None,
        }
    }

    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.is_empty() { return; }

        let lines: Vec<&str> = RATATUI_MASCOT.lines().collect();
        for (y, chunk) in lines.chunks(2).enumerate() {
            let line1 = chunk[0];
            let line2 = if chunk.len() > 1 { chunk[1] } else { "" };
            
            let chars1: Vec<char> = line1.chars().collect();
            let chars2: Vec<char> = line2.chars().collect();
            let min_len = chars1.len().min(chars2.len());

            for x in 0..min_len {
                let actual_x = area.left() + x as u16;
                let actual_y = area.top() + y as u16;

                if actual_x >= area.right() || actual_y >= area.bottom() {
                    continue;
                }

                let mut ch1 = chars1[x];
                let mut ch2 = chars2[x];

                if self.hide_monitor {
                    if matches!(ch1, '░' | '▒' | '▓') { ch1 = EMPTY; }
                    if matches!(ch2, '░' | '▒' | '▓') { ch2 = EMPTY; }
                }

                if ch1 == EMPTY && ch2 == EMPTY { continue; }

                let cell = buf.get_mut(actual_x, actual_y);
                let (fg, bg) = match (ch1, ch2) {
                    (EMPTY, EMPTY) => (None, None),
                    (c, EMPTY) | (EMPTY, c) => (self.color_for(c), None),
                    (SCREEN, BEZEL) => (self.color_for(BEZEL), self.color_for(SCREEN)),
                    (SCREEN, c) | (c, SCREEN) => (self.color_for(c), self.color_for(SCREEN)),
                    (c1, c2) => (self.color_for(c1), self.color_for(c2)),
                };
                
                let symbol = match (ch1, ch2) {
                    (EMPTY, EMPTY) => None,
                    (SCREEN, SCREEN) => Some(EMPTY),
                    (EMPTY, _) => Some('▄'),
                    (_, EMPTY) => Some('▀'),
                    (SCREEN, _) => Some('▄'),
                    (_, SCREEN) => Some('▀'),
                    (c, d) if c == d => Some(EMPTY),
                    (_, _) => Some('▀'),
                };

                if let Some(f) = fg { cell.fg = f; }
                if let Some(b) = bg { cell.bg = b; }
                if let Some(s) = symbol { cell.set_char(s); }
            }
        }
    }
}

fn draw_grid_halfblocks(
    grid: &[Vec<char>],
    area: Rect,
    buf: &mut Buffer,
    color_fn: impl Fn(char) -> Option<Color>,
) {
    let area = area.intersection(buf.area);
    if area.is_empty() || grid.is_empty() { return; }

    let h_lines = grid.len();
    let w_cols = grid[0].len();
    let terminal_rows = (h_lines + 1) / 2;
    let is_text = |c: char| !matches!(c, EMPTY | SCREEN | BEZEL | CONTENT);

    for y in 0..terminal_rows {
        let line1_idx = y * 2;
        let line2_idx = y * 2 + 1;
        let chars1 = if line1_idx < h_lines { grid[line1_idx].as_slice() } else { &[] };
        let chars2 = if line2_idx < h_lines { grid[line2_idx].as_slice() } else { &[] };

        for x in 0..w_cols {
            let actual_x = area.left() + x as u16;
            let actual_y = area.top() + y as u16;
            if actual_x >= area.right() || actual_y >= area.bottom() { continue; }

            let ch1 = chars1.get(x).copied().unwrap_or(EMPTY);
            let ch2 = chars2.get(x).copied().unwrap_or(EMPTY);
            
            if ch1 == EMPTY && ch2 == EMPTY { continue; }

            let cell = buf.get_mut(actual_x, actual_y);
            let is_t1 = is_text(ch1);
            let is_t2 = is_text(ch2);

            let (fg, bg) = if is_t1 {
                (color_fn(ch1), color_fn(ch2))
            } else if is_t2 {
                (color_fn(ch2), color_fn(ch1))
            } else {
                match (ch1, ch2) {
                    (c, EMPTY) | (EMPTY, c) => (color_fn(c), None),
                    (SCREEN, BEZEL) => (color_fn(BEZEL), color_fn(SCREEN)),
                    (SCREEN, c) | (c, SCREEN) => (color_fn(c), color_fn(SCREEN)),
                    (c1, c2) => (color_fn(c1), color_fn(c2)),
                }
            };

            let symbol = if is_t1 {
                Some(ch1)
            } else if is_t2 {
                Some(ch2)
            } else {
                match (ch1, ch2) {
                    (EMPTY, EMPTY) => None,
                    (SCREEN, SCREEN) => Some(EMPTY),
                    (EMPTY, _) => Some('▄'),
                    (_, EMPTY) => Some('▀'),
                    (SCREEN, _) => Some('▄'),
                    (_, SCREEN) => Some('▀'),
                    (c, d) if c == d => Some(EMPTY),
                    (_, _) => Some('▀'),
                }
            };

            if let Some(f) = fg { cell.fg = f; }
            if let Some(b) = bg { cell.bg = b; }
            if let Some(s) = symbol { cell.set_char(s); }
        }
    }
}

fn draw_line(grid: &mut Vec<Vec<char>>, x0: isize, y0: isize, x1: isize, y1: isize, ch: char) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if y >= 0 && y < grid.len() as isize && x >= 0 && x < grid[0].len() as isize {
            grid[y as usize][x as usize] = ch;
        }
        if x == x1 && y == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x += sx; }
        if e2 <= dx { err += dx; y += sy; }
    }
}

fn map_vert_to_diag(x: f64, y: f64) -> (f64, f64) {
    let u = (x - 4.0) / 27.0;
    let v = (y - 18.0) / 13.0;
    
    let d_tl = (14.0, 18.0);
    let d_tr = (30.0, 33.0);
    let d_bl = (4.0, 28.0);
    let d_br = (20.0, 43.0);

    let top_x = d_tl.0 + u * (d_tr.0 - d_tl.0);
    let top_y = d_tl.1 + u * (d_tr.1 - d_tl.1);
    let bot_x = d_bl.0 + u * (d_br.0 - d_bl.0);
    let bot_y = d_bl.1 + u * (d_br.1 - d_bl.1);

    let cur_x = top_x + v * (bot_x - top_x);
    let cur_y = top_y + v * (bot_y - top_y);
    (cur_x, cur_y)
}

fn render_vector_morph(
    global_t: f64,
    area: Rect,
    buf: &mut Buffer,
    color_fn: impl Fn(char) -> Option<Color>,
) {
    let area = area.intersection(buf.area);
    if area.is_empty() { return; }

    let w_cols = 46;
    let h_lines = 32;
    let mut grid = vec![vec![EMPTY; w_cols]; h_lines];

    if global_t >= 2.0 {
        let mut mascot_area = area;
        mascot_area.x += 4;
        RatatuiMascot::default().render(mascot_area, buf);
        return;
    }

    let is_typing_phase = global_t < 1.0;
    let typing_t = if is_typing_phase { global_t } else { 1.0 };
    let morph_t = if is_typing_phase { 0.0 } else { global_t - 1.0 };

    let v_tl_l = (4.0, 18.0);   
    let v_tl_r = (5.0, 18.0);   
    let v_tr = (31.0, 18.0);  
    let v_bl = (4.0, 31.0);  

    let d_tl_l = (14.0, 18.0);
    let d_tl_r = (15.0, 18.0);
    let d_tr = (30.0, 33.0);
    let d_bl = (4.0, 28.0);

    let t = morph_t;

    let cur_tl_r = (
        v_tl_r.0 + (d_tl_r.0 - v_tl_r.0) * t,
        v_tl_r.1 + (d_tl_r.1 - v_tl_r.1) * t,
    );

    let v_top_vec = (v_tr.0 - v_tl_r.0, v_tr.1 - v_tl_r.1);
    let d_top_vec = (d_tr.0 - d_tl_r.0, d_tr.1 - d_tl_r.1);

    let v_top_len = (v_top_vec.0 * v_top_vec.0 + v_top_vec.1 * v_top_vec.1).sqrt();
    let d_top_len = (d_top_vec.0 * d_top_vec.0 + d_top_vec.1 * d_top_vec.1).sqrt();
    let cur_top_len = v_top_len + (d_top_len - v_top_len) * t;

    let v_top_angle = v_top_vec.1.atan2(v_top_vec.0);
    let d_top_angle = d_top_vec.1.atan2(d_top_vec.0);
    let cur_top_angle = v_top_angle + (d_top_angle - v_top_angle) * t;

    let top_vec = (cur_top_len * cur_top_angle.cos(), cur_top_len * cur_top_angle.sin());

    let v_side_vec: (f64, f64) = (v_bl.0 - v_tl_l.0, v_bl.1 - v_tl_l.1);
    let d_side_vec: (f64, f64) = (d_bl.0 - d_tl_l.0, d_bl.1 - d_tl_l.1);

    let v_side_len = (v_side_vec.0 * v_side_vec.0 + v_side_vec.1 * v_side_vec.1).sqrt();
    let d_side_len = (d_side_vec.0 * d_side_vec.0 + d_side_vec.1 * d_side_vec.1).sqrt();
    let cur_side_len = v_side_len + (d_side_len - v_side_len) * t;

    let v_side_angle = v_side_vec.1.atan2(v_side_vec.0);
    let d_side_angle = d_side_vec.1.atan2(d_side_vec.0);
    let cur_side_angle = v_side_angle + (d_side_angle - v_side_angle) * t;

    let side_vec = (cur_side_len * cur_side_angle.cos(), cur_side_len * cur_side_angle.sin());

    let cur_tl_l = (cur_tl_r.0 - 1.0, cur_tl_r.1);
    let cur_tr = (cur_tl_r.0 + top_vec.0, cur_tl_r.1 + top_vec.1);
    let cur_bl = (cur_tl_l.0 + side_vec.0, cur_tl_l.1 + side_vec.1);
    let cur_br_l = (cur_bl.0 + top_vec.0, cur_bl.1 + top_vec.1);
    let cur_br_r = (cur_br_l.0 + 1.0, cur_br_l.1);

    draw_line(&mut grid, cur_tl_l.0.round() as isize, cur_tl_l.1.round() as isize, cur_tl_r.0.round() as isize, cur_tl_r.1.round() as isize, BEZEL);
    draw_line(&mut grid, cur_tl_r.0.round() as isize, cur_tl_r.1.round() as isize, cur_tr.0.round() as isize, cur_tr.1.round() as isize, BEZEL);
    draw_line(&mut grid, cur_tr.0.round() as isize, cur_tr.1.round() as isize, cur_br_r.0.round() as isize, cur_br_r.1.round() as isize, BEZEL);
    draw_line(&mut grid, cur_br_r.0.round() as isize, cur_br_r.1.round() as isize, cur_br_l.0.round() as isize, cur_br_l.1.round() as isize, BEZEL);
    draw_line(&mut grid, cur_br_l.0.round() as isize, cur_br_l.1.round() as isize, cur_bl.0.round() as isize, cur_bl.1.round() as isize, BEZEL);
    draw_line(&mut grid, cur_bl.0.round() as isize, cur_bl.1.round() as isize, cur_tl_l.0.round() as isize, cur_tl_l.1.round() as isize, BEZEL);

    for y in 0..h_lines {
        let mut first_bezel = None;
        let mut last_bezel = None;
        for x in 0..w_cols {
            if grid[y][x] == BEZEL {
                if first_bezel.is_none() { first_bezel = Some(x); }
                last_bezel = Some(x);
            }
        }
        if let (Some(f), Some(l)) = (first_bezel, last_bezel) {
            if l > f + 1 {
                for x in (f + 1)..l {
                    if grid[y][x] != BEZEL { grid[y][x] = SCREEN; }
                }
            }
        }
    }

    if is_typing_phase {
        let full_text = "> ratatui";
        let chars_to_show = ((typing_t * 9.0) as usize).min(9);
        let display_text = format!("{}_", &full_text[0..chars_to_show]);
        
        for (i, ch) in display_text.chars().enumerate() {
            if ch == ' ' { continue; }
            let px = 7 + i;
            let py = 21;
            grid[py][px] = ch;
        }
    } else {
        let fading_chars = [
            (7.0, 21.0, '>'),
            (16.0, 21.0, '_'),
        ];
        
        for &(sx, sy, ch) in &fading_chars {
            if t < 0.6 {
                let (ex, ey) = map_vert_to_diag(sx, sy);
                let cx = (sx * (1.0 - t) + ex * t).round() as isize;
                let cy = (sy * (1.0 - t) + ey * t).round() as isize;
                if cx >= 0 && cx < w_cols as isize && cy >= 0 && cy < h_lines as isize {
                    grid[cy as usize][cx as usize] = ch;
                }
            }
        }
        
        let mutating_chars = [
            (9.0, 21.0, 'r', 14.0, 21.0),
            (10.0, 21.0, 'a', 13.0, 22.0),
            (11.0, 21.0, 't', 14.0, 22.0),
            (12.0, 21.0, 'a', 14.0, 25.0),
            (13.0, 21.0, 't', 15.0, 25.0),
            (14.0, 21.0, 'u', 16.0, 26.0),
            (15.0, 21.0, 'i', 17.0, 26.0),
        ];
        
        for &(sx, sy, ch, ex, ey) in &mutating_chars {
            let cx = (sx * (1.0 - t) + ex * t).round() as isize;
            let cy = (sy * (1.0 - t) + ey * t).round() as isize;
            if cx >= 0 && cx < w_cols as isize && cy >= 0 && cy < h_lines as isize {
                let display_ch = if t < 0.85 { ch } else { CONTENT };
                grid[cy as usize][cx as usize] = display_ch;
            }
        }
    }

    let mut temp_buf = Buffer::empty(Rect::new(0, 0, 46, 16));
    let mut mascot = RatatuiMascot::default();
    mascot.hide_monitor = true;
    mascot.render(temp_buf.area, &mut temp_buf);

    let dy = ((1.0 - morph_t) * 12.0).round() as u16;

    for src_y in 0..16 {
        let dest_y = src_y + dy;
        if dest_y < 16 {
            let actual_dest_y = area.y + dest_y;
            for x in 0..46 {
                let actual_dest_x = area.x + x + 4;
                if buf.area.contains((actual_dest_x, actual_dest_y).into()) {
                    let cell = temp_buf.get(x, src_y);
                    let is_empty = cell.symbol() == " " && cell.bg == Color::Reset && cell.fg == Color::Reset;
                    if !is_empty {
                        let dest_cell = buf.get_mut(actual_dest_x, actual_dest_y);
                        *dest_cell = cell.clone();
                    }
                }
            }
        }
    }

    draw_grid_halfblocks(&grid, area, buf, color_fn);
}

// --- PURE SPLASH SCREEN APP ---

struct App {
    started_at: Instant,
}

impl App {
    pub fn run<B: Backend>(&self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            let elapsed_ms = self.started_at.elapsed().as_millis() as f64;
            
            // Timeout after 3.5 seconds to auto-transition
            if elapsed_ms >= 3500.0 {
                break;
            }

            terminal.draw(|frame| self.render(frame))?;

            if event::poll(Duration::from_millis(16))? {
                if let Event::Key(key) = event::read()? {
                    if key.code != KeyCode::Null { break; }
                }
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.size();
        let elapsed_ms = self.started_at.elapsed().as_millis() as f64;
        
        // Animation speed: 1.0 (Typewriter) = 1.5 seconds.
        // Therefore we can use global_t = elapsed_ms / 1500.0.
        let global_t = (elapsed_ms / 1500.0).clamp(0.0, 2.0);

        let bg_block = Block::default().style(Style::default().bg(Color::Reset));
        frame.render_widget(bg_block, area);

        // Center Stacked Layout
        // Mascot Height: 16 (32 rows of ascii art / 2)
        // Text Box Height: 9. Total Height: 25
        let stack_height = 25;
        let stack_width = 62;

        let mut start_y = area.y;
        if area.height > stack_height {
            start_y += (area.height - stack_height) / 2;
        }

        let main_v = Layout::vertical([
            Constraint::Length(16), // Mascot Area
            Constraint::Length(9),  // Big Text Area
        ]);

        let combined_rect = Rect::new(
            area.x,
            start_y,
            area.width,
            stack_height.min(area.height),
        );
        let chunks_combined = main_v.split(combined_rect);
        let mascot_row = chunks_combined[0];
        let text_row = chunks_combined[1];

        // Mascot centered horizontally
        let mascot_w = 46;
        let mut mascot_x = mascot_row.x;
        if mascot_row.width > mascot_w {
            mascot_x += (mascot_row.width - mascot_w) / 2;
        }
        let mascot_area = Rect::new(
            mascot_x,
            mascot_row.y,
            mascot_w.min(mascot_row.width),
            mascot_row.height,
        );

        // Text Box centered horizontally
        let mut text_x = text_row.x;
        if text_row.width > stack_width {
            text_x += (text_row.width - stack_width) / 2;
        }
        let text_box_area = Rect::new(
            text_x,
            text_row.y,
            stack_width.min(text_row.width),
            text_row.height,
        );

        let buf = frame.buffer_mut();
        let mascot_config = RatatuiMascot::default();
        let color_mapper = |c| {
            match c {
                CONTENT | '>' | '_' | 'r' | 'a' | 't' | 'u' | 'i' => Some(Color::Indexed(248)),
                _ => mascot_config.color_for(c),
            }
        };

        // --- 1. MASCOT ANIMATION ---
        render_vector_morph(global_t, mascot_area, buf, color_mapper);

        // --- 2. RATATUI OUTLINES AND TEXT ANIMATION ---
        if global_t >= 1.0 {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(120, 120, 120)));
            
            let inner_text_area = block.inner(text_box_area);
            frame.render_widget(block, text_box_area);

            // Padding to center the text
            let text_v_layout = Layout::vertical([
                Constraint::Length(1), // Top padding
                Constraint::Length(5), // Text height
                Constraint::Min(0),
            ]);
            let chunks_v = text_v_layout.split(inner_text_area);
            let text_area_v = chunks_v[1];

            let text_h_layout = Layout::horizontal([
                Constraint::Length(1), // Left padding
                Constraint::Length(58), // Text width
                Constraint::Min(0),
            ]);
            let chunks_h = text_h_layout.split(text_area_v);
            let text_area = chunks_h[1];

            // Text animation step
            let morph_t = global_t - 1.0;
            // 8 step animation (STEPS.len() = 8).
            let step_idx = (morph_t * 8.0) as usize;
            let step = step_idx.min(7);

            let mut logo_text = String::new();
            for line in &LOGO_LINES {
                let char_count = STEPS[step];
                let typed_part: String = line.chars().take(char_count).collect();
                
                if step < 7 {
                    logo_text.push_str(&format!("{typed_part}████\n"));
                } else {
                    logo_text.push_str(&format!("{typed_part}\n"));
                }
            }

            let ascii_paragraph = Paragraph::new(logo_text)
                .style(Style::default().fg(Color::Rgb(255, 255, 255)).add_modifier(Modifier::BOLD));
            
            frame.render_widget(ascii_paragraph, text_area);
        }
    }
}

pub fn run<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    let app = App { started_at: Instant::now() };
    app.run(terminal)
}

pub fn render_final_state(frame: &mut Frame) {
    let app = App { started_at: Instant::now() - Duration::from_secs(10) };
    app.render(frame);
}
