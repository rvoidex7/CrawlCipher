//! UI rendering for CrawlCipher Terminal using Ratatui.
//! Pure view layer: takes FFI data from the Native Engine and renders to terminal.

use ratatui::{
    layout::{Constraint, Layout, Rect, Direction as LayoutDirection, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Clear},
    Frame,
};
use std::collections::HashMap;

use crate::ffi::{NativeEngine, CellInfo, SimulationState, PlayerState};
use crate::stellar::profile::ProfileStats;
use crate::config::AppConfig;
use crate::background::BackgroundPattern;

// ===== Main Render Function =====

pub fn render(
    frame: &mut Frame,
    state: &SimulationState,
    player: &PlayerState,
    all_players: &HashMap<i32, PlayerState>,
    game: &NativeEngine,
    show_grid: bool,
    cam_x: f64,
    cam_y: i32,
    show_ghost: bool,
    profile_stats: &Option<ProfileStats>,
    config: &AppConfig,
    background: &BackgroundPattern,
) {
    let size = frame.size();

    // Check aspect ratio to determine layout mode
    if size.width > 120 {
        render_horizontal_layout(frame, size, state, player, all_players, game, show_grid, cam_x, cam_y, show_ghost, background);
    } else {
        render_vertical_layout(frame, size, state, player, all_players, game, show_grid, cam_x, cam_y, show_ghost, background);
    }

    // Boss Warning
    for wave in &config.expedition.boss_waves {
        let diff = wave.trigger_time_seconds - state.match_time_seconds;
        // Show warning if 10s or less before trigger, AND we haven't passed this wave yet
        if diff >= 0 && diff <= 10 && state.current_wave < wave.wave_number {
             render_boss_warning(frame, size, wave.wave_number, &wave.boss_type, diff, wave.multiplier);
             break; // Only show one warning
        }
    }

    // Render Pause Overlay if paused
    if state.simulation_state == 2 { // 2 = Paused
        render_pause_overlay(frame, size, profile_stats);
    } else if state.simulation_state == 3 { // 3 = GameOver
        let hash = game.get_replay_hash();
        render_game_over_overlay(frame, size, state, player, &hash);
    }
}

// ===== Horizontal Layout (Landscape) =====
// Left Sidebar (Ultra-Compact) + Right Game Grid

fn render_horizontal_layout(
    frame: &mut Frame,
    area: Rect,
    state: &SimulationState,
    player: &PlayerState,
    all_players: &HashMap<i32, PlayerState>,
    game: &NativeEngine,
    show_grid: bool,
    cam_x: f64,
    cam_y: i32,
    show_ghost: bool,
    background: &BackgroundPattern,
) {
    let chunks = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([
            Constraint::Length(4), // Ultra-thin: Border(1) + Content(2) + Border(1)
            Constraint::Min(0),     // Game Grid (Remaining space)
        ])
        .split(area);

    render_compact_sidebar(frame, chunks[0], state, player);
    render_game_grid(frame, chunks[1], state, player, all_players, game, show_grid, cam_x, cam_y, show_ghost, background);
}

// ===== Vertical Layout (Portrait) =====
// Top Status Bar (Full labels) + Bottom Game Grid

fn render_vertical_layout(
    frame: &mut Frame,
    area: Rect,
    state: &SimulationState,
    player: &PlayerState,
    all_players: &HashMap<i32, PlayerState>,
    game: &NativeEngine,
    show_grid: bool,
    cam_x: f64,
    cam_y: i32,
    show_ghost: bool,
    background: &BackgroundPattern,
) {
    let chunks = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length(3),  // Top Status Bar (Compact)
            Constraint::Min(0),     // Game Grid (Remaining space)
        ])
        .split(area);

    render_top_statusbar_content(frame, chunks[0], state, player);
    render_game_grid(frame, chunks[1], state, player, all_players, game, show_grid, cam_x, cam_y, show_ghost, background);
}

// ===== Component: Top Status Bar (Vertical Layout) =====

fn render_top_statusbar_content(frame: &mut Frame, area: Rect, state: &SimulationState, player: &PlayerState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Energy Bar (Horizontal, Left)
    // Style: "██ ██ ██" + "++ ++" (Bonus)
    let max_energy = player.max_energy.max(1) as usize;
    let energy_val = player.energy as usize;
    let bonus_energy = player.bonus_energy as usize;
    let energy_color = if player.energy <= 2 { Color::Red } else if player.energy <= 4 { Color::Yellow } else { Color::Green };

    let mut energy_line = Line::default();

    // Normal Energy
    for i in 0..max_energy {
        if i > 0 { energy_line.spans.push(Span::raw(" ")); }
        if i < energy_val {
            energy_line.spans.push(Span::styled("██", Style::default().fg(energy_color)));
        } else {
            energy_line.spans.push(Span::styled("░░", Style::default().fg(energy_color)));
        }
    }

    // Bonus Energy
    if bonus_energy > 0 {
        energy_line.spans.push(Span::raw("  ")); // Separator
        for i in 0..bonus_energy {
            if i > 0 { energy_line.spans.push(Span::raw(" ")); }
            energy_line.spans.push(Span::styled("++", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)));
        }
    }

    // Status Indicators
    if player.is_autopilot == 1 {
        energy_line.spans.push(Span::raw("  "));
        energy_line.spans.push(Span::styled("[AUTOPILOT]", Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)));
    } else if player.is_idle == 1 {
        energy_line.spans.push(Span::raw("  "));
        energy_line.spans.push(Span::styled("[IDLE]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
    }

    // Stats (Right side)
    let stats = format!("Score: {}  Kills: {}  Wave: {}  Time: {:02}:{:02}",
        player.score, player.kills, state.current_wave,
        state.match_time_seconds / 60, state.match_time_seconds % 60);

    // Label removed as requested ("üst panel iken 'Energy: []' yapısını kaldıralım")
    let left_text = energy_line;
    let right_text = Span::raw(stats);

    let chunks = Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    frame.render_widget(Paragraph::new(left_text).alignment(Alignment::Left), chunks[0]);
    frame.render_widget(Paragraph::new(right_text).alignment(Alignment::Right), chunks[1]);
}

// ===== Component: Ultra-Compact Side Bar (Horizontal Layout) =====

fn render_compact_sidebar(frame: &mut Frame, area: Rect, state: &SimulationState, player: &PlayerState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray)); // Unified DarkGray border color
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Layout:
    // Top section: Vertical Energy Bar
    // Middle section: Stats

    let chunks = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Length((player.max_energy + 2) as u16), // Height = MaxEnergy + padding
            Constraint::Min(0),     // Stats
        ])
        .split(inner);

    // 1. Vertical Energy Bar (Single Column)
    // Style:
    // ++ (Bonus)
    // ██
    // ██

    let max_energy = player.max_energy.max(1) as usize;
    let energy_val = player.energy as usize;
    let bonus_energy = player.bonus_energy as usize;
    let energy_color = if player.energy <= 2 { Color::Red } else if player.energy <= 4 { Color::Yellow } else { Color::Green };

    let mut energy_lines = Vec::new();
    // Header "E"
    energy_lines.push(Line::from(Span::styled("E", Style::default().fg(Color::Gray))));

    // Bonus on Top
    for _ in 0..bonus_energy {
        energy_lines.push(Line::from(Span::styled("++", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))));
    }

    // Normal from Top (High) to Bottom (Low)
    for i in (0..max_energy).rev() {
        let symbol = if i < energy_val { "██" } else { "░░" };
        energy_lines.push(Line::from(Span::styled(symbol, Style::default().fg(energy_color))));
    }

    if player.is_autopilot == 1 {
        energy_lines.push(Line::from(""));
        energy_lines.push(Line::from(Span::styled("AU", Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD))));
        energy_lines.push(Line::from(Span::styled("TO", Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD))));
    } else if player.is_idle == 1 {
        energy_lines.push(Line::from(""));
        energy_lines.push(Line::from(Span::styled("ID", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
        energy_lines.push(Line::from(Span::styled("LE", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))));
    }

    frame.render_widget(Paragraph::new(energy_lines).alignment(Alignment::Left), chunks[0]);

    // 2. Stats (Vertical Stack)
    let stats_lines = vec![
        Line::from(""),
        Line::from(Span::styled("S", Style::default().fg(Color::Gray))),
        Line::from(Span::raw(format!("{}", player.score))),
        Line::from(""),
        Line::from(Span::styled("K", Style::default().fg(Color::Gray))),
        Line::from(Span::raw(format!("{}", player.kills))),
        Line::from(""),
        Line::from(Span::styled("P", Style::default().fg(Color::Gray))),
        Line::from(Span::raw(format!("{}", state.player_count))),
    ];

    frame.render_widget(Paragraph::new(stats_lines).alignment(Alignment::Left), chunks[1]);
}

// ===== Game Grid (Maximized) =====

fn to_excel_col(n: i32) -> String {
    let mut n = n;
    let mut s = String::new();
    loop {
        let rem = (n % 26) as u8;
        s.push((b'A' + rem) as char);
        n = n / 26 - 1;
        if n < 0 { break; }
    }
    s.chars().rev().collect()
}

fn render_game_grid(
    frame: &mut Frame,
    area: Rect,
    state: &SimulationState,
    player: &PlayerState,
    all_players: &HashMap<i32, PlayerState>,
    game: &NativeEngine,
    show_grid: bool,
    cam_x: f64,
    cam_y: i32,
    show_ghost: bool,
    background: &BackgroundPattern,
) {
    let _ = player;
    
    let row_header_w = (state.grid_height.to_string().len() + 1) as u16;
    let col_header_h = 2;

    // Inner grid area
    let grid_rect = Rect {
        x: area.x + row_header_w,
        y: area.y + col_header_h,
        width: area.width.saturating_sub(row_header_w),
        height: area.height.saturating_sub(col_header_h),
    };

    if grid_rect.width < 2 || grid_rect.height < 1 { return; }

    // Sub-grid Offset Calculation
    // cam_x = 10.5 -> floor=10, offset=0.5 -> char_offset=1
    let cam_x_int = cam_x.floor() as i32;
    let char_offset = (cam_x.fract() * 2.0).round() as i32;

    // View Calculation
    // We render one extra column to fill the gap created by shifting
    let view_w = (grid_rect.width / 2) as i32 + 1;
    let view_h = grid_rect.height as i32;
    
    let (view_x, view_y) = if state.enable_walls != 0 {
        (
            (cam_x_int - view_w / 2).max(0).min(state.grid_width - view_w),
            (cam_y - view_h / 2).max(0).min(state.grid_height - view_h),
        )
    } else {
        (
            cam_x_int - view_w / 2,
            cam_y - view_h / 2,
        )
    };

    let buffer_size = (view_w * view_h) as usize;
    if buffer_size == 0 { return; }
    
    let mut cells = vec![CellInfo::default(); buffer_size];
    let cells_read = game.get_grid_cells(&mut cells, view_x, view_y, view_w, view_h);

    if cells_read <= 0 { return; }

    let buf = frame.buffer_mut();

    // RENDER HEADERS
    let header_style = Style::default().fg(Color::DarkGray);
    
    // 1. Top Column Headers (Split format)
    for cx in 0..view_w {
        let world_x = view_x + cx;
        let display_x = world_x.rem_euclid(state.grid_width);
        let col_str = to_excel_col(display_x);
        
        // Apply sub-grid char offset to header position
        let base_x = grid_rect.x as i32 + (cx as i32) * 2 - char_offset;

        // Skip/Clip logic similar to grid cells
        if base_x < grid_rect.x as i32 {
             if base_x + 1 < grid_rect.x as i32 { continue; }
        }
        let screen_x = if base_x < 0 { 0 } else { base_x as u16 };

        if screen_x + 1 >= area.x + area.width { break; }
        
        // Highlight if focused column
        let style = if display_x == player.focused_x {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            header_style
        };

        // Split logic: Last char on bottom, rest on top
        let (top, bottom) = if col_str.len() > 1 {
            let split_idx = col_str.len() - 1;
            (&col_str[..split_idx], &col_str[split_idx..])
        } else {
            ("", col_str.as_str())
        };

        // Top Line (y)
        let top_text = format!("{:^2}", top); 
        buf.set_string(screen_x, area.y, top_text, style);

        // Bottom Line (y+1)
        let bottom_text = format!("{:^2}", bottom);
        buf.set_string(screen_x, area.y + 1, bottom_text, style);
    }

    // 2. Left Row Headers (0, 1 ... 99)
    for cy in 0..view_h {
        let world_y = view_y + cy;
        let display_y = world_y.rem_euclid(state.grid_height);
        let row_str = format!("{:>width$}", display_y, width = (row_header_w as usize - 1));
        
        let screen_y = grid_rect.y + (cy as u16);
        if screen_y >= area.y + area.height { break; }
        
        // Highlight if focused row
        let style = if display_y == player.focused_y {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            header_style
        };

        buf.set_string(area.x, screen_y, row_str, style);
    }

    // RENDER GRID CELLS
    for idx in 0..cells_read as usize {
        let cx = (idx % view_w as usize) as u16;
        let cy = (idx / view_w as usize) as u16;

        let base_x = grid_rect.x as i32 + (cx as i32) * 2 - char_offset;
        let screen_y = grid_rect.y + cy;

        if base_x < grid_rect.x as i32 {
            if base_x + 1 < grid_rect.x as i32 { continue; }
        }

        let screen_x = if base_x < 0 { 0 } else { base_x as u16 };

        if screen_x + 1 >= grid_rect.x + grid_rect.width || screen_y >= grid_rect.y + grid_rect.height {
            continue;
        }

        let world_x = view_x + cx as i32;
        let world_y = view_y + cy as i32;
        let cell = &cells[idx];
        
        // --- Resolving Cell Style (Background + Entity) ---

        // 1. Determine Background (High-Res Sampling: 2 chars per grid cell)
        // We calculate world_x * 2 to sample two distinct characters from the background pattern
        let (bg_color, bg_left_char, bg_right_char) = if background.width == 0 {
             // Checkerboard fallback
             let is_even = (world_x + world_y).rem_euclid(2) == 0;
             let c = if show_grid {
                 if is_even { Color::Rgb(8, 8, 8) } else { Color::Rgb(18, 18, 18) }
             } else {
                 Color::Reset
             };
             (c, ' ', ' ')
        } else {
             // Sample 2 distinct characters horizontally to avoid stretching
             let c1 = background.get_char(world_x * 2, world_y);
             let c2 = background.get_char(world_x * 2 + 1, world_y);

             // Check for smart block in either half (simplified: if either is block, whole cell is block-colored)
             // Or strictly sample? If c1 is block, it's block color.
             // The user wants '█' to act as BG color.
             // If both are block, solid color.
             // If mixed, it's tricky. Let's assume block texture is coarse (2-wide) or we just take the first one?
             // Actually, if we want high-res text, we should just print c1 and c2.
             // UNLESS it is '█'.

             if c1 == '█' {
                 (Color::Rgb(30, 30, 30), ' ', ' ')
             } else {
                 (Color::Reset, c1, c2)
             }
        };

        // 2. Determine Entity Symbol & Color
        let (mut symbol_str, mut fg_color) = resolve_entity_style(cell, all_players);

        // 3. Occlusion / Layering Logic
        if cell.cell_type == 0 {
             // No entity: Show background chars directly
             symbol_str = format!("{}{}", bg_left_char, bg_right_char);
             // User requested text characters to match the pattern color (floor color) exactly
             fg_color = Color::Rgb(30, 30, 30);
        } else {
            // Entity exists.
            // If background was '█', preserve the BG color!
            // If background was normal char, standard behavior (entity occludes).
            // BUT: User says "food, snail... geldiğinde o arkaplan rengini bozuyor."
            // This implies that for ANY background pattern, the background COLOR/Texture should persist if possible?
            // "o gridlik o arkaplanın rengini bozuyor" -> It spoils the background color of that grid cell.

            // If the background has a specific color (from █ or checkerboard), we should use it as the BG for the entity.

            // Check if we derived a specific BG color
            // If bg_color is Reset or Black, maybe we shouldn't force it unless it's the Smart Block color.
            // Let's assume the resolved `bg_color` is authoritative.

            // symbol_str is the entity symbol (e.g. "()", "**").
            // We just print it with the derived bg_color.

            // NOTE: If the entity has its own BG preference (like Agent Body sometimes?), we might need to check.
            // But usually entities just have FG color.

            // Special Case: Text Art Backgrounds usually have Black BG.
            // If we are on top of a text-art char, we probably want to occlude it with the entity's BG?
            // Or render the entity with Black BG?

            // User: "o gridlik o arkaplanın rengini bozuyor"
            // This strongly suggests they are using the '█' blocks to paint a "floor", and when an item spawns, it effectively "erases" the floor color.
            // So, we MUST preserve `bg_color`.
        }

        // Final Render String
        // Ensure 2-char width
        // If symbol_str is 1 char (e.g. from bg), padding added above.
        // If symbol_str is 2 chars (e.g. "()"), use as is.
        let final_sym = format!("{:2.2}", symbol_str); // Truncate/Pad to 2

        let style = Style::default().fg(fg_color).bg(bg_color);
        buf.set_string(screen_x, screen_y, final_sym, style);
    }

    // Render Portal Overlay
    if state.portal_state > 0 {
        let px = state.portal_x;
        let py = state.portal_y;

        let lx = if state.enable_walls != 0 { px - view_x } else { (px - view_x).rem_euclid(state.grid_width) };
        let ly = if state.enable_walls != 0 { py - view_y } else { (py - view_y).rem_euclid(state.grid_height) };

        if lx >= 0 && lx < view_w && ly >= 0 && ly < view_h {
            let base_x = grid_rect.x as i32 + (lx as i32) * 2 - char_offset;
            let screen_y = grid_rect.y + ly as u16;

            if base_x >= grid_rect.x as i32 && (base_x as u16 + 1) < grid_rect.x + grid_rect.width && screen_y < grid_rect.y + grid_rect.height {
                let symbol = if state.portal_state == 2 { "◉◉" } else { "()" };
                let color = if state.portal_state == 2 { Color::Green } else { Color::Blue };
                buf.set_string(base_x as u16, screen_y, symbol, Style::default().fg(color).add_modifier(Modifier::BOLD));

                if state.extraction_countdown >= 0 {
                     buf.set_string(base_x as u16, screen_y.saturating_sub(1), format!("{}", state.extraction_countdown / 10), Style::default().fg(Color::White));
                }
            }
        }
    }

    // Render Valid Move Ghosts (-)
    if show_ghost {
        let ghost_style = Style::default().fg(Color::DarkGray);
        let head_x = player.x;
        let head_y = player.y;
        let current_dir = player.current_direction;

        let deltas = [
            (0, -1), (1, -1), (1, 0), (1, 1),
            (0, 1), (-1, 1), (-1, 0), (-1, -1)
        ];

        for d in 0..8 {
            if (player.valid_moves_mask >> d) & 1 == 0 { continue; }
            if d == current_dir { continue; }

            let (dx, dy) = deltas[d as usize];
            let tx = head_x + dx * 2;
            let ty = head_y + dy * 2;

            let lx = if state.enable_walls != 0 { tx - view_x } else { (tx - view_x).rem_euclid(state.grid_width) };
            let ly = if state.enable_walls != 0 { ty - view_y } else { (ty - view_y).rem_euclid(state.grid_height) };

            if lx >= 0 && lx < view_w && ly >= 0 && ly < view_h {
                let base_x = grid_rect.x as i32 + (lx as i32) * 2 - char_offset;
                let screen_y = grid_rect.y + ly as u16;

                if base_x >= grid_rect.x as i32 && (base_x as u16 + 1) < grid_rect.x + grid_rect.width && screen_y < grid_rect.y + grid_rect.height {
                    let idx = (ly * view_w + lx) as usize;
                    if idx < cells.len() && cells[idx].cell_type == 0 {
                        let sym = match d {
                            1 | 2 | 3 => "- ",
                            _ => " -",
                        };
                        buf.set_string(base_x as u16, screen_y, sym, ghost_style);
                    }
                }
            }
        }
    }
}

fn resolve_entity_style(cell: &CellInfo, all_players: &HashMap<i32, PlayerState>) -> (String, Color) {
    match cell.cell_type {
            0 => ("".to_string(), Color::Reset), // Handled by BG logic
            4 => ("██".to_string(), Color::DarkGray), // Wall
            5 => ("()".to_string(), Color::Red), // Food
            6 => ("**".to_string(), Color::White), // Obstacle/Other
            1 | 2 | 3 | 7 => {
                let p_color = if let Some(p) = all_players.get(&cell.player_id) {
                    Color::Rgb(p.color_r, p.color_g, p.color_b)
                } else { Color::Magenta };

                let (r, g, b) = if let Color::Rgb(cr, cg, cb) = p_color { (cr, cg, cb) } else { (255, 0, 255) };
                let is_powered = cell.extra_data == 1;

                let final_color = if is_powered {
                     p_color
                } else {
                     Color::Rgb(r.saturating_sub(100), g.saturating_sub(100), b.saturating_sub(100))
                };

                let sym = if is_powered { "██" } else { "▒▒" };
                let render_color = if cell.cell_type == 7 { Color::Yellow } else { final_color };
                (sym.to_string(), render_color)
            },
            8 => {
                 let dir = cell.extra_data;
                 let sym = match dir {
                     0 | 4 => "||", 2 | 6 => "==", 1 | 5 => "//", 3 | 7 => "\\\\", _ => "??",
                 };
                 (sym.to_string(), Color::Yellow)
            },
            9 => ("▒▒".to_string(), Color::White), // Ghost Segment
            10 => {
                let p_color = if let Some(p) = all_players.get(&cell.player_id) {
                    Color::Rgb(p.color_r, p.color_g, p.color_b)
                } else { Color::Cyan };

                let dist = cell.extra_data;
                if dist > 0 {
                    (format!("{:>2}", dist.min(99)), p_color)
                } else {
                    ("}{".to_string(), p_color)
                }
            },
            12 => {
                let p_color = if let Some(p) = all_players.get(&cell.player_id) {
                    Color::Rgb(p.color_r, p.color_g, p.color_b)
                } else { Color::Cyan };
                (" +".to_string(), p_color)
            },
            11 => { // Snail
                let dir = cell.extra_data;
                let sym = match dir {
                    0 => "¡@", 1 => "@/", 2 => "@-", 3 => "@\\",
                    4 => "@!", 5 => "/@", 6 => "-@", 7 => "\\@",
                    _ => "@@",
                };
                (sym.to_string(), Color::LightGreen)
            },
            _ => ("  ".to_string(), Color::Reset),
        }
}

// ===== Pause Overlay =====

fn render_pause_overlay(frame: &mut Frame, area: Rect, profile_stats: &Option<ProfileStats>) {
    let block = Block::default().borders(Borders::ALL).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    let area = centered_rect(60, 60, area); // Slightly taller for stats
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let inner = area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 2 });

    let mut lines = vec![
        Line::from(Span::styled("PAUSED", Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow))),
        Line::from(""),
        Line::from("Controls:"),
        Line::from("  WASD / Arrows : Move"),
        Line::from("  SPACE         : Fire"),
        Line::from("  F             : Strike (Lunge)"),
        Line::from("  P             : Toggle Autopilot"),
        Line::from("  A / Z         : Focus Head/Tail"),
        Line::from("  I             : Inventory"),
        Line::from("  X / C         : Attach Weapon L/R"),
        Line::from("  ESC           : Resume"),
        Line::from("  M             : Return to Main Menu"),
        Line::from("  Ctrl+Q        : Quit Simulation"),
        Line::from(""),
    ];

    if let Some(stats) = profile_stats {
        lines.push(Line::from(Span::styled("PLAYER PROFILE (Stellar)", Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))));
        lines.push(Line::from(format!("  Lifetime Kills: {}", stats.total_kills)));
        lines.push(Line::from(format!("  Matches Played: {}", stats.matches_played)));
        lines.push(Line::from(format!("  Max Length:     {}", stats.max_length)));
        lines.push(Line::from(format!("  Rank Points:    {}", stats.rank_points)));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled("Press ESC to Resume", Style::default().add_modifier(Modifier::ITALIC))));

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(paragraph, inner);
}

fn render_boss_warning(frame: &mut Frame, area: Rect, wave_num: i32, boss_type: &str, countdown: i32, multiplier: f32) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red))
        .style(Style::default().bg(Color::Black)); // Transparent-ish background? No, black to pop

    let area = centered_rect(50, 20, area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let inner = area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 2 });

    let lines = vec![
        Line::from(Span::styled(format!("⚠️  BOSS WAVE #{}  ⚠️", wave_num), Style::default().fg(Color::Red).add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK))),
        Line::from(""),
        Line::from(Span::styled(format!("\"{}\" APPROACHING", boss_type.to_uppercase().replace("_", " ")), Style::default().fg(Color::Yellow))),
        Line::from(""),
        Line::from(format!("Spawning in {}...", countdown)),
        Line::from(""),
        Line::from(Span::styled(format!("Multiplier increasing to {:.1}x", multiplier), Style::default().fg(Color::Cyan))),
    ];

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(paragraph, inner);
}

fn render_game_over_overlay(frame: &mut Frame, area: Rect, state: &SimulationState, player: &PlayerState, replay_hash: &str) {
    let bg_color = if state.extraction_countdown == 0 { Color::Green } else { Color::Red };
    let block = Block::default().borders(Borders::ALL).style(Style::default().bg(bg_color).fg(Color::White));

    let area = centered_rect(60, 60, area); // Increased height for hash
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let inner = area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 2 });

    let title = if state.extraction_countdown == 0 { "EXTRACTION SUCCESS!" } else { "GAME OVER" };

    let lines = vec![
        Line::from(Span::styled(title, Style::default().add_modifier(Modifier::BOLD).fg(Color::White))),
        Line::from(""),
        Line::from(format!("Final Score: {}", player.score)),
        Line::from(format!("Wave Reached: {}", state.current_wave)),
        Line::from(format!("Survival Time: {}:{:02}", state.match_time_seconds / 60, state.match_time_seconds % 60)),
        Line::from(""),
        Line::from(Span::styled("Proof of Gameplay (Hash):", Style::default().add_modifier(Modifier::UNDERLINED))),
        Line::from(Span::styled(format!("{:.16}...", replay_hash), Style::default().fg(Color::Yellow))), // Truncate for UI
        Line::from(""),
        Line::from("Press R to Restart"),
        Line::from("Press M to Main Menu"),
        Line::from("Press Ctrl+Q to Quit"),
    ];

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(paragraph, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(LayoutDirection::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(LayoutDirection::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
