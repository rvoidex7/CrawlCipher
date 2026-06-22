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
    // Main Menu
    MenuBlockchainPlay,
    MenuOfflinePlay,
    MenuLanP2PPlay,
    MenuSettingsHelp,
    ExitTerminal,

    // Blockchain Menu
    BlockchainStart,
    BlockchainManageCreds,

    // Mission Select Actions
    StartExpedition,
    StartPuzzle1,
    StartPuzzle2,
    StartPuzzle3,

    // Settings/Help Menu
    SettingsBackgrounds,
    SettingsHelpManual,

    // Backgrounds Menu
    BackgroundCustom,
    BackgroundSelect(usize),
    
    // Navigation
    BackToMainMenu,
    BackToSettings,
}

// ===== Menu Item =====

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub label: String,
    pub x: f64,       // Grid position (percentage of grid width, 0.0-1.0)
    pub y: f64,       // Grid position (percentage of grid height, 0.0-1.0)
    pub action: MenuAction,
    pub is_focused: bool,
    pub preview_bg: Option<String>,
    pub is_left_aligned: bool,
    pub group_max_len: usize,
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
    pub force_idle_direction: Option<i32>,
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
            force_idle_direction: None,
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

        // Auto-approach and movement
        if !self.user_steering && self.force_idle_direction == Some(2) {
            // SPECIAL TREADMILL LOGIC for List Menus
            self.move_timer += dt;
            if self.move_timer >= self.move_interval {
                self.move_timer -= self.move_interval;

                let head = self.body[0];
                let mut head_dy = 0.0;
                let mut target_x = head.0;
                
                if let Some(target) = self.approach_target {
                    target_x = target.0;
                    if (head.1 - target.1).abs() >= 0.5 {
                        head_dy = if head.1 < target.1 { 1.0 } else { -1.0 };
                        // Look diagonally when sliding!
                        self.direction = if head_dy > 0.0 { 3 /* SE */ } else { 1 /* NE */ };
                    } else {
                        // Look straight when aligned
                        self.direction = 2; // East
                    }
                }
                
                // Normal East movement + Y drift
                let intended_new_head_x = head.0 + 1.0;
                let intended_new_head_y = head.1 + head_dy;

                // Shift body (move tail to head)
                let last = self.body.len() - 1;
                for i in (1..=last).rev() {
                    self.body[i] = self.body[i - 1];
                }
                self.body[0] = (intended_new_head_x, intended_new_head_y);

                // TREADMILL: Shift entire snake left by 1
                for i in 0..self.body.len() {
                    self.body[i].0 -= 1.0;
                }
                
                // Force head X exactly to target X to prevent float drift
                self.body[0].0 = target_x;
            }
        } else {
            // CLASSIC LOGIC for Main Menu
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
                        if let Some(dir) = self.force_idle_direction {
                            self.direction = dir;
                        }
                    }
                } else {
                    self.is_approaching = false;
                }
            }

            // Normal movement
            self.move_timer += dt;
            if self.move_timer >= self.move_interval {
                self.move_timer -= self.move_interval;

                if !self.is_approaching && !self.user_steering {
                    // Do not move body if we are fully idle in classic mode
                    // except if we want to add an idle animation later
                    return;
                }

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
    BlockchainMenu,
    SettingsHelpMenu,
    BackgroundsMenu,

    // Classic dialogs
    CredentialsInput,
    CustomBackgroundInput,
    MissionSelect,
    HelpManual,
}

// ===== Menu UI =====

pub struct MenuUI {
    pub secret_key: String,
    pub nickname: String,
    pub state: MenuState,
    pub cred_stage: usize,
    pub embedded_bgs: Vec<String>,
    pub selected_bg_index: usize,
    pub error_msg: Option<String>,
    pub custom_bg_path: String,
    pub custom_bg_loaded: bool,
    pub mission_selection: usize,

    pub bg_pattern: background::BackgroundPattern,
    pub bg_previews: std::collections::HashMap<String, background::BackgroundPattern>,
    pub bg_offset_x: f64,
    pub bg_offset_y: f64,
    pub bg_progress: f64,
    pub bg_tick_counter: u32,
    pub breadcrumb_path: Vec<String>,

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

        let menu_items = vec![];

        let embedded_bgs = background::list_embedded_backgrounds();
        
        let mut bg_previews = std::collections::HashMap::new();
        for bg_name in &embedded_bgs {
            let mut pat = background::BackgroundPattern::new();
            pat.set_seed(12345);
            if bg_name == "PROCEDURAL_CRYPTO" {
                pat.enable_procedural();
            } else {
                pat.load_from_embedded(bg_name);
            }
            bg_previews.insert(bg_name.clone(), pat);
        }
        
        let empty_pat = background::BackgroundPattern::new();
        bg_previews.insert("NONE".to_string(), empty_pat);

        let mut ui = Self {
            secret_key: String::new(),
            nickname: "Pilot".to_string(),
            state: MenuState::MainMenu,
            cred_stage: 0,
            embedded_bgs,
            selected_bg_index: 0,
            error_msg: None,
            custom_bg_path: String::new(),
            custom_bg_loaded: false,
            mission_selection: 0,
            bg_pattern: background::BackgroundPattern::new(),
            bg_previews,
            bg_offset_x: 0.0,
            bg_offset_y: 0.0,
            bg_progress: 0.0,
            bg_tick_counter: 0,
            breadcrumb_path: Vec::new(),

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
        };
        ui.update_items_for_state();
        ui.reload_background();
        ui
    }

    pub fn build_list_menu(actions: Vec<(&str, MenuAction)>, start_y: f64, y_step: f64) -> Vec<MenuItem> {
        let mut items = vec![];
        for (i, (label_text, action)) in actions.into_iter().enumerate() {
            items.push(MenuItem {
                label: format!("[ {} ]", label_text),
                x: 0.65,
                y: start_y + (i as f64) * y_step,
                action,
                is_focused: false,
                preview_bg: None,
                is_left_aligned: true,
                group_max_len: 0,
            });
        }
        let max_len = items.iter().map(|i| i.label.len()).max().unwrap_or(0);
        for i in &mut items {
            i.group_max_len = max_len;
        }
        items
    }

    pub fn update_items_for_state(&mut self) {
        self.menu_items.clear();
        self.focused_index = None;
        self.snake.clear_dash_action();

        match self.state {
            MenuState::MainMenu => {
                self.menu_items = Self::build_list_menu(vec![
                    ("BLOCKCHAIN PLAY", MenuAction::MenuBlockchainPlay),
                    ("OFFLINE PLAY", MenuAction::MenuOfflinePlay),
                    ("LAN / P2P PLAY", MenuAction::MenuLanP2PPlay),
                    ("SETTINGS / HELP", MenuAction::MenuSettingsHelp),
                    ("EXIT TERMINAL", MenuAction::ExitTerminal),
                ], 0.35, 0.12);
            }
            MenuState::BlockchainMenu => {
                self.menu_items = Self::build_list_menu(vec![
                    ("START", MenuAction::BlockchainStart),
                    ("MANAGE CREDENTIALS", MenuAction::BlockchainManageCreds),
                    ("BACK", MenuAction::BackToMainMenu),
                ], 0.5 - 0.15, 0.15);
            }
            MenuState::SettingsHelpMenu => {
                self.menu_items = Self::build_list_menu(vec![
                    ("BACKGROUNDS", MenuAction::SettingsBackgrounds),
                    ("HELP", MenuAction::SettingsHelpManual),
                    ("BACK", MenuAction::BackToMainMenu),
                ], 0.5 - 0.15, 0.15);
            }
            MenuState::BackgroundsMenu => {
                let mut items = Vec::new();
                let bgs = &self.embedded_bgs;
                let total_bgs = bgs.len() + 1; // +1 for None (Classic Grid)

                // Collect all items (bg selectors + custom + back)
                let mut all_labels: Vec<(String, MenuAction, Option<String>)> = Vec::new();
                for i in 0..total_bgs {
                    let is_none = i == bgs.len();
                    let bg_name = if is_none { "NONE".to_string() } else { bgs[i].clone() };
                    let label = if is_none { " [CLASSIC GRID] ".to_string() } else { format!(" [{}] ", bg_name) };
                    all_labels.push((label, MenuAction::BackgroundSelect(i), Some(bg_name)));
                }
                all_labels.push(("[ CUSTOM FILE ]".to_string(), MenuAction::BackgroundCustom, None));
                all_labels.push(("[ BACK ]".to_string(), MenuAction::BackToSettings, None));

                // BackgroundsMenu has NO logo — use full screen for slot placement.
                // Generate slots on a 4x4 coarse grid across the full screen
                let grid_cols = 4_i32;
                let grid_rows = 4_i32;
                let mut valid_slots = Vec::new();

                for r in 0..grid_rows {
                    for c in 0..grid_cols {
                        let cx = 0.1 + (0.8 / grid_cols as f64) * (c as f64 + 0.5);
                        let cy = 0.1 + (0.8 / grid_rows as f64) * (r as f64 + 0.5);
                        valid_slots.push((cx, cy));
                    }
                }

                valid_slots.sort_by(|a, b| {
                    let a_val = (a.0 * 13.0 + a.1 * 17.0) % 1.0;
                    let b_val = (b.0 * 13.0 + b.1 * 17.0) % 1.0;
                    a_val.partial_cmp(&b_val).unwrap_or(std::cmp::Ordering::Equal)
                });

                // Assign items to slots with tiny jitter
                for (i, (label, action, bg_name)) in all_labels.into_iter().enumerate() {
                    let (mut slot_x, mut slot_y) = if i < valid_slots.len() {
                        valid_slots[i]
                    } else {
                        (0.5, 0.9) // fallback
                    };

                    let jitter_x = ((i * 7) % 11) as f64 / 11.0 * 0.04 - 0.02;
                    let jitter_y = ((i * 13) % 17) as f64 / 17.0 * 0.04 - 0.02;
                    slot_x = (slot_x + jitter_x).clamp(0.08, 0.92);
                    slot_y = (slot_y + jitter_y).clamp(0.08, 0.92);

                    items.push(MenuItem {
                        label, x: slot_x, y: slot_y,
                        action, is_focused: false,
                        preview_bg: bg_name,
                        is_left_aligned: false, group_max_len: 0,
                    });
                }

                self.menu_items = items;
            }
            MenuState::MissionSelect => {
                self.menu_items = Self::build_list_menu(vec![
                    ("EXPEDITION", MenuAction::StartExpedition),
                    ("PUZZLE: THE NARROW PATH", MenuAction::StartPuzzle1),
                    ("PUZZLE: LASER GATE", MenuAction::StartPuzzle2),
                    ("PUZZLE: PRISM CHAMBER", MenuAction::StartPuzzle3),
                    ("BACK", MenuAction::BackToMainMenu),
                ], 0.5 - 2.0 * 0.15, 0.15);
            }
            _ => {}
        }

        // After placing items at their ideal positions, resolve any
        // collisions with the logo's reserved area
        self.resolve_collisions();
    }

    /// "Arsa kiralama" sistemi: Logo sol üstte sabit bir alan kiralıyor.
    /// Bu alan üzerine düşen menü öğeleri otomatik olarak boş alana kaydırılır.
    /// Aynı satırdaki bir öğe çarptıysa, tüm satır birlikte kayar.
    fn resolve_collisions(&mut self) {
        let gw = self.grid_width;
        let gh = self.grid_height;
        if gw < 1.0 || gh < 1.0 { return; }

        // Logo reserved area in screen coordinates
        let logo_right_chars = 62.0_f64;
        let logo_bottom_rows = 15.0_f64;

        // Convert logo bottom edge to normalized y (with 1 row padding)
        let logo_bottom_norm = (logo_bottom_rows + 1.0) / gh;

        // Minimum vertical gap between items (2 screen rows)
        let min_gap = 2.0 / gh;

        // Pass 1: Find which y-values (rows) have ANY item colliding with the logo.
        // If one item in a row collides, the WHOLE row will be pushed.
        let mut colliding_rows: Vec<f64> = Vec::new(); // y values that collide
        let epsilon = 0.001;
        for item in self.menu_items.iter() {
            let screen_y = item.y * gh;
            let center_x_screen = item.x * gw * 2.0;
            let half_label = item.label.len() as f64 / 2.0;
            let left_x_screen = center_x_screen - half_label;

            if screen_y < logo_bottom_rows && left_x_screen < logo_right_chars {
                if !colliding_rows.iter().any(|&y| (y - item.y).abs() < epsilon) {
                    colliding_rows.push(item.y);
                }
            }
        }

        // Push ALL items in colliding rows below the logo
        for item in self.menu_items.iter_mut() {
            if colliding_rows.iter().any(|&y| (y - item.y).abs() < epsilon) {
                item.y = item.y.max(logo_bottom_norm);
            }
        }

        // Pass 2: Resolve item-to-item vertical stacking.
        // Items in the same column (similar x) that are too close get spaced out.
        // Run multiple passes until stable (cascading pushes).
        for _ in 0..self.menu_items.len() {
            let mut changed = false;
            for i in 1..self.menu_items.len() {
                let prev_x = self.menu_items[i - 1].x;
                let prev_y = self.menu_items[i - 1].y;
                let curr_x = self.menu_items[i].x;
                let curr_y = self.menu_items[i].y;

                // Only enforce gap for items in the same column
                let dx = (curr_x - prev_x).abs();
                if dx < 0.1 && curr_y - prev_y < min_gap && curr_y - prev_y >= 0.0 {
                    self.menu_items[i].y = prev_y + min_gap;
                    changed = true;
                }
            }
            if !changed { break; }
        }

        // Pass 3: Clamp all items within screen bounds
        for item in self.menu_items.iter_mut() {
            item.y = item.y.clamp(0.02, 0.95);
        }
    }

    pub fn reload_background(&mut self) {
        let mut bg = background::BackgroundPattern::new();
        bg.set_seed(12345); // Default seed for menu preview
        if self.custom_bg_loaded {
            let _ = bg.load_from_file(&self.custom_bg_path);
        } else if self.selected_bg_index < self.embedded_bgs.len() {
            let filename = &self.embedded_bgs[self.selected_bg_index];
            if filename == "PROCEDURAL_CRYPTO" {
                bg.enable_procedural();
            } else {
                bg.load_from_embedded(filename);
            }
        }
        self.bg_pattern = bg;
    }

    /// Update the snake simulation and focus system. Call every frame.
    pub fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;

        // Cap dt to avoid huge jumps
        let dt = dt.min(0.1);
        let old_timer = self.snake.move_timer;
        self.snake.tick(dt, self.grid_width, self.grid_height);
        
        let snake_ticked = self.snake.move_timer < old_timer + dt - 0.0001;
        let (sdx, sdy) = MenuSnake::direction_delta(self.snake.direction);
        
        if self.snake.is_dashing {
            self.bg_progress += dt * 12.5; // Half of dash speed
            while self.bg_progress >= 1.0 {
                self.bg_offset_x += sdx;
                self.bg_offset_y += sdy;
                self.bg_progress -= 1.0;
            }
        } else if snake_ticked {
            self.bg_tick_counter += 1;
            if self.bg_tick_counter % 2 == 0 { // 1/2 speed phase-locked parallax
                self.bg_offset_x += sdx;
                self.bg_offset_y += sdy;
            }
        }

        // Update focus based on snake direction
        self.update_focus();
    }

    pub fn focus_prev(&mut self) {
        if self.menu_items.is_empty() { return; }
        self.mouse_focus_active = true;
        let current = self.focused_index.unwrap_or(0);
        let new_idx = if current == 0 { self.menu_items.len() - 1 } else { current - 1 };
        self.set_focus(new_idx);
    }

    pub fn focus_next(&mut self) {
        if self.menu_items.is_empty() { return; }
        self.mouse_focus_active = true;
        let current = self.focused_index.unwrap_or(self.menu_items.len());
        let new_idx = (current + 1) % self.menu_items.len();
        self.set_focus(new_idx);
    }

    /// Focus item by 1-based number (keyboard shortcut 1-9)
    pub fn focus_by_number(&mut self, n: usize) -> bool {
        if n == 0 || n > self.menu_items.len() { return false; }
        self.mouse_focus_active = true;
        self.set_focus(n - 1);
        true
    }

    /// Common focus setter — updates is_focused flags, approach target, and snake
    fn set_focus(&mut self, idx: usize) {
        self.focused_index = Some(idx);
        for (i, item) in self.menu_items.iter_mut().enumerate() {
            item.is_focused = i == idx;
        }
        let (tx, ty, dir) = self.get_approach_target_for_item(idx);
        self.snake.approach_target = Some((tx, ty));
        self.snake.force_idle_direction = dir;
        self.snake.user_steering = false;
        self.snake.user_steer_cooldown = 0.0;
    }

    /// Place snake immediately at the first item position (no animation),
    /// used when entering a menu so the snake starts focused.
    pub fn place_snake_at_first_item(&mut self) {
        if self.menu_items.is_empty() { return; }

        // Get the approach target for item 0 - this is where the snake actually idles.
        // Placing the head HERE means dist=0 < stop_distance, so the snake won't move.
        let (ax, ay, dir) = self.get_approach_target_for_item(0);
        let idle_dir = dir.unwrap_or(2); // East by default

        // Build body trailing left from the approach target
        let mut body = Vec::new();
        for i in 0..6_i32 {
            let (bx, by) = match idle_dir {
                0 => (ax, ay + i as f64),         // facing North → tail goes South
                2 => (ax - i as f64, ay),          // facing East  → tail goes West
                4 => (ax, ay - i as f64),          // facing South → tail goes North
                6 => (ax + i as f64, ay),          // facing West  → tail goes East
                _ => (ax - i as f64, ay),
            };
            body.push((bx, by));
        }
        self.snake.body = body;
        self.snake.direction = idle_dir;
        self.snake.is_dashing = false;
        self.snake.is_approaching = false;
        self.snake.user_steering = false;
        self.snake.trail.clear();
        self.mouse_focus_active = true;

        // Set focus state directly (don't call set_focus which would re-set approach_target
        // to the same value but potentially cause redundant work)
        self.focused_index = Some(0);
        for (i, item) in self.menu_items.iter_mut().enumerate() {
            item.is_focused = i == 0;
        }
        self.snake.approach_target = Some((ax, ay));
        self.snake.force_idle_direction = dir;
        self.snake.user_steer_cooldown = 0.0;
    }

    fn update_focus(&mut self) {
        if self.snake.is_dashing {
            return;
        }

        let is_list_mode = self.menu_items.first().map_or(false, |i| i.is_left_aligned);

        // Mouse focus (hover/click/number key) is always highest priority: lock it.
        if self.mouse_focus_active || is_list_mode {
            if let Some(idx) = self.focused_index {
                let (tx, ty, dir) = self.get_approach_target_for_item(idx);
                self.snake.approach_target = Some((tx, ty));
                self.snake.force_idle_direction = dir;
            }
            return;
        }

        // User is NOT actively steering (released keys) AND we already have a focus:
        // keep it locked so the snake doesn't drift to another item while passing by.
        if !self.snake.user_steering && self.focused_index.is_some() {
            if let Some(idx) = self.focused_index {
                let (tx, ty, dir) = self.get_approach_target_for_item(idx);
                self.snake.approach_target = Some((tx, ty));
                self.snake.force_idle_direction = dir;
            }
            return;
        }

        // User IS actively steering with arrow/WASD keys: scan for nearest item
        // in the current steering direction and update focus.
        if !self.snake.user_steering { return; }

        let head = self.snake.head();
        let (sdx, sdy) = MenuSnake::direction_delta(self.snake.direction);

        let mut best_index: Option<usize> = None;
        let mut best_score = f64::MAX;

        for (i, item) in self.menu_items.iter().enumerate() {
            let (min_x, min_y, max_x, max_y) = self.get_item_bounding_box(item);
            let closest_x = head.0.clamp(min_x, max_x);
            let closest_y = head.1.clamp(min_y, max_y);
            let dx = closest_x - head.0;
            let dy = closest_y - head.1;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist < 1.0 { continue; }

            let dot = dx * sdx + dy * sdy;
            if dot <= 0.0 { continue; }

            let perp = (dy * sdx - dx * sdy).abs();
            let score = perp * 2.0 + dot * 0.5;

            if score < best_score {
                best_score = score;
                best_index = Some(i);
            }
        }

        // Only update focus if we found something
        if let Some(idx) = best_index {
            for (i, item) in self.menu_items.iter_mut().enumerate() {
                item.is_focused = i == idx;
            }
            self.focused_index = Some(idx);
            let (tx, ty, dir) = self.get_approach_target_for_item(idx);
            self.snake.approach_target = Some((tx, ty));
            self.snake.force_idle_direction = dir;
        } else if self.focused_index.is_none() {
            self.snake.approach_target = None;
        }
        // If user is steering but no new item found in that direction,
        // keep the current focus (don't clear it)
    }

    /// Focus an item by mouse screen position (terminal coordinates).
    /// Returns true if an item was focused.
    pub fn focus_by_screen_pos(&mut self, screen_x: u16, screen_y: u16) -> bool {
        let ga_x = self.layout_game_area_x;
        let ga_y = self.layout_game_area_y;
        let ga_w = self.layout_game_area_w;
        let ga_h = self.layout_game_area_h;

        if screen_x < ga_x || screen_y < ga_y { return false; }
        if screen_x >= ga_x + ga_w || screen_y >= ga_y + ga_h { return false; }

        let grid_x = self.layout_view_x + ((screen_x - ga_x) / 2) as i32;
        let grid_y = self.layout_view_y + (screen_y - ga_y) as i32;

        let mut best_index: Option<usize> = None;
        let mut best_dist = f64::MAX;

        for (i, item) in self.menu_items.iter().enumerate() {
            let item_world_x = item.x * self.grid_width;
            let item_world_y = item.y * self.grid_height;

            // Check if mouse is inside the preview box (if any)
            let mut inside_box = false;
            if item.preview_bg.is_some() {
                let box_w = 16.0 / 2.0;
                let box_h = 8.0;
                let box_center_y = item_world_y - (box_h / 2.0) - 1.0;
                let box_dy = (grid_y as f64 - box_center_y).abs() - box_h / 2.0;
                let box_dx = (grid_x as f64 - item_world_x).abs() - box_w / 2.0;
                if box_dx <= 0.0 && box_dy <= 0.0 {
                    inside_box = true;
                }
            }

            let match_dist = if inside_box {
                0.0
            } else {
                // Generous hit area: 5.0 grid cells from label edge
                let label_half_w = item.label.len() as f64 / 4.0;
                let dx = ((grid_x as f64 - item_world_x).abs() - label_half_w).max(0.0);
                let dy = (grid_y as f64 - item_world_y).abs();
                (dx * dx + dy * dy).sqrt()
            };

            // Accept if within 5 grid cells
            if match_dist < 5.0 && match_dist < best_dist {
                best_dist = match_dist;
                best_index = Some(i);
            }
        }

        if let Some(idx) = best_index {
            self.mouse_focus_active = true;
            self.set_focus(idx);
            true
        } else {
            false
        }
    }

    fn get_item_bounding_box(&self, item: &MenuItem) -> (f64, f64, f64, f64) {
        let center_x = item.x * self.grid_width;
        let center_y = item.y * self.grid_height;
        let half_w = item.label.len() as f64 / 4.0; // grid cells
        let half_h = 1.0; // grid cells

        if item.is_left_aligned {
            let left_x = center_x - (item.group_max_len as f64 / 4.0);
            (left_x, center_y - half_h, left_x + half_w * 2.0, center_y + half_h)
        } else {
            (center_x - half_w, center_y - half_h, center_x + half_w, center_y + half_h)
        }
    }

    fn get_approach_target_for_item(&self, idx: usize) -> (f64, f64, Option<i32>) {
        let item = &self.menu_items[idx];
        let (min_x, min_y, max_x, max_y) = self.get_item_bounding_box(item);
        let center_x = (min_x + max_x) / 2.0;
        let center_y = (min_y + max_y) / 2.0;

        if item.is_left_aligned {
            (min_x - 4.5, center_y, Some(2))
        } else {
            let cx = 0.5 * self.grid_width;
            let cy = 0.5 * self.grid_height;
            
            if (center_x - cx).abs() > (center_y - cy).abs() {
                // Horizontal item
                if center_x > cx {
                    (min_x - 1.0, center_y, None) // approach from left
                } else {
                    (max_x + 1.0, center_y, None) // approach from right
                }
            } else {
                // Vertical item
                if center_y > cy {
                    (center_x, min_y - 1.0, None)
                } else {
                    (center_x, max_y + 1.0, None)
                }
            }
        }
    }

    /// Trigger dash toward the focused menu item
    pub fn trigger_dash(&mut self) -> bool {
        if self.snake.is_dashing { return false; }

        if let Some(idx) = self.focused_index {
            let item = self.menu_items[idx].clone();
            let target_x = item.x * self.grid_width;
            let target_y = item.y * self.grid_height;
            let action = item.action;

            // Dynamic breadcrumb tracking
            match action {
                MenuAction::BackToMainMenu => {
                    self.breadcrumb_path.clear();
                }
                MenuAction::BackToSettings => {
                    self.breadcrumb_path.pop();
                }
                MenuAction::ExitTerminal | MenuAction::BackgroundSelect(_) | MenuAction::BackgroundCustom |
                MenuAction::StartExpedition | MenuAction::StartPuzzle1 | MenuAction::StartPuzzle2 | MenuAction::StartPuzzle3 |
                MenuAction::BlockchainStart | MenuAction::BlockchainManageCreds | MenuAction::SettingsHelpManual => {
                    // Non-menu-navigation actions don't affect breadcrumb
                }
                _ => {
                    let clean_label = item.label.replace("[ ", "").replace(" ]", "");
                    self.breadcrumb_path.push(clean_label);
                }
            }

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
        let status_h: u16 = 1;

        let ga_x: u16 = 0;
        let ga_y: u16 = 0; // Removed title height
        let ga_w: u16 = area_width;
        let ga_h: u16 = area_height.saturating_sub(status_h);

        let size_changed = self.layout_game_area_w != ga_w || self.layout_game_area_h != ga_h;

        // Dynamically update virtual grid dimensions to match the actual terminal area!
        // This makes menu item positions (which use percentages) responsive to window resizing.
        let dynamic_w = (ga_w / 2).max(10) as f64; // Minimum 10 grid width
        let dynamic_h = ga_h.max(10) as f64;
        self.grid_width = dynamic_w;
        self.grid_height = dynamic_h;

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

        // When terminal size changes, recalculate item positions
        if size_changed {
            self.update_items_for_state();
            // Restore or re-set focus
            if let Some(idx) = self.focused_index {
                if idx < self.menu_items.len() {
                    // Snap snake to the approach target of the focused item
                    // (where it actually idles) so it doesn't walk after resize
                    let (ax, ay, dir) = self.get_approach_target_for_item(idx);
                    let old_head_x = self.snake.body[0].0;
                    let old_head_y = self.snake.body[0].1;
                    for seg in self.snake.body.iter_mut() {
                        seg.0 += ax - old_head_x;
                        seg.1 += ay - old_head_y;
                    }
                    // Update focus state
                    self.focused_index = Some(idx);
                    for (i, item) in self.menu_items.iter_mut().enumerate() {
                        item.is_focused = i == idx;
                    }
                    self.snake.approach_target = Some((ax, ay));
                    self.snake.force_idle_direction = dir;
                    self.snake.is_approaching = false;
                }
            } else {
                // No focus yet: place snake at first item
                self.place_snake_at_first_item();
            }
        }
    }

    /// Reset snake position (after returning from submenu)
    pub fn reset_snake(&mut self) {
        self.snake = MenuSnake::new(self.grid_width / 2.0, self.grid_height / 2.0);
        self.last_tick = Instant::now();
        self.mouse_focus_active = false;
    }

    pub fn handle_event(&mut self, ev: crossterm::event::Event, menu_input_handler: &mut crate::input::InputHandler) -> Option<bool> {
        use crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};
        
        if matches!(self.state, MenuState::MainMenu | MenuState::BlockchainMenu | MenuState::SettingsHelpMenu | MenuState::BackgroundsMenu | MenuState::MissionSelect) {
            if let Event::Mouse(mouse_ev) = &ev {
                match mouse_ev.kind {
                    MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                        self.focus_by_screen_pos(mouse_ev.column, mouse_ev.row);
                    }
                    MouseEventKind::Down(_) => {
                        if self.focus_by_screen_pos(mouse_ev.column, mouse_ev.row) {
                            self.trigger_dash();
                        }
                    }
                    _ => {}
                }
                return Some(true);
            }
        }

        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press { return Some(true); }

            match self.state {
                MenuState::MainMenu
                | MenuState::BlockchainMenu
                | MenuState::SettingsHelpMenu
                | MenuState::BackgroundsMenu
                | MenuState::MissionSelect => {
                    match key.code {
                        KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                            if self.menu_items.first().map_or(false, |i| i.is_left_aligned) {
                                self.focus_prev();
                            } else {
                                self.mouse_focus_active = false;
                                menu_input_handler.handle_key_direction(0, -1);
                            }
                        }
                        KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                            if self.menu_items.first().map_or(false, |i| i.is_left_aligned) {
                                self.focus_next();
                            } else {
                                self.mouse_focus_active = false;
                                menu_input_handler.handle_key_direction(0, 1);
                            }
                        }
                        KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                            if !self.menu_items.first().map_or(false, |i| i.is_left_aligned) {
                                self.mouse_focus_active = false;
                                menu_input_handler.handle_key_direction(-1, 0);
                            }
                        }
                        KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                            if !self.menu_items.first().map_or(false, |i| i.is_left_aligned) {
                                self.mouse_focus_active = false;
                                menu_input_handler.handle_key_direction(1, 0);
                            }
                        }
                        KeyCode::Char('q') | KeyCode::Char('Q') => { self.mouse_focus_active = false; menu_input_handler.handle_key_direction(-1, -1); }
                        KeyCode::Char('e') | KeyCode::Char('E') => { self.mouse_focus_active = false; menu_input_handler.handle_key_direction(1, -1); }
                        KeyCode::Char('z') | KeyCode::Char('Z') => { self.mouse_focus_active = false; menu_input_handler.handle_key_direction(-1, 1); }
                        KeyCode::Char('c') | KeyCode::Char('C') => { self.mouse_focus_active = false; menu_input_handler.handle_key_direction(1, 1); }
                        KeyCode::Enter | KeyCode::Char('f') | KeyCode::Char('F') => {
                            self.trigger_dash();
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                            let n = c as usize - '0' as usize;
                            if self.focus_by_number(n) {
                                self.trigger_dash();
                            }
                        }
                        KeyCode::Esc => {
                            if !matches!(self.state, MenuState::MainMenu) {
                                self.state = MenuState::MainMenu;
                                self.update_items_for_state();
                                self.reset_snake();
                            } else {
                                return Some(false); // Signal to break app loop
                            }
                        }
                        _ => {}
                    }
                }
                MenuState::CredentialsInput => {
                    match key.code {
                        KeyCode::Enter => {
                            if self.cred_stage == 0 {
                                if crate::stellar::validate_secret_key(&self.secret_key).is_some() {
                                    self.cred_stage = 1;
                                    self.error_msg = None;
                                } else {
                                    self.error_msg = Some("INVALID KEY FORMAT".to_string());
                                }
                            } else {
                                self.state = MenuState::BlockchainMenu;
                                self.update_items_for_state();
                                self.reset_snake();
                            }
                        }
                        KeyCode::Backspace => {
                            if self.cred_stage == 0 { self.secret_key.pop(); }
                            else { self.nickname.pop(); }
                        }
                        KeyCode::Tab => {
                            let _ = webbrowser::open("https://laboratory.stellar.org/#account-creator?network=test");
                        }
                        KeyCode::Char(c) => {
                            if self.cred_stage == 0 { self.secret_key.push(c); }
                            else { self.nickname.push(c); }
                        }
                        KeyCode::Esc => { self.state = MenuState::BlockchainMenu; self.update_items_for_state(); self.reset_snake(); }
                        _ => {}
                    }
                }
                MenuState::CustomBackgroundInput => {
                     match key.code {
                        KeyCode::Enter => {
                            let mut bg = crate::background::BackgroundPattern::new();
                            if bg.load_from_file(&self.custom_bg_path).is_ok() {
                                self.custom_bg_loaded = true;
                                self.state = MenuState::MainMenu;
                                self.reset_snake();
                                self.reload_background();
                            } else {
                                self.error_msg = Some("FAILED TO LOAD FILE".to_string());
                            }
                        }
                        KeyCode::Backspace => { self.custom_bg_path.pop(); }
                        KeyCode::Char(c) => { self.custom_bg_path.push(c); }
                        KeyCode::Esc => {
                            self.state = MenuState::BackgroundsMenu;
                            self.update_items_for_state();
                            self.reset_snake();
                        }
                        _ => {}
                     }
                }
                MenuState::HelpManual => {
                    if key.code == KeyCode::Esc {
                        self.state = MenuState::SettingsHelpMenu;
                        self.update_items_for_state();
                        self.reset_snake();
                    }
                }
            }
        }
        Some(true)
    }
}

// ===== Rendering =====

pub fn render_menu(frame: &mut Frame, area: Rect, ui: &MenuUI) {
    match ui.state {
        MenuState::MainMenu
        | MenuState::BlockchainMenu
        | MenuState::SettingsHelpMenu
        | MenuState::BackgroundsMenu
        | MenuState::MissionSelect => render_snake_menu(frame, area, ui),
        
        MenuState::CredentialsInput => {
            render_classic_bg(frame, area);
            render_credentials_input(frame, area, ui);
        }
        MenuState::CustomBackgroundInput => {
            render_classic_bg(frame, area);
            render_custom_bg_input(frame, area, ui);
        }
        MenuState::HelpManual => {
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
    let grid_w = ui.grid_width as i32;
    let grid_h = ui.grid_height as i32;
    let status_h: u16 = 1;

    let game_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height.saturating_sub(status_h),
    };

    let view_w = (game_area.width / 2) as i32;
    let view_h = game_area.height as i32;
    let view_x = (grid_w / 2 - view_w / 2).max(0);
    let view_y = (grid_h / 2 - view_h / 2).max(0);

    render_menu_background(frame, game_area, ui, view_x, view_y, view_w, view_h, grid_w, grid_h);
    render_menu_items(frame, game_area, ui, view_x, view_y, view_w, view_h, grid_w, grid_h);
    render_menu_dash_trail(frame, game_area, ui, view_x, view_y, view_w, view_h);
    render_menu_snake(frame, game_area, ui, view_x, view_y, view_w, view_h);
    render_menu_gaze(frame, game_area, ui, view_x, view_y, view_w, view_h);
    render_menu_logo(frame, game_area, ui);
    render_menu_numbers(frame, game_area, ui, view_x, view_y, view_w, view_h, grid_w, grid_h);
    
    render_snake_menu_status(frame, Rect {
        x: area.x,
        y: area.y + area.height - status_h,
        width: area.width,
        height: status_h,
    }, ui);
}

fn render_menu_background(frame: &mut Frame, game_area: Rect, ui: &MenuUI, view_x: i32, view_y: i32, view_w: i32, view_h: i32, grid_w: i32, grid_h: i32) {
    let buf = frame.buffer_mut();
    for cy in 0..view_h.min(grid_h) {
        for cx in 0..view_w.min(grid_w) {
            let world_x = view_x + cx;
            let world_y = view_y + cy;
            let screen_x = game_area.x + (cx as u16) * 2;
            let screen_y = game_area.y + cy as u16;

            if screen_x + 1 >= game_area.x + game_area.width || screen_y >= game_area.y + game_area.height {
                continue;
            }

            let bg_world_x = world_x + ui.bg_offset_x.round() as i32;
            let bg_world_y = world_y + ui.bg_offset_y.round() as i32;

            let (bg_color, bg_left_char, bg_right_char) = if ui.bg_pattern.width == 0 {
                let is_even = (bg_world_x + bg_world_y).rem_euclid(2) == 0;
                let c = if is_even { Color::Rgb(6, 6, 12) } else { Color::Rgb(12, 12, 20) };
                (c, ' ', ' ')
            } else {
                let c1 = ui.bg_pattern.get_char(bg_world_x * 2, bg_world_y);
                let c2 = ui.bg_pattern.get_char(bg_world_x * 2 + 1, bg_world_y);
                if c1 == '█' {
                    (Color::Rgb(30, 30, 30), ' ', ' ')
                } else {
                    (Color::Reset, c1, c2)
                }
            };

            let fg_color = if ui.bg_pattern.is_procedural { Color::Rgb(0, 80, 0) } else { Color::Rgb(30, 30, 30) };
            let style = Style::default().bg(bg_color).fg(fg_color);
            let text = format!("{}{}", bg_left_char, bg_right_char);
            buf.set_string(screen_x, screen_y, &text, style);
        }
    }
}

fn render_menu_items(frame: &mut Frame, game_area: Rect, ui: &MenuUI, view_x: i32, view_y: i32, _view_w: i32, view_h: i32, grid_w: i32, grid_h: i32) {
    let buf = frame.buffer_mut();
    for item in &ui.menu_items {
        let item_world_x = (item.x * grid_w as f64) as i32;
        let item_world_y = (item.y * grid_h as f64) as i32;
        let local_x = item_world_x - view_x;
        let local_y = item_world_y - view_y;

        if local_y < 0 || local_y >= view_h { continue; }

        let label_char_len = item.label.len() as i32;
        let label_start_grid_x = local_x - label_char_len as i32 / 4;
        let screen_y = game_area.y + local_y as u16;
        let screen_x_start = game_area.x as i32 + label_start_grid_x * 2;

        if screen_y >= game_area.y + game_area.height { continue; }

        let (fg_color, bg_color, mods) = if item.is_focused {
            (Color::Black, Color::Cyan, Modifier::BOLD)
        } else {
            (Color::Cyan, Color::Rgb(6, 6, 18), Modifier::empty())
        };

        let style = Style::default().fg(fg_color).bg(bg_color).add_modifier(mods);

        if let Some(bg_name) = &item.preview_bg {
            if let Some(bg_pat) = ui.bg_previews.get(bg_name) {
                let box_w = 16;
                let box_h = 8;
                let box_start_y = screen_y.saturating_sub(box_h as u16 + 1);
                let box_start_x = screen_x_start + (item.label.len() as i32 / 2) - (box_w / 2);
                let border_style = if item.is_focused { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) };

                let mut scroll_offset = 0;
                if item.is_focused {
                    let t_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or(std::time::Duration::from_secs(0))
                        .as_millis() as u64;
                    scroll_offset = ((t_ms / 125) % 1000000) as i32;
                }

                for by in 0..box_h {
                    for bx in 0..(box_w/2) {
                        let draw_x = box_start_x + (bx * 2);
                        let draw_y = box_start_y + by as u16;
                        if draw_x >= game_area.x as i32 && draw_x + 1 < (game_area.x + game_area.width) as i32 && draw_y >= game_area.y && draw_y < game_area.y + game_area.height {
                            let px = item_world_x + bx;
                            let py = item_world_y + by - scroll_offset;
                            if bg_pat.width == 0 {
                                let is_even = (px + py).rem_euclid(2) == 0;
                                let bg_c = if is_even { Color::Rgb(20, 20, 40) } else { Color::Rgb(40, 40, 80) };
                                buf.set_string(draw_x as u16, draw_y, "  ", Style::default().bg(bg_c));
                            } else {
                                let c1 = bg_pat.get_char(px * 2, py);
                                let c2 = bg_pat.get_char(px * 2 + 1, py);
                                let fg = if bg_pat.is_procedural { Color::Rgb(0, 180, 0) } else { Color::Rgb(100, 100, 100) };
                                buf.set_string(draw_x as u16, draw_y, &format!("{}{}", c1, c2), Style::default().fg(fg).bg(Color::Black));
                            }
                        }
                    }
                }
                
                let left_x = box_start_x - 1;
                let right_x = box_start_x + box_w;
                let top_y = box_start_y.saturating_sub(1);
                let bottom_y = box_start_y + (box_h as u16);

                for bx in 0..box_w {
                    let draw_x = box_start_x + bx;
                    if draw_x >= game_area.x as i32 && draw_x < (game_area.x + game_area.width) as i32 {
                        if top_y >= game_area.y { buf.set_string(draw_x as u16, top_y, "─", border_style); }
                        if bottom_y < game_area.y + game_area.height { buf.set_string(draw_x as u16, bottom_y, "─", border_style); }
                    }
                }
                
                for by in 0..box_h {
                    let draw_y = box_start_y + by as u16;
                    if draw_y >= game_area.y && draw_y < game_area.y + game_area.height {
                        if left_x >= game_area.x as i32 { buf.set_string(left_x as u16, draw_y, "│", border_style); }
                        if right_x < (game_area.x + game_area.width) as i32 { buf.set_string(right_x as u16, draw_y, "│", border_style); }
                    }
                }
                
                if top_y >= game_area.y {
                    if left_x >= game_area.x as i32 { buf.set_string(left_x as u16, top_y, "┌", border_style); }
                    if right_x < (game_area.x + game_area.width) as i32 { buf.set_string(right_x as u16, top_y, "┐", border_style); }
                }
                if bottom_y < game_area.y + game_area.height {
                    if left_x >= game_area.x as i32 { buf.set_string(left_x as u16, bottom_y, "└", border_style); }
                    if right_x < (game_area.x + game_area.width) as i32 { buf.set_string(right_x as u16, bottom_y, "┘", border_style); }
                }
            }
        }

        for (i, ch) in item.label.chars().enumerate() {
            let sx = screen_x_start + i as i32;
            if sx >= game_area.x as i32 && sx < (game_area.x + game_area.width) as i32 {
                buf.set_string(sx as u16, screen_y, &ch.to_string(), style);
                if screen_y + 1 < game_area.y + game_area.height {
                    buf.set_string(sx as u16, screen_y + 1, " ", Style::default().bg(bg_color));
                }
            }
        }

        if item.is_focused {
            let arrow = "►";
            let arrow_x = screen_x_start - 2;
            if arrow_x >= game_area.x as i32 && arrow_x < (game_area.x + game_area.width - 1) as i32 {
                buf.set_string(arrow_x as u16, screen_y, arrow, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
            }
        }
    }
}

fn render_menu_dash_trail(frame: &mut Frame, game_area: Rect, ui: &MenuUI, view_x: i32, view_y: i32, view_w: i32, view_h: i32) {
    let buf = frame.buffer_mut();
    for trail_point in &ui.snake.trail {
        let local_x = trail_point.0 as i32 - view_x;
        let local_y = trail_point.1 as i32 - view_y;

        if local_x < 0 || local_x >= view_w || local_y < 0 || local_y >= view_h { continue; }

        let screen_x = game_area.x + (local_x as u16) * 2;
        let screen_y = game_area.y + local_y as u16;

        if screen_x + 1 >= game_area.x + game_area.width || screen_y >= game_area.y + game_area.height {
            continue;
        }

        let intensity = (trail_point.2 * 255.0).clamp(0.0, 255.0) as u8;
        let trail_color = Color::Rgb(0, intensity / 2, intensity);
        buf.set_string(screen_x, screen_y, "░░", Style::default().fg(trail_color));
    }
}

fn render_menu_snake(frame: &mut Frame, game_area: Rect, ui: &MenuUI, view_x: i32, view_y: i32, view_w: i32, view_h: i32) {
    let buf = frame.buffer_mut();
    let snake_head_color = Color::Rgb(0, 255, 220);

    for (i, seg) in ui.snake.body.iter().enumerate() {
        let local_x = seg.0.round() as i32 - view_x;
        let local_y = seg.1.round() as i32 - view_y;

        if local_x < 0 || local_x >= view_w || local_y < 0 || local_y >= view_h { continue; }

        let screen_x = game_area.x + (local_x as u16) * 2;
        let screen_y = game_area.y + local_y as u16;

        if screen_x + 1 >= game_area.x + game_area.width || screen_y >= game_area.y + game_area.height {
            continue;
        }

        let (symbol, color) = if i == 0 {
            let head_sym = match ui.snake.direction {
                0 => "▲▲", 1 => "▶▲", 2 => "▶▶", 3 => "▶▼", 4 => "▼▼", 5 => "◀▼", 6 => "◀◀", 7 => "◀▲", _ => "██",
            };
            if ui.snake.is_dashing { (head_sym, Color::Rgb(255, 255, 100)) } else { (head_sym, snake_head_color) }
        } else {
            let fade = 1.0 - (i as f64 / ui.snake.body.len() as f64) * 0.6;
            let r = (0.0 * fade) as u8;
            let g = (200.0 * fade) as u8;
            let b = (160.0 * fade) as u8;
            ("██", Color::Rgb(r, g, b))
        };
        buf.set_string(screen_x, screen_y, symbol, Style::default().fg(color));
    }
}

fn render_menu_gaze(frame: &mut Frame, game_area: Rect, ui: &MenuUI, view_x: i32, view_y: i32, view_w: i32, view_h: i32) {
    let buf = frame.buffer_mut();
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

            if screen_x + 1 >= game_area.x + game_area.width || screen_y >= game_area.y + game_area.height {
                continue;
            }

            let fade = 1.0 - (step as f64 / 4.0);
            let intensity = (fade * 80.0) as u8;
            let gaze_color = if ui.focused_index.is_some() { Color::Rgb(intensity, intensity + 40, intensity + 60) } else { Color::Rgb(intensity / 2, intensity / 2, intensity / 2) };
            buf.set_string(screen_x, screen_y, "··", Style::default().fg(gaze_color));
        }
    }
}

fn render_menu_logo(frame: &mut Frame, game_area: Rect, ui: &MenuUI) {
    let buf = frame.buffer_mut();
    let start_x = game_area.x.saturating_add(2);
    let start_y = game_area.y.saturating_add(1);

    if !matches!(ui.state, MenuState::BackgroundsMenu) {
        let logo = [
            r#"  .,-::::: :::::::..    :::.  .::    .   .::::::         "#,
            r#",;;;'````' ;;;;``;;;;   ;;`;; ';;,  ;;  ;;;' ;;;         "#,
            r#"[[[         [[[,/[[['  ,[[ '[[,'[[, [[, [['  [[[         "#,
            r#"$$$         $$$$$$c   c$$$cc$$$c Y$c$$$c$P   $$'         "#,
            r#"`88bo,__,o, 888b "88bo,888   888, "88"888   o88oo,.__    "#,
            r#"  "YUMMMMMP"MMMM   "W" YMM   ""`   "M "M"   """"YUMMM    "#,
            r#"  .,-:::::  :::::::::::::.  ::   .: .,:::::: :::::::..   "#,
            r#",;;;'````'  ;;; `;;;```.;;;,;;   ;;,;;;;'''' ;;;;``;;;;  "#,
            r#"[[[         [[[  `]]nnn]]',[[[,,,[[[ [[cccc   [[[,/[[['  "#,
            r#"$$$         $$$   $$$""   "$$$"""$$$ $$""""   $$$$$$c    "#,
            r#"`88bo,__,o, 888   888o     888   "88o888oo,__ 888b "88bo,"#,
            r#"  "YUMMMMMP"MMM   YMMMb    MMM    YMM""""YUMMMMMMM   "W" "#,
        ];

        for (i, line) in logo.iter().enumerate() {
            let y = start_y + i as u16;
            if y < game_area.y + game_area.height {
                buf.set_string(start_x, y, *line, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
            }
        }

        if !ui.breadcrumb_path.is_empty() {
            let breadcrumb_str = format!("> {}", ui.breadcrumb_path.join(" > "));
            let bc_y = start_y + logo.len() as u16;
            if bc_y < game_area.y + game_area.height {
                buf.set_string(start_x + 2, bc_y, &breadcrumb_str, Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD));
            }
        }
    } else {
        if !ui.breadcrumb_path.is_empty() {
            let breadcrumb_str = format!("> {}", ui.breadcrumb_path.join(" > "));
            if start_y < game_area.y + game_area.height {
                buf.set_string(start_x + 2, start_y, &breadcrumb_str, Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD));
            }
        }
    }
}

fn render_menu_numbers(frame: &mut Frame, game_area: Rect, ui: &MenuUI, view_x: i32, view_y: i32, _view_w: i32, view_h: i32, grid_w: i32, grid_h: i32) {
    let buf = frame.buffer_mut();
    for (num, item) in ui.menu_items.iter().enumerate() {
        if num >= 9 { break; }
        let item_world_x = (item.x * grid_w as f64) as i32;
        let item_world_y = (item.y * grid_h as f64) as i32;
        let local_x = item_world_x - view_x;
        let local_y = item_world_y - view_y + 1;
        if local_y < 0 || local_y >= view_h { continue; }
        let screen_y_num = game_area.y + local_y as u16;
        let screen_x_num = (game_area.x as i32 + local_x * 2) as u16;
        if screen_y_num < game_area.y + game_area.height && (screen_x_num as i32) >= game_area.x as i32 {
            let num_str = format!("{}", num + 1);
            let num_color = if item.is_focused { Color::Yellow } else { Color::Rgb(60, 60, 80) };
            buf.set_string(screen_x_num, screen_y_num, &num_str, Style::default().fg(num_color));
        }
    }
}

fn render_snake_menu_status(frame: &mut Frame, area: Rect, ui: &MenuUI) {
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
        format!(" V{} | {} ", env!("CARGO_PKG_VERSION"), pilot_info),
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
            Constraint::Length(4), // Key info
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

    // Key info
    let key_info_lines = vec![
        Line::from("To generate a Stellar Testnet Keypair:"),
        Line::from(Span::styled("https://laboratory.stellar.org/#account-creator?network=test", Style::default().fg(Color::Blue).add_modifier(Modifier::UNDERLINED))),
        Line::from("[TAB] Open in browser"),
    ];
    let key_info_p = Paragraph::new(key_info_lines).alignment(Alignment::Center).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(key_info_p, chunks[2]);

    // Info
    let info = Paragraph::new("[ENTER] Confirm  [ESC] Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(info, chunks[3]);

    // Error
    if let Some(err) = &ui.error_msg {
        let err_text = Paragraph::new(format!("ERROR: {}", err))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center);
        frame.render_widget(err_text, chunks[4]);
    }
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



fn render_manual(frame: &mut Frame, area: Rect) {
    let area = centered_rect(70, 85, area);
    let block = Block::default()
        .title(" [ TERMINAL MANUAL ] ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));

    let text = vec![
        Line::from(Span::styled("SYSTEM VERSION INFORMATION", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(format!("  CrawlCipher Release  : V{}", env!("CARGO_PKG_VERSION"))),
        Line::from(format!("  TUI Module           : tui-v{}", env!("CARGO_PKG_VERSION"))),
        Line::from(format!("  Core Engine          : core-v{}", env!("CORE_VERSION"))),
        Line::from(format!("  Smart Contract       : contract-v{}", env!("CONTRACT_VERSION"))),
        Line::from(""),
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
