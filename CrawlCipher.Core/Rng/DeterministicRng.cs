using System;

namespace CrawlCipher.Core.Rng
{
    /// <summary>
    /// CrawlCipher's own deterministic pseudo-random number generator.
    ///
    /// Replaces <see cref="System.Random"/> as the engine's sole source of randomness because
    /// System.Random's algorithm is undocumented and not guaranteed to be identical across .NET
    /// versions/platforms — unacceptable for a proof-of-play system where the same seed must
    /// always reproduce the exact same run for verification.
    ///
    /// Algorithm: xoshiro256** (public-domain, David Blackman &amp; Sebastiano Vigna,
    /// https://prng.di.unimi.it/xoshiro256starstar.c) seeded via splitmix64 (public-domain,
    /// https://prng.di.unimi.it/splitmix64.c) to expand a single 64-bit seed into the
    /// generator's 256-bit state, per the reference authors' recommended seeding method.
    /// Implemented from the reference algorithms — no external packages.
    /// </summary>
    public sealed class DeterministicRng
    {
        private ulong _s0;
        private ulong _s1;
        private ulong _s2;
        private ulong _s3;

        /// <summary>Seeds the generator with the full 64-bit seed value.</summary>
        public DeterministicRng(long seed) : this(unchecked((ulong)seed))
        {
        }

        /// <summary>Seeds the generator with the full 64-bit seed value.</summary>
        public DeterministicRng(ulong seed)
        {
            // splitmix64 seed expansion: deterministically fills the 256-bit xoshiro256**
            // state from a single 64-bit seed (recommended by the xoshiro reference authors).
            ulong state = seed;
            _s0 = SplitMix64Next(ref state);
            _s1 = SplitMix64Next(ref state);
            _s2 = SplitMix64Next(ref state);
            _s3 = SplitMix64Next(ref state);
        }

        private static ulong SplitMix64Next(ref ulong x)
        {
            x += 0x9E3779B97F4A7C15UL;
            ulong z = x;
            z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9UL;
            z = (z ^ (z >> 27)) * 0x94D049BB133111EBUL;
            return z ^ (z >> 31);
        }

        private static ulong RotateLeft(ulong x, int k) => (x << k) | (x >> (64 - k));

        /// <summary>Returns the next raw 64-bit output of the xoshiro256** generator.</summary>
        public ulong NextUInt64()
        {
            ulong result = RotateLeft(_s1 * 5, 7) * 9;

            ulong t = _s1 << 17;

            _s2 ^= _s0;
            _s3 ^= _s1;
            _s1 ^= _s2;
            _s0 ^= _s3;

            _s2 ^= t;

            _s3 = RotateLeft(_s3, 45);

            return result;
        }

        /// <summary>
        /// Returns a non-negative random integer in [0, maxExclusive).
        /// Semantics match <see cref="System.Random.Next(int)"/>.
        /// </summary>
        public int Next(int maxExclusive)
        {
            if (maxExclusive <= 0)
                throw new ArgumentOutOfRangeException(nameof(maxExclusive), "maxExclusive must be positive.");

            // Multiply-shift bounded range reduction (Lemire-style, biasless enough for
            // gameplay use): use the high 32 bits of a 64-bit draw so the result stays
            // deterministic and platform-independent.
            return (int)(((NextUInt64() >> 32) * (uint)maxExclusive) >> 32);
        }

        /// <summary>
        /// Returns a random integer in [minInclusive, maxExclusive).
        /// Semantics match <see cref="System.Random.Next(int, int)"/>.
        /// </summary>
        public int Next(int minInclusive, int maxExclusive)
        {
            if (minInclusive >= maxExclusive)
                throw new ArgumentOutOfRangeException(nameof(maxExclusive), "maxExclusive must be greater than minInclusive.");

            uint range = (uint)((long)maxExclusive - minInclusive);
            uint result = (uint)(((NextUInt64() >> 32) * range) >> 32);
            return minInclusive + (int)result;
        }
    }
}
