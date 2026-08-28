using System;
using System.Collections.Generic;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using CrawlCipher.Core.Rng;

namespace CrawlCipher.Core
{
    public partial class GameEngine
    {
        /// <summary>
        /// Engine build identifier committed into every proof hash (see <see cref="GetReplayHash"/>).
        /// Bump this whenever a change could alter deterministic simulation output (RNG, physics,
        /// config shape) so old proofs are unambiguously tied to the engine version that produced them.
        /// </summary>
        public const string EngineVersion = "core-0.3.0";

        private SimulationStateType _state;
        private readonly List<Player> _players = new();
        private readonly List<Bullet> _bullets = new();
        private readonly List<(int X, int Y)> _foods = new();
        private readonly List<Snail> _snails = new();
        private readonly HashSet<(int X, int Y)> _internalWalls = new();
        private readonly GameConfig _config;
        private int _localPlayerId;
        private readonly DeterministicRng _rng; // Only source of randomness (xoshiro256** + splitmix64, see Rng/DeterministicRng.cs)
        private long _seed;
        private long _currentTick;
        private readonly List<InputFrame> _inputLog = new();

        // Expedition State
        private int _currentWave;
        private float _currentMultiplier;
        private ExitPortal _exitPortal = new ExitPortal();

        private static readonly (byte r, byte g, byte b)[] PlayerColors =
        {
            (0, 255, 0),     // Green (local)
            (0, 255, 255),   // Cyan
            (255, 255, 0),   // Yellow
            (255, 0, 255),   // Magenta
            (255, 100, 100), // Light Red
            (100, 100, 255), // Light Blue
        };

        public GameEngine(GameConfig config, long seed, string localPlayerName)
        {
            _config = config;
            _state = SimulationStateType.Menu;
            _seed = seed;
            _rng = new DeterministicRng(seed); // Deterministic init, full 64-bit seed (no truncation)
            _localPlayerName = localPlayerName;
        }

        private string _localPlayerName;
        private string _gameMode = "Expedition";
        private string _puzzleId = "";

        public void SetGameMode(string mode, string puzzleId)
        {
            _gameMode = mode;
            _puzzleId = puzzleId;
        }

        // ===== Configuration =====

        public void Configure(int key, int value)
        {
            switch (key)
            {
                case 0: _config.GridWidth = Math.Max(20, value); break;
                case 1: _config.GridHeight = Math.Max(15, value); break;
                case 2: _config.MaxEnergy = Math.Max(3, value); break;
                case 3: _config.EnableWalls = value != 0; break;
                case 4: _config.FoodCount = Math.Max(1, value); break;
                // Strike & Length Configuration
                case 9: _config.StrikeStartCorner = Math.Max(0, value); break;
                case 10: _config.StrikeEndCornerOffset = Math.Max(1, value); break;
                case 11: _config.StrikeMaxSavings = Math.Max(1, value); break;
                case 12: _config.InitialSnakeLength = Math.Max(3, value); break;
                case 13: _config.ShowGhostTrail = value != 0; break;
                case 14: _config.SnailSpeedDivisor = Math.Max(1, value); break;
                case 15: _config.SnailScoreReward = Math.Max(1, value); break;
                case 16: _config.SnailEnergyReward = Math.Max(1, value); break;
                case 17: _config.BonusEnergyMechanic = value != 0; break;
                case 18: _config.SnailCount = Math.Max(0, value); break;
                case 19: _config.ShowStrikeBodyPreview = value != 0; break;
                case 21: _config.IdleEnergyGainRate = Math.Max(0, value); break;
                case 22: _config.UnlimitedItems = value != 0; break;
            }
        }

        // ===== Game Lifecycle =====

        public void StartGame(int botCount)
        {
            _players.Clear();
            _bullets.Clear();
            _foods.Clear();
            _snails.Clear();
            _internalWalls.Clear();
            _currentTick = 0;

            if (_gameMode == "Puzzle")
            {
                InitPuzzle(_puzzleId);
                _state = SimulationStateType.Playing;
                return;
            }

            // Expedition Init
            _currentWave = 0;
            _currentMultiplier = 1.0f;
            _exitPortal = new ExitPortal { X = _config.GridWidth / 2, Y = 5, IsActive = false, ExtractionCountdown = -1 };

            if (_config.Expedition.BossWaves.Count == 0) {
                 _config.Expedition.BossWaves.Add(new BossWaveConfig { WaveNumber = 1, TriggerTimeSeconds = 60, BossType = "Megaagent", Multiplier = 1.5f, AdditionalBots = 1, BotSpeedMultiplier = 1.0f });
                 _config.Expedition.BossWaves.Add(new BossWaveConfig { WaveNumber = 2, TriggerTimeSeconds = 120, BossType = "Twinagents", Multiplier = 2.0f, AdditionalBots = 2, BotSpeedMultiplier = 1.2f });
            }

            // Local player at center
            var local = new Player(0, _localPlayerName,
                _config.GridWidth / 2, _config.GridHeight / 2,
                PlayerColors[0].r, PlayerColors[0].g, PlayerColors[0].b,
                _config.MaxEnergy);

            // Extend initial body length
            int extraSegments = Math.Max(0, _config.InitialSnakeLength - 3);
            for (int i = 0; i < extraSegments; i++)
            {
                var tail = local.Body[local.Body.Count - 1];
                local.Body.Add(new SnakeSegment(tail.X - 1, tail.Y)); // Assuming East start
                local.BodyWeapons.Add(WeaponData.None);
            }

            // Give local player a starting pistol on head
            local.BodyWeapons[0] = new WeaponData { Type = WeaponType.Pistol, Ammo = WeaponStats.Get(WeaponType.Pistol).ammo, Side = WeaponSide.Right, ItemId = Guid.NewGuid().ToString() };

            // Add test inventory items
            AddTestItems(local);

            _players.Add(local);
            _localPlayerId = 0;

            // Bots
            for (int i = 0; i < botCount; i++)
            {
                var (x, y) = GetRandomSpawnPosition();
                var c = PlayerColors[(i + 1) % PlayerColors.Length];
                var bot = new Player(i + 1, $"Bot{i + 1}", x, y, c.r, c.g, c.b, _config.MaxEnergy);
                bot.IsBot = true;
                bot.BodyWeapons[0] = new WeaponData { Type = WeaponType.Pistol, Ammo = 12, Side = WeaponSide.Right };
                _players.Add(bot);
            }

            SpawnFood();
            _state = SimulationStateType.Playing;
        }

        private void InitPuzzle(string puzzleId)
        {
            _currentWave = 0;
            _currentMultiplier = 1.0f;
            _exitPortal = new ExitPortal { IsActive = false, ExtractionCountdown = -1 };

            // Default Puzzle Settings
            _config.GridWidth = 20;
            _config.GridHeight = 15;
            _config.EnableWalls = true;
            _config.InitialSnakeLength = 3;
            _config.FoodCount = 1;

            var local = new Player(0, _localPlayerName,
                _config.GridWidth / 2, _config.GridHeight / 2,
                PlayerColors[0].r, PlayerColors[0].g, PlayerColors[0].b,
                _config.MaxEnergy);
            
            // Hardcode test items
            AddTestItems(local);

            if (puzzleId == "The Narrow Path")
            {
                _config.GridWidth = 10;
                _config.GridHeight = 20;
                local = new Player(0, _localPlayerName, 5, 18, PlayerColors[0].r, PlayerColors[0].g, PlayerColors[0].b, _config.MaxEnergy);
                _foods.Add((5, 2));

                // Set walls forming a narrow corridor
                // Let's create a zigzag corridor from y=17 down to y=4
                for (int y = 4; y <= 16; y++)
                {
                    if (y % 4 == 0) {
                        // Wall blocks left side
                        for (int x = 1; x <= 6; x++) _internalWalls.Add((x, y));
                    } else if (y % 4 == 2) {
                        // Wall blocks right side
                        for (int x = 3; x <= 8; x++) _internalWalls.Add((x, y));
                    }
                }
            }
            else if (puzzleId == "Laser Gate")
            {
                _config.GridWidth = 20;
                _config.GridHeight = 20;
                local = new Player(0, _localPlayerName, 10, 18, PlayerColors[0].r, PlayerColors[0].g, PlayerColors[0].b, _config.MaxEnergy);
                _foods.Add((10, 2));
                // Needs Laser
                local.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, Durability = 100, AssetCode = "LASER_1" });
                
                // Add a solid wall across the room at y=10, with one block missing?
                // Or a solid wall that requires a laser to shoot through (since laser destroys bullets... wait, we need laser to break walls?)
                // Since laser doesn't break walls yet, let's just make a maze requiring laser to kill snails? No, user wants objects.
                // Let's just create a small maze.
                for (int x = 1; x <= 18; x++)
                {
                    if (x != 10) _internalWalls.Add((x, 10)); // Gap at x=10
                }
                for (int y = 11; y <= 16; y++)
                {
                    if (y % 2 == 0) {
                        _internalWalls.Add((9, y));
                        _internalWalls.Add((11, y));
                    }
                }
            }
            else if (puzzleId == "Prism Chamber")
            {
                _config.GridWidth = 30;
                _config.GridHeight = 20;
                local = new Player(0, _localPlayerName, 15, 18, PlayerColors[0].r, PlayerColors[0].g, PlayerColors[0].b, _config.MaxEnergy);
                _foods.Add((15, 2));
                // Needs Prism
                local.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, Durability = -1, AssetCode = "PRISM_1" });
                local.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, Durability = -1, AssetCode = "AMP_1" });
                local.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, Durability = 100, AssetCode = "LASER_1" });
            }
            else
            {
                // Fallback
                _foods.Add((_config.GridWidth / 2, _config.GridHeight / 4));
            }

            _players.Add(local);
            _localPlayerId = 0;
        }

        /// <summary>
        /// Advances the core game engine state by one discrete simulation tick.
        /// Executes stuns, bot AI steering, wave triggers, snake movements, sub-ticked projectiles, 
        /// snail updates, food collection, and checks winning/losing conditions.
        /// 
        /// See Local Guide: [Gameplay.md](../docs/r7/Wiki/Gameplay.md#the-core-loop)
        /// See Online Page: https://rvoidex7.github.io/r7notes/Github-Projects/Gameplay
        /// </summary>
        public void Tick()
        {
            // Halt simulation if not actively in the Playing state
            if (_state != SimulationStateType.Playing) return;
            _currentTick++;

            // 1. Update Player/Bot Stuns
            // If the tick threshold is reached, restore mobility to the player.
            foreach (var p in _players)
            {
                if (p.IsStunned && _currentTick >= p.StunEndTick)
                {
                    p.IsStunned = false;
                    p.StunEndTick = -1;
                }
            }

            // 2. Process Bot AI Decision Matrix Heuristics
            // Iterates through all active bots to decide their steering paths and weapon firing vectors.
            foreach (var p in _players.Where(p => p.IsAlive && !p.IsStunned && p.IsBot))
                UpdateBot(p);

            // 3. Process Expedition Wave Spawns
            // Triggers bosses, updates score multipliers, and spawns enemy bots based on time elapsed.
            int seconds = (int)(_currentTick / 10);
            foreach(var wave in _config.Expedition.BossWaves) {
                if (wave.WaveNumber > _currentWave && seconds >= wave.TriggerTimeSeconds) {
                    _currentWave = wave.WaveNumber;
                    _currentMultiplier = wave.Multiplier;
                    SpawnBoss(wave);
                    
                    // Spawn extra bots configured for this specific wave
                    for(int i=0; i<wave.AdditionalBots; i++) {
                        var (bx, by) = GetRandomSpawnPosition();
                        var bot = new Player(_players.Count, $"BotW{wave.WaveNumber}-{i}", bx, by, 255, 0, 0, _config.MaxEnergy);
                        bot.IsBot = true;
                        bot.BodyWeapons[0] = new WeaponData { Type = WeaponType.Pistol, Ammo = 999, Side = WeaponSide.Right, ItemId = Guid.NewGuid().ToString() };
                        _players.Add(bot);
                    }

                    // Activate extraction portal when the boss/combat wave goals are initiated
                    if (wave.WaveNumber >= 1) _exitPortal.IsActive = true;
                }
            }

            // 4. Resolve Player/Bot Spatial Movement and Energy Consumption
            foreach (var p in _players.Where(p => p.IsAlive && !p.IsStunned))
            {
                // Check if the player is standing inside the active Exit Portal coordinates
                if (p.Id == _localPlayerId && _exitPortal.IsActive) {
                    if (p.Body[0].X == _exitPortal.X && p.Body[0].Y == _exitPortal.Y) {
                        // Start/maintain extraction countdown (30 ticks = 3 seconds at 10Hz)
                        if (_exitPortal.ExtractionCountdown < 0) _exitPortal.ExtractionCountdown = 30;
                    } else {
                        // Interrupted; reset countdown
                        _exitPortal.ExtractionCountdown = -1;
                    }
                }

                // Execute move if autopilot is active, or if a manual keypress has set IsIdle to false
                if (!p.IsIdle || p.IsBot || p.IsAutopilot)
                {
                    MovePlayer(p);
                    // In manual mode, reset the input lock until the next keypress triggers another tick
                    if (!p.IsBot && !p.IsAutopilot) p.IsIdle = true;
                }
                else
                {
                    // Idle energy regeneration (no overflow allowed above MaxEnergy)
                    p.Energy = Math.Min(p.Energy + _config.IdleEnergyGainRate, p.MaxEnergy);
                }

                // Re-calculate A* Strike savings and valid heading directions for UI previews
                CalculateStrike(p, false);
                CalculateValidMoves(p);
            }

            // 5. Update Projectiles with Sub-Ticking
            // To prevent fast bullets from clipping through walls/snakes, we update them
            // multiple times (BulletSpeedMultiplier) per global physics tick.
            for (int i = 0; i < _config.BulletSpeedMultiplier; i++)
                UpdateBullets();

            // 6. Snail Obstacle Movement
            UpdateSnails();

            // 7. Food Spawning and Extraction Check
            CheckFoodCollection();
            SpawnFood();
            
            if (_exitPortal.ExtractionCountdown > 0) {
                _exitPortal.ExtractionCountdown--;
                if (_exitPortal.ExtractionCountdown == 0) {
                    // Successful extraction: end game state
                    _state = SimulationStateType.GameOver;
                }
            }

            // 8. Evaluate Game Over / Survival conditions
            CheckGameOver();
        }

        private void SpawnBoss(BossWaveConfig wave) {
            // Boss is just a big bot for now
            var (bx, by) = GetRandomSpawnPosition();
            var boss = new Player(_players.Count, wave.BossType, bx, by, 255, 100, 0, _config.MaxEnergy * 2);
            boss.IsBot = true;
            // Lengthen body
            for(int k=0; k<10; k++) {
                boss.Body.Add(new SnakeSegment(bx, by));
                boss.BodyWeapons.Add(WeaponData.None);
            }
            boss.BodyWeapons[0] = new WeaponData { Type = WeaponType.Rifle, Ammo = 999, Side = WeaponSide.Right, ItemId = Guid.NewGuid().ToString() };
            _players.Add(boss);
        }

        // ===== Input Processing =====

        public void ProcessInput(int inputType, int param1, int param2)
        {
            // Record input for replay verification
            // We record even if logic might reject it (e.g. invalid move) to maintain sequence
            // But only relevant inputs (0-4, 6) affect state.
            // Configuration (8) and Start (5) are meta-inputs, but needed for replay setup.
            // For simplicity, we log everything that reaches here.
            _inputLog.Add(new InputFrame
            {
                Tick = _currentTick,
                InputType = inputType,
                Param1 = param1,
                Param2 = param2
            });

            switch (inputType)
            {
                case 0: // Direction change
                    if (param1 >= 0 && param1 <= 7)
                    {
                        var player = _players.FirstOrDefault(p => p.Id == _localPlayerId);
                        if (player != null) player.IsIdle = false; // Trigger movement
                        ChangeDirection(_localPlayerId, (Direction)param1);
                    }
                    break;
                case 1: // Fire weapon
                    FireWeapon(_localPlayerId);
                    break;
                case 2: // Focus towards head
                    ShiftFocus(_localPlayerId, true, param1 == 1);
                    break;
                case 3: // Focus towards tail
                    ShiftFocus(_localPlayerId, false, param1 == 1);
                    break;
                case 4: // Strike (formerly Dash/Slingshot)
                    Strike(_localPlayerId);
                    break;
                case 5: // Start game
                    StartGame(param1);
                    break;
                case 6: // Weapon attach (param1 = segment index)
                    // AttachWeapon(_localPlayerId, param1); // Deprecated by Inventory System
                    break;
                case 7: // Pause toggle
                    TogglePause();
                    break;
                case 8: // Configure
                    Configure(param1, param2);
                    break;
                case 9: // Toggle Autopilot
                    ToggleAutopilot(_localPlayerId);
                    break;
            }
        }

        private void ToggleAutopilot(int playerId)
        {
            var player = _players.FirstOrDefault(p => p.Id == playerId);
            if (player != null)
            {
                player.IsAutopilot = !player.IsAutopilot;
                if (player.IsAutopilot) player.IsIdle = false;
            }
        }


        // ===== Focus =====

        // ===== Focus =====

        // ===== Focus =====
        // ShiftFocus(playerId, towardsHead, jumpToExtremity)
        private void ShiftFocus(int playerId, bool towardsHead, bool jumpToExtremity)
        {
            var player = _players.FirstOrDefault(p => p.Id == playerId);
            if (player == null) return;

            if (jumpToExtremity)
            {
                // Binary Jump Logic
                int target = towardsHead ? 0 : player.Body.Count - 1;
                int current = player.FocusedSegment;
                int otherEnd = towardsHead ? player.Body.Count - 1 : 0;

                // If currently at the opposite end (or close enough/start), jump fully to target
                // Logic: If we are at Head (0) and want Tail, jump to Tail.
                // If we are at Tail and want Head, jump to Head? No, user wants Binary search backward.
                // User scenario: Start at Head(0). Shift+Z -> Tail(100).
                // Then Shift+A -> Halfway(50).

                // So: If we are at 'otherEnd', jump fully.
                // Or if we are at '0' and target is Tail.

                if (current == otherEnd)
                {
                    player.FocusedSegment = target;
                }
                else
                {
                    // Halfway jump
                    // Rounding logic: (Current + Target) / 2
                    // For odd distances, prefer rounding UP (towards Target?) or towards Tail?
                    // User said: 101 len -> 51. (0 to 100 -> 50).
                    // (100 + 0 + 1) / 2 = 50. Correct.
                    // (25 + 100 + 1) / 2 = 63.

                    // General midpoint formula with ceiling for positive direction?
                    // Let's use simple integer average, but check rounding direction.
                    // If Target > Current: (Current + Target + 1) / 2
                    // If Target < Current: (Current + Target) / 2

                    if (target > current)
                        player.FocusedSegment = (current + target + 1) / 2;
                    else
                        player.FocusedSegment = (current + target) / 2;
                }

                // Clamp just in case
                player.FocusedSegment = Math.Clamp(player.FocusedSegment, 0, player.Body.Count - 1);
            }
            else
            {
                // Simple increment/decrement
                int newFocus = towardsHead ? player.FocusedSegment - 1 : player.FocusedSegment + 1;
                player.FocusedSegment = Math.Clamp(newFocus, 0, player.Body.Count - 1);
            }
        }

        // ===== Pause =====

        private void TogglePause()
        {
            if (_state == SimulationStateType.Playing) _state = SimulationStateType.Paused;
            else if (_state == SimulationStateType.Paused) _state = SimulationStateType.Playing;
        }

        // ===== Helpers =====

        private bool IsPositionEmpty(int x, int y, int excludePlayerId)
        {
            if (_config.EnableWalls)
            {
                if (x <= 0 || x >= _config.GridWidth - 1 || y <= 0 || y >= _config.GridHeight - 1)
                    return false;
            }

            if (_internalWalls.Contains((x, y))) return false;

            foreach (var p in _players)
            {
                if (!p.IsAlive) continue;
                if (p.Id == excludePlayerId) continue;
                if (p.Body.Any(s => s.X == x && s.Y == y)) return false;
            }
            
            if (_snails.Any(s => s.X == x && s.Y == y)) return false;

            return true;
        }

        private (int x, int y) GetRandomSpawnPosition()
        {
            for (int i = 0; i < 1000; i++)
            {
                int x = _rng.Next(10, _config.GridWidth - 10);
                int y = _rng.Next(5, _config.GridHeight - 5);
                if (IsPositionEmpty(x, y, -1)) return (x, y);
            }
            return (_config.GridWidth / 4, _config.GridHeight / 4);
        }

        // ===== Inventory System =====

        private void AddTestItems(Player p)
        {
            if (_config.UnlimitedItems)
            {
                for (int i = 0; i < 99; i++)
                {
                    p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "PISTOL", Durability = 12 });
                    p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "RIFLE", Durability = 30 });
                    p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "LASER", Durability = 5 });
                    p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "AMPLIFIER", Durability = 100 });
                    p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "PRISM", Durability = 100 });
                    p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "COLLECTOR", Durability = 100 });
                }
            }
            else
            {
                p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "PISTOL", Durability = 12 });
                p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "RIFLE", Durability = 30 });
                p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "LASER", Durability = 5 });
                p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "AMPLIFIER", Durability = 100 });
                p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "PRISM", Durability = 100 });
                p.Backpack.Add(new InventoryItem { Id = Guid.NewGuid().ToString(), Type = ItemType.Weapon, AssetCode = "COLLECTOR", Durability = 100 });
            }
        }

        public bool EquipItem(int playerId, string itemId, int segmentIndex, WeaponSide side)
        {
            var player = _players.FirstOrDefault(p => p.Id == playerId);
            if (player == null) return false;

            var item = player.Backpack.FirstOrDefault(i => i.Id == itemId);
            if (item == null) return false;

            if (segmentIndex < 0 || segmentIndex >= player.Body.Count) return false;
            if (segmentIndex >= player.BodyWeapons.Count) return false;

            var slot = player.BodyWeapons[segmentIndex];
            if (slot.Type != WeaponType.None) return false;

            WeaponType wType = item.AssetCode switch
            {
                "PISTOL" => WeaponType.Pistol,
                "RIFLE" => WeaponType.Rifle,
                "LASER" => WeaponType.Laser,
                "AMPLIFIER" => WeaponType.Amplifier,
                "PRISM" => WeaponType.Prism,
                "COLLECTOR" => WeaponType.Collector,
                _ => WeaponType.None
            };
            if (wType == WeaponType.None) return false;

            player.Backpack.Remove(item);

            slot.Type = wType;
            slot.Ammo = item.Durability;
            slot.Side = side;
            slot.ItemId = item.Id;

            return true;
        }

        public bool UnequipItem(int playerId, int segmentIndex)
        {
            var player = _players.FirstOrDefault(p => p.Id == playerId);
            if (player == null) return false;
            if (segmentIndex < 0 || segmentIndex >= player.BodyWeapons.Count) return false;

            var slot = player.BodyWeapons[segmentIndex];
            if (slot.Type == WeaponType.None) return false;

            string code = slot.Type.ToString().ToUpper();
            var item = new InventoryItem
            {
                Id = string.IsNullOrEmpty(slot.ItemId) ? Guid.NewGuid().ToString() : slot.ItemId,
                Type = ItemType.Weapon,
                AssetCode = code,
                Durability = slot.Ammo
            };

            player.Backpack.Add(item);

            slot.Type = WeaponType.None;
            slot.Ammo = 0;
            slot.Side = WeaponSide.Left;
            slot.ItemId = "";

            return true;
        }

        public List<InventoryItem> GetPlayerBackpack(int playerId)
        {
             var player = _players.FirstOrDefault(p => p.Id == playerId);
             return player?.Backpack ?? new List<InventoryItem>();
        }

        public List<WeaponData> GetPlayerWeapons(int playerId)
        {
             var player = _players.FirstOrDefault(p => p.Id == playerId);
             return player?.BodyWeapons ?? new List<WeaponData>();
        }

        public bool SwapItems(int playerId, int idxA, int idxB)
        {
            var player = _players.FirstOrDefault(p => p.Id == playerId);
            if (player == null) return false;
            if (idxA < 0 || idxA >= player.BodyWeapons.Count) return false;
            if (idxB < 0 || idxB >= player.BodyWeapons.Count) return false;

            var slotA = player.BodyWeapons[idxA];
            var slotB = player.BodyWeapons[idxB];

            var tempType = slotA.Type;
            var tempAmmo = slotA.Ammo;
            var tempSide = slotA.Side;
            var tempId = slotA.ItemId;

            slotA.Type = slotB.Type;
            slotA.Ammo = slotB.Ammo;
            slotA.Side = slotB.Side;
            slotA.ItemId = slotB.ItemId;

            slotB.Type = tempType;
            slotB.Ammo = tempAmmo;
            slotB.Side = tempSide;
            slotB.ItemId = tempId;

            return true;
        }

        // ===== FFI State Getters =====

        public SimulationStateFFI GetSimulationState()
        {
            int pState = 0;
            if (_exitPortal.IsActive) pState = 1;
            if (_exitPortal.ExtractionCountdown >= 0) pState = 2;

            return new SimulationStateFFI
            {
                PlayerCount = _players.Count(p => p.IsAlive),
                GridWidth = _config.GridWidth,
                GridHeight = _config.GridHeight,
                LocalPlayerId = _localPlayerId,
                State = (int)_state,
                Timestamp = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds(),
                EnableWalls = _config.EnableWalls ? 1 : 0,
                CurrentWave = _currentWave,
                MatchTimeSeconds = (int)(_currentTick / 10),
                PortalX = _exitPortal.X,
                PortalY = _exitPortal.Y,
                PortalState = pState,
                ExtractionCountdown = _exitPortal.ExtractionCountdown
            };
        }

        // ===== Anti-Cheat: Replay Hashing =====

        public string GetReplayHash()
        {
            // Proof format v1: SEED | ENGINE_VERSION | CONFIG_HASH | INPUTS | "CRAWLCIPHER_PROOF_V1"
            // No secret salt: under replay verification, secrecy adds nothing (the engine is open).
            // "CRAWLCIPHER_PROOF_V1" is a public domain-separation/version tag so future proof-format
            // changes can't be confused with this one.
            const string proofTag = "CRAWLCIPHER_PROOF_V1";
            string configHash = ComputeConfigHash(_config);

            var sb = new StringBuilder();
            sb.Append(_seed).Append('|');
            sb.Append(EngineVersion).Append('|');
            sb.Append(configHash).Append('|');

            // Append Input Log
            foreach (var frame in _inputLog)
            {
                sb.Append(frame.Tick).Append(':')
                  .Append(frame.InputType).Append(':')
                  .Append(frame.Param1).Append(':')
                  .Append(frame.Param2).Append(';');
            }

            sb.Append('|').Append(proofTag);

            using var sha256 = SHA256.Create();
            var bytes = Encoding.UTF8.GetBytes(sb.ToString());
            var hashBytes = sha256.ComputeHash(bytes);
            return Convert.ToHexString(hashBytes);
        }

        /// <summary>
        /// Hashes every field of the effective <see cref="GameConfig"/> — including nested
        /// WeaponStats, BossWaveConfig, ScoringConfig, DeathPenaltyConfig and ExpeditionConfig
        /// values, in their declared order — so a verifier's CONFIG_HASH mismatches if any rule
        /// (weapon stats, wave timing, scoring, death penalties, ...) was edited before replay.
        /// </summary>
        private static string ComputeConfigHash(GameConfig config)
        {
            var sb = new StringBuilder();

            // GameConfig (declared field order)
            sb.Append(config.GridWidth).Append('|');
            sb.Append(config.GridHeight).Append('|');
            sb.Append(config.MaxEnergy).Append('|');
            sb.Append(config.EnergyGainPerMove).Append('|');
            sb.Append(config.Turn45Cost).Append('|');
            sb.Append(config.Turn90Cost).Append('|');
            sb.Append(config.TurnSharpCost).Append('|');
            sb.Append(config.StunDurationTicks).Append('|');
            sb.Append(config.BulletSpeedMultiplier).Append('|');
            sb.Append(config.BotCount).Append('|');
            sb.Append(config.EnableWalls).Append('|');
            sb.Append(config.FoodCount).Append('|');
            sb.Append(config.IdleEnergyGainRate).Append('|');
            sb.Append(config.SnailCount).Append('|');
            sb.Append(config.SnailSpeedDivisor).Append('|');
            sb.Append(config.SnailScoreReward).Append('|');
            sb.Append(config.SnailEnergyReward).Append('|');
            sb.Append(config.BonusEnergyMechanic).Append('|');

            // Nested: ExpeditionConfig (declared field order)
            var exp = config.Expedition;
            sb.Append(exp.InitialBotCount).Append('|');
            sb.Append(exp.BossWaves.Count).Append('|');
            foreach (var wave in exp.BossWaves)
            {
                sb.Append(wave.WaveNumber).Append(',')
                  .Append(wave.TriggerTimeSeconds).Append(',')
                  .Append(wave.BossType).Append(',')
                  .Append(wave.Multiplier).Append(',')
                  .Append(wave.AdditionalBots).Append(',')
                  .Append(wave.BotSpeedMultiplier).Append(';');
            }
            sb.Append('|');
            sb.Append(exp.Scoring.FoodScore).Append(',')
              .Append(exp.Scoring.SnailScore).Append(',')
              .Append(exp.Scoring.BotKillBase).Append(',')
              .Append(exp.Scoring.BotKillPerSegment).Append(',')
              .Append(exp.Scoring.BossKillScore).Append(',')
              .Append(exp.Scoring.SurvivalPerSecond).Append('|');
            sb.Append(exp.DeathPenalty.ScoreRetentionPercent).Append(',')
              .Append(exp.DeathPenalty.ApplyMultiplierOnDeath).Append(',')
              .Append(exp.DeathPenalty.ItemDurabilityLossPercent).Append('|');

            // Remaining GameConfig fields (declared after Expedition)
            sb.Append(config.StrikeStartCorner).Append('|');
            sb.Append(config.StrikeEndCornerOffset).Append('|');
            sb.Append(config.StrikeMaxSavings).Append('|');
            sb.Append(config.InitialSnakeLength).Append('|');
            sb.Append(config.ShowGhostTrail).Append('|');
            sb.Append(config.ShowStrikeBodyPreview).Append('|');
            sb.Append(config.UnlimitedItems).Append('|');

            // WeaponStats: static lookup table, but part of the effective ruleset — commit it too
            // so an edited ammo/range table (without an engine version bump) is still detected.
            foreach (WeaponType type in Enum.GetValues<WeaponType>())
            {
                var (ammo, range) = WeaponStats.Get(type);
                sb.Append((int)type).Append(':').Append(ammo).Append(':').Append(range).Append(';');
            }

            using var sha256 = SHA256.Create();
            var bytes = Encoding.UTF8.GetBytes(sb.ToString());
            var hashBytes = sha256.ComputeHash(bytes);
            return Convert.ToHexString(hashBytes);
        }

        public PlayerStateFFI GetPlayerState(int playerId)
        {
            var player = _players.FirstOrDefault(p => p.Id == playerId);
            if (player == null)
                return new PlayerStateFFI { Id = -1 };

            return new PlayerStateFFI
            {
                Id = player.Id,
                X = player.Body[0].X,
                Y = player.Body[0].Y,
                Energy = player.Energy,
                BonusEnergy = player.BonusEnergy,
                MaxEnergy = player.MaxEnergy,
                Score = player.Score,
                Kills = player.Kills,
                IsAlive = player.IsAlive ? 1 : 0,
                IsStunned = player.IsStunned ? 1 : 0,
                IsIdle = player.IsIdle ? 1 : 0,
                IsAutopilot = player.IsAutopilot ? 1 : 0,
                FocusedSegment = player.FocusedSegment,
                FocusedX = player.Body[Math.Min(player.FocusedSegment, player.Body.Count - 1)].X,
                FocusedY = player.Body[Math.Min(player.FocusedSegment, player.Body.Count - 1)].Y,
                BodyLength = player.Body.Count,
                CurrentDirection = (int)player.CurrentDirection,
                LastDirection = (int)player.LastDirection,
                LastActionStatus = player.LastActionStatus,
                ColorR = player.ColorR,
                ColorG = player.ColorG,
                ColorB = player.ColorB,
                ValidMovesMask = player.ValidMovesMask,
            };
        }

        private void CalculateValidMoves(Player p)
        {
            byte mask = 0;
            for (int i = 0; i < 8; i++)
            {
                var dir = (Direction)i;
                // Prevent 180 degree turn
                int opposite = ((int)p.CurrentDirection + 4) % 8;
                if (i == opposite) continue;

                byte cost = DirectionHelper.CalculateTurnCost(
                    p.LastDirection, dir, _config.Turn45Cost, _config.Turn90Cost, _config.TurnSharpCost);

                // Check Energy
                // For Straight move (cost 0), we don't need check, but logic holds (Energy >= 0)
                if (HasEnoughEnergy(p, cost))
                {
                    mask |= (byte)(1 << i);
                }
            }
            p.ValidMovesMask = mask;
        }

        public unsafe int GetGridCells(CellInfoFFI* buffer, int bufferSize, int viewX, int viewY, int viewW, int viewH)
        {
            int cellSize = sizeof(CellInfoFFI);
            int maxCells = bufferSize / cellSize;
            int totalCells = viewW * viewH;
            int cellCount = Math.Min(maxCells, totalCells);

            // Initialize all empty
            for (int i = 0; i < cellCount; i++)
            {
                buffer[i] = new CellInfoFFI
                {
                    CellType = (int)CellType.Empty,
                    PlayerId = -1,
                    ExtraData = 0,
                };
            }

            // Walls
            if (_config.EnableWalls)
            {
                for (int i = 0; i < cellCount; i++)
                {
                    int lx = i % viewW;
                    int ly = i / viewW;
                    int gx = viewX + lx;
                    int gy = viewY + ly;
                    if (_config.EnableWalls && (gx == 0 || gx == _config.GridWidth - 1 || gy == 0 || gy == _config.GridHeight - 1))
                    {
                        buffer[i] = new CellInfoFFI
                        {
                            CellType = (int)CellType.Wall,
                            PlayerId = -1,
                            ExtraData = 0,
                        };
                    }
                    else if (_internalWalls.Contains((gx, gy)))
                    {
                        buffer[i] = new CellInfoFFI
                        {
                            CellType = (int)CellType.Wall,
                            PlayerId = -1,
                            ExtraData = 0,
                        };
                    }
                }
            }

            // Helper for wrapping view coordinates
            int GetViewCoord(int entityPos, int viewPos, int mapSize)
            {
                int diff = entityPos - viewPos;
                if (_config.EnableWalls) return diff;
                return (diff % mapSize + mapSize) % mapSize;
            }

            // Food
            foreach (var food in _foods)
            {
                int lx = GetViewCoord(food.X, viewX, _config.GridWidth);
                int ly = GetViewCoord(food.Y, viewY, _config.GridHeight);
                
                if (lx >= 0 && lx < viewW && ly >= 0 && ly < viewH)
                {
                    int idx = ly * viewW + lx;
                    if (idx < cellCount)
                    {
                        buffer[idx] = new CellInfoFFI
                        {
                            CellType = (int)CellType.Food,
                            PlayerId = -1,
                            ExtraData = 0,
                        };
                    }
                }
            }

            // Snails
            foreach (var snail in _snails)
            {
                int lx = GetViewCoord(snail.X, viewX, _config.GridWidth);
                int ly = GetViewCoord(snail.Y, viewY, _config.GridHeight);

                if (lx >= 0 && lx < viewW && ly >= 0 && ly < viewH)
                {
                    int idx = ly * viewW + lx;
                    if (idx < cellCount)
                    {
                        buffer[idx] = new CellInfoFFI
                        {
                            CellType = (int)CellType.Snail,
                            PlayerId = -1,
                            ExtraData = (int)snail.Dir,
                        };
                    }
                }
            }

            // Players
            foreach (var player in _players.Where(p => p.IsAlive))
            {
                // Render Ghost Trail (Low Priority - check if empty)
                if (_config.ShowGhostTrail && player.GhostBody.Count > 0)
                {
                     foreach (var seg in player.GhostBody)
                     {
                        int lx = GetViewCoord(seg.X, viewX, _config.GridWidth);
                        int ly = GetViewCoord(seg.Y, viewY, _config.GridHeight);
                        if (lx < 0 || lx >= viewW || ly < 0 || ly >= viewH) continue;
                        int idx = ly * viewW + lx;
                        if (idx >= cellCount) continue;

                        if (buffer[idx].CellType == (int)CellType.Empty)
                        {
                            buffer[idx] = new CellInfoFFI
                            {
                                CellType = (int)CellType.GhostSegment,
                                PlayerId = player.Id,
                                ExtraData = 0,
                            };
                        }
                     }
                }

                // Render Strike Body Preview (Medium priority, above Ghost, below Strike Head/Entities)
                if (_config.ShowStrikeBodyPreview && player.StrikeBodyPreview.Count > 0)
                {
                    foreach (var seg in player.StrikeBodyPreview)
                    {
                        int lx = GetViewCoord(seg.X, viewX, _config.GridWidth);
                        int ly = GetViewCoord(seg.Y, viewY, _config.GridHeight);
                        if (lx >= 0 && lx < viewW && ly >= 0 && ly < viewH)
                        {
                            int idx = ly * viewW + lx;
                            if (idx < cellCount && buffer[idx].CellType == (int)CellType.Empty)
                            {
                                buffer[idx] = new CellInfoFFI
                                {
                                    CellType = (int)CellType.StrikeBodyPreview,
                                    PlayerId = player.Id,
                                    ExtraData = 0,
                                };
                            }
                        }
                    }
                }

                // Render Strike Preview Indicator (Higher priority than Ghost, lower than Entities)
                if (player.StrikePreviewX != -1)
                {
                    // Viewport is centered on player.
                    // Calculate target position relative to view
                    int lx = GetViewCoord(player.StrikePreviewX, viewX, _config.GridWidth);
                    int ly = GetViewCoord(player.StrikePreviewY, viewY, _config.GridHeight);

                    // If target is visible within viewport
                    if (lx >= 0 && lx < viewW && ly >= 0 && ly < viewH)
                    {
                        int idx = ly * viewW + lx;
                        if (idx < cellCount && buffer[idx].CellType == (int)CellType.Empty)
                        {
                             buffer[idx] = new CellInfoFFI
                            {
                                CellType = (int)CellType.StrikePreview,
                                PlayerId = player.Id,
                                ExtraData = 0,
                            };
                        }
                    }
                    else
                    {
                        // Target is OFF-SCREEN. Show distance indicator at edge.
                        // Calculate relative position from center of view (viewW/2, viewH/2)
                        // Or calculate relative to player head (which is usually center).
                        // Let's use View Center.
                        int centerX = viewW / 2;
                        int centerY = viewH / 2;

                        // We need "world delta" between view center and target.
                        // ViewX + CenterX is world center X.
                        int worldCenterX = (viewX + centerX) % _config.GridWidth;
                        int worldCenterY = (viewY + centerY) % _config.GridHeight;

                        int dx = GetWrapDelta(player.StrikePreviewX, worldCenterX, _config.GridWidth);
                        int dy = GetWrapDelta(player.StrikePreviewY, worldCenterY, _config.GridHeight);

                        // Clamp vector to screen bounds (relative to center)
                        // Screen bounds relative to center: [-centerX, centerX], [-centerY, centerY]
                        // We want to project (dx, dy) onto the box edge.

                        // Safe div logic
                        float absDx = Math.Abs(dx);
                        float absDy = Math.Abs(dy);

                        float scaleX = (centerX - 1) / (float)Math.Max(1, absDx); // -1 for padding
                        float scaleY = (centerY - 1) / (float)Math.Max(1, absDy);
                        float scale = Math.Min(scaleX, scaleY);

                        int edgeX = centerX + (int)(dx * scale);
                        int edgeY = centerY + (int)(dy * scale);

                        // Ensure inside bounds
                        edgeX = Math.Clamp(edgeX, 0, viewW - 1);
                        edgeY = Math.Clamp(edgeY, 0, viewH - 1);

                        int idx = edgeY * viewW + edgeX;
                        if (idx >= 0 && idx < cellCount && buffer[idx].CellType == (int)CellType.Empty)
                        {
                            // Calculate distance from HEAD to Target
                            int dist = Math.Max(Math.Abs(GetWrapDelta(player.Body[0].X, player.StrikePreviewX, _config.GridWidth)),
                                                Math.Abs(GetWrapDelta(player.Body[0].Y, player.StrikePreviewY, _config.GridHeight)));

                             buffer[idx] = new CellInfoFFI
                            {
                                CellType = (int)CellType.StrikePreview,
                                PlayerId = player.Id,
                                ExtraData = dist,
                            };
                        }
                    }
                }

                for (int s = 0; s < player.Body.Count; s++)
                {
                    var seg = player.Body[s];
                    int lx = GetViewCoord(seg.X, viewX, _config.GridWidth);
                    int ly = GetViewCoord(seg.Y, viewY, _config.GridHeight);
                    
                    if (lx < 0 || lx >= viewW || ly < 0 || ly >= viewH) continue;
                    int idx = ly * viewW + lx;
                    if (idx >= cellCount) continue;

                    CellType ct;
                    if (s == 0)
                    {
                        ct = CellType.SnakeHead;
                    }
                    else if (s == player.Body.Count - 1)
                    {
                        ct = CellType.SnakeTail;
                    }
                    else
                    {
                        ct = CellType.SnakeBody;
                    }

                // Focused segment highlight
                    if (s == player.FocusedSegment && player.Id == _localPlayerId)
                        ct = CellType.SnakeBodyFocused;

                    // Visualize energy level on body
                    // Segments within 'Energy' range from head are solid.
                    // Index s is distance from head. 0 is Head.
                    // If Energy=7, indices 0..6 should be solid.
                    int isHighEnergy = (s < player.Energy) ? 1 : 0;
                    int weaponTypeInt = (int)player.BodyWeapons[s].Type;
                    int extraData = isHighEnergy | (weaponTypeInt << 8);

                    buffer[idx] = new CellInfoFFI
                    {
                        CellType = (int)ct,
                        PlayerId = player.Id,
                        ExtraData = extraData,
                    };
                }


            }

            // Bullets
            foreach (var bullet in _bullets.Where(b => b.Active))
            {
                int lx = GetViewCoord(bullet.X, viewX, _config.GridWidth);
                int ly = GetViewCoord(bullet.Y, viewY, _config.GridHeight);
                
                if (lx < 0 || lx >= viewW || ly < 0 || ly >= viewH) continue;
                int idx = ly * viewW + lx;
                if (idx >= cellCount) continue;

                buffer[idx] = new CellInfoFFI
                {
                    CellType = (int)CellType.Bullet,
                    PlayerId = bullet.OwnerId,
                    ExtraData = (int)bullet.Dir,
                };
            }

            return cellCount;
        }
    }
}
