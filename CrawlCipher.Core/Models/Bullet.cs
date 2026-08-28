namespace CrawlCipher.Core
{
    public class Bullet
    {
        public int X;
        public int Y;
        public Direction Dir;
        public int OwnerId;
        public int TicksAlive;
        public int MaxTicks;
        public int Damage;
        public bool Active;

        public Bullet(int x, int y, Direction dir, int ownerId, int maxTicks, int damage = 1)
        {
            X = x; Y = y; Dir = dir; OwnerId = ownerId; MaxTicks = maxTicks; Damage = damage; Active = true; TicksAlive = 0;
        }
    }
}
