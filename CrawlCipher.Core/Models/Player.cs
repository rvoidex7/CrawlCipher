using System.Collections.Generic;

namespace CrawlCipher.Core
{
    public class SnakeSegment
    {
        public int X;
        public int Y;

        public SnakeSegment(int x, int y)
        {
            X = x; Y = y;
        }

        public SnakeSegment Clone() => new SnakeSegment(X, Y);
    }

    public class WeaponData
    {
        public WeaponType Type;
        public int Ammo;
        public WeaponSide Side;
        public string ItemId = ""; // Link to Inventory Item ID

        public static WeaponData None => new WeaponData { Type = WeaponType.None, Ammo = 0, Side = WeaponSide.Left, ItemId = "" };
        public WeaponData Clone() => new WeaponData { Type = Type, Ammo = Ammo, Side = Side, ItemId = ItemId };
    }

    public class Player
    {
        public int Id;
        public string Name;
        public List<SnakeSegment> Body; // Positions
        public List<WeaponData> BodyWeapons; // Weapons (Parallel to Body)
        public List<InventoryItem> Backpack = new(); // Unequipped items
        public Direction CurrentDirection;
        public Direction LastDirection;
        public bool IsIdle; // Manual movement state
        public bool IsAutopilot; // Continuous movement toggle
        public int Energy;
        public int BonusEnergy;
        public int MaxEnergy;
        public int Score;
        public int Kills;
        public int FoodCollected;
        public int SnailsKilled;
        public int BotsKilled;
        public int BossesKilled;
        public bool IsAlive;
        public bool IsStunned;
        public long StunEndTick;
        public int FocusedSegment;
        public int LastActionStatus; // Debug: 0=None, 1=Attached, 2=NoEnergy, 3=Fail
        public byte ColorR, ColorG, ColorB;
        public bool IsBot;
        private bool _pendingGrow;
        public List<SnakeSegment> GhostBody = new();
        public List<SnakeSegment> StrikeBodyPreview = new();
        public int StrikePreviewX = -1;
        public int StrikePreviewY = -1;
        public byte ValidMovesMask;

        public Player(int id, string name, int startX, int startY, byte r, byte g, byte b, int maxEnergy)
        {
            Id = id;
            Name = name;
            IsIdle = true; // Start idle
            Body = new List<SnakeSegment>
            {
                new SnakeSegment(startX, startY),
                new SnakeSegment(startX - 1, startY),
                new SnakeSegment(startX - 2, startY),
            };
            BodyWeapons = new List<WeaponData>
            {
                WeaponData.None,
                WeaponData.None,
                WeaponData.None
            };
            GhostBody.Clear();
            CurrentDirection = Direction.East;
            LastDirection = Direction.East;
            Energy = 5;
            BonusEnergy = 0;
            MaxEnergy = maxEnergy;
            Score = 0;
            Kills = 0;
            IsAlive = true;
            IsStunned = false;
            StunEndTick = -1;
            FocusedSegment = 0;
            ColorR = r; ColorG = g; ColorB = b;
            IsBot = false;
            _pendingGrow = false;
        }

        public void QueueGrow() => _pendingGrow = true;

        public bool ConsumePendingGrow()
        {
            if (!_pendingGrow) return false;
            _pendingGrow = false;
            return true;
        }
    }
}
