<div align="center">
<img width="128" height="128" alt="icon" src="https://github.com/user-attachments/assets/7d5c0438-859d-4b55-822a-dcd5290941b3" />
</div>

# cmatrix (Rust port for Windows)

A Rust rewrite of [cmatrix 2.0](https://github.com/abishekvashok/cmatrix) for the
Windows terminal. Same options, same key bindings, same animation logic.
Run `cmatrix.exe` with no arguments and the green matrix rain starts.

## Install

There is no installer - `cmatrix.exe` is a single portable file.

**Option A: download a release (no Rust needed)**

1. Open the repository's **Releases** page on GitHub.
2. Download `cmatrix.exe` from the newest release.
3. Run it from any terminal:

       cmatrix.exe

**Option B: build it yourself** - see [Build](#build) below.

**Run it as just `cmatrix` from any folder**

1. Put `cmatrix.exe` in a folder such as `C:\Tools`.
2. Add that folder to your PATH (PowerShell):

       [Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\Tools", "User")

3. Open a new terminal and type `cmatrix`.

## Quick usage

    cmatrix                 default green matrix rain
    cmatrix -b -u 2         bold characters, faster
    cmatrix -C red          red rain
    cmatrix -r              rainbow mode
    cmatrix -s              screensaver mode (exits on any key)
    cmatrix -h              full option list

Press `q` to quit. Full options and keys are listed in the tables below.

## Build

1. Install Rust from https://rustup.rs (accept the defaults; if asked, install the
   "Visual Studio C++ Build Tools" - they are required for the MSVC toolchain).
2. Open a **new** terminal in this folder and run:

       build.bat

3. Your exe is at `dist\cmatrix.exe`  (also `target\release\cmatrix.exe`).

Or manually: `cargo build --release`

## Usage

    cmatrix -[abBcfhlLnorsmxV] [-u delay] [-C color]

| Option | Meaning |
|-------:|---------|
| -a | Asynchronous scroll |
| -b | Bold characters on |
| -B | All bold characters (overrides -b) |
| -c | Japanese characters (needs a font that has them) |
| -f | Force linux TERM (accepted, no effect on Windows) |
| -l | Linux console font mode (accepted, no effect on Windows) |
| -L | Lock mode (q / Ctrl+C ignored) |
| -o | Old-style scrolling |
| -n | No bold (overrides -b and -B, default) |
| -s | Screensaver mode, exits on first keystroke |
| -x | X window mode (uses the extended character range) |
| -u N | Update delay 0-10 (default 4) |
| -C color | green, red, blue, white, yellow, cyan, magenta, black |
| -r | Rainbow mode |
| -m | Lambda mode |
| -h / -V | Help / version |

## Keys while running

| Key | Action |
|-----|--------|
| q | Quit (ignored in lock mode) |
| a | Toggle asynchronous scroll |
| b / B / n | Bold some / bold all / bold off |
| 0-9 | Update delay |
| ! @ # $ % ^ & | Red, green, yellow, blue, magenta, cyan, white |
| r | Rainbow mode |
| m | Toggle lambda mode |
| p / P | Pause |
| L | Lock |
| Ctrl+C | Quit (ignored in lock mode) |

For best results use Windows Terminal (or any VT-capable console, Windows 10+).

