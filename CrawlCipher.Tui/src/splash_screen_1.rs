//! # Splash Screen
//!
//! An animated screen split vertically with a secret optical illusion text:
//! - Stage 1 (0ms - 1000ms): Purple (top) and Green (bottom) slide in to meet in the middle
//! - Stage 2 (1000ms - 2000ms): The horizontal division boundary rotates 63 degrees counterclockwise
//! - Stage 3 (2000ms - 3000ms): Purple slides left, Green slides right — a "door opening" effect
//!   that reveals the terminal's own background color in the center
//! - The "r" and "7" ASCII art glyphs are placed at the center. When a colored panel (purple/green)
//!   touches a character, it turns white.
//!   During the door opening, "r" slides left with purple and "7" slides right with green, merging
//!   perfectly with the revealed "RVOIDEX7" ASCII art in the center!

use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::style::{Color, Style};
use ratatui::widgets::Block;
use ratatui::{backend::Backend, Frame, Terminal};

const RVOIDEX7_ASCII: &str = r#"                               /$$       /$$                     /$$$$$$$$
                              |__/      | $$                    |_____ $$/
  /$$$$$$  /$$    /$$ /$$$$$$  /$$  /$$$$$$$  /$$$$$$  /$$   /$$     /$$/ 
 /$$__  $$|  $$  /$$//$$__  $$| $$ /$$__  $$ /$$__  $$|  $$ /$$/    /$$/  
| $$  \__/ \  $$/$$/| $$  \ $$| $$| $$  | $$| $$$$$$$$ \  $$$$/    /$$/   
| $$        \  $$$/ | $$  | $$| $$| $$  | $$| $$_____/  >$$  $$   /$$/    
| $$         \  $/  |  $$$$$$/| $$|  $$$$$$$|  $$$$$$$ /$$/\  $$ /$$/     
|__/          \_/    \______/ |__/ \_______/ \_______/|__/  \__/|__/      "#;

const LOGO_R: &str = r#"

  /$$$$$$
 /$$__  $$
| $$  \__/
| $$
| $$
|__/      "#;

const LOGO_7: &str = r#" /$$$$$$$$
|_____ $$/
    /$$/
   /$$/
  /$$/
 /$$/
|__/      "#;

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
                    if key.code != KeyCode::Null {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.size();
        let elapsed_ms = self.started_at.elapsed().as_millis() as f64;

        if elapsed_ms < 1000.0 {
            // Render final state of splash screen 2 as the background!
            crate::splash_screen_2::render_final_state(frame);
        } else {
            // After panels have met in the middle, clear with Color::Reset
            let bg_block = Block::default().style(Style::default().bg(Color::Reset));
            frame.render_widget(bg_block, area);
        }


        let w = area.width;
        let h = area.height;
        let xc = w as f64 / 2.0;
        let yc = h as f64 / 2.0;

        let max_theta: f64 = 1.099557; // 63 degrees (final rotation angle)
        let tan_theta = max_theta.tan();

        let main_buf = frame.buffer_mut();

        // ==========================================
        // 1. Draw the Purple and Green Panels
        // ==========================================
        if elapsed_ms < 1000.0 {
            // Stage 1: Slide in horizontally to meet in the middle
            let progress = elapsed_ms / 1000.0;
            let purple_limit = yc * progress;
            let green_limit = h as f64 - (h as f64 - yc) * progress;

            for y in 0..h {
                for x in 0..w {
                    let cell = main_buf.get_mut(x + area.x, y + area.y);
                    if (y as f64) < purple_limit {
                        cell.bg = Color::Rgb(147, 51, 234); // Purple
                        cell.set_char(' ');
                    } else if (y as f64) >= green_limit {
                        cell.bg = Color::Rgb(34, 197, 94); // Green
                        cell.set_char(' ');
                    }
                }
            }
        } else if elapsed_ms >= 1000.0 && elapsed_ms < 2000.0 {
            // Stage 2: Rotate the division line
            let progress = (elapsed_ms - 1000.0) / 1000.0;
            let theta = max_theta * progress;
            let current_tan = theta.tan();

            for y in 0..h {
                for x in 0..w {
                    let cell = main_buf.get_mut(x + area.x, y + area.y);
                    let y_boundary = yc - current_tan * (x as f64 - xc) * 0.5;
                    if (y as f64) < y_boundary {
                        cell.bg = Color::Rgb(147, 51, 234); // Purple
                        cell.set_char(' ');
                    } else {
                        cell.bg = Color::Rgb(34, 197, 94); // Green
                        cell.set_char(' ');
                    }
                }
            }
        } else {
            // Stage 3: Doors slide apart horizontally
            let stage3_progress = ((elapsed_ms - 2000.0) / 1000.0).clamp(0.0, 1.0);
            let max_offset = (w / 2) as f64;
            let offset = max_offset * stage3_progress;

            for y in 0..h {
                for x in 0..w {
                    let cell = main_buf.get_mut(x + area.x, y + area.y);
                    let purple_src_x = x as f64 + offset;
                    let y_boundary_purple = yc - tan_theta * (purple_src_x - xc) * 0.5;

                    let green_src_x = x as f64 - offset;
                    let y_boundary_green = yc - tan_theta * (green_src_x - xc) * 0.5;

                    if purple_src_x < w as f64 && (y as f64) < y_boundary_purple {
                        cell.bg = Color::Rgb(147, 51, 234); // Purple
                        cell.set_char(' ');
                    } else if green_src_x >= 0.0 && (y as f64) >= y_boundary_green {
                        cell.bg = Color::Rgb(34, 197, 94); // Green
                        cell.set_char(' ');
                    }
                }
            }
        }

        // ==========================================
        // 2. Text Render Preparation
        // ==========================================
        let art_h: u16 = 8;
        let art_y = area.y + h.saturating_sub(art_h) / 2;
        let center_x = area.x + w / 2;

        let text_offset: i32 = if elapsed_ms >= 2000.0 {
            let stage3_progress = ((elapsed_ms - 2000.0) / 1000.0).clamp(0.0, 1.0);
            (27.0 * stage3_progress) as i32
        } else {
            0
        };

        let buf = frame.buffer_mut();

        // ==========================================
        // 3. Draw LOGO_R and LOGO_7 (Only in Stage 1 & 2)
        // ==========================================
        if elapsed_ms < 2000.0 {
            // Render "r"
            for (line_idx, line) in LOGO_R.lines().enumerate() {
                for (col_idx, ch) in line.chars().enumerate() {
                    if ch == ' ' {
                        continue;
                    }
                    let x = center_x as i32 - 10 + col_idx as i32;
                    let y = art_y + line_idx as u16;
                    if x >= area.x as i32
                        && x < (area.x + w) as i32
                        && y >= area.y
                        && y < area.y + h
                    {
                        let cell = buf.get_mut(x as u16, y);
                        let is_colored = matches!(cell.bg, Color::Rgb(147, 51, 234) | Color::Rgb(34, 197, 94));
                        if elapsed_ms >= 1000.0 || is_colored {
                            cell.set_char(ch);
                            cell.fg = Color::Rgb(255, 255, 255);
                        }
                    }
                }
            }

            // Render "7"
            for (line_idx, line) in LOGO_7.lines().enumerate() {
                for (col_idx, ch) in line.chars().enumerate() {
                    if ch == ' ' {
                        continue;
                    }
                    let x = center_x as i32 + col_idx as i32;
                    let y = art_y + line_idx as u16;
                    if x >= area.x as i32
                        && x < (area.x + w) as i32
                        && y >= area.y
                        && y < area.y + h
                    {
                        let cell = buf.get_mut(x as u16, y);
                        let is_colored = matches!(cell.bg, Color::Rgb(147, 51, 234) | Color::Rgb(34, 197, 94));
                        if elapsed_ms >= 1000.0 || is_colored {
                            cell.set_char(ch);
                            cell.fg = Color::Rgb(255, 255, 255);
                        }
                    }
                }
            }
        }

        // ==========================================
        // 4. Draw Split RVOIDEX7_ASCII behind opening doors
        // ==========================================
        if elapsed_ms >= 2000.0 {
            let center_x_f64 = area.x as f64 + (w as f64 / 2.0);
            let center_y_f64 = area.y as f64 + (h as f64 / 2.0);

            let left_start_x = center_x as i32 - 10 - text_offset;
            let right_start_x = center_x as i32 - 64 + text_offset;

            for (line_idx, line) in RVOIDEX7_ASCII.lines().enumerate() {
                for (col_idx, ch) in line.chars().enumerate() {
                    if ch == ' ' { continue; }

                    let y = art_y + line_idx as u16;

                    // --- Draw Left Text ---
                    let x_left = left_start_x + col_idx as i32;
                    if x_left >= area.x as i32 && x_left < (area.x + w) as i32 && y >= area.y && y < area.y + h {
                        let dx = x_left as f64 - center_x_f64;
                        let y_fixed = center_y_f64 - tan_theta * dx * 0.5;
                        if (y as f64) < y_fixed { // Purple side (Top/Left)
                            let cell = buf.get_mut(x_left as u16, y);
                            cell.set_char(ch);
                            cell.fg = Color::Rgb(255, 255, 255);
                        }
                    }

                    // --- Draw Right Text ---
                    let x_right = right_start_x + col_idx as i32;
                    if x_right >= area.x as i32 && x_right < (area.x + w) as i32 && y >= area.y && y < area.y + h {
                        let dx = x_right as f64 - center_x_f64;
                        let y_fixed = center_y_f64 - tan_theta * dx * 0.5;
                        if (y as f64) >= y_fixed { // Green side (Bottom/Right)
                            let cell = buf.get_mut(x_right as u16, y);
                            cell.set_char(ch);
                            cell.fg = Color::Rgb(255, 255, 255);
                        }
                    }
                }
            }
        }
    }
}

pub fn run<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    let app = App {
        started_at: Instant::now(),
    };
    app.run(terminal)
}
