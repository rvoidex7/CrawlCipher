#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec, symbol_short};

#[contract]
pub struct SessionLockContract;

#[contractimpl]
impl SessionLockContract {
    /// Locks a list of asset IDs for a player's session.
    pub fn lock_session(env: Env, player: Address, assets: Vec<String>) {
        player.require_auth();
        env.storage().instance().set(&player, &assets);
    }

    /// Unlocks the session and releases assets.
    /// In a real implementation, this would verify the game result hash.
    pub fn unlock_session(env: Env, player: Address, _game_hash: String) {
        player.require_auth();
        env.storage().instance().remove(&player);
    }

    /// Returns the list of currently locked assets for a player.
    pub fn get_locked_assets(env: Env, player: Address) -> Vec<String> {
        env.storage().instance().get(&player).unwrap_or(Vec::new(&env))
    }
}
