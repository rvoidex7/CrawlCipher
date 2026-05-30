//! Input handling for CrawlCipher Terminal.
//! Captures keyboard events and forwards them to the Native Engine via FFI.
//! Only combo detection for 8-directional movement lives here.

use crate::ffi::{self, NativeEngine};

pub struct InputHandler {
    // Accumulates input for the current frame
    current_dx: i32,
    current_dy: i32,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            current_dx: 0,
            current_dy: 0,
        }
    }

    pub fn reset(&mut self) {
        self.current_dx = 0;
        self.current_dy = 0;
    }

    /// Accumulates directional input.
    /// Call this for every key event in the frame.
    pub fn handle_key_direction(&mut self, dx: i32, dy: i32) {
        if dx != 0 { self.current_dx = dx; }
        if dy != 0 { self.current_dy = dy; }
    }

    /// Resolves the accumulated input into a single direction command and sends it.
    /// Call this ONCE per simulation tick before update.
    pub fn resolve_and_send(&mut self, simulation: &NativeEngine) {
        if let Some(dir) = self.resolve_direction() {
            simulation.process_input(0, dir, 0);
        }
    }

    /// Resolves accumulated input without sending it to NativeEngine.
    /// Useful for UI/Menu navigation that needs the same diagonal chording logic.
    pub fn resolve_direction(&mut self) -> Option<i32> {
        if self.current_dx == 0 && self.current_dy == 0 {
            return None;
        }

        let dx = self.current_dx.clamp(-1, 1);
        let dy = self.current_dy.clamp(-1, 1);

        let dir = ffi::direction_from_delta(dx, dy);
        self.reset();
        dir
    }
}