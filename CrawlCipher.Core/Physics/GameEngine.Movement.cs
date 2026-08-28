using System;
using System.Collections.Generic;
using System.Linq;

namespace CrawlCipher.Core
{
    public partial class GameEngine
    {
        // ===== Movement =====

        private bool HasEnoughEnergy(Player p, int cost)
        {
            return (p.Energy + p.BonusEnergy) >= cost;
        }

        private void ConsumeEnergy(Player p, int cost)
        {
            if (p.BonusEnergy >= cost)
            {
                p.BonusEnergy -= cost;
            }
            else
            {
                int remainder = cost - p.BonusEnergy;
                p.BonusEnergy = 0;
                p.Energy -= remainder;
            }
        }

        private void AddEnergy(Player p, int amount)
        {
            int needed = p.MaxEnergy - p.Energy;
            if (amount <= needed)
            {
                p.Energy += amount;
            }
            else
            {
                p.Energy = p.MaxEnergy;
                int overflow = amount - needed;
                if (_config.BonusEnergyMechanic)
                {
                    p.BonusEnergy += overflow;
                }
            }
        }

        private void ChangeDirection(int playerId, Direction newDir)
        {
            var player = _players.FirstOrDefault(p => p.Id == playerId);
            if (player == null || !player.IsAlive || player.IsStunned) return;

            // Prevent 180 degree turn (opposite direction)
            // Directions are 0-7. Opposite is (dir + 4) % 8.
            int opposite = ((int)player.CurrentDirection + 4) % 8;
            if ((int)newDir == opposite) return;

            byte cost = DirectionHelper.CalculateTurnCost(
                player.LastDirection, newDir, _config.Turn45Cost, _config.Turn90Cost, _config.TurnSharpCost);

            if (HasEnoughEnergy(player, cost))
            {
                if (cost > 0) ConsumeEnergy(player, cost);
                player.CurrentDirection = newDir;
            }
        }

        private void MovePlayer(Player player)
        {
            // 1. Calculate Next Head Position Coordinates
            // Translate the current direction vector to 2D grid offset coordinates (dx, dy).
            var (dx, dy) = DirectionHelper.ToDelta(player.CurrentDirection);
            int newX = player.Body[0].X + dx;
            int newY = player.Body[0].Y + dy;

            // 2. Resolve Straight Movement and Energy Gain
            // Energy accumulation or segment growth only triggers if the player maintains their heading.
            if (player.CurrentDirection == player.LastDirection)
            {
                if (_config.BonusEnergyMechanic && player.BonusEnergy > 0)
                {
                    // Bonus Energy conversion: Instead of increasing active capacity,
                    // we consume 1 unit of bonus overflow energy to trigger segment growth (QueueGrow).
                    player.BonusEnergy--;
                    player.QueueGrow();
                }
                else
                {
                    // Regular straight movement restores energy up to the maximum capacity.
                    player.Energy = Math.Min(player.Energy + _config.EnergyGainPerMove, player.MaxEnergy);
                }
            }

            // 3. Resolve Toroidal Grid Wrapping
            // If walls are disabled, wrapping coordinates are calculated.
            if (!_config.EnableWalls)
            {
                // Double modulo is required because C#'s native % operator on negative numbers yields negative results.
                // e.g. (-1 % 50) + 50 -> 49 % 50 -> 49 (safely wrapped).
                newX = ((newX % _config.GridWidth) + _config.GridWidth) % _config.GridWidth;
                newY = ((newY % _config.GridHeight) + _config.GridHeight) % _config.GridHeight;
            }

            // 4. Spatial Obstacle Collision Checks
            // Evaluates if the target coordinate is occupied by walls, snails, or other players.
            if (!IsPositionEmpty(newX, newY, player.Id))
            {
                player.IsAlive = false;
                return;
            }

            // 5. Self-Collision Checks
            // Checks if the head's new coordinate collides with any of the snake's own trailing segments.
            // Starts at index 1 because the head (index 0) is relocating.
            for (int i = 1; i < player.Body.Count; i++)
            {
                if (player.Body[i].X == newX && player.Body[i].Y == newY)
                {
                    player.IsAlive = false;
                    return;
                }
            }

            // 6. Relocate Head (Slither Step)
            // Inserts the new head segment at index 0. This temporarily increases the body list size by 1.
            player.Body.Insert(0, new SnakeSegment(newX, newY));

            // Note: We do NOT insert an element into the BodyWeapons list here.
            // Items must remain anchored to their physical segment index, not their coordinates.
            // e.g. A rifle mounted on the 2nd segment (Index 1) must stay on Index 1 relative to the head.
            // As the coordinates flow down the Body list, the equipment remains mapped to the same index.

            // 7. Resolve Growth vs Tail Truncation
            if (player.ConsumePendingGrow())
            {
                // Under growth: We preserve the tail segment, increasing total length by 1.
                // We append a None weapon slot to the end of the weapons list to keep the coordinates
                // list and equipment list parallel.
                player.BodyWeapons.Add(WeaponData.None);
            }
            else
            {
                // Under standard movement: We remove the tail segment at the end of the body list
                // to maintain constant length.
                // The weapons list size remains unchanged. Length parity between Body and BodyWeapons is preserved:
                // (e.g. Body grows by 1 (head insert) and shrinks by 1 (tail remove) -> Net change 0.
                // Weapons list has no insert and no remove -> Net change 0.)
                if (player.Body.Count > 0)
                {
                    player.Body.RemoveAt(player.Body.Count - 1);
                }
            }

            // Record the current direction for the next tick's angular cost check
            player.LastDirection = player.CurrentDirection;
        }
    }
}
