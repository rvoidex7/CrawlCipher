using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace CrawlCipher.Core
{
    /// <summary>
    /// FFI export layer. All functions here are exported as unmanaged C functions
    /// that Rust calls via #[link(name = "CrawlCipher.Core")] extern "C" { ... }
    /// 
    /// Detailed specifications:
    /// - Local memory model: [Memory-and-FFI-Bridge.md](../docs/r7/Development/Memory-and-FFI-Bridge.md)
    /// - Online wiki page: https://rvoidex7.github.io/r7notes/Github-Projects/Memory-and-FFI-Bridge
    /// </summary>
    public static class FFIExports
    {
        // Store game instances keyed by their GCHandle IntPtr
        // For simplicity, we use a single static instance since there's only one game

        /// <summary>
        /// Allocates the managed GameEngine instance and anchors it on the heap via a pinned GCHandle.
        /// Returns the raw IntPtr representation to Rust.
        /// 
        /// See: [Memory-and-FFI-Bridge.md](../docs/r7/Development/Memory-and-FFI-Bridge.md#1-managed-lifetime-control-gchandle)
        /// See: https://rvoidex7.github.io/r7notes/Github-Projects/Memory-and-FFI-Bridge
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "CreateGame")]
        public static unsafe IntPtr CreateGame(
            long seed, IntPtr namePtr,
            int gridW, int gridH, int foodCount, int enableWalls, 
            int maxEnergy, int energyGain, int turnCost45, int turnCost90, int turnCostSharp)
        {
            try
            {
                string playerName = Marshal.PtrToStringAnsi(namePtr) ?? "Unknown";
                var config = new GameConfig
                {
                    GridWidth = gridW,
                    GridHeight = gridH,
                    FoodCount = foodCount,
                    EnableWalls = enableWalls != 0,
                    MaxEnergy = maxEnergy,
                    EnergyGainPerMove = energyGain,
                    Turn45Cost = (byte)turnCost45,
                    Turn90Cost = (byte)turnCost90,
                    TurnSharpCost = (byte)turnCostSharp
                };
                var engine = new GameEngine(config, seed, playerName);
                var handle = GCHandle.Alloc(engine);
                return GCHandle.ToIntPtr(handle);
            }
            catch
            {
                return IntPtr.Zero;
            }
        }

        /// <summary>
        /// Releases the GCHandle pinning on the GameEngine, allowing the GC to collect it.
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "DestroyGame")]
        public static unsafe void DestroyGame(IntPtr gamePtr)
        {
            if (gamePtr == IntPtr.Zero) return;
            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                handle.Free();
            }
            catch { }
        }

        [UnmanagedCallersOnly(EntryPoint = "Update")]
        public static unsafe void Update(IntPtr gamePtr)
        {
            if (gamePtr == IntPtr.Zero) return;
            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                engine.Tick();
            }
            catch { }
        }

        [UnmanagedCallersOnly(EntryPoint = "GetSimulationState")]
        public static unsafe SimulationStateFFI GetSimulationState(IntPtr gamePtr)
        {
            if (gamePtr == IntPtr.Zero)
                return default;

            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                return engine.GetSimulationState();
            }
            catch
            {
                return default;
            }
        }

        [UnmanagedCallersOnly(EntryPoint = "ProcessInput")]
        public static unsafe void ProcessInput(IntPtr gamePtr, int inputType, int param1, int param2)
        {
            if (gamePtr == IntPtr.Zero) return;
            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                engine.ProcessInput(inputType, param1, param2);
            }
            catch { }
        }

        [UnmanagedCallersOnly(EntryPoint = "GetPlayerState")]
        public static unsafe PlayerStateFFI GetPlayerState(IntPtr gamePtr, int playerId)
        {
            if (gamePtr == IntPtr.Zero)
                return new PlayerStateFFI { Id = -1 };

            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                return engine.GetPlayerState(playerId);
            }
            catch
            {
                return new PlayerStateFFI { Id = -1 };
            }
        }

        [UnmanagedCallersOnly(EntryPoint = "GetGridCells")]
        public static unsafe int GetGridCells(
            IntPtr gamePtr,
            CellInfoFFI* buffer,
            int bufferSize,
            int viewX, int viewY,
            int viewWidth, int viewHeight)
        {
            if (gamePtr == IntPtr.Zero || buffer == null)
                return -1;

            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                return engine.GetGridCells(buffer, bufferSize, viewX, viewY, viewWidth, viewHeight);
            }
            catch
            {
                return -1;
            }
        }

        [UnmanagedCallersOnly(EntryPoint = "GetBackpack")]
        public static unsafe int GetBackpack(IntPtr gamePtr, int playerId, InventoryItemFFI* buffer, int bufferSize)
        {
            if (gamePtr == IntPtr.Zero || buffer == null) return -1;
            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                var items = engine.GetPlayerBackpack(playerId);
                int count = Math.Min(items.Count, bufferSize);

                for (int i = 0; i < count; i++)
                {
                    buffer[i].Type = (int)items[i].Type;
                    buffer[i].Durability = items[i].Durability;

                    var idBytes = System.Text.Encoding.ASCII.GetBytes(items[i].Id);
                    int idLen = Math.Min(idBytes.Length, 36);
                    for (int j = 0; j < idLen; j++) buffer[i].Id[j] = idBytes[j];
                    buffer[i].Id[idLen] = 0;

                    var codeBytes = System.Text.Encoding.ASCII.GetBytes(items[i].AssetCode);
                    int codeLen = Math.Min(codeBytes.Length, 15);
                    for (int j = 0; j < codeLen; j++) buffer[i].AssetCode[j] = codeBytes[j];
                    buffer[i].AssetCode[codeLen] = 0;
                }
                return count;
            }
            catch { return -1; }
        }

        [UnmanagedCallersOnly(EntryPoint = "GetEquippedItems")]
        public static unsafe int GetEquippedItems(IntPtr gamePtr, int playerId, InventoryItemFFI* buffer, int bufferSize)
        {
            if (gamePtr == IntPtr.Zero || buffer == null) return -1;
            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                var items = engine.GetPlayerWeapons(playerId);
                int count = Math.Min(items.Count, bufferSize);

                for (int i = 0; i < count; i++)
                {
                    buffer[i].Type = (int)items[i].Type; // Assuming enum values match or mapping works
                    buffer[i].Durability = items[i].Ammo;

                    var idBytes = System.Text.Encoding.ASCII.GetBytes(string.IsNullOrEmpty(items[i].ItemId) ? "" : items[i].ItemId);
                    int idLen = Math.Min(idBytes.Length, 36);
                    for (int j = 0; j < idLen; j++) buffer[i].Id[j] = idBytes[j];
                    buffer[i].Id[idLen] = 0;

                    var codeStr = items[i].Type.ToString().ToUpper();
                    if (items[i].Type == WeaponType.None) codeStr = "";
                    var codeBytes = System.Text.Encoding.ASCII.GetBytes(codeStr);
                    int codeLen = Math.Min(codeBytes.Length, 15);
                    for (int j = 0; j < codeLen; j++) buffer[i].AssetCode[j] = codeBytes[j];
                    buffer[i].AssetCode[codeLen] = 0;
                }
                return count;
            }
            catch { return -1; }
        }

        [UnmanagedCallersOnly(EntryPoint = "EquipItemFFI")]
        public static unsafe int EquipItemFFI(IntPtr gamePtr, int playerId, IntPtr itemIdPtr, int segmentIndex, int side)
        {
            if (gamePtr == IntPtr.Zero) return 0;
            try
            {
                string itemId = Marshal.PtrToStringAnsi(itemIdPtr) ?? "";
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                return engine.EquipItem(playerId, itemId, segmentIndex, (WeaponSide)side) ? 1 : 0;
            }
            catch { return 0; }
        }

        [UnmanagedCallersOnly(EntryPoint = "UnequipItemFFI")]
        public static unsafe int UnequipItemFFI(IntPtr gamePtr, int playerId, int segmentIndex)
        {
            if (gamePtr == IntPtr.Zero) return 0;
            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                return engine.UnequipItem(playerId, segmentIndex) ? 1 : 0;
            }
            catch { return 0; }
        }

        [UnmanagedCallersOnly(EntryPoint = "SwapItemsFFI")]
        public static unsafe int SwapItemsFFI(IntPtr gamePtr, int playerId, int idxA, int idxB)
        {
            if (gamePtr == IntPtr.Zero) return 0;
            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                return engine.SwapItems(playerId, idxA, idxB) ? 1 : 0;
            }
            catch { return 0; }
        }

        /// <summary>
        /// Generates the SHA-256 Session Verification Hash representing the proof of play.
        /// 
        /// See: [Anti-Cheat-Verification.md](../docs/r7/Development/Anti-Cheat-Verification.md#4-replay-verification--state-hashing)
        /// See: https://rvoidex7.github.io/r7notes/Github-Projects/Anti-Cheat-Verification
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "GetReplayHash")]
        public static unsafe void GetReplayHash(IntPtr gamePtr, IntPtr buffer, int bufferSize)
        {
            if (gamePtr == IntPtr.Zero || buffer == IntPtr.Zero) return;
            try
            {
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                string hash = engine.GetReplayHash();

                // Copy string to buffer (null terminated)
                var bytes = System.Text.Encoding.ASCII.GetBytes(hash);
                var ptr = (byte*)buffer;
                int len = Math.Min(bytes.Length, bufferSize - 1);

                for (int i = 0; i < len; i++) ptr[i] = bytes[i];
                ptr[len] = 0;
            }
            catch { }
        }

        [UnmanagedCallersOnly(EntryPoint = "SetGameModeFFI")]
        public static unsafe void SetGameModeFFI(IntPtr gamePtr, IntPtr modePtr, IntPtr puzzleIdPtr)
        {
            if (gamePtr == IntPtr.Zero) return;
            try
            {
                string mode = Marshal.PtrToStringAnsi(modePtr) ?? "Expedition";
                string puzzleId = Marshal.PtrToStringAnsi(puzzleIdPtr) ?? "";
                var handle = GCHandle.FromIntPtr(gamePtr);
                var engine = (GameEngine)handle.Target!;
                engine.SetGameMode(mode, puzzleId);
            }
            catch { }
        }
    }
}
