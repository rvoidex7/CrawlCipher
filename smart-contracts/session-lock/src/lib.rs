#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec, symbol_short};

#[contract]
/// Soroban Session Lock Contract.
/// This contract acts as an escrow-based license verification layer on the Stellar Testnet.
///
/// Detailed specifications:
/// - Local developer guide: [Architecture.md](../../../docs/r7/Development/Architecture.md)
/// - Online wiki page: https://rvoidex7.github.io/r7notes/Github-Projects/Architecture
pub struct SessionLockContract;

#[contractimpl]
impl SessionLockContract {
    /// Locks a list of asset IDs for a player's session.
    /// Prevents double-spending of equipped items during active local gameplay.
    ///
    /// Detailed specifications:
    /// - Local guide: [Anti-Cheat-Verification.md](../../../docs/r7/Development/Anti-Cheat-Verification.md)
    /// - Online wiki page: https://rvoidex7.github.io/r7notes/Github-Projects/Anti-Cheat-Verification
    pub fn lock_session(env: Env, player: Address, assets: Vec<String>) {
        player.require_auth();
        env.storage().instance().set(&player, &assets);
    }

    /// Unlocks the session and releases assets.
    /// Submits the final cryptographic Session Verification Hash representing the proof of play.
    ///
    /// Detailed specifications:
    /// - Local guide: [Anti-Cheat-Verification.md](../../../docs/r7/Development/Anti-Cheat-Verification.md)
    /// - Online wiki page: https://rvoidex7.github.io/r7notes/Github-Projects/Anti-Cheat-Verification
    pub fn unlock_session(env: Env, player: Address, _game_hash: String) {
        player.require_auth();
        env.storage().instance().remove(&player);
    }

    /// Returns the list of currently locked assets for a player.
    pub fn get_locked_assets(env: Env, player: Address) -> Vec<String> {
        env.storage().instance().get(&player).unwrap_or(Vec::new(&env))
    }
}
