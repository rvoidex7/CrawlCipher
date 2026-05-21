use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::ffi::InventoryItem;

pub struct GroupedItem {
    pub asset_code: String,
    pub count: usize,
    pub first_id: String,
    pub durability: i32,
}

pub fn group_backpack(backpack: &[InventoryItem]) -> Vec<GroupedItem> {
    let mut grouped: Vec<GroupedItem> = Vec::new();
    for item in backpack {
        let name = String::from_utf8_lossy(&item.asset_code).trim_matches('\0').to_string();
        if let Some(existing) = grouped.iter_mut().find(|g| g.asset_code == name) {
            existing.count += 1;
        } else {
            grouped.push(GroupedItem {
                asset_code: name,
                count: 1,
                first_id: String::from_utf8_lossy(&item.id).trim_matches('\0').to_string(),
                durability: item.durability,
            });
        }
    }
    grouped
}

pub fn render_inventory(
    frame: &mut Frame,
    area: Rect,
    grouped_backpack: &[GroupedItem],
    selected_index: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" INVENTORY (BACKPACK) - [E/Enter] Equip R | [X] L | [C] R | [U] Unequip | [A/Z] Focus | [I/ESC] Close ")
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    let list_items: Vec<ListItem> = grouped_backpack
        .iter()
        .enumerate()
        .map(|(i, group)| {
            let style = if i == selected_index {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            let content = format!("{}: {} (x{}) - (Dur: {})", i + 1, group.asset_code, group.count, group.durability);
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(list_items).block(block);
    frame.render_widget(list, area);
}

