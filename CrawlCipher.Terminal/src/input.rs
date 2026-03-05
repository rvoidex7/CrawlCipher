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
        if self.current_dx == 0 && self.current_dy == 0 {
            return;
        }

        // Clamp to ensure valid range (-1 to 1)
        let dx = self.current_dx.clamp(-1, 1);
        let dy = self.current_dy.clamp(-1, 1);

        if let Some(dir) = ffi::direction_from_delta(dx, dy) {
            simulation.process_input(0, dir, 0);
        }
        
        // Reset after sending
        self.reset();
    }
}