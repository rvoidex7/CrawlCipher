mod ffi;
mod input;
mod ui;
mod inventory_ui;
mod stellar;
mod config;
mod menu_ui;
mod background;
mod splash_screen_1;
mod splash_screen_2;

use stellar::{entropy, profile, session, profile::ProfileStats};

use menu_ui::{MenuUI, MenuState, MenuAction, render_menu};
use background::BackgroundPattern;
use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, KeyEventKind, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    collections::HashMap,
    io::stdout,
    time::{Duration, Instant},
};

use ffi::NativeEngine;
use input::InputHandler;

/// CrawlCipher - Terminal-based tactical agent simulation
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable sandbox mode with custom parameters
    #[arg(long)]
    sandbox: bool,

    /// Grid width
    #[arg(long, default_value = "87")]
    grid_width: i32,

    /// Grid height
    #[arg(long, default_value = "50")]
    grid_height: i32,

    /// Maximum energy
    #[arg(long, default_value = "7")]
    energy_max: i32,

    /// Number of bots
    #[arg(long, default_value = "0")]
    bots: i32,

    /// Enable walls (true/false)
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    walls: bool,

    /// Food count
    #[arg(long, default_value = "10")]
    food_count: i32,

    /// Game tick rate in milliseconds (higher is slower)
    #[arg(long, default_value = "150")]
    simulation_speed: u64,

    /// Show grid background (checkerboard)
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    grid_visible: bool,

    /// Energy gain per step
    #[arg(long, default_value = "1")]
    energy_gain: i32,

    /// Cost for 45 degree turn
    #[arg(long, default_value = "2")]
    turn_cost_45: i32,

    /// Cost for 90 degree turn
    #[arg(long, default_value = "5")]
    turn_cost_90: i32,

    /// Cost for >90 degree turn (sharp)
    #[arg(long, default_value = "12")]
    turn_cost_sharp: i32,

    // Strike Settings
    #[arg(long, default_value = "2")]
    strike_start: i32,

    #[arg(long, default_value = "4")]
    strike_end_offset: i32,

    #[arg(long, default_value = "87")]
    strike_max_savings: i32,

    #[arg(long, default_value = "3")]
    initial_length: i32,

    /// Show ghost trail of previous position after Strike (debug)
    #[arg(long, default_value = "false", action = clap::ArgAction::Set)]
    show_ghost_trail: bool,

    /// Show valid move indicators (-)
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    show_move_indicators: bool,

    /// Snail Speed Divisor (1 = same as agent, 2 = half speed, etc.)
    #[arg(long, default_value = "2")]
    snail_speed: i32,

    /// Snail Score Reward
    #[arg(long, default_value = "50")]
    snail_score: i32,

    /// Snail Energy Reward
    #[arg(long, default_value = "5")]
    snail_energy: i32,

    /// Snail Count (Number of snails on map)
    #[arg(long, default_value = "2")]
    snail_count: i32,

    /// Enable Bonus Energy Overflow Mechanic
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    bonus_mechanic: bool,

    /// Show predicted body position for Strike
    #[arg(long, default_value = "false", action = clap::ArgAction::Set)]
    show_strike_body: bool,

    /// Energy gain per tick while idle (manual mode)
    #[arg(long, default_value = "2")]
    idle_gain: i32,

    /// Give unlimited items for testing
    #[arg(long, default_value = "false", action = clap::ArgAction::Set)]
    unlimited_items: bool,

    /// Show energy drain on body segments
    #[arg(long, default_value = "false", action = clap::ArgAction::Set)]
    energy_body_indicator: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Load Config
    let config = config::load_config("expedition_config.json").unwrap_or_default();

    // Initial Setup
    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Play cinematics
    let _ = splash_screen_2::run(&mut terminal);
    let _ = splash_screen_1::run(&mut terminal);

    let mut menu = MenuUI::new();
    let mut menu_input_handler = input::InputHandler::new();
    let mut menu_input_timer = std::time::Instant::now();

    // OUTER APP LOOP
    'app_loop: loop {

    // Reset menu to main state on app restart
    menu.state = MenuState::MainMenu;
    menu.reset_snake();

    // MAIN MENU LOOP
    'menu_loop: loop {
        // Tick the menu snake simulation for any snake-based menu
        if matches!(menu.state, MenuState::MainMenu | MenuState::BlockchainMenu | MenuState::SettingsHelpMenu | MenuState::BackgroundsMenu | MenuState::MissionSelect) {
            menu.tick();

            // Check if a dash action completed
            if let Some(action) = menu.poll_dash_action() {
                match action {
                    MenuAction::MenuBlockchainPlay => {
                        menu.state = MenuState::BlockchainMenu;
                        menu.update_items_for_state();
                        menu.reset_snake();
                    }
                    MenuAction::MenuOfflinePlay => {
                        menu.secret_key.clear();
                        menu.nickname = "GHOST".to_string();
                        menu.state = MenuState::MissionSelect;
                        menu.mission_selection = 0;
                        menu.update_items_for_state();
                        menu.reset_snake();
                    }
                    MenuAction::MenuLanP2PPlay => {
                        // Coming Soon (do nothing or show a message later)
                    }
                    MenuAction::MenuSettingsHelp => {
                        menu.state = MenuState::SettingsHelpMenu;
                        menu.update_items_for_state();
                        menu.reset_snake();
                    }
                    MenuAction::ExitTerminal => {
                        break 'app_loop;
                    }
                    MenuAction::BlockchainStart => {
                        if !menu.secret_key.is_empty() {
                            menu.state = MenuState::MissionSelect;
                            menu.update_items_for_state();
                            menu.reset_snake();
                        } else {
                            menu.state = MenuState::CredentialsInput;
                            menu.cred_stage = 0;
                            menu.error_msg = None;
                        }
                    }
                    MenuAction::StartExpedition => { menu.mission_selection = 0; break 'menu_loop; }
                    MenuAction::StartPuzzle1 => { menu.mission_selection = 1; break 'menu_loop; }
                    MenuAction::StartPuzzle2 => { menu.mission_selection = 2; break 'menu_loop; }
                    MenuAction::StartPuzzle3 => { menu.mission_selection = 3; break 'menu_loop; }
                    MenuAction::BlockchainManageCreds => {
                        menu.state = MenuState::CredentialsInput;
                        menu.cred_stage = 0;
                        menu.error_msg = None;
                    }
                    MenuAction::SettingsBackgrounds => {
                        menu.state = MenuState::BackgroundsMenu;
                        menu.update_items_for_state();
                        menu.reset_snake();
                    }
                    MenuAction::SettingsHelpManual => {
                        menu.state = MenuState::HelpManual;
                    }
                    MenuAction::BackToMainMenu => {
                        menu.state = MenuState::MainMenu;
                        menu.update_items_for_state();
                        menu.reset_snake();
                    }
                    MenuAction::BackgroundSelect(idx) => {
                        menu.selected_bg_index = idx;
                        menu.custom_bg_loaded = false;
                        menu.reload_background();
                    }
                    MenuAction::BackgroundCustom => {
                        menu.state = MenuState::CustomBackgroundInput;
                        menu.error_msg = None;
                        menu.custom_bg_path.clear();
                    }
                    MenuAction::BackToSettings => {
                        menu.state = MenuState::SettingsHelpMenu;
                        menu.update_items_for_state();
                        menu.reset_snake();
                    }
                }
            }
        }

        // Update layout info for mouse mapping
        if matches!(menu.state, MenuState::MainMenu | MenuState::BlockchainMenu | MenuState::SettingsHelpMenu | MenuState::BackgroundsMenu | MenuState::MissionSelect) {
            let term_size = terminal.size()?;
            menu.update_layout(term_size.width, term_size.height);
        }

        terminal.draw(|f| render_menu(f, f.size(), &menu))?;

        let start_frame = std::time::Instant::now();
        let frame_duration = Duration::from_millis(16); // ~60fps for smooth animation

        loop {
            let elapsed = start_frame.elapsed();
            if elapsed >= frame_duration {
                break;
            }
            let timeout = frame_duration - elapsed;

            if event::poll(timeout)? {
                // Drain all available events
                while event::poll(Duration::from_millis(0))? {
                let ev = event::read()?;

                // Handle mouse events for MainMenu
                if matches!(menu.state, MenuState::MainMenu | MenuState::BlockchainMenu | MenuState::SettingsHelpMenu | MenuState::BackgroundsMenu | MenuState::MissionSelect) {
                    if let Event::Mouse(mouse_ev) = &ev {
                        match mouse_ev.kind {
                            MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                                // Mouse hover: focus nearest item
                                menu.focus_by_screen_pos(mouse_ev.column, mouse_ev.row);
                            }
                            MouseEventKind::Down(_) => {
                                // Mouse click: focus + trigger dash
                                if menu.focus_by_screen_pos(mouse_ev.column, mouse_ev.row) {
                                    menu.trigger_dash();
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                }

                if let Event::Key(key) = ev {
                    if key.kind != KeyEventKind::Press { continue; }

                    match menu.state {
                        MenuState::MainMenu
                        | MenuState::BlockchainMenu
                        | MenuState::SettingsHelpMenu
                        | MenuState::BackgroundsMenu
                        | MenuState::MissionSelect => {
                            match key.code {
                                // Arrow keys & WASD (accumulate dx/dy to support diagonal chording)
                                KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                                    if menu.menu_items.first().map_or(false, |i| i.is_left_aligned) {
                                        menu.focus_prev();
                                    } else {
                                        menu_input_handler.handle_key_direction(0, -1);
                                    }
                                }
                                KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                                    if menu.menu_items.first().map_or(false, |i| i.is_left_aligned) {
                                        menu.focus_next();
                                    } else {
                                        menu_input_handler.handle_key_direction(0, 1);
                                    }
                                }
                                KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                                    if !menu.menu_items.first().map_or(false, |i| i.is_left_aligned) {
                                        menu_input_handler.handle_key_direction(-1, 0);
                                    }
                                }
                                KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                                    if !menu.menu_items.first().map_or(false, |i| i.is_left_aligned) {
                                        menu_input_handler.handle_key_direction(1, 0);
                                    }
                                }
                                // Diagonal shortcuts
                                KeyCode::Char('q') | KeyCode::Char('Q') => menu_input_handler.handle_key_direction(-1, -1),
                                KeyCode::Char('e') | KeyCode::Char('E') => menu_input_handler.handle_key_direction(1, -1),
                                KeyCode::Char('z') | KeyCode::Char('Z') => menu_input_handler.handle_key_direction(-1, 1),
                                KeyCode::Char('c') | KeyCode::Char('C') => menu_input_handler.handle_key_direction(1, 1),
                                // Dash select (Enter or F)
                                KeyCode::Enter | KeyCode::Char('f') | KeyCode::Char('F') => {
                                    menu.trigger_dash();
                                }
                                KeyCode::Esc => {
                                    if !matches!(menu.state, MenuState::MainMenu) {
                                        menu.state = MenuState::MainMenu;
                                        menu.update_items_for_state();
                                        menu.reset_snake();
                                    } else {
                                        break 'app_loop;
                                    }
                                }
                                _ => {}
                            }
                        }
                        MenuState::CredentialsInput => {
                            match key.code {
                                KeyCode::Enter => {
                                    if menu.cred_stage == 0 {
                                        // Validate Key
                                        if stellar::validate_secret_key(&menu.secret_key).is_some() {
                                            menu.cred_stage = 1; // Next: Nickname
                                            menu.error_msg = None;
                                        } else {
                                            menu.error_msg = Some("INVALID KEY FORMAT".to_string());
                                        }
                                    } else {
                                        // Complete
                                        menu.state = MenuState::BlockchainMenu;
                                        menu.update_items_for_state();
                                        menu.reset_snake();
                                    }
                            }
                            KeyCode::Backspace => {
                                if menu.cred_stage == 0 { menu.secret_key.pop(); }
                                else { menu.nickname.pop(); }
                            }
                            KeyCode::Tab => {
                                let _ = webbrowser::open("https://laboratory.stellar.org/#account-creator?network=test");
                            }
                            KeyCode::Char(c) => {
                                if menu.cred_stage == 0 { menu.secret_key.push(c); }
                                else { menu.nickname.push(c); }
                            }
                            KeyCode::Esc => { menu.state = MenuState::BlockchainMenu; menu.update_items_for_state(); menu.reset_snake(); }
                            _ => {}
                        }
                    }

                    MenuState::CustomBackgroundInput => {
                         match key.code {
                            KeyCode::Enter => {
                                // Try to load
                                let mut bg = BackgroundPattern::new();
                                if bg.load_from_file(&menu.custom_bg_path).is_ok() {
                                    menu.custom_bg_loaded = true;
                                    menu.state = MenuState::MainMenu;
                                    menu.reset_snake();
                                    menu.reload_background();
                                } else {
                                    menu.error_msg = Some("FAILED TO LOAD FILE".to_string());
                                }
                            }
                            KeyCode::Backspace => { menu.custom_bg_path.pop(); }
                            KeyCode::Char(c) => { menu.custom_bg_path.push(c); }
                            KeyCode::Esc => {
                                menu.state = MenuState::BackgroundsMenu;
                                menu.update_items_for_state();
                                menu.reset_snake();
                            }
                            _ => {}
                         }
                    }
                    MenuState::HelpManual => {
                        if key.code == KeyCode::Esc {
                            menu.state = MenuState::SettingsHelpMenu;
                            menu.update_items_for_state();
                            menu.reset_snake();
                        }
                    }

            }
        } // closes if let Event::Key
                } // end while (draining events)
            } // end if event::poll
        } // end loop waiting for frame_duration

        // Resolve the accumulated direction for MainMenu every 50ms window
        // This provides a "chording" window for the user to press e.g., Up+Right together
        if matches!(menu.state, MenuState::MainMenu | MenuState::BackgroundsMenu | MenuState::BlockchainMenu | MenuState::SettingsHelpMenu | MenuState::MissionSelect) && menu_input_timer.elapsed() >= Duration::from_millis(50) {
            if let Some(dir) = menu_input_handler.resolve_direction() {
                menu.mouse_focus_active = false;
                menu.snake.set_direction(dir);
            }
            menu_input_timer = std::time::Instant::now();
        }
    }

    // Determine Seed & Profile based on Mode
    let is_offline = menu.secret_key.is_empty();

    let seed;
    let profile_stats;

    if !is_offline {
        // Online Mode: Fetch from Stellar (Using blocking call or waiting)
        // Since we are inside tui now, printing is bad.
        // We should render a loading screen. For MVP we just wait.
        seed = match entropy::fetch_latest_ledger_hash().await {
            Ok(hash) => entropy::hash_to_seed(&hash),
            Err(_) => 12345,
        };

        // Validate Public Key if possible or just use dummy account
        let account_id = std::env::var("STELLAR_ACCOUNT_ID").unwrap_or("GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF".to_string());
        profile_stats = profile::fetch_profile(&account_id).await.ok();
    } else {
        // Offline Mode
        seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as i64;
        profile_stats = None;
    }

    // Load Background
    let mut bg = BackgroundPattern::new();
    bg.set_seed(seed);
    
    if menu.custom_bg_loaded {
        let _ = bg.load_from_file(&menu.custom_bg_path);
    } else if menu.selected_bg_index < menu.embedded_bgs.len() {
        let selected_bg = &menu.embedded_bgs[menu.selected_bg_index];
        if selected_bg == "PROCEDURAL_CRYPTO" {
            bg.enable_procedural();
        } else {
            bg.load_from_embedded(selected_bg);
        }
    }

    // Create the Proprietary Engine simulation instance via FFI with full configuration
    // Apply overrides from config if present, else use CLI/Defaults
    let initial_len = if config.expedition.spawn.initial_snake_length > 0 { config.expedition.spawn.initial_snake_length } else { args.initial_length };
    let initial_bots = if config.expedition.spawn.initial_bot_count > 0 { config.expedition.spawn.initial_bot_count } else { args.bots };

    // Explicitly seed the simulation with fetched entropy for deterministic verification
    let simulation = NativeEngine::new(
        seed,
        &menu.nickname,
        args.grid_width,
        args.grid_height,
        args.food_count,
        args.walls,
        args.energy_max,
        args.energy_gain,
        args.turn_cost_45,
        args.turn_cost_90,
        args.turn_cost_sharp,
    );

    // Configure Strike & Length
    simulation.process_input(8, 9, args.strike_start);
    simulation.process_input(8, 10, args.strike_end_offset);
    simulation.process_input(8, 11, args.strike_max_savings);
    simulation.process_input(8, 12, initial_len);
    simulation.process_input(8, 13, if args.show_ghost_trail { 1 } else { 0 });
    simulation.process_input(8, 14, args.snail_speed);
    simulation.process_input(8, 15, args.snail_score);
    simulation.process_input(8, 16, args.snail_energy);
    simulation.process_input(8, 17, if args.bonus_mechanic { 1 } else { 0 });
    simulation.process_input(8, 18, args.snail_count);
    simulation.process_input(8, 19, if args.show_strike_body { 1 } else { 0 });
    simulation.process_input(8, 21, args.idle_gain);
    simulation.process_input(8, 22, if args.unlimited_items { 1 } else { 0 });

    let (mode, puzzle_id) = match menu.mission_selection {
        0 => ("Expedition", ""),
        1 => ("Puzzle", "The Narrow Path"),
        2 => ("Puzzle", "Laser Gate"),
        3 => ("Puzzle", "Prism Chamber"),
        _ => ("Expedition", "")
    };
    simulation.set_game_mode(mode, puzzle_id);

    // Smart Contract: Session Lock
    if !is_offline {
        let loadout_items = vec!["PISTOL_1".to_string(), "RIFLE_1".to_string(), "LASER_1".to_string()];

        // Temporarily leave raw mode to print contract output clearly
        crossterm::terminal::disable_raw_mode().unwrap();
        if let Err(e) = session::lock_session(&menu.secret_key, loadout_items).await {
            eprintln!("Failed to lock session assets: {}", e);
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        crossterm::terminal::enable_raw_mode().unwrap();
    }

    // Start simulation with bots
    simulation.process_input(5, initial_bots, 0);

    // Input handler for 8-directional combo detection
    let mut input_handler = InputHandler::new();

    // Simulation loop
    let tick_rate = Duration::from_millis(args.simulation_speed);
    let mut last_tick = Instant::now();

    // Camera Smoothing Variables (f64 for precision)
    let mut state = simulation.get_simulation_state();
    let mut player = simulation.get_player_state(state.local_player_id);
    let mut camera_x = player.focused_x as f64;
    let mut camera_y = player.focused_y as f64;

    // Inventory State
    let mut show_inventory = false;
    let mut inventory_index = 0;

    loop {
        // Poll for input events (10ms timeout serves as render loop pace)
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Release {
                    continue;
                }

                if show_inventory {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('i') | KeyCode::Char('I') => show_inventory = false,
                        KeyCode::Up | KeyCode::Char('w') => {
                            if inventory_index > 0 { inventory_index -= 1; }
                        }
                        KeyCode::Down | KeyCode::Char('s') => {
                            let count = simulation.get_backpack(player.id).len();
                            if inventory_index < count.saturating_sub(1) { inventory_index += 1; }
                        }
                        KeyCode::Char('a') | KeyCode::Char('A') => {
                            let jump = if key.modifiers.contains(KeyModifiers::SHIFT) { 1 } else { 0 };
                            simulation.process_input(2, jump, 0);
                        }
                        KeyCode::Char('z') | KeyCode::Char('Z') => {
                            let jump = if key.modifiers.contains(KeyModifiers::SHIFT) { 1 } else { 0 };
                            simulation.process_input(3, jump, 0);
                        }
                        KeyCode::Char('e') | KeyCode::Char('E') | KeyCode::Enter => {
                            let backpack = simulation.get_backpack(player.id);
                            let grouped = crate::inventory_ui::group_backpack(&backpack);
                            if let Some(group) = grouped.get(inventory_index) {
                                simulation.equip_item(player.id, &group.first_id, player.focused_segment, 1); // 1 = Right (default)
                            }
                        }
                        KeyCode::Char('x') | KeyCode::Char('X') => {
                            let backpack = simulation.get_backpack(player.id);
                            let grouped = crate::inventory_ui::group_backpack(&backpack);
                            if let Some(group) = grouped.get(inventory_index) {
                                simulation.equip_item(player.id, &group.first_id, player.focused_segment, 0); // 0 = Left
                            }
                        }
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            let backpack = simulation.get_backpack(player.id);
                            let grouped = crate::inventory_ui::group_backpack(&backpack);
                            if let Some(group) = grouped.get(inventory_index) {
                                simulation.equip_item(player.id, &group.first_id, player.focused_segment, 1); // 1 = Right
                            }
                        }
                        KeyCode::Char('u') | KeyCode::Char('U') => {
                            simulation.unequip_item(player.id, player.focused_segment);
                        }
                        _ => {}
                    }
                } else {
                    // Check if we are returning to main menu
                    if (state.simulation_state == 2 || state.simulation_state == 3) && (key.code == KeyCode::Char('m') || key.code == KeyCode::Char('M')) {
                        break; // Breaks the simulation loop, moving to the post-simulation logic and then back to app_loop
                    }

                    // Complete quit from anywhere
                    if key.modifiers.contains(KeyModifiers::CONTROL) && (key.code == KeyCode::Char('q') || key.code == KeyCode::Char('Q')) {
                        break 'app_loop;
                    }

                    if key.code == KeyCode::Char('i') || key.code == KeyCode::Char('I') {
                        show_inventory = true;
                        inventory_index = 0;
                        // Reset input handler to stop movement if any pending
                        input_handler.reset();
                    } else if handle_key_event(key, &simulation, &mut input_handler, args.bots) {
                        // Exit requested from within handle_key_event
                        break 'app_loop;
                    }
                }
            }
        }

        // Resolve accumulated inputs before simulation update
        if last_tick.elapsed() >= tick_rate {
            // Only resolve movement if inventory is closed
            if !show_inventory {
                input_handler.resolve_and_send(&simulation);
            }
            simulation.update();
            last_tick = Instant::now();
        }

        // Refresh State
        state = simulation.get_simulation_state();
        player = simulation.get_player_state(state.local_player_id);

        // Smooth Camera Logic
        // Target is player focus
        let target_x = player.focused_x as f64;
        let target_y = player.focused_y as f64;
        let grid_w = state.grid_width as f64;
        let grid_h = state.grid_height as f64;

        // Handle Wrapping for interpolation
        // If distance > half width, warp the "target" closer (add/sub width)
        let mut adj_target_x = target_x;
        let mut adj_target_y = target_y;

        if !args.walls {
            if (target_x - camera_x).abs() > grid_w / 2.0 {
                if target_x > camera_x { adj_target_x -= grid_w; } else { adj_target_x += grid_w; }
            }
            if (target_y - camera_y).abs() > grid_h / 2.0 {
                if target_y > camera_y { adj_target_y -= grid_h; } else { adj_target_y += grid_h; }
            }
        }

        // Distance check to prevent jitter on single steps
        let dx = adj_target_x - camera_x;
        let dy = adj_target_y - camera_y;
        let dist_sq = dx * dx + dy * dy;

        // Hybrid approach:
        // If distance is small (<= 1.5 units), snap instantly (prevents jitter during normal move).
        // If distance is large (Strike, Jump), use sub-grid Lerp for smoothness.

        if dist_sq <= 2.25 { // 1.5^2
            camera_x = adj_target_x;
            camera_y = adj_target_y;
        } else {
            let lerp = 0.15;
            camera_x += dx * lerp;
            camera_y += dy * lerp;
        }

        // Normalize back to grid range if wrapped
        if !args.walls {
            camera_x = (camera_x + grid_w) % grid_w;
            camera_y = (camera_y + grid_h) % grid_h;
        }

        let mut all_players = HashMap::new();
        // Fetch active players to get their visual properties (colors)
        // Scan a reasonable range of IDs (local + bots usually < 20)
        for i in 0..20 {
            let p = simulation.get_player_state(i);
            if p.id != -1 && p.is_alive != 0 {
                all_players.insert(p.id, p);
            }
        }
        // Ensure local player is included for color lookup even if dead?
        // If dead, they might not be on grid, but good to have.
        all_players.insert(player.id, player);

        // Ghost indicators are controlled by CLI arg now (always on by default)
        let show_indicators = args.show_move_indicators;

        terminal.draw(|f| {
            ui::render(f, &state, &player, &all_players, &simulation, args.grid_visible, camera_x, camera_y.round() as i32, show_indicators, &profile_stats, &config, &bg, show_inventory, inventory_index, args.energy_body_indicator);
        })?;
    }

    // Handle Simulation Complete - Submit Profile & Session Unlock
    let final_state = simulation.get_simulation_state();
    if final_state.simulation_state == 3 || final_state.portal_state == 2 { // Simulation Complete or Extract
        let final_player = simulation.get_player_state(final_state.local_player_id);

        let new_stats = ProfileStats {
            total_kills: profile_stats.as_ref().map(|s| s.total_kills).unwrap_or(0) + final_player.kills as i64,
            max_length: profile_stats.as_ref().map(|s| s.max_length).unwrap_or(0).max(final_player.body_length as i64),
            matches_played: profile_stats.as_ref().map(|s| s.matches_played).unwrap_or(0) + 1,
            rank_points: profile_stats.as_ref().map(|s| s.rank_points).unwrap_or(0) + final_player.score as i64 / 10,
        };

        if !menu.secret_key.is_empty() {
            // Disable raw mode so terminal output is readable
            crossterm::terminal::disable_raw_mode().unwrap_or_default();

            let simulation_hash = simulation.get_replay_hash();

            // 1. Unlock Session
            if let Err(e) = session::unlock_session(&menu.secret_key, &simulation_hash).await {
                eprintln!("Failed to unlock session: {}", e);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;

            // 2. Update Profile
            match profile::update_profile(&menu.secret_key, &new_stats).await {
                Ok(_) => {}, // Success
                Err(_) => {}, // Error
            }
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Re-enable raw mode before looping back to main menu
            crossterm::terminal::enable_raw_mode().unwrap_or_default();
        }
    }

    // End of app_loop iteration, clears terminal and resets to main menu
    let _ = terminal.clear();
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}

fn handle_key_event(
    key: KeyEvent,
    simulation: &NativeEngine,
    input_handler: &mut InputHandler,
    bot_count: i32,
) -> bool {
    match key.code {
        // Exit is handled directly in main loop now to break 'app_loop
        KeyCode::Char('q') | KeyCode::Char('Q')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            return true; // Still returns true, handled appropriately
        }

        // Restart
        KeyCode::Char('r') | KeyCode::Char('R') => {
            simulation.process_input(5, bot_count, 0);
            input_handler.reset();
        }

        // Arrow keys - accumulate for this frame
        KeyCode::Up => input_handler.handle_key_direction(0, -1),
        KeyCode::Down => input_handler.handle_key_direction(0, 1),
        KeyCode::Left => input_handler.handle_key_direction(-1, 0),
        KeyCode::Right => input_handler.handle_key_direction(1, 0),

        // WASD (A is reserved for focus)
        KeyCode::Char('w') | KeyCode::Char('W') => input_handler.handle_key_direction(0, -1),
        KeyCode::Char('s') | KeyCode::Char('S') => input_handler.handle_key_direction(0, 1),
        KeyCode::Char('d') | KeyCode::Char('D') => input_handler.handle_key_direction(1, 0),

        // Focus: A = towards head, Z = towards tail
        KeyCode::Char('a') | KeyCode::Char('A') => {
            let jump = if key.modifiers.contains(KeyModifiers::SHIFT) { 1 } else { 0 };
            simulation.process_input(2, jump, 0);
        }
        KeyCode::Char('z') | KeyCode::Char('Z') => {
            let jump = if key.modifiers.contains(KeyModifiers::SHIFT) { 1 } else { 0 };
            simulation.process_input(3, jump, 0);
        }

        // Fire weapon
        KeyCode::Char(' ') => simulation.process_input(1, 0, 0),

        // Dash/Slingshot
        KeyCode::Char('f') | KeyCode::Char('F') => simulation.process_input(4, 0, 0),

        // Autopilot Toggle
        KeyCode::Char('p') | KeyCode::Char('P') => simulation.process_input(9, 0, 0),

        // Pause / Resume
        KeyCode::Esc => simulation.process_input(7, 0, 0),

        // Weapon Attach: X (Left), C (Right)
        KeyCode::Char('x') | KeyCode::Char('X') => simulation.process_input(6, 0, 0), // Side 0 = Left
        KeyCode::Char('c') | KeyCode::Char('C') => simulation.process_input(6, 1, 0), // Side 1 = Right

        _ => {}
    }

    false
}