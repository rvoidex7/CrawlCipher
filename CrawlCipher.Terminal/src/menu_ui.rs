use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::background;

pub enum MenuState {
    MainMenu,
    CredentialsInput,
    KeyInfo,
    Settings,
    CustomBackgroundInput,
    Manual, // New State for instructions
}

pub struct MenuUI {
    pub secret_key: String,
    pub nickname: String,
    pub state: MenuState,
    pub main_selection: usize,
    pub cred_stage: usize,
    pub settings_selection: usize,
    pub embedded_bgs: Vec<String>,
    pub selected_bg_index: usize,
    pub error_msg: Option<String>,
    pub custom_bg_path: String, // New Field
    pub custom_bg_loaded: bool, // New Field
}

impl MenuUI {
    pub fn new() -> Self {
        Self {
            secret_key: String::new(),
            nickname: "Pilot".to_string(),
            state: MenuState::MainMenu,
            main_selection: 0,
            cred_stage: 0,
            settings_selection: 0,
            embedded_bgs: background::list_embedded_backgrounds(),
            selected_bg_index: 0,
            error_msg: None,
            custom_bg_path: String::new(),
            custom_bg_loaded: false,
        }
    }
}

pub fn render_menu(frame: &mut Frame, area: Rect, ui: &MenuUI) {
    // Background
    let bg_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::DarkGray));
    frame.render_widget(bg_block, area);

    match ui.state {
        MenuState::MainMenu => render_main_menu(frame, area, ui),
        MenuState::CredentialsInput => render_credentials_input(frame, area, ui),
        MenuState::KeyInfo => render_key_info(frame, area),
        MenuState::Settings => render_settings(frame, area, ui),
        MenuState::CustomBackgroundInput => render_custom_bg_input(frame, area, ui),
        MenuState::Manual => render_manual(frame, area),
    }
}

fn render_main_menu(frame: &mut Frame, area: Rect, ui: &MenuUI) {
    let area = centered_rect(60, 80, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(7),    // Options
            Constraint::Length(3), // Status
        ])
        .split(area);

    // Title
    let title = Paragraph::new("S I M U L A T I O N  //  S T E L L A R  L I N K")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    frame.render_widget(title, chunks[0]);

    // Options
    let options = vec![
        " [ INITIATE PROTOCOL ] ",
        " [ ENTER CREDENTIALS ] ",
        " [ GHOST PROTOCOL ] ", // Offline Mode
        " [ ACQUIRE ACCESS KEY ] ",
        " [ TERMINAL MANUAL ] ",
        " [ SYSTEM SETTINGS ] ",
        " [ ABORT MISSION ] "
    ];

    let mut spans = Vec::new();
    for (i, opt) in options.iter().enumerate() {
        if i == ui.main_selection {
            spans.push(Line::from(Span::styled(*opt, Style::default().fg(Color::Black).bg(Color::Cyan))));
        } else {
            spans.push(Line::from(Span::styled(*opt, Style::default().fg(Color::Cyan))));
        }
    }

    let opts_block = Block::default().borders(Borders::NONE);
    frame.render_widget(Paragraph::new(spans).alignment(Alignment::Center).block(opts_block), chunks[1]);

    // Status Footer
    let bg_name = if ui.custom_bg_loaded {
        "CUSTOM FILE"
    } else if ui.selected_bg_index < ui.embedded_bgs.len() {
        &ui.embedded_bgs[ui.selected_bg_index]
    } else {
        "NONE"
    };

    let status_text = if !ui.secret_key.is_empty() {
        format!("PILOT: {} | BG: {}", ui.nickname.to_uppercase(), bg_name)
    } else {
        format!("NO CREDENTIALS | BG: {}", bg_name)
    };

    let status_color = if !ui.secret_key.is_empty() { Color::Green } else { Color::Yellow };
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(status_color))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::TOP));
    frame.render_widget(status, chunks[2]);
}

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
    let area = centered_rect(60, 60, area); // Taller for more options
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
    // Add "None" option (index len)
    // Add "Load Custom File" option (index len + 1)

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
