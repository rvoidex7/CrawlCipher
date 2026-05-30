use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::time::Instant;

use crate::background;

// ===== Menu Action Enum =====

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuAction {
    InitiateProtocol,  // 0
    EnterCredentials,  // 1
    GhostProtocol,     // 2
    AcquireKey,        // 3
    TerminalManual,    // 4
    SystemSettings,    // 5
    AbortMission,      // 6
}

// ===== Menu Item =====

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub label: String,
    pub x: f64,       // Grid position (percentage of grid width, 0.0-1.0)
    pub y: f64,       // Grid position (percentage of grid height, 0.0-1.0)
    pub action: MenuAction,
    pub is_focused: bool,
}

// ===== Menu Snake =====

#[derive(Clone, Debug)]
pub struct MenuSnake {
    pub body: Vec<(f64, f64)>,     // Body segments (grid coords, float for smooth movement)
    pub direction: i32,             // 0-7 (N, NE, E, SE, S, SW, W, NW)
    pub move_timer: f64,            // Accumulator for movement ticks
    pub move_interval: f64,         // Seconds between movement steps
    pub is_dashing: bool,
    pub dash_target: (f64, f64),
    pub dash_progress: f64,
    pub dash_start: (f64, f64),
    pub dash_action: Option<MenuAction>,
    pub trail: Vec<(f64, f64, f64)>, // (x, y, opacity) for dash trail effect
    pub idle_time: f64,              // Time since last input
    pub tail_wave_phase: f64,        // For idle tail animation
    pub approach_target: Option<(f64, f64)>, // Target to approach (focused item position)
    pub is_approaching: bool,        // True when auto-navigating toward target
    pub user_steering: bool,         // True when user is actively pressing direction keys
    pub user_steer_cooldown: f64,    // Seconds remaining of user-steering override
}

impl MenuSnake {
    pub fn new(start_x: f64, start_y: f64) -> Self {
        let mut body = Vec::new();
        // 6-segment snake, initially horizontal pointing east
        for i in 0..6 {
            body.push((start_x - i as f64, start_y));
        }
        Self {
            body,
            direction: 2, // East
            move_timer: 0.0,
            move_interval: 0.12,
            is_dashing: false,
            dash_target: (0.0, 0.0),
            dash_progress: 0.0,
            dash_start: (0.0, 0.0),
            dash_action: None,
            trail: Vec::new(),
            idle_time: 0.0,
            tail_wave_phase: 0.0,
            approach_target: None,
            is_approaching: false,
            user_steering: false,
            user_steer_cooldown: 0.0,
        }
    }

    pub fn head(&self) -> (f64, f64) {
        self.body[0]
    }

    pub fn set_direction(&mut self, dir: i32) {
        if dir >= 0 && dir < 8 {
            self.direction = dir;
            self.idle_time = 0.0;
            self.user_steering = true;
            self.user_steer_cooldown = 0.8; // User has priority for 0.8s
        }
    }

    fn direction_delta(dir: i32) -> (f64, f64) {
        match dir {
            0 => (0.0, -1.0),  // N
            1 => (1.0, -1.0),  // NE
            2 => (1.0, 0.0),   // E
            3 => (1.0, 1.0),   // SE
            4 => (0.0, 1.0),   // S
            5 => (-1.0, 1.0),  // SW
            6 => (-1.0, 0.0),  // W
            7 => (-1.0, -1.0), // NW
            _ => (0.0, 0.0),
        }
    }

    pub fn tick(&mut self, dt: f64, grid_w: f64, grid_h: f64) {
        self.tail_wave_phase += dt * 3.0;
        self.idle_time += dt;

        // Update trail fade
        self.trail.retain_mut(|t| {
            t.2 -= dt * 4.0;
            t.2 > 0.0
        });

        if self.is_dashing {
            self.dash_progress += dt * 6.0; // Fast dash

            if self.dash_progress >= 1.0 {
                self.dash_progress = 1.0;
                self.is_dashing = false;
            }

            // Lerp head to target
            let old_head = self.body[0];
            let new_x = self.dash_start.0 + (self.dash_target.0 - self.dash_start.0) * self.dash_progress;
            let new_y = self.dash_start.1 + (self.dash_target.1 - self.dash_start.1) * self.dash_progress;

            // Add trail
            self.trail.push((old_head.0, old_head.1, 1.0));

            self.body[0] = (new_x, new_y);

            // Pull body segments along
            for i in 1..self.body.len() {
                let prev = self.body[i - 1];
                let curr = self.body[i];
                let dx = prev.0 - curr.0;
                let dy = prev.1 - curr.1;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > 1.2 {
                    let ratio = 1.0 / dist;
                    self.body[i] = (prev.0 - dx * ratio, prev.1 - dy * ratio);
                }
            }

            return;
        }

        // User steering cooldown
        if self.user_steer_cooldown > 0.0 {
            self.user_steer_cooldown -= dt;
            if self.user_steer_cooldown <= 0.0 {
                self.user_steering = false;
                self.user_steer_cooldown = 0.0;
            }
        }

        // Auto-approach: steer toward approach_target if not user-steering
        if !self.user_steering {
            if let Some(target) = self.approach_target {
                let head = self.body[0];
                let dx = target.0 - head.0;
                let dy = target.1 - head.1;
                let dist = (dx * dx + dy * dy).sqrt();

                // Stop distance: close enough but not on top of the item
                let stop_distance = 5.0;

                if dist > stop_distance {
                    self.is_approaching = true;
                    // Calculate best direction toward target
                    let angle = dy.atan2(dx);
                    let best_dir = Self::angle_to_direction(angle);

                    // Prevent 180-degree reversal during approach too
                    let diff = (best_dir - self.direction + 8) % 8;
                    if diff != 4 {
                        self.direction = best_dir;
                    }
                } else {
                    self.is_approaching = false;
                    // Near item: just idle, stay still by not moving
                    // But still update wave phase for visual life
                    return;
                }
            } else {
                self.is_approaching = false;
            }
        }

        // Normal movement
        self.move_timer += dt;
        if self.move_timer >= self.move_interval {
            self.move_timer -= self.move_interval;

            let (dx, dy) = Self::direction_delta(self.direction);
            let new_head_x = self.body[0].0 + dx;
            let new_head_y = self.body[0].1 + dy;

            // Wrap around grid boundaries
            let wrapped_x = ((new_head_x % grid_w) + grid_w) % grid_w;
            let wrapped_y = ((new_head_y % grid_h) + grid_h) % grid_h;

            // Shift body (move tail to head)
            let last = self.body.len() - 1;
            for i in (1..=last).rev() {
                self.body[i] = self.body[i - 1];
            }
            self.body[0] = (wrapped_x, wrapped_y);
        }
    }

    /// Convert a math angle (radians) to the nearest 8-direction index
    fn angle_to_direction(angle: f64) -> i32 {
        // angle: atan2(dy, dx)
        // Map to 0..2PI
        let a = (angle + 2.0 * std::f64::consts::PI) % (2.0 * std::f64::consts::PI);
        // Each direction spans PI/4 (45 degrees)
        // Direction 2 (East) = 0 rad, so offset
        // dir 0=N(-PI/2), 1=NE(-PI/4), 2=E(0), 3=SE(PI/4), 4=S(PI/2), 5=SW(3PI/4), 6=W(PI), 7=NW(-3PI/4)
        // Mapping: direction = round(angle / (PI/4)) mapped to our convention
        // Our dir 0 = N = angle -PI/2 = 3PI/2 in [0,2PI]
        // Use: sector = (a + PI/8) / (PI/4) → 0=E,1=SE,2=S,3=SW,4=W,5=NW,6=N,7=NE
        let sector = ((a + std::f64::consts::FRAC_PI_8) / std::f64::consts::FRAC_PI_4).floor() as i32 % 8;
        // Map sector to our direction system
        match sector {
            0 => 2, // E
            1 => 3, // SE
            2 => 4, // S
            3 => 5, // SW
            4 => 6, // W
            5 => 7, // NW
            6 => 0, // N
            7 => 1, // NE
            _ => 2,
        }
    }

    pub fn start_dash(&mut self, target_x: f64, target_y: f64, action: MenuAction) {
        if self.is_dashing { return; }
        self.is_dashing = true;
        self.dash_start = self.body[0];
        self.dash_target = (target_x, target_y);
        self.dash_progress = 0.0;
        self.dash_action = Some(action);
        self.idle_time = 0.0;
    }

    pub fn dash_completed(&self) -> Option<MenuAction> {
        if !self.is_dashing && self.dash_action.is_some() {
            self.dash_action
        } else {
            None
        }
    }

    pub fn clear_dash_action(&mut self) {
        self.dash_action = None;
    }
}

// ===== Menu State =====

pub enum MenuState {
    MainMenu,
    CredentialsInput,
    KeyInfo,
    Settings,
    CustomBackgroundInput,
    MissionSelect,
    Manual,
}

// ===== Menu UI =====

pub struct MenuUI {
    pub secret_key: String,
    pub nickname: String,
    pub state: MenuState,
    pub cred_stage: usize,
    pub settings_selection: usize,
    pub embedded_bgs: Vec<String>,
    pub selected_bg_index: usize,
    pub error_msg: Option<String>,
    pub custom_bg_path: String,
    pub custom_bg_loaded: bool,
    pub mission_selection: usize,

    // New: Snake menu system
    pub snake: MenuSnake,
    pub menu_items: Vec<MenuItem>,
    pub focused_index: Option<usize>,
    pub last_tick: Instant,
    pub grid_width: f64,
    pub grid_height: f64,

    // Layout info for mouse coordinate mapping (set during render)
    pub layout_game_area_x: u16,
    pub layout_game_area_y: u16,
    pub layout_game_area_w: u16,
    pub layout_game_area_h: u16,
    pub layout_view_x: i32,
    pub layout_view_y: i32,

    // Mouse focus override: when true, update_focus() won't recalculate from snake direction
    pub mouse_focus_active: bool,
}

impl MenuUI {
    pub fn new() -> Self {
        let grid_w = 80.0;
        let grid_h = 40.0;

        // Place snake in center of the grid
        let snake = MenuSnake::new(grid_w / 2.0, grid_h / 2.0);

        // Menu items positioned across the grid
        let menu_items = vec![
            MenuItem {
                label: "[ INITIATE PROTOCOL ]".to_string(),
                x: 0.50, y: 0.22,
                action: MenuAction::InitiateProtocol,
                is_focused: false,
            },
            MenuItem {
                label: "[ ENTER CREDENTIALS ]".to_string(),
                x: 0.22, y: 0.40,
                action: MenuAction::EnterCredentials,
                is_focused: false,
            },
            MenuItem {
                label: "[ GHOST PROTOCOL ]".to_string(),
                x: 0.75, y: 0.40,
                action: MenuAction::GhostProtocol,
                is_focused: false,
            },
            MenuItem {
                label: "[ ACQUIRE ACCESS KEY ]".to_string(),
                x: 0.50, y: 0.55,
                action: MenuAction::AcquireKey,
                is_focused: false,
            },
            MenuItem {
                label: "[ TERMINAL MANUAL ]".to_string(),
                x: 0.22, y: 0.70,
                action: MenuAction::TerminalManual,
                is_focused: false,
            },
            MenuItem {
                label: "[ SYSTEM SETTINGS ]".to_string(),
                x: 0.75, y: 0.70,
                action: MenuAction::SystemSettings,
                is_focused: false,
            },
            MenuItem {
                label: "[ ABORT MISSION ]".to_string(),
                x: 0.50, y: 0.85,
                action: MenuAction::AbortMission,
                is_focused: false,
            },
        ];

        Self {
            secret_key: String::new(),
            nickname: "Pilot".to_string(),
            state: MenuState::MainMenu,
            cred_stage: 0,
            settings_selection: 0,
            embedded_bgs: background::list_embedded_backgrounds(),
            selected_bg_index: 0,
            error_msg: None,
            custom_bg_path: String::new(),
            custom_bg_loaded: false,
            mission_selection: 0,

            snake,
            menu_items,
            focused_index: None,
            last_tick: Instant::now(),
            grid_width: grid_w,
            grid_height: grid_h,

            layout_game_area_x: 0,
            layout_game_area_y: 0,
            layout_game_area_w: 0,
            layout_game_area_h: 0,
            layout_view_x: 0,
            layout_view_y: 0,

            mouse_focus_active: false,
        }
    }

    /// Update the snake simulation and focus system. Call every frame.
    pub fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        // Cap dt to avoid huge jumps
        let dt = dt.min(0.1);

        self.snake.tick(dt, self.grid_width, self.grid_height);

        // Update focus based on snake direction
        self.update_focus();
    }

    fn update_focus(&mut self) {
        if self.snake.is_dashing {
            return; // Don't change focus during dash
        }

        // If mouse set the focus, don't override it with snake direction
        // The snake will approach the mouse-selected item instead
        if self.mouse_focus_active {
            // Still update approach target to ensure snake heads there
            if let Some(idx) = self.focused_index {
                let item = &self.menu_items[idx];
                self.snake.approach_target = Some((item.x * self.grid_width, item.y * self.grid_height));
            }
            return;
        }

        let head = self.snake.head();

        let (sdx, sdy) = MenuSnake::direction_delta(self.snake.direction);
        let snake_angle = sdy.atan2(sdx);

        let mut best_index: Option<usize> = None;
        let mut best_score = f64::MAX;

        for (i, item) in self.menu_items.iter().enumerate() {
            let item_world_x = item.x * self.grid_width;
            let item_world_y = item.y * self.grid_height;

            let dx = item_world_x - head.0;
            let dy = item_world_y - head.1;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < 1.0 { continue; } // Too close, skip

            let angle_to_item = dy.atan2(dx);

            // Angle difference (normalized to -PI..PI)
            let mut angle_diff = angle_to_item - snake_angle;
            while angle_diff > std::f64::consts::PI { angle_diff -= 2.0 * std::f64::consts::PI; }
            while angle_diff < -std::f64::consts::PI { angle_diff += 2.0 * std::f64::consts::PI; }

            let abs_angle_diff = angle_diff.abs();

            // Score: weighted combination of angle difference and distance
            // Items directly in front will have a very small angle difference.
            // Items behind will have angle difference ~PI (which penalizes them heavily).
            let score = abs_angle_diff * 3.0 + dist * 0.05;

            if score < best_score {
                best_score = score;
                best_index = Some(i);
            }
        }

        // Update focus
        for (i, item) in self.menu_items.iter_mut().enumerate() {
            item.is_focused = best_index == Some(i);
        }
        self.focused_index = best_index;

        // Set approach target for the focused item
        if let Some(idx) = self.focused_index {
            let item = &self.menu_items[idx];
            self.snake.approach_target = Some((item.x * self.grid_width, item.y * self.grid_height));
        } else {
            self.snake.approach_target = None;
        }
    }

    /// Focus an item by mouse screen position (terminal coordinates).
    /// Returns true if an item was focused.
    pub fn focus_by_screen_pos(&mut self, screen_x: u16, screen_y: u16) -> bool {
        // Convert screen coords to grid coords using stored layout info
        let ga_x = self.layout_game_area_x;
        let ga_y = self.layout_game_area_y;
        let ga_w = self.layout_game_area_w;
        let ga_h = self.layout_game_area_h;

        if screen_x < ga_x || screen_y < ga_y { return false; }
        if screen_x >= ga_x + ga_w || screen_y >= ga_y + ga_h { return false; }

        // Each grid cell = 2 terminal chars wide, 1 char tall
        let grid_x = self.layout_view_x + ((screen_x - ga_x) / 2) as i32;
        let grid_y = self.layout_view_y + (screen_y - ga_y) as i32;

        // Find closest menu item to this grid position
        let mut best_index: Option<usize> = None;
        let mut best_dist = f64::MAX;

        for (i, item) in self.menu_items.iter().enumerate() {
            let item_world_x = item.x * self.grid_width;
            let item_world_y = item.y * self.grid_height;

            // Item label occupies a horizontal range
            let label_half_w = item.label.len() as f64 / 4.0; // In grid cells
            let dx = (grid_x as f64 - item_world_x).abs() - label_half_w;
            let dx = dx.max(0.0); // 0 if inside label width
            let dy = (grid_y as f64 - item_world_y).abs();

            let dist = (dx * dx + dy * dy).sqrt();

            // Only match if reasonably close (within ~3 grid cells)
            if dist < 3.0 && dist < best_dist {
                best_dist = dist;
                best_index = Some(i);
            }
        }

        if let Some(idx) = best_index {
            // Update focus
            for (i, item) in self.menu_items.iter_mut().enumerate() {
                item.is_focused = i == idx;
            }
            self.focused_index = Some(idx);

            // Also steer snake toward this item
            let item = &self.menu_items[idx];
            self.snake.approach_target = Some((item.x * self.grid_width, item.y * self.grid_height));
            self.snake.user_steering = false;
            self.snake.user_steer_cooldown = 0.0;

            // Lock focus to mouse selection
            self.mouse_focus_active = true;

            true
        } else {
            false
        }
    }

    /// Trigger dash toward the focused menu item
    pub fn trigger_dash(&mut self) -> bool {
        if self.snake.is_dashing { return false; }

        if let Some(idx) = self.focused_index {
            let item = &self.menu_items[idx];
            let target_x = item.x * self.grid_width;
            let target_y = item.y * self.grid_height;
            let action = item.action;
            self.snake.start_dash(target_x, target_y, action);
            true
        } else {
            false
        }
    }

    /// Check if a dash action was completed and return it
    pub fn poll_dash_action(&mut self) -> Option<MenuAction> {
        let action = self.snake.dash_completed();
        if action.is_some() {
            self.snake.clear_dash_action();
        }
        action
    }

    /// Update layout info for mouse coordinate mapping. Call with terminal size each frame.
    pub fn update_layout(&mut self, area_width: u16, area_height: u16) {
        let title_h: u16 = 4;
        let status_h: u16 = 2;

        let ga_x: u16 = 0;
        let ga_y: u16 = title_h;
        let ga_w: u16 = area_width;
        let ga_h: u16 = area_height.saturating_sub(title_h + status_h);

        let grid_w = self.grid_width as i32;
        let grid_h = self.grid_height as i32;
        let view_w = (ga_w / 2) as i32;
        let view_h = ga_h as i32;

        let view_x = (grid_w / 2 - view_w / 2).max(0);
        let view_y = (grid_h / 2 - view_h / 2).max(0);

        self.layout_game_area_x = ga_x;
        self.layout_game_area_y = ga_y;
        self.layout_game_area_w = ga_w;
        self.layout_game_area_h = ga_h;
        self.layout_view_x = view_x;
        self.layout_view_y = view_y;
    }

    /// Reset snake position (after returning from submenu)
    pub fn reset_snake(&mut self) {
        self.snake = MenuSnake::new(self.grid_width / 2.0, self.grid_height / 2.0);
        self.last_tick = Instant::now();
        self.mouse_focus_active = false;
    }
}

// ===== Rendering =====

pub fn render_menu(frame: &mut Frame, area: Rect, ui: &MenuUI) {
    match ui.state {
        MenuState::MainMenu => render_snake_menu(frame, area, ui),
        MenuState::CredentialsInput => {
            render_classic_bg(frame, area);
            render_credentials_input(frame, area, ui);
        }
        MenuState::KeyInfo => {
            render_classic_bg(frame, area);
            render_key_info(frame, area);
        }
        MenuState::Settings => {
            render_classic_bg(frame, area);
            render_settings(frame, area, ui);
        }
        MenuState::CustomBackgroundInput => {
            render_classic_bg(frame, area);
            render_custom_bg_input(frame, area, ui);
        }
        MenuState::MissionSelect => {
            render_classic_bg(frame, area);
            render_mission_select(frame, area, ui);
        }
        MenuState::Manual => {
            render_classic_bg(frame, area);
            render_manual(frame, area);
        }
    }
}

fn render_classic_bg(frame: &mut Frame, area: Rect) {
    let bg_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::DarkGray));
    frame.render_widget(bg_block, area);
}

// ===== Snake Menu Rendering =====

fn render_snake_menu(frame: &mut Frame, area: Rect, ui: &MenuUI) {
    let buf = frame.buffer_mut();

    let grid_w = ui.grid_width as i32;
    let grid_h = ui.grid_height as i32;

    // Map grid to terminal area (2 chars per grid cell horizontally, 1 char vertically)
    // Leave space for title (3 lines top) and status (2 lines bottom)
    let title_h: u16 = 4;
    let status_h: u16 = 2;

    let game_area = Rect {
        x: area.x,
        y: area.y + title_h,
        width: area.width,
        height: area.height.saturating_sub(title_h + status_h),
    };

    // Calculate visible grid range based on terminal size
    let view_w = (game_area.width / 2) as i32;
    let view_h = game_area.height as i32;

    // Center camera on grid center (or snake head for immersion)
    let _cam_x = (grid_w / 2).min(view_w / 2);
    let _cam_y = (grid_h / 2).min(view_h / 2);

    let view_x = (grid_w / 2 - view_w / 2).max(0);
    let view_y = (grid_h / 2 - view_h / 2).max(0);

    // 1. Render checkerboard background (no headers)
    for cy in 0..view_h.min(grid_h) {
        for cx in 0..view_w.min(grid_w) {
            let world_x = view_x + cx;
            let world_y = view_y + cy;

            let screen_x = game_area.x + (cx as u16) * 2;
            let screen_y = game_area.y + cy as u16;

            if screen_x + 1 >= area.x + area.width || screen_y >= game_area.y + game_area.height {
                continue;
            }

            let is_even = (world_x + world_y) % 2 == 0;
            let bg_color = if is_even {
                Color::Rgb(6, 6, 12)
            } else {
                Color::Rgb(12, 12, 20)
            };

            let style = Style::default().bg(bg_color).fg(Color::Rgb(20, 20, 35));
            buf.set_string(screen_x, screen_y, "  ", style);
        }
    }

    // 2. Render menu items as text on grid
    for item in &ui.menu_items {
        let item_world_x = (item.x * grid_w as f64) as i32;
        let item_world_y = (item.y * grid_h as f64) as i32;

        // Convert to screen coordinates
        let local_x = item_world_x - view_x;
        let local_y = item_world_y - view_y;

        if local_y < 0 || local_y >= view_h { continue; }

        // Center the label on the item position
        let label_char_len = item.label.len() as i32;
        let label_start_grid_x = local_x - label_char_len as i32 / 4; // Approximate centering (2 chars per grid cell)

        let screen_y = game_area.y + local_y as u16;
        // Screen x: each grid cell = 2 terminal chars, but label is char-by-char
        let screen_x_start = game_area.x as i32 + label_start_grid_x * 2;

        if screen_y >= game_area.y + game_area.height { continue; }

        let (fg_color, bg_color, mods) = if item.is_focused {
            (Color::Black, Color::Cyan, Modifier::BOLD)
        } else {
            (Color::Cyan, Color::Rgb(6, 6, 18), Modifier::empty())
        };

        let style = Style::default().fg(fg_color).bg(bg_color).add_modifier(mods);

        // Draw the label character by character
        for (i, ch) in item.label.chars().enumerate() {
            let sx = screen_x_start + i as i32;
            if sx >= area.x as i32 && sx < (area.x + area.width) as i32 {
                buf.set_string(sx as u16, screen_y, &ch.to_string(), style);
            }
        }

        // Draw focus indicator arrow if focused
        if item.is_focused {
            let arrow = "►";
            let arrow_x = screen_x_start - 2;
            if arrow_x >= area.x as i32 && arrow_x < (area.x + area.width - 1) as i32 {
                buf.set_string(
                    arrow_x as u16,
                    screen_y,
                    arrow,
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                );
            }
        }
    }

    // 3. Render dash trail
    for trail_point in &ui.snake.trail {
        let local_x = trail_point.0 as i32 - view_x;
        let local_y = trail_point.1 as i32 - view_y;

        if local_x < 0 || local_x >= view_w || local_y < 0 || local_y >= view_h { continue; }

        let screen_x = game_area.x + (local_x as u16) * 2;
        let screen_y = game_area.y + local_y as u16;

        if screen_x + 1 >= area.x + area.width || screen_y >= game_area.y + game_area.height {
            continue;
        }

        let intensity = (trail_point.2 * 255.0).clamp(0.0, 255.0) as u8;
        let trail_color = Color::Rgb(0, intensity / 2, intensity);
        buf.set_string(screen_x, screen_y, "░░", Style::default().fg(trail_color));
    }

    // 4. Render snake body
    let snake_head_color = Color::Rgb(0, 255, 220);

    for (i, seg) in ui.snake.body.iter().enumerate() {
        let local_x = seg.0.round() as i32 - view_x;
        let local_y = seg.1.round() as i32 - view_y;

        if local_x < 0 || local_x >= view_w || local_y < 0 || local_y >= view_h { continue; }

        let screen_x = game_area.x + (local_x as u16) * 2;
        let screen_y = game_area.y + local_y as u16;

        if screen_x + 1 >= area.x + area.width || screen_y >= game_area.y + game_area.height {
            continue;
        }

        let (symbol, color) = if i == 0 {
            // Head - direction-dependent symbol
            let head_sym = match ui.snake.direction {
                0 => "▲▲", // N
                1 => "▶▲", // NE
                2 => "▶▶", // E
                3 => "▶▼", // SE
                4 => "▼▼", // S
                5 => "◀▼", // SW
                6 => "◀◀", // W
                7 => "◀▲", // NW
                _ => "██",
            };

            if ui.snake.is_dashing {
                (head_sym, Color::Rgb(255, 255, 100)) // Bright yellow during dash
            } else {
                (head_sym, snake_head_color)
            }
        } else {
            // Body - gradient fade
            let fade = 1.0 - (i as f64 / ui.snake.body.len() as f64) * 0.6;
            let r = (0.0 * fade) as u8;
            let g = (200.0 * fade) as u8;
            let b = (160.0 * fade) as u8;
            ("██", Color::Rgb(r, g, b))
        };

        buf.set_string(screen_x, screen_y, symbol, Style::default().fg(color));
    }

    // 5. Render gaze line (dotted line from head in direction)
    if !ui.snake.is_dashing {
        let (dx, dy) = MenuSnake::direction_delta(ui.snake.direction);
        let head = ui.snake.head();
        for step in 1..=3 {
            let gaze_x = head.0 + dx * step as f64;
            let gaze_y = head.1 + dy * step as f64;
            let local_x = gaze_x.round() as i32 - view_x;
            let local_y = gaze_y.round() as i32 - view_y;

            if local_x < 0 || local_x >= view_w || local_y < 0 || local_y >= view_h { continue; }

            let screen_x = game_area.x + (local_x as u16) * 2;
            let screen_y = game_area.y + local_y as u16;

            if screen_x + 1 >= area.x + area.width || screen_y >= game_area.y + game_area.height {
                continue;
            }

            let fade = 1.0 - (step as f64 / 4.0);
            let intensity = (fade * 80.0) as u8;
            let gaze_color = if ui.focused_index.is_some() {
                Color::Rgb(intensity, intensity + 40, intensity + 60) // Cyan-ish when targeting
            } else {
                Color::Rgb(intensity / 2, intensity / 2, intensity / 2) // Gray when no target
            };
            buf.set_string(screen_x, screen_y, "··", Style::default().fg(gaze_color));
        }
    }

    // 6. Render title at top
    render_snake_menu_title(frame, Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: title_h,
    });

    // 7. Render status bar at bottom
    render_snake_menu_status(frame, Rect {
        x: area.x,
        y: area.y + area.height - status_h,
        width: area.width,
        height: status_h,
    }, ui);
}

fn render_snake_menu_title(frame: &mut Frame, area: Rect) {
    // ASCII art title
    let title_lines = vec![
        Line::from(Span::styled(
            "╔═══════════════════════════════════════╗",
            Style::default().fg(Color::Cyan)
        )),
        Line::from(Span::styled(
            "║  C R A W L   C I P H E R  //  v0.1   ║",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        )),
        Line::from(Span::styled(
            "╚═══════════════════════════════════════╝",
            Style::default().fg(Color::Cyan)
        )),
    ];

    let paragraph = Paragraph::new(title_lines)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Black));
    frame.render_widget(paragraph, area);
}

fn render_snake_menu_status(frame: &mut Frame, area: Rect, ui: &MenuUI) {
    let bg_name = if ui.custom_bg_loaded {
        "CUSTOM FILE"
    } else if ui.selected_bg_index < ui.embedded_bgs.len() {
        &ui.embedded_bgs[ui.selected_bg_index]
    } else {
        "NONE"
    };

    let pilot_info = if !ui.secret_key.is_empty() {
        format!("PILOT: {}", ui.nickname.to_uppercase())
    } else {
        "NO CREDENTIALS".to_string()
    };

    let pilot_color = if !ui.secret_key.is_empty() { Color::Green } else { Color::Yellow };

    let control_hint = if ui.focused_index.is_some() {
        " [ENTER/F] DASH SELECT "
    } else {
        " [ARROWS] NAVIGATE "
    };

    let left = Span::styled(
        format!(" {} | BG: {} ", pilot_info, bg_name),
        Style::default().fg(pilot_color).bg(Color::Rgb(10, 10, 20)),
    );

    let right = Span::styled(
        control_hint,
        Style::default().fg(Color::DarkGray).bg(Color::Rgb(10, 10, 20)),
    );

    let line = Line::from(vec![left, Span::raw("  "), right]);

    let paragraph = Paragraph::new(line)
        .alignment(Alignment::Center)
        .style(Style::default().bg(Color::Rgb(10, 10, 20)));
    frame.render_widget(paragraph, area);
}

// ===== Classic Sub-Menu Rendering (Preserved) =====

fn render_credentials_input(frame: &mut Frame, area: Rect, ui: &MenuUI) {
    let area = centered_rect(60, 40, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Secret Key
            Constraint::Length(3), // Nickname
            Constraint::Length(3), // Controls info
            Constraint::Min(1),    // Error
        ])
        .split(area);

    // Secret Key Input
    let secret_display = if ui.secret_key.is_empty() {
        "Enter Secret Key (S...)"
    } else {
        "********************************************************"
    };
    let secret_style = if ui.cred_stage == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let secret_block = Block::default().borders(Borders::ALL).title(" ACCESS KEY ");
    frame.render_widget(Paragraph::new(secret_display).block(secret_block).style(secret_style), chunks[0]);

    // Nickname Input
    let nick_style = if ui.cred_stage == 1 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let nick_block = Block::default().borders(Borders::ALL).title(" CODENAME ");
    frame.render_widget(Paragraph::new(ui.nickname.as_str()).block(nick_block).style(nick_style), chunks[1]);

    // Info
    let info = Paragraph::new("[ENTER] Confirm  [ESC] Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(info, chunks[2]);

    // Error
    if let Some(err) = &ui.error_msg {
        let err_text = Paragraph::new(format!("ERROR: {}", err))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        frame.render_widget(err_text, chunks[3]);
    }
}

fn render_key_info(frame: &mut Frame, area: Rect) {
    let area = centered_rect(70, 30, area);
    let block = Block::default().borders(Borders::ALL).title(" ACQUIRE ACCESS KEY ").style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from("To generate a Stellar Testnet Keypair:"),
        Line::from(""),
        Line::from(Span::styled("https://laboratory.stellar.org/#account-creator?network=test", Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED))),
        Line::from(""),
        Line::from("Press [ENTER] to open in browser."),
        Line::from("Press [ESC] to return."),
    ];

    let p = Paragraph::new(lines).alignment(Alignment::Center).block(Block::default().borders(Borders::NONE));
    frame.render_widget(p, inner);
}

fn render_settings(frame: &mut Frame, area: Rect, ui: &MenuUI) {
    let area = centered_rect(60, 60, area);
    let block = Block::default().borders(Borders::ALL).title(" SYSTEM SETTINGS ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // List
            Constraint::Length(3), // Footer
        ])
        .split(inner);

    let header = Paragraph::new("Select Background Pattern:")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(header, chunks[0]);

    // List BGs
    let mut spans = Vec::new();

    let count = ui.embedded_bgs.len() + 2;

    for i in 0..count {
        let name = if i < ui.embedded_bgs.len() {
            ui.embedded_bgs[i].as_str()
        } else if i == ui.embedded_bgs.len() {
            "None (Classic)"
        } else {
            "Load Custom File..."
        };

        let is_selected = i == ui.settings_selection;
        let is_active = if i < ui.embedded_bgs.len() + 1 {
            !ui.custom_bg_loaded && i == ui.selected_bg_index
        } else {
            ui.custom_bg_loaded
        };

        let prefix = if is_active { ">> " } else { "   " };
        let text = format!("{}{}", prefix, name);

        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else if is_active {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Gray)
        };

        spans.push(Line::from(Span::styled(text, style)));
    }

    frame.render_widget(Paragraph::new(spans), chunks[1]);

    let footer = Paragraph::new("[ENTER] Select  [ESC] Back")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}

fn render_custom_bg_input(frame: &mut Frame, area: Rect, ui: &MenuUI) {
    let area = centered_rect(60, 20, area);
    let block = Block::default().borders(Borders::ALL).title(" LOAD CUSTOM BACKGROUND ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Label
            Constraint::Length(3), // Input
            Constraint::Length(2), // Help
        ])
        .split(inner);

    frame.render_widget(Paragraph::new("Enter file path:"), chunks[0]);

    let input_block = Block::default().borders(Borders::ALL).style(Style::default().fg(Color::Yellow));
    frame.render_widget(Paragraph::new(ui.custom_bg_path.as_str()).block(input_block), chunks[1]);

    if let Some(err) = &ui.error_msg {
        frame.render_widget(Paragraph::new(format!("Error: {}", err)).style(Style::default().fg(Color::Red)), chunks[2]);
    } else {
        frame.render_widget(Paragraph::new("[ENTER] Load  [ESC] Cancel").style(Style::default().fg(Color::DarkGray)), chunks[2]);
    }
}

fn render_mission_select(frame: &mut Frame, area: Rect, ui: &MenuUI) {
    let area = centered_rect(60, 60, area);
    let block = Block::default().borders(Borders::ALL).title(" SELECT MISSION ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // List
            Constraint::Length(3), // Footer
        ])
        .split(inner);

    let header = Paragraph::new("Choose Operation Mode:")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));
    frame.render_widget(header, chunks[0]);

    let options = vec![
        " [ EXPEDITION ] (Sandbox Survival) ",
        " [ PUZZLE: THE NARROW PATH ] ",
        " [ PUZZLE: LASER GATE ] ",
        " [ PUZZLE: PRISM CHAMBER ] "
    ];

    let mut spans = Vec::new();
    for (i, opt) in options.iter().enumerate() {
        let is_selected = i == ui.mission_selection;
        let style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        spans.push(Line::from(Span::styled(*opt, style)));
    }

    frame.render_widget(Paragraph::new(spans).alignment(Alignment::Center), chunks[1]);

    let footer = Paragraph::new("[ENTER] Start Mission  [ESC] Back")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}

fn render_manual(frame: &mut Frame, area: Rect) {
    let area = centered_rect(70, 80, area);
    let block = Block::default()
        .title(" [ TERMINAL MANUAL ] ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let text = vec![
        Line::from(Span::styled("CONTROLS", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  W, A, S, D / Arrows  : Move"),
        Line::from("  Release Keys         : Idle (Regenerates Energy)"),
        Line::from("  I                    : Open Inventory"),
        Line::from("  Spacebar             : Fire Weapon"),
        Line::from("  F                    : Strike (A* Pathfinding Lunge)"),
        Line::from("  P                    : Toggle Autopilot (Continuous Move)"),
        Line::from("  A / Z                : Focus Target Head / Tail"),
        Line::from("  ESC                  : Pause Simulation"),
        Line::from(""),
        Line::from(Span::styled("CRYPTOGRAPHIC MECHANICS", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  1. Entropy: Seed is derived from the latest Stellar Ledger Hash."),
        Line::from("  2. Session Lock: The Soroban contract escrows your items during the match."),
        Line::from("  3. Fraud Proof: Your inputs are logged and hashed into a Simulation Hash."),
        Line::from("  4. Verification: The hash is submitted to the blockchain for validation."),
        Line::from(""),
        Line::from(Span::styled("[ESC] Return to Menu", Style::default().fg(Color::DarkGray))),
    ];

    let p = Paragraph::new(text).block(block).alignment(Alignment::Left);
    frame.render_widget(p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
