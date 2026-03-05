use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};

use crate::ffi::InventoryItem;

#[derive(Clone, Copy, PartialEq)]
pub enum InventoryPanel {
    Backpack,
    Equipped,
}

pub fn render_inventory(
    frame: &mut Frame,
    area: Rect,
    backpack: &[InventoryItem],
    equipped: &[InventoryItem],
    selected_panel: InventoryPanel,
    selected_index: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" INVENTORY - [E]quip [U]nequip [M]Swap [TAB]Switch Panel [ESC]Close ")
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    // Center the popup
    let area = centered_rect(80, 80, area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let inner_area = area.inner(&ratatui::layout::Margin { vertical: 1, horizontal: 2 });

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_area);

    // Left: Backpack
    render_backpack(frame, chunks[0], backpack, selected_panel == InventoryPanel::Backpack, selected_index);

    // Right: Equipped
    render_equipped(frame, chunks[1], equipped, selected_panel == InventoryPanel::Equipped, selected_index);
}

fn render_backpack(frame: &mut Frame, area: Rect, items: &[InventoryItem], is_active: bool, selected_index: usize) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Backpack ")
        .border_style(if is_active { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Gray) });

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let name_bytes = &item.asset_code;
            let name = String::from_utf8_lossy(name_bytes).trim_matches('\0').to_string();
            let style = if is_active && i == selected_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            let content = format!("{}: {} (Dur: {})", i + 1, name, item.durability);
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(list_items).block(block);
    frame.render_widget(list, area);
}

fn render_equipped(frame: &mut Frame, area: Rect, items: &[InventoryItem], is_active: bool, selected_index: usize) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Equipped (Body Segments) ")
        .border_style(if is_active { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Gray) });

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let name_bytes = &item.asset_code;
            let name = String::from_utf8_lossy(name_bytes).trim_matches('\0').to_string();
            let label = if item.item_type == 0 && name.is_empty() {
                format!("Slot {}: (Empty)", i)
            } else {
                format!("Slot {}: {} (Dur: {})", i, name, item.durability)
            };

            let style = if is_active && i == selected_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            ListItem::new(label).style(style)
        })
        .collect();

     let list = List::new(list_items).block(block);
    frame.render_widget(list, area);
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
