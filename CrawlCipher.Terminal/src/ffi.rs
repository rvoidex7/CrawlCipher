//! FFI bridge to the proprietary Native Engine shared library.
//! This module contains:
//! - repr(C) structs matching the C-ABI StructLayout(Sequential)
//! - NativeEngine wrapper using dynamic loading (libloading)

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::sync::Arc;

// ===== FFI Structs (must match Native ABI StructLayout(Sequential)) =====

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SimulationState {
    pub player_count: i32,
    pub grid_width: i32,
    pub grid_height: i32,
    pub local_player_id: i32,
    pub simulation_state: i32,
    pub timestamp: i64,
    pub enable_walls: i32, // 1 = true, 0 = false
    pub current_wave: i32,
    pub match_time_seconds: i32,
    pub portal_x: i32,
    pub portal_y: i32,
    pub portal_state: i32,
    pub extraction_countdown: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PlayerState {
    pub id: i32,
    pub x: i32,
    pub y: i32,
    pub energy: i32,
    pub bonus_energy: i32,
    pub max_energy: i32,
    pub score: i32,
    pub kills: i32,
    pub is_alive: i32,
    pub is_stunned: i32,
    pub is_idle: i32,
    pub is_autopilot: i32,
    pub focused_segment: i32,
    pub focused_x: i32,
    pub focused_y: i32,
    pub body_length: i32,
    pub current_direction: i32,
    pub last_direction: i32,
    pub last_action_status: i32, // Added for debug
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
    pub valid_moves_mask: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CellInfo {
    pub cell_type: i32,
    pub player_id: i32,
    pub extra_data: i32,
}

impl Default for CellInfo {
    fn default() -> Self {
        Self {
            cell_type: 0,
            player_id: -1,
            extra_data: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InventoryItem {
    pub id: [u8; 37],
    pub item_type: i32,
    pub asset_code: [u8; 16],
    pub durability: i32,
}

impl Default for InventoryItem {
    fn default() -> Self {
        Self {
            id: [0; 37],
            item_type: 0,
            asset_code: [0; 16],
            durability: 0,
        }
    }
}

// ===== Function Types for Dynamic Loading =====

type FnCreateGame = unsafe extern "C" fn(i64, *const i8, i32, i32, i32, i32, i32, i32, i32, i32, i32) -> *mut c_void;
type FnDestroyGame = unsafe extern "C" fn(*mut c_void);
type FnUpdate = unsafe extern "C" fn(*mut c_void);
type FnGetSimulationState = unsafe extern "C" fn(*mut c_void) -> SimulationState;
type FnProcessInput = unsafe extern "C" fn(*mut c_void, i32, i32, i32);
type FnGetPlayerState = unsafe extern "C" fn(*mut c_void, i32) -> PlayerState;
type FnGetGridCells = unsafe extern "C" fn(*mut c_void, *mut CellInfo, i32, i32, i32, i32, i32) -> i32;
type FnGetBackpack = unsafe extern "C" fn(*mut c_void, i32, *mut InventoryItem, i32) -> i32;
type FnGetEquippedItems = unsafe extern "C" fn(*mut c_void, i32, *mut InventoryItem, i32) -> i32;
type FnEquipItem = unsafe extern "C" fn(*mut c_void, i32, *const i8, i32, i32) -> i32;
type FnUnequipItem = unsafe extern "C" fn(*mut c_void, i32, i32) -> i32;
type FnSwapItems = unsafe extern "C" fn(*mut c_void, i32, i32, i32) -> i32;
type FnGetReplayHash = unsafe extern "C" fn(*mut c_void, *mut u8, i32);

// ===== Safe Rust Wrapper using libloading =====

pub struct NativeEngine {
    game_ptr: *mut c_void,
    _lib: Arc<Library>, // Keep library loaded
    // Function pointers
    destroy_fn: Symbol<'static, FnDestroyGame>,
    update_fn: Symbol<'static, FnUpdate>,
    get_gamestate_fn: Symbol<'static, FnGetSimulationState>,
    process_input_fn: Symbol<'static, FnProcessInput>,
    get_playerstate_fn: Symbol<'static, FnGetPlayerState>,
    get_gridcells_fn: Symbol<'static, FnGetGridCells>,
    get_backpack_fn: Symbol<'static, FnGetBackpack>,
    get_equipped_items_fn: Symbol<'static, FnGetEquippedItems>,
    equip_item_fn: Symbol<'static, FnEquipItem>,
    unequip_item_fn: Symbol<'static, FnUnequipItem>,
    swap_items_fn: Symbol<'static, FnSwapItems>,
    get_replay_hash_fn: Symbol<'static, FnGetReplayHash>,
}

impl NativeEngine {
    pub fn new(
        seed: i64,
        player_name: &str,
        grid_w: i32, grid_h: i32, food_count: i32, enable_walls: bool,
        max_energy: i32, energy_gain: i32, 
        turn_cost_45: i32, turn_cost_90: i32, turn_cost_sharp: i32
    ) -> Self {
        unsafe {
            #[cfg(target_os = "windows")]
            let lib_name = "CrawlCipher.Core.dll";
            #[cfg(target_os = "linux")]
            let lib_name = "libCrawlCipher.Core.so";
            #[cfg(target_os = "macos")]
            let lib_name = "libCrawlCipher.Core.dylib";

            let lib = Arc::new(Library::new(lib_name).expect(&format!("Failed to load {}", lib_name)));

            // Load symbols
            let create_game: Symbol<FnCreateGame> = lib.get(b"CreateGame").expect("Missing CreateGame");
            let destroy_game: Symbol<FnDestroyGame> = lib.get(b"DestroyGame").expect("Missing DestroyGame");
            let update: Symbol<FnUpdate> = lib.get(b"Update").expect("Missing Update");
            let get_gamestate: Symbol<FnGetSimulationState> = lib.get(b"GetSimulationState").expect("Missing GetSimulationState");
            let process_input: Symbol<FnProcessInput> = lib.get(b"ProcessInput").expect("Missing ProcessInput");
            let get_playerstate: Symbol<FnGetPlayerState> = lib.get(b"GetPlayerState").expect("Missing GetPlayerState");
            let get_gridcells: Symbol<FnGetGridCells> = lib.get(b"GetGridCells").expect("Missing GetGridCells");
            let get_backpack: Symbol<FnGetBackpack> = lib.get(b"GetBackpack").expect("Missing GetBackpack");
            let get_equipped_items: Symbol<FnGetEquippedItems> = lib.get(b"GetEquippedItems").expect("Missing GetEquippedItems");
            let equip_item: Symbol<FnEquipItem> = lib.get(b"EquipItemFFI").expect("Missing EquipItemFFI");
            let unequip_item: Symbol<FnUnequipItem> = lib.get(b"UnequipItemFFI").expect("Missing UnequipItemFFI");
            let swap_items: Symbol<FnSwapItems> = lib.get(b"SwapItemsFFI").expect("Missing SwapItemsFFI");
            let get_replay_hash: Symbol<FnGetReplayHash> = lib.get(b"GetReplayHash").expect("Missing GetReplayHash");

            let c_name = std::ffi::CString::new(player_name).unwrap();

            // Create game instance
            let game_ptr = create_game(
                seed,
                c_name.as_ptr(),
                grid_w, grid_h, food_count, if enable_walls { 1 } else { 0 },
                max_energy, energy_gain, turn_cost_45, turn_cost_90, turn_cost_sharp
            );
            assert!(!game_ptr.is_null(), "Failed to create Native game instance");

            // Extension of lifetime for symbols is safe because we hold Arc<Library>
            let destroy_fn = std::mem::transmute(destroy_game);
            let update_fn = std::mem::transmute(update);
            let get_gamestate_fn = std::mem::transmute(get_gamestate);
            let process_input_fn = std::mem::transmute(process_input);
            let get_playerstate_fn = std::mem::transmute(get_playerstate);
            let get_gridcells_fn = std::mem::transmute(get_gridcells);
            let get_backpack_fn = std::mem::transmute(get_backpack);
            let get_equipped_items_fn = std::mem::transmute(get_equipped_items);
            let equip_item_fn = std::mem::transmute(equip_item);
            let unequip_item_fn = std::mem::transmute(unequip_item);
            let swap_items_fn = std::mem::transmute(swap_items);
            let get_replay_hash_fn = std::mem::transmute(get_replay_hash);

            Self {
                game_ptr,
                _lib: lib,
                destroy_fn,
                update_fn,
                get_gamestate_fn,
                process_input_fn,
                get_playerstate_fn,
                get_gridcells_fn,
                get_backpack_fn,
                get_equipped_items_fn,
                equip_item_fn,
                unequip_item_fn,
                swap_items_fn,
                get_replay_hash_fn,
            }
        }
    }

    pub fn update(&self) {
        unsafe { (self.update_fn)(self.game_ptr) };
    }

    pub fn get_simulation_state(&self) -> SimulationState {
        unsafe { (self.get_gamestate_fn)(self.game_ptr) }
    }

    pub fn process_input(&self, input_type: i32, param1: i32, param2: i32) {
        unsafe { (self.process_input_fn)(self.game_ptr, input_type, param1, param2) };
    }

    pub fn get_player_state(&self, player_id: i32) -> PlayerState {
        unsafe { (self.get_playerstate_fn)(self.game_ptr, player_id) }
    }

    pub fn get_grid_cells(
        &self,
        buffer: &mut [CellInfo],
        view_x: i32,
        view_y: i32,
        view_width: i32,
        view_height: i32,
    ) -> i32 {
        unsafe {
            (self.get_gridcells_fn)(
                self.game_ptr,
                buffer.as_mut_ptr(),
                (buffer.len() * std::mem::size_of::<CellInfo>()) as i32,
                view_x,
                view_y,
                view_width,
                view_height,
            )
        }
    }

    pub fn get_backpack(&self, player_id: i32) -> Vec<InventoryItem> {
        let mut items = vec![InventoryItem::default(); 50];
        let count = unsafe { (self.get_backpack_fn)(self.game_ptr, player_id, items.as_mut_ptr(), 50) };
        if count >= 0 {
            items.truncate(count as usize);
            items
        } else {
            Vec::new()
        }
    }

    pub fn get_equipped_items(&self, player_id: i32) -> Vec<InventoryItem> {
        let mut items = vec![InventoryItem::default(); 50];
        let count = unsafe { (self.get_equipped_items_fn)(self.game_ptr, player_id, items.as_mut_ptr(), 50) };
        if count >= 0 {
            items.truncate(count as usize);
            items
        } else {
            Vec::new()
        }
    }

    pub fn equip_item(&self, player_id: i32, item_id: &str, segment_index: i32, side: i32) -> bool {
        let c_item_id = std::ffi::CString::new(item_id).unwrap();
        unsafe { (self.equip_item_fn)(self.game_ptr, player_id, c_item_id.as_ptr(), segment_index, side) != 0 }
    }

    pub fn unequip_item(&self, player_id: i32, segment_index: i32) -> bool {
        unsafe { (self.unequip_item_fn)(self.game_ptr, player_id, segment_index) != 0 }
    }

    pub fn swap_items(&self, player_id: i32, idx_a: i32, idx_b: i32) -> bool {
        unsafe { (self.swap_items_fn)(self.game_ptr, player_id, idx_a, idx_b) != 0 }
    }

    pub fn get_replay_hash(&self) -> String {
        let mut buffer = vec![0u8; 128]; // SHA256 hex is 64 chars, + slack
        unsafe { (self.get_replay_hash_fn)(self.game_ptr, buffer.as_mut_ptr(), 128) };

        let end = buffer.iter().position(|&x| x == 0).unwrap_or(buffer.len());
        String::from_utf8_lossy(&buffer[0..end]).to_string()
    }
}

impl Drop for NativeEngine {
    fn drop(&mut self) {
        if !self.game_ptr.is_null() {
            unsafe { (self.destroy_fn)(self.game_ptr) };
        }
    }
}

unsafe impl Send for NativeEngine {}
unsafe impl Sync for NativeEngine {}

pub fn direction_from_delta(dx: i32, dy: i32) -> Option<i32> {
    match (dx, dy) {
        (0, -1) => Some(0),  // North
        (1, -1) => Some(1),  // NorthEast
        (1, 0) => Some(2),   // East
        (1, 1) => Some(3),   // SouthEast
        (0, 1) => Some(4),   // South
        (-1, 1) => Some(5),  // SouthWest
        (-1, 0) => Some(6),  // West
        (-1, -1) => Some(7), // NorthWest
        _ => None,
    }
}