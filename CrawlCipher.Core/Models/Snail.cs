namespace CrawlCipher.Core
{
    public class Snail
    {
        public int X;
        public int Y;
        public Direction Dir;
        public int MoveTickCounter;

        public Snail(int x, int y, Direction dir)
        {
            X = x; Y = y;
            Dir = dir;
            MoveTickCounter = 0;
        }
    }
}
