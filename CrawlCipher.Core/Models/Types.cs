using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace CrawlCipher.Core
{
    // ===== Enums =====

    /// <summary>
    /// Represents the 8-directional movement grid.
    /// Values wind clockwise starting from North (0) to NorthWest (7).
    /// Used for angular turn calculations and bullet velocity deltas.
    /// </summary>
    public enum Direction : int
    {
        North = 0, NorthEast = 1, East = 2, SouthEast = 3,
        South = 4, SouthWest = 5, West = 6, NorthWest = 7
    }

    /// <summary>
    /// The primary state machine of the simulation loop.
    /// Coordinates UI renders and halts calculations when Paused/GameOver.
    /// </summary>
    public enum SimulationStateType : int
    {
        Menu = 0, Playing = 1, Paused = 2, GameOver = 3
    }

    /// <summary>
    /// Represents the physical layout of grid cells in the game board.
    /// Used directly in FFI viewports to instruct the TUI renderer.
    /// </summary>
    public enum CellType : int
    {
        Empty = 0, SnakeHead = 1, SnakeBody = 2, SnakeTail = 3,
        Wall = 4, Food = 5, Bullet = 6, SnakeBodyFocused = 7,
        Weapon = 8, // New cell type for physical weapons
        GhostSegment = 9, // For Strike trail visualization
        StrikePreview = 10, // ++ Indicator for Strike head destination
        Snail = 11, // @- @/ etc.
        StrikeBodyPreview = 12 // + body preview
    }

    /// <summary>
    /// Categorizes weapons and passive modules mounted on body segments.
    /// Types 1-3 fire projectiles; types 4-6 modify surrounding cells.
    /// </summary>
    public enum WeaponType : int
    {
        None = 0, Pistol = 1, Rifle = 2, Laser = 3,
        Amplifier = 4, Prism = 5, Collector = 6
    }

    /// <summary>
    /// Specifies which side of a body segment an item is physically mounted.
    /// Determines the perpendicular firing trajectory vector (Left = -90°, Right = +90°).
    /// </summary>
    public enum WeaponSide : int
    {
        Left = 0, Right = 1
    }

    /// <summary>
    /// Distinguishes between interactive equipment categories.
    /// </summary>
    public enum ItemType : int
    {
        Weapon = 0,
        Module = 1,
        Consumable = 2
    }

    /// <summary>
    /// Represents a single item stored in the player's off-grid inventory (backpack).
    /// </summary>
    public class InventoryItem
    {
        public string Id = "";
        public ItemType Type;
        public string AssetCode = "";
        public int Durability;

        public InventoryItem Clone() => new InventoryItem { Id = Id, Type = Type, AssetCode = AssetCode, Durability = Durability };
    }

    // ===== FFI Structs (must match Rust #[repr(C)] exactly) =====
    // For more details on struct alignments and padding constraints:
    // See Local Guide: [Memory-and-FFI-Bridge.md](../docs/r7/Development/Memory-and-FFI-Bridge.md#2-abi-struct-alignments--padding)
    // See Online Page: https://rvoidex7.github.io/r7notes/Github-Projects/Memory-and-FFI-Bridge

    /// <summary>
    /// Transmits global game engine settings and state metrics across the FFI.
    /// Marshalled with LayoutKind.Sequential for exact 4/8-byte native alignment.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct SimulationStateFFI
    {
        public int PlayerCount;
        public int GridWidth;
        public int GridHeight;
        public int LocalPlayerId;
        public int State;       // SimulationStateType as int
        public long Timestamp;  // Aligned to 8-byte boundary automatically
        public int EnableWalls; // Marshalled as 1 = true, 0 = false to ensure consistent byte padding
        public int CurrentWave;
        public int MatchTimeSeconds;
        public int PortalX;
        public int PortalY;
        public int PortalState; // 0=Inactive, 1=Active, 2=Extracting
        public int ExtractionCountdown;
    }

    /// <summary>
    /// Transmits specific player and snake telemetry across the FFI.
    /// Packed sequentially to align perfectly with Rust's PlayerState representation.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct PlayerStateFFI
    {
        public int Id;
        public int X;
        public int Y;
        public int Energy;
        public int BonusEnergy;
        public int MaxEnergy;
        public int Score;
        public int Kills;
        public int IsAlive;      // 1 = true, 0 = false (consistent size)
        public int IsStunned;    // 1 = true, 0 = false (consistent size)
        public int IsIdle;       // 1 = true, 0 = false (consistent size)
        public int IsAutopilot;  // 1 = true, 0 = false (consistent size)
        public int FocusedSegment;
        public int FocusedX;
        public int FocusedY;
        public int BodyLength;
        public int CurrentDirection;
        public int LastDirection;
        public int LastActionStatus; // Heuristic status: 0=None, 1=Attached, 2=NoEnergy, 3=NoSpace
        public byte ColorR;
        public byte ColorG;
        public byte ColorB;
        public byte ValidMovesMask; // Bitmask for valid directional headings (bits 0-7)
    }

    /// <summary>
    /// Information packet describing a single grid cell coordinate.
    /// Fills the viewport buffer allocated by the Rust TUI.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public struct CellInfoFFI
    {
        public int CellType;
        public int PlayerId;
        public int ExtraData; // Orientation index or visual subtype data
    }

    /// <summary>
    /// Fixed-size unmanaged representation of an InventoryItem.
    /// Uses inline byte buffers to avoid allocating managed garbage-collected strings across the FFI.
    /// </summary>
    [StructLayout(LayoutKind.Sequential)]
    public unsafe struct InventoryItemFFI
    {
        public fixed byte Id[37]; // 36-byte UUID + null terminator
        public int Type;
        public fixed byte AssetCode[16]; // Zero-padded ASCII representation
        public int Durability;
    }

    // ===== Direction Helper =====

    /// <summary>
    /// Utility class for translating directions to grid coordinate offsets and calculating angular turn costs.
    ///
    /// See Local Guide: [Energy-and-Movement.md](../docs/r7/Wiki/Energy-and-Movement.md#2-energy-costs-of-movement)
    /// See Online Page: https://rvoidex7.github.io/r7notes/Github-Projects/Energy-and-Movement
    /// </summary>
    public static class DirectionHelper
    {
        /// <summary>
        /// Maps each direction (0-7) to a coordinate delta (dx, dy).
        /// Winding clockwise starting North (0, -1) down to NorthWest (-1, -1).
        /// </summary>
        private static readonly (int dx, int dy)[] Deltas =
        {
            (0, -1),   // North
            (1, -1),   // NorthEast
            (1, 0),    // East
            (1, 1),    // SouthEast
            (0, 1),    // South
            (-1, 1),   // SouthWest
            (-1, 0),   // West
            (-1, -1),  // NorthWest
        };

        /// <summary>
        /// Translates a Direction enum to a 2D coordinate movement delta.
        /// </summary>
        public static (int dx, int dy) ToDelta(Direction dir) => Deltas[(int)dir];

        /// <summary>
        /// Calculates the energy cost for a heading change between two directions.
        /// Uses modular arithmetic to find the shortest angular distance on the 8-directional wind circle.
        ///
        /// Math Details:
        /// - Winding difference = Abs(to - from)
        /// - Shortest path turn amount = Min(diff, 8 - diff) -> ranges from 0 (straight) to 4 (180-deg reversal).
        /// </summary>
        public static byte CalculateTurnCost(Direction from, Direction to, byte cost45, byte cost90, byte costSharp)
        {
            int diff = Math.Abs((int)to - (int)from);
            int turnAmount = Math.Min(diff, 8 - diff);

            return turnAmount switch
            {
                0 => 0,
                1 => cost45,    // 45° shift (1 step)
                2 => cost90,    // 90° shift (2 steps)
                _ => costSharp, // >90° sharp turn (3 or 4 steps: 135° or 180° reversal)
            };
        }
    }

    // ===== Game Configuration =====

    public class GameConfig
    {
        public int GridWidth = 87;
        public int GridHeight = 50;
        public int MaxEnergy = 7;
        public int EnergyGainPerMove = 1;
        public byte Turn45Cost = 2;
        public byte Turn90Cost = 5;
        public byte TurnSharpCost = 12; // >90° cost (User requested 12 to make it impossible with default 7 max energy)
        public int StunDurationTicks = 20; // 2s at 10 FPS
        public int BulletSpeedMultiplier = 3;
        public int BotCount = 0;
        public bool EnableWalls = true;
        public int FoodCount = 10;
        public int IdleEnergyGainRate = 2;

        // Snail settings
        public int SnailCount = 2; // Default
        public int SnailSpeedDivisor = 2; // Higher is slower (e.g. 2 = every 2 ticks)
        public int SnailScoreReward = 50;
        public int SnailEnergyReward = 5;
        public bool BonusEnergyMechanic = true; // Enable overflow/bonus mechanics

        public ExpeditionConfig Expedition = new ExpeditionConfig();

        // Strike settings
        public int StrikeStartCorner = 2; // Default
        public int StrikeEndCornerOffset = 4; // Default
        public int StrikeMaxSavings = 87; // GridWidth default
        public int InitialSnakeLength = 3; // Default
        public bool ShowGhostTrail = false; // Debug
        public bool ShowStrikeBodyPreview = false; // New request
        public bool UnlimitedItems = false; // Add unlimited items for sandbox
    }

    // Weapon stat lookup
    public static class WeaponStats
    {
        public static (int ammo, int range) Get(WeaponType type) => type switch
        {
            WeaponType.Pistol => (12, 10),
            WeaponType.Rifle => (30, 20),
            WeaponType.Laser => (5, 30),
            WeaponType.Amplifier => (0, 0),
            WeaponType.Prism => (0, 0),
            WeaponType.Collector => (0, 0),
            _ => (0, 0),
        };
    }

    // ===== Expedition Types =====

    public struct BossWaveConfig
    {
        public int WaveNumber;
        public int TriggerTimeSeconds;
        public string BossType;
        public float Multiplier;
        public int AdditionalBots;
        public float BotSpeedMultiplier;
    }

    public struct ScoringConfig
    {
        public int FoodScore;
        public int SnailScore;
        public int BotKillBase;
        public int BotKillPerSegment;
        public int BossKillScore;
        public int SurvivalPerSecond;
    }

    public struct DeathPenaltyConfig
    {
        public float ScoreRetentionPercent;
        public bool ApplyMultiplierOnDeath;
        public float ItemDurabilityLossPercent;
    }

    public class ExpeditionConfig
    {
        public int InitialBotCount = 2;
        public List<BossWaveConfig> BossWaves = new();
        public ScoringConfig Scoring = new ScoringConfig { FoodScore = 10, SnailScore = 50, BotKillBase = 100, BotKillPerSegment = 10, BossKillScore = 1000, SurvivalPerSecond = 2 };
        public DeathPenaltyConfig DeathPenalty = new DeathPenaltyConfig { ScoreRetentionPercent = 70, ApplyMultiplierOnDeath = false, ItemDurabilityLossPercent = 30 };
    }

    public class ExitPortal
    {
        public int X;
        public int Y;
        public bool IsActive;
        public int ExtractionCountdown; // -1 = not extracting
    }

    public struct InputFrame
    {
        public long Tick;
        public int InputType;
        public int Param1;
        public int Param2;
    }
}
