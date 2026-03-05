# Terminal Simulation - Termux (Android) Run Guide

If you wish to test this cryptographic terminal simulation on an Android device via Termux, the easiest method is to use the pre-compiled Linux-ARM64 (aarch64) binaries.

Because the NativeAOT compiler and Rust might encounter complex issues building natively on Termux's local environment (Bionic libc), we recommend running the simulation within a `proot-distro` Ubuntu environment.

## Step-by-Step Setup

### 1. Termux Installation & Preparation
Download the latest version of Termux via F-Droid (The Google Play version is deprecated).

Open Termux and run the following commands to install the base packages:

`pkg update && pkg upgrade -y`
`pkg install proot-distro wget tar -y`

### 2. Setting Up the Ubuntu Environment
To ensure smooth execution of standard Linux (glibc) binaries, we create an Ubuntu environment inside Termux:

`proot-distro install ubuntu`
`proot-distro login ubuntu`

*(You are now in a terminal as `root@localhost`.)*

### 3. Installing Dependencies
While inside the Ubuntu environment, install the required base libraries:

`apt update`
`apt install wget unzip libgcc-s1 libstdc++6 libc6 -y`

### 4. Downloading and Running the Simulation
*(Note: You can download the compiled linux-arm64 files from the GitHub Actions Artifacts or the Releases page of your repository).*

`mkdir -p ~/terminal-sim && cd ~/terminal-sim`

Ensure the proprietary engine library (`libCrawlCipher.Core.so`) is in the same directory as the executable. Grant execution permissions:
`chmod +x crawlcipher`

Launch the Interface:
`./crawlcipher`

---

## 🛠️ Alternative: Compiling Directly on Termux

If you *must* compile the Rust frontend or the proprietary core from source on your phone, you should do so inside the `proot-distro ubuntu` environment (not on the main Termux screen). Install the necessary SDKs (.NET 8, Rustup via standard curl commands) and run the build scripts from this repository.

This method may take a significant amount of time depending on your phone's processor, but it provides a completely independent development environment!
