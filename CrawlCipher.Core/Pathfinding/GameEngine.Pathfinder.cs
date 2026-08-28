using System;
using System.Collections.Generic;
using System.Linq;

namespace CrawlCipher.Core
{
    public partial class GameEngine
    {
        private List<int> GetCorners(Player p)
        {
            var corners = new List<int>();
            if (p.Body.Count < 3) return corners;

            for (int i = 1; i < p.Body.Count - 1; i++)
            {
                var prev = p.Body[i - 1];
                var curr = p.Body[i];
                var next = p.Body[i + 1];

                int dx1 = GetWrapDelta(prev.X, curr.X, _config.GridWidth);
                int dy1 = GetWrapDelta(prev.Y, curr.Y, _config.GridHeight);

                int dx2 = GetWrapDelta(curr.X, next.X, _config.GridWidth);
                int dy2 = GetWrapDelta(curr.Y, next.Y, _config.GridHeight);

                if (dx1 != dx2 || dy1 != dy2)
                {
                    corners.Add(i);
                }
            }
            return corners;
        }

        private int GetWrapDelta(int p1, int p2, int size)
        {
            int d = p1 - p2;
            if (!_config.EnableWalls)
            {
                int half = size / 2;
                if (d > half) d -= size;
                else if (d < -half) d += size;
            }
            return d;
        }

        private int GetMaxHeadSpace(Player p, int maxDist)
        {
            var (dx, dy) = DirectionHelper.ToDelta(p.CurrentDirection);
            int cx = p.Body[0].X;
            int cy = p.Body[0].Y;

            for (int i = 1; i <= maxDist; i++)
            {
                cx += dx;
                cy += dy;

                if (!_config.EnableWalls)
                {
                    cx = ((cx % _config.GridWidth) + _config.GridWidth) % _config.GridWidth;
                    cy = ((cy % _config.GridHeight) + _config.GridHeight) % _config.GridHeight;
                }

                if (!IsPositionEmpty(cx, cy, p.Id))
                    return i - 1;
            }
            return maxDist;
        }

        private List<(int x, int y)>? GetShortestPath(int x0, int y0, int x1, int y1, int playerId)
        {
            // A* Pathfinding with Tie-Breaker for Straight Lines
            // Priority is float/double to allow small nudges for tie-breaking
            var openSet = new PriorityQueue<(int x, int y), double>();
            openSet.Enqueue((x0, y0), 0);

            var cameFrom = new Dictionary<(int x, int y), (int x, int y)>();
            var gScore = new Dictionary<(int x, int y), int>();
            gScore[(x0, y0)] = 0;

            // Ideal Line Vector (Start -> End)
            // Used for cross-product tie-breaking
            // We calculate cross product of vector (Start->Current) and (Start->End)
            // The magnitude of cross product corresponds to area of parallelogram,
            // effectively distance from the line.
            // But we have wrapping... wrapping makes "ideal line" tricky.
            // If walls disabled, shortest path might cross wrap boundary.
            // For now, let's assume direct line logic for tie-breaking works locally.
            // If wrapping occurs, heuristic might pull wrong way?
            // GetChebyshevDist handles wrap for H score.
            // For tie-breaker, let's use Manhattan distance to line or just cross product?
            // Cross product requires continuous space logic.
            // Simple heuristic modification: p * (distance from line)
            // Or simpler: nudge H slightly higher (0.001) to break ties in favor of explored nodes?
            // Actually, we want to favor nodes closest to the straight line.

            // Let's use 1000 scale for costs.
            // Movement cost = 1000.
            // Tie-breaker = Cross Product magnitude.

            // Calculate unwrapped target relative to start (for vector math)
            int tx = x1, ty = y1;
            if (!_config.EnableWalls)
            {
                int dx = x1 - x0;
                int dy = y1 - y0;
                int w = _config.GridWidth;
                int h = _config.GridHeight;
                if (dx > w/2) tx -= w; else if (dx < -w/2) tx += w;
                if (dy > h/2) ty -= h; else if (dy < -h/2) ty += h;
            }

            // Vector Start->End
            long dxSE = tx - x0;
            long dySE = ty - y0;

            while (openSet.Count > 0)
            {
                var current = openSet.Dequeue();
                if (current.x == x1 && current.y == y1)
                {
                    return ReconstructPath(cameFrom, current);
                }

                // Neighbors (8-way)
                for (int dx = -1; dx <= 1; dx++)
                {
                    for (int dy = -1; dy <= 1; dy++)
                    {
                        if (dx == 0 && dy == 0) continue;

                        int nx = current.x + dx;
                        int ny = current.y + dy;

                        // Bounds / Wrap
                        if (!_config.EnableWalls)
                        {
                            nx = ((nx % _config.GridWidth) + _config.GridWidth) % _config.GridWidth;
                            ny = ((ny % _config.GridHeight) + _config.GridHeight) % _config.GridHeight;
                        }

                        if (!IsPositionEmpty(nx, ny, playerId)) continue;

                        int tentativeG = gScore[current] + 1; // Cost 1 per step (Chebyshev logic: diagonal is 1)

                        if (!gScore.ContainsKey((nx, ny)) || tentativeG < gScore[(nx, ny)])
                        {
                            cameFrom[(nx, ny)] = current;
                            gScore[(nx, ny)] = tentativeG;

                            // H Score (Distance to goal)
                            int h = GetChebyshevDist(nx, ny, x1, y1);

                            // Tie-breaker: Cross Product
                            // Vector Start->Neighbor (unwrapped relative to start)
                            // We need unwrapped coordinates for 'nx, ny' relative to 'x0, y0'
                            // This is tricky with wrapping.
                            // Let's use relative offset from 'current' + accum?
                            // Simpler: Just compute cross product with whatever coordinates we have if strict walls.
                            // If wrapping, skip tie-breaker or do best effort.

                            double tieBreaker = 0;
                            if (_config.EnableWalls) // Only apply straight-line bias in walled mode for simplicity
                            {
                                long dxSN = nx - x0;
                                long dySN = ny - y0;
                                long cross = Math.Abs(dxSE * dySN - dySE * dxSN);
                                tieBreaker = cross * 0.001;
                            }

                            double priority = tentativeG + h + tieBreaker;
                            openSet.Enqueue((nx, ny), priority);
                        }
                    }
                }
            }
            return null; // No path
        }

        private List<(int x, int y)> ReconstructPath(Dictionary<(int x, int y), (int x, int y)> cameFrom, (int x, int y) current)
        {
            var path = new List<(int x, int y)> { current };
            while (cameFrom.ContainsKey(current))
            {
                current = cameFrom[current];
                path.Add(current);
            }
            path.Reverse();
            return path;
        }

        private int GetChebyshevDist(int x0, int y0, int x1, int y1)
        {
            int dx = Math.Abs(x0 - x1);
            int dy = Math.Abs(y0 - y1);

            if (!_config.EnableWalls)
            {
                int w = _config.GridWidth;
                int h = _config.GridHeight;
                dx = Math.Min(dx, w - dx);
                dy = Math.Min(dy, h - dy);
            }
            return Math.Max(dx, dy);
        }

    }
}
