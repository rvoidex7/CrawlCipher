using System;
using System.Collections.Generic;
using System.Linq;

namespace CrawlCipher.Core
{
    public partial class GameEngine
    {
        // ===== Bot AI =====

        private void UpdateBot(Player bot)
        {
            // Find nearest food
            (int X, int Y) nearest = (-1, -1);
            int minDist = int.MaxValue;
            foreach (var food in _foods)
            {
                int dist = Math.Abs(food.X - bot.Body[0].X) + Math.Abs(food.Y - bot.Body[0].Y);
                if (dist < minDist) { minDist = dist; nearest = food; }
            }

            if (nearest.X >= 0)
            {
                int dx = Math.Sign(nearest.X - bot.Body[0].X);
                int dy = Math.Sign(nearest.Y - bot.Body[0].Y);
                if (dx == 0 && dy == 0) return;

                // Try to find a valid direction towards food
                Direction? bestDir = TryGetDirection(dx, dy);
                if (bestDir.HasValue)
                {
                    byte cost = DirectionHelper.CalculateTurnCost(
                        bot.LastDirection, bestDir.Value, _config.Turn45Cost, _config.Turn90Cost, _config.TurnSharpCost);
                    if (bot.Energy >= cost)
                    {
                        if (cost > 0) bot.Energy -= cost;
                        bot.CurrentDirection = bestDir.Value;
                    }
                }
            }

            // Random direction change occasionally
            if (_rng.Next(100) < 5)
            {
                var dirs = Enum.GetValues<Direction>();
                bot.CurrentDirection = dirs[_rng.Next(dirs.Length)];
            }

            // Bot fires weapon occasionally
            if (_rng.Next(100) < 10)
                FireWeapon(bot.Id);
        }

        private Direction? TryGetDirection(int dx, int dy)
        {
            // Map delta to direction
            return (dx, dy) switch
            {
                (0, -1) => Direction.North,
                (1, -1) => Direction.NorthEast,
                (1, 0) => Direction.East,
                (1, 1) => Direction.SouthEast,
                (0, 1) => Direction.South,
                (-1, 1) => Direction.SouthWest,
                (-1, 0) => Direction.West,
                (-1, -1) => Direction.NorthWest,
                _ => null,
            };
        }

        // ===== Food =====

        private void CheckFoodCollection()
        {
            foreach (var player in _players.Where(p => p.IsAlive))
            {
                // Find all collection zones for this player.
                // Head is always a collection zone with radius 0.
                // Any segment with Collector module has radius 2 (5x5).
                var zones = new List<(int x, int y, int radius)>();
                zones.Add((player.Body[0].X, player.Body[0].Y, 0));
                
                for(int i = 0; i < player.Body.Count; i++) {
                    if (player.BodyWeapons[i].Type == WeaponType.Collector) {
                        zones.Add((player.Body[i].X, player.Body[i].Y, 2));
                    }
                }

                // Check static food
                for (int i = _foods.Count - 1; i >= 0; i--) {
                    var f = _foods[i];
                    bool collected = false;
                    foreach (var z in zones) {
                        if (Math.Abs(f.X - z.x) <= z.radius && Math.Abs(f.Y - z.y) <= z.radius) {
                            collected = true; break;
                        }
                    }
                    if (collected) {
                        _foods.RemoveAt(i);
                        player.Score += _config.Expedition.Scoring.FoodScore;
                        player.FoodCollected++;
                        AddEnergy(player, 2);
                        player.QueueGrow();
                    }
                }

                // Snails
                for (int i = _snails.Count - 1; i >= 0; i--) {
                    var s = _snails[i];
                    bool collected = false;
                    foreach (var z in zones) {
                        if (Math.Abs(s.X - z.x) <= z.radius && Math.Abs(s.Y - z.y) <= z.radius) {
                            collected = true; break;
                        }
                    }
                    if (collected) {
                        _snails.RemoveAt(i);
                        player.Score += _config.SnailScoreReward; 
                        player.SnailsKilled++;
                        AddEnergy(player, _config.SnailEnergyReward);
                    }
                }
            }
        }

        private void CheckSnailCollection(Player player, int hx, int hy)
        {
            int midx = _snails.FindIndex(f => f.X == hx && f.Y == hy);
            if (midx >= 0)
            {
                _snails.RemoveAt(midx);
                player.Score += _config.SnailScoreReward; // Use SnailScore from Expedition config if needed
                player.SnailsKilled++;
                AddEnergy(player, _config.SnailEnergyReward);
            }
        }

        private void SpawnFood()
        {
            // Static Food
            int attempts = 0;
            while (_foods.Count < _config.FoodCount && attempts < 100)
            {
                int x = _rng.Next(2, _config.GridWidth - 2);
                int y = _rng.Next(2, _config.GridHeight - 2);
                if (IsPositionEmpty(x, y, -1) && !_foods.Any(f => f.X == x && f.Y == y))
                    _foods.Add((x, y));
                attempts++;
            }

            // Snails
            while (_snails.Count < _config.SnailCount && attempts < 200)
            {
                int x = _rng.Next(2, _config.GridWidth - 2);
                int y = _rng.Next(2, _config.GridHeight - 2);
                if (IsPositionEmpty(x, y, -1) && !_foods.Any(f => f.X == x && f.Y == y))
                {
                    var dir = (Direction)_rng.Next(8);
                    _snails.Add(new Snail(x, y, dir));
                }
                attempts++;
            }
        }

        private void UpdateSnails()
        {
            // Pre-move check: Did a agent just eat me?
            // (Handled by Tick sequence: agent Move -> UpdateSnails -> CheckFoodCollection)
            // But if we want to be safe, we can check collision before moving.

            for (int i = _snails.Count - 1; i >= 0; i--)
            {
                var snail = _snails[i];

                // Speed Check
                snail.MoveTickCounter++;
                if (snail.MoveTickCounter < _config.SnailSpeedDivisor) continue;
                snail.MoveTickCounter = 0;

                var (dx, dy) = DirectionHelper.ToDelta(snail.Dir);
                int nextX = snail.X + dx;
                int nextY = snail.Y + dy;
                bool bounceX = false;
                bool bounceY = false;

                if (_config.EnableWalls)
                {
                    if (nextX <= 0 || nextX >= _config.GridWidth - 1) { bounceX = true; }
                    if (nextY <= 0 || nextY >= _config.GridHeight - 1) { bounceY = true; }
                }
                else
                {
                    nextX = ((nextX % _config.GridWidth) + _config.GridWidth) % _config.GridWidth;
                    nextY = ((nextY % _config.GridHeight) + _config.GridHeight) % _config.GridHeight;
                }

                // Obstacle collision (Players/Walls treated as blocked)
                if (!bounceX && !bounceY)
                {
                    if (!IsPositionEmpty(nextX, nextY, -1))
                    {
                        // Determine reflection axis based on what is blocked.
                        // Check if X-only move is blocked
                        int testX = snail.X + dx;
                        bool blockX = !IsPositionEmpty(testX, snail.Y, -1);

                        // Check if Y-only move is blocked
                        int testY = snail.Y + dy;
                        bool blockY = !IsPositionEmpty(snail.X, testY, -1);

                        if (blockX) bounceX = true;
                        if (blockY) bounceY = true;

                        // If diagonal into corner/single block, might need both?
                        if (!blockX && !blockY) { bounceX = true; bounceY = true; } // Corner hit
                    }
                }

                if (bounceX || bounceY)
                {
                    if (bounceX) dx = -dx;
                    if (bounceY) dy = -dy;

                    snail.Dir = TryGetDirection(dx, dy) ?? snail.Dir;
                    // Move in new direction immediately? Or wait next tick?
                    // User said "sekme hamlesi yüzünden yılanın yeme fonksiyonuna hiç girmiyor".
                    // If we bounce, we might stay in place or move to new spot.
                    // Let's try to move in reflected direction if clear.

                    int rx = snail.X + dx;
                    int ry = snail.Y + dy;
                     if (_config.EnableWalls)
                    {
                        if (rx > 0 && rx < _config.GridWidth - 1 && ry > 0 && ry < _config.GridHeight - 1 && IsPositionEmpty(rx, ry, -1))
                        {
                            snail.X = rx; snail.Y = ry;
                        }
                    }
                    else
                    {
                        rx = ((rx % _config.GridWidth) + _config.GridWidth) % _config.GridWidth;
                        ry = ((ry % _config.GridHeight) + _config.GridHeight) % _config.GridHeight;
                         if (IsPositionEmpty(rx, ry, -1))
                        {
                            snail.X = rx; snail.Y = ry;
                        }
                    }
                }
                else
                {
                    snail.X = nextX; snail.Y = nextY;
                }

                // Post-move clamp
                if (_config.EnableWalls)
                {
                    snail.X = Math.Clamp(snail.X, 1, _config.GridWidth - 2);
                    snail.Y = Math.Clamp(snail.Y, 1, _config.GridHeight - 2);
                }

                // Check collision with ANY player head immediately after move
                foreach (var p in _players)
                {
                    if (p.IsAlive && p.Body[0].X == snail.X && p.Body[0].Y == snail.Y)
                    {
                        CheckSnailCollection(p, snail.X, snail.Y);
                        // Snail removed inside CheckSnailCollection (by list index search)
                        // But since we are iterating backwards by index 'i', we need to be careful.
                        // CheckSnailCollection finds by coordinates. It might remove THIS snail.
                        // If removed, break this loop iteration.
                        // To be safe, verify if snail still exists in list or just break.
                        break;
                    }
                }
            }
        }

        private void CheckGameOver()
        {
            var localPlayer = _players.FirstOrDefault(p => p.Id == _localPlayerId);
            if (localPlayer != null && !localPlayer.IsAlive)
                _state = SimulationStateType.GameOver;
        }

    }
}
