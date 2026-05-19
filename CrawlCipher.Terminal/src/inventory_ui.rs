use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::ffi::InventoryItem;

pub fn render_inventory(
    frame: &mut Frame,
    area: Rect,
    backpack: &[InventoryItem],
    selected_index: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" INVENTORY (BACKPACK) - [E/Enter] Equip R | [X] L | [C] R | [U] Unequip | [A/Z] Focus | [I/ESC] Close ")
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    let list_items: Vec<ListItem> = backpack
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let name_bytes = &item.asset_code;
            let name = String::from_utf8_lossy(name_bytes).trim_matches('\0').to_string();
            let style = if i == selected_index {
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

