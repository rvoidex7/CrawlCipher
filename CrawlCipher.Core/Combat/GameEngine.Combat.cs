using System;
using System.Collections.Generic;
using System.Linq;

namespace CrawlCipher.Core
{
    public partial class GameEngine
    {
        // ===== Weapons =====

        private List<WeaponType> GetModulesInRadius(Player p, int cx, int cy, int radius)
        {
            var modules = new List<WeaponType>();
            for (int i = 0; i < p.Body.Count; i++)
            {
                var type = p.BodyWeapons[i].Type;
                if (type == WeaponType.Amplifier || type == WeaponType.Prism || type == WeaponType.Collector)
                {
                    if (Math.Abs(p.Body[i].X - cx) <= radius && Math.Abs(p.Body[i].Y - cy) <= radius)
                    {
                        modules.Add(type);
                    }
                }
            }
            return modules;
        }

        private void FireWeapon(int playerId)
        {
            var player = _players.FirstOrDefault(p => p.Id == playerId);
            if (player == null || !player.IsAlive) return;

            // 1. Verify Energy Requirements
            // Firing a weapon requires a base cost of 3 energy.
            // Ammo checks are bypassed in this sandbox build to permit continuous testing of mounted hardware.
            if (!HasEnoughEnergy(player, 3))
            {
                player.LastActionStatus = 2; // Error Code 2: Insufficient Energy
                return;
            }

            // Determine the segment index currently under the pilot's cursor focus
            int fi = Math.Min(player.FocusedSegment, player.Body.Count - 1);
            var seg = player.Body[fi];
            var weapon = player.BodyWeapons[fi];

            // Only fire active projectile weapons. Modules (Amplifier, Prism, Collector) cannot be fired.
            if (weapon.Type == WeaponType.None || weapon.Type == WeaponType.Amplifier || weapon.Type == WeaponType.Prism || weapon.Type == WeaponType.Collector) return;

            var segDir = GetSegmentDirection(player, fi);
            var (wx, wy, weaponDir) = GetWeaponInfo(seg, weapon, segDir);

            // Proximity Synergy
            var modules = GetModulesInRadius(player, seg.X, seg.Y, 1);
            int damage = 1;
            
            if (modules.Contains(WeaponType.Amplifier)) {
                damage = 3; // Amplifier effect: Increases damage
            }

            if (modules.Contains(WeaponType.Prism)) {
                // Prism effect: 45 degree rotation (1 step in 8-dir enum)
                int dirInt = (int)weaponDir;
                weaponDir = (Direction)((dirInt + 1) % 8);
            }

            ConsumeEnergy(player, 3);
            var (_, range) = WeaponStats.Get(weapon.Type);
            _bullets.Add(new Bullet(wx, wy, weaponDir, player.Id, range, damage));
        }

        private void AttachWeapon(int playerId, int sideInt)
        {
            var player = _players.FirstOrDefault(p => p.Id == playerId);
            if (player == null || !player.IsAlive) return;

            // 1. Verify Energy Requirements
            // Mounting a new weapon to the chassis requires a base cost of 7 energy.
            if (!HasEnoughEnergy(player, 7))
            {
                player.LastActionStatus = 2; // Error Code 2: Insufficient Energy
                return;
            }

            int fi = Math.Min(player.FocusedSegment, player.Body.Count - 1);
            var weapon = player.BodyWeapons[fi];

            // 2. Equip Hardware
            // We only attach if the slot is currently empty.
            if (weapon.Type == WeaponType.None)
            {
                ConsumeEnergy(player, 7);

                // Default sandbox attach mounts a standard Pistol.
                weapon.Type = WeaponType.Pistol;
                weapon.Ammo = WeaponStats.Get(WeaponType.Pistol).ammo; // 12 durability units
                weapon.Side = (sideInt == 1) ? WeaponSide.Right : WeaponSide.Left;
                
                // WeaponData is a class reference, so updating the fields directly
                // mutates the player.BodyWeapons parallel list item.

                player.LastActionStatus = 1; // Success Code 1: Attached
            }
            else
            {
                player.LastActionStatus = 3; // Error Code 3: Already has weapon
            }
        }


        private Direction GetSegmentDirection(Player p, int index)
        {
            // The head segment (index 0) always moves in the player's active command direction.
            if (index == 0) return p.CurrentDirection;
            
            var prev = p.Body[index - 1]; // Neighboring segment closer to the head
            var curr = p.Body[index];     // This segment
            
            // Calculate the directional vector pointing from this segment towards the head (prev)
            int dx = prev.X - curr.X;
            int dy = prev.Y - curr.Y;

            // 1. Handle Toroidal Border Wrap-Around Math
            // On a wrapped grid, adjacent segments can sit on opposite edges of the map.
            // e.g. If prev is at X=0 (left edge) and curr is at X=86 (right edge),
            // the difference dx is 0 - 86 = -86. Since -86 is less than -1, we map dx to 1
            // because the segment is physically adjacent and pointing East.
            if (!_config.EnableWalls)
            {
                if (dx > 1) dx = -1;       // Wrapped around from East edge to West edge
                else if (dx < -1) dx = 1;  // Wrapped around from West edge to East edge
                if (dy > 1) dy = -1;       // Wrapped around from South edge to North edge
                else if (dy < -1) dy = 1;  // Wrapped around from North edge to South edge
            }

            // Convert resolved coordinate deltas (dx, dy) back to a Direction enum
            return TryGetDirection(dx, dy) ?? p.CurrentDirection;
        }

        private (int x, int y, Direction dir) GetWeaponInfo(SnakeSegment seg, WeaponData weapon, Direction segDir)
        {
            // 2. Perpendicular Trajectory Calculation
            // Weapons fire at right angles relative to the body segment orientation.
            // On an 8-directional wind circle, 90 degrees is exactly 2 steps.
            // Right side adds +2 steps (clockwise); Left side subtracts -2 steps (counter-clockwise).
            int step = weapon.Side == WeaponSide.Right ? 2 : -2;
            int dirInt = (int)segDir;
            
            // Normalize with +8 before modulo to prevent negative results under C# integer divisions
            int weaponDirInt = (dirInt + step + 8) % 8;
            Direction weaponDir = (Direction)weaponDirInt;

            // 3. Offset Spawn Point Coordinates
            // Bullets are spawned exactly 1 cell to the side of the segment (wx, wy)
            // to avoid clipping directly into the snake's own segment body.
            var (dx, dy) = DirectionHelper.ToDelta(weaponDir);
            int wx = seg.X + dx;
            int wy = seg.Y + dy;

            return (wx, wy, weaponDir);
        }

        // ===== Bullets =====

        private void UpdateBullets()
        {
            // Iterate backward to allow safe removal of bullet elements from the list during iteration.
            for (int b = _bullets.Count - 1; b >= 0; b--)
            {
                var bullet = _bullets[b];
                // Clean up inactive bullets queued for removal
                if (!bullet.Active) { _bullets.RemoveAt(b); continue; }

                // Translate bullet heading to grid offsets and propagate coordinates
                var (dx, dy) = DirectionHelper.ToDelta(bullet.Dir);
                bullet.X += dx;
                bullet.Y += dy;
                bullet.TicksAlive++;

                // Check range limits or boundary exits (destroys bullet)
                if (bullet.TicksAlive >= bullet.MaxTicks ||
                    bullet.X < 0 || bullet.X >= _config.GridWidth ||
                    bullet.Y < 0 || bullet.Y >= _config.GridHeight)
                {
                    _bullets.RemoveAt(b);
                    continue;
                }

                // If coordinates intersect a non-empty tile (obstacle or entity)
                if (!IsPositionEmpty(bullet.X, bullet.Y, bullet.OwnerId))
                {
                    bool hit = false;

                    // 1. Check Snail Intersections
                    // Snails are slow-moving obstacles. Intersecting bullets instantly destroy them.
                    for (int s = _snails.Count - 1; s >= 0; s--)
                    {
                        if (_snails[s].X == bullet.X && _snails[s].Y == bullet.Y)
                        {
                            _snails.RemoveAt(s);
                            hit = true;
                        }
                    }

                    // 2. Check Player/Bot Intersections (excluding the bullet's owner)
                    foreach (var player in _players)
                    {
                        if (player.Id == bullet.OwnerId || !player.IsAlive) continue;

                        for (int s = 0; s < player.Body.Count; s++)
                        {
                            if (player.Body[s].X == bullet.X && player.Body[s].Y == bullet.Y)
                            {
                                if (s == 0)
                                {
                                    // HEAD SHOT: Deals stun status effect and drains target's energy to 0.
                                    player.IsStunned = true;
                                    player.Energy = 0;
                                    player.StunEndTick = _currentTick + _config.StunDurationTicks;
                                }
                                else
                                {
                                    // BODY HIT: Triggers segment detachment (severing).
                                    // The snake is cut in half at the impact segment.
                                    // All segments trailing after the impact node are severed and removed from coordinates list.
                                    int removeCount = player.Body.Count - s;
                                    player.Body.RemoveRange(s, removeCount);
                                    
                                    // Adjust target pilot's equipment cursor focus if it exceeded the new length
                                    if (player.FocusedSegment >= player.Body.Count)
                                        player.FocusedSegment = Math.Max(0, player.Body.Count - 1);
                                        
                                    // If only the head segment remains, the snake chassis is destroyed (death)
                                    if (player.Body.Count <= 1) player.IsAlive = false;
                                }

                                // 3. Credit Shooter Stats
                                var shooter = _players.FirstOrDefault(p => p.Id == bullet.OwnerId);
                                if (shooter != null) {
                                    shooter.Kills++;
                                    if (player.IsBot) {
                                        shooter.Score += _config.Expedition.Scoring.BotKillBase;
                                    }
                                }
                                hit = true;
                                break; // Break out of segment check for this player
                            }
                        }
                        if (hit) break; // Break out of player list evaluation if a hit occurred
                    }

                    // If it hit nothing specific (e.g. static wall), deactivate and destroy bullet
                    bullet.Active = false;
                    _bullets.RemoveAt(b);
                    continue;
                }
            }
        }

        // ===== Strike =====

        private void Strike(int playerId)
        {
            var player = _players.FirstOrDefault(p => p.Id == playerId);
            if (player != null) CalculateStrike(player, true);
        }

        /// <summary>
        /// Analyzes the snake's body coordinates to identify loops (corners) and calculates the shortest 
        /// A* path to bridge them. Straightening these loops "saves" body segments, which are then redistributed 
        /// to project the snake's head forward in a high-speed dash (Strike).
        /// 
        /// See Local Guide: [Strike-Algorithm-Geometry.md](../docs/r7/Development/Strike-Algorithm-Geometry.md)
        /// See Online Page: https://rvoidex7.github.io/r7notes/Github-Projects/Strike-Algorithm-Geometry
        /// </summary>
        /// <param name="player">The player or bot instance executing the strike evaluation.</param>
        /// <param name="apply">If true, executes the dash and collapses the body loops. If false, only updates preview coordinates.</param>
        private void CalculateStrike(Player player, bool apply)
        {
            // Abort evaluation if player is dead or immobilized by stuns
            if (!player.IsAlive || player.IsStunned)
            {
                player.StrikePreviewX = -1;
                player.StrikePreviewY = -1;
                player.StrikeBodyPreview.Clear();
                return;
            }

            // 1. Identify Corner Nodes
            // Corners are body segment indices where the direction vector changes.
            // These indicate the "joints" of our loops.
            var corners = GetCorners(player);
            
            // Add the final tail segment index as an implicit corner boundary
            if (player.Body.Count > 0) corners.Add(player.Body.Count - 1);

            // Straightening requires at least two joints (e.g. Head -> C1 -> C2 -> Tail)
            if (corners.Count < 2)
            {
                player.StrikePreviewX = -1;
                player.StrikePreviewY = -1;
                player.StrikeBodyPreview.Clear();
                return;
            }

            // 2. Select Loop Segment Boundaries
            // Evaluates a range of corners starting from the configured start corner index
            // up to the offset limit specified in the game config.
            int cStartIndex = Math.Max(0, _config.StrikeStartCorner - 1);
            if (cStartIndex >= corners.Count)
            {
                player.StrikePreviewX = -1;
                player.StrikeBodyPreview.Clear();
                return;
            }

            // Calculate the boundary indices for our corner check based on offset configuration limits.
            int cEndIndexLimit = cStartIndex + _config.StrikeEndCornerOffset;
            int maxEndIndex = Math.Min(corners.Count - 1, cEndIndexLimit);

            int bestSavings = 0;
            List<(int x, int y)>? bestPath = null;
            int bestStartIdx = -1;
            int bestEndIdx = -1;
            int bestHeadMove = 0;

            int bodyStartIdx = corners[cStartIndex];
            var startNode = player.Body[bodyStartIdx];

            // 3. Evaluate Corner Pairs
            // We loop backward from the furthest corner (maxEndIndex) to find the largest loop segment first.
            // This prioritizes maximizing the physical segment savings (longest dash).
            for (int cEndIndex = maxEndIndex; cEndIndex > cStartIndex; cEndIndex--)
            {
                int bodyEndIdx = corners[cEndIndex];
                var endNode = player.Body[bodyEndIdx];

                // The original body segment distance along the loop curve.
                int currentLen = bodyEndIdx - bodyStartIdx;

                // 4. Calculate Shortest A* Path
                // Computes the shortest straight path between the two corners.
                // We pass player.Id to GetShortestPath so the A* grid pathfinder temporarily
                // ignores the body segments currently in this loop (which are being collapsed),
                // otherwise the pathfinder would report the path as blocked by the snake itself.
                var path = GetShortestPath(startNode.X, startNode.Y, endNode.X, endNode.Y, player.Id);
                if (path == null) continue; // Skip if blocked by permanent obstacles (walls, snails, other snakes)

                // The length of the new, straightened shortcut path.
                int newLen = path.Count - 1;
                
                // Savings is the count of segments removed by straightening the loop.
                int savings = currentLen - newLen;

                // Enforce safety limits configured by the client.
                if (savings > _config.StrikeMaxSavings) savings = _config.StrikeMaxSavings;

                if (savings > 0)
                {
                    // 5. Verify Head Movement Space
                    // To keep the snake's length mathematically constant, the head must move forward
                    // by the exact number of segments saved from the collapsed loop.
                    // We run a sweep raycast to check how many free cells lie ahead of the head.
                    int maxHeadMove = GetMaxHeadSpace(player, savings);

                    // We only execute if there is enough clearance. If savings > maxHeadMove,
                    // the dash would force the head to crash into a wall or obstacle.
                    if (savings <= maxHeadMove)
                    {
                        // Keep track of the loop that yields the maximum valid savings.
                        if (savings > bestSavings)
                        {
                            bestSavings = savings;
                            bestPath = path;
                            bestStartIdx = bodyStartIdx;
                            bestEndIdx = bodyEndIdx;
                            bestHeadMove = savings;
                        }
                    }
                }
            }

            // 6. Apply or Preview Results
            if (bestSavings > 0 && bestPath != null)
            {
                if (apply)
                {
                    // Execute the strike: relocate the head, collapse the intermediate nodes, and charge energy.
                    ApplyStrike(player, bestStartIdx, bestEndIdx, bestPath, bestHeadMove);
                    player.StrikePreviewX = -1; // Clear preview metrics after successful execution
                    player.StrikePreviewY = -1;
                    player.StrikeBodyPreview.Clear();
                }
                else
                {
                    // Calculate the preview coordinates of the head's projected landing site.
                    var (dx, dy) = DirectionHelper.ToDelta(player.CurrentDirection);
                    int hx = player.Body[0].X;
                    int hy = player.Body[0].Y;
                    int px = hx + dx * bestHeadMove;
                    int py = hy + dy * bestHeadMove;

                    // If walls are disabled (toroidal wrap mode), wrap coordinates safely.
                    if (!_config.EnableWalls)
                    {
                        // Double modulo math ensures negative coordinates wrap correctly to positive boundaries.
                        px = ((px % _config.GridWidth) + _config.GridWidth) % _config.GridWidth;
                        py = ((py % _config.GridHeight) + _config.GridHeight) % _config.GridHeight;
                    }
                    player.StrikePreviewX = px;
                    player.StrikePreviewY = py;

                    // Build the preview trail representing the optimized shortcut path.
                    if (_config.ShowStrikeBodyPreview)
                    {
                        player.StrikeBodyPreview.Clear();
                        foreach(var p in bestPath)
                        {
                            player.StrikeBodyPreview.Add(new SnakeSegment(p.x, p.y));
                        }
                    }
                }
            }
            else
            {
                player.StrikePreviewX = -1;
                player.StrikePreviewY = -1;
                player.StrikeBodyPreview.Clear();
            }
        }

        private void ApplyStrike(Player player, int startIdx, int endIdx, List<(int x, int y)> newPath, int headMove)
        {
            // Save Ghost Trail if enabled
            if (_config.ShowGhostTrail)
            {
                player.GhostBody.Clear();
                for (int i = 0; i <= endIdx && i < player.Body.Count; i++)
                {
                    player.GhostBody.Add(player.Body[i].Clone());
                }
            }

            // 1. Create new head segments (extending from current head)
            var newHeadSegments = new List<SnakeSegment>();
            var (dx, dy) = DirectionHelper.ToDelta(player.CurrentDirection);
            int hx = player.Body[0].X;
            int hy = player.Body[0].Y;

            for (int i = 1; i <= headMove; i++)
            {
                int nx = hx + dx * i;
                int ny = hy + dy * i;
                 if (!_config.EnableWalls)
                {
                    nx = ((nx % _config.GridWidth) + _config.GridWidth) % _config.GridWidth;
                    ny = ((ny % _config.GridHeight) + _config.GridHeight) % _config.GridHeight;
                }
                newHeadSegments.Add(new SnakeSegment(nx, ny));
            }
            newHeadSegments.Reverse(); // Head is first

            // 2. Construct new body positions
            var newBody = new List<SnakeSegment>();
            newBody.AddRange(newHeadSegments);

            // Add existing segments from OldHead up to StartIdx
            for (int i = 0; i < startIdx; i++)
            {
                newBody.Add(player.Body[i]);
            }

            // Add new path (Replace StartIdx..EndIdx)
            foreach (var p in newPath)
            {
                newBody.Add(new SnakeSegment(p.x, p.y));
            }

            // Add remaining tail (from endIdx + 1)
            for (int i = endIdx + 1; i < player.Body.Count; i++)
            {
                newBody.Add(player.Body[i]);
            }

            player.Body = newBody;

            // 3. Handle Weapons (SoA)
            // Ensure BodyWeapons length matches.
            while (player.BodyWeapons.Count < player.Body.Count) player.BodyWeapons.Add(WeaponData.None);
            while (player.BodyWeapons.Count > player.Body.Count) player.BodyWeapons.RemoveAt(player.BodyWeapons.Count - 1);
        }

    }
}
