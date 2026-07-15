#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

/// A player's active session lock: the equipped assets held in escrow plus the ledger
/// sequence at lock time. The lock-time sequence is the anchor for the "seed committed
/// before play" guarantee — the session seed is derived from the ledger closed right after it.
#[contracttype]
#[derive(Clone)]
pub struct LockRecord {
    pub assets: Vec<String>,
    pub lock_seq: u32,
}

/// A completed session's proof of play: the submitted replay hash plus the ledger
/// sequences that bounded the session (lock time and unlock time). Persisted so a
/// verifier can later re-derive the seed and replay window without trusting the client.
#[contracttype]
#[derive(Clone)]
pub struct ProofRecord {
    pub game_hash: String,
    pub lock_seq: u32,
    pub unlock_seq: u32,
}

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
    /// Also records the ledger sequence at lock time so the session seed (derived from
    /// the ledger closed right after this transaction) is verifiably committed before play.
    ///
    /// Detailed specifications:
    /// - Local guide: [Anti-Cheat-Verification.md](../../../docs/r7/Development/Anti-Cheat-Verification.md)
    /// - Online wiki page: https://rvoidex7.github.io/r7notes/Github-Projects/Anti-Cheat-Verification
    pub fn lock_session(env: Env, player: Address, assets: Vec<String>) {
        player.require_auth();
        let lock_seq = env.ledger().sequence();
        let record = LockRecord { assets, lock_seq };
        env.storage().instance().set(&player, &record);
    }

    /// Unlocks the session and releases assets.
    /// Persists the final cryptographic Session Verification Hash (the proof of play),
    /// together with the lock/unlock ledger sequences, in persistent storage before the
    /// lock record is removed. One record per player (their last session) is kept.
    ///
    /// Detailed specifications:
    /// - Local guide: [Anti-Cheat-Verification.md](../../../docs/r7/Development/Anti-Cheat-Verification.md)
    /// - Online wiki page: https://rvoidex7.github.io/r7notes/Github-Projects/Anti-Cheat-Verification
    pub fn unlock_session(env: Env, player: Address, game_hash: String) {
        player.require_auth();

        let lock: Option<LockRecord> = env.storage().instance().get(&player);
        let lock_seq = lock.map(|r| r.lock_seq).unwrap_or(0);
        let unlock_seq = env.ledger().sequence();

        let proof = ProofRecord {
            game_hash,
            lock_seq,
            unlock_seq,
        };
        env.storage().persistent().set(&player, &proof);

        env.storage().instance().remove(&player);
    }

    /// Returns the list of currently locked assets for a player.
    pub fn get_locked_assets(env: Env, player: Address) -> Vec<String> {
        let record: Option<LockRecord> = env.storage().instance().get(&player);
        record.map(|r| r.assets).unwrap_or(Vec::new(&env))
    }

    /// Returns the ledger sequence at which the player's active session was locked, if any.
    /// The TUI reads this to derive the session seed from the first ledger closed afterward.
    pub fn get_lock_seq(env: Env, player: Address) -> Option<u32> {
        let record: Option<LockRecord> = env.storage().instance().get(&player);
        record.map(|r| r.lock_seq)
    }

    /// Returns the persisted proof of the player's last completed session, if any.
    pub fn get_proof(env: Env, player: Address) -> Option<ProofRecord> {
        env.storage().persistent().get(&player)
    }
}
