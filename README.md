# Hoshi

A simple SGF viewer/editor and GUI for KataGo AI to analyze one's Go games.

<img width="1920" height="1043" alt="Hoshi-Screenshot" src="https://github.com/user-attachments/assets/a80e3dc7-d24f-48bd-a5fd-9e1d5dd97923" />

## Features
* **Markup & Analysis:** Full suite of SGF markup tools, local KataGo evaluation bubbles, and score estimation graphs.
* **Interactive Stone Editing:** Right-click a stone on the board (or a node in the tree) to pick it up and move it. Hoshi will automatically recalculate the timelines and board states for all future variations. 

## KataGo Engine
* Comes with an **Auto-setup** feature, that scans your system and recommends a KataGo engine version to download with the click of a button.
* Alternatively, you can always provide your own, custom KataGo files.

## How to Download
1. Go to the [Releases page](https://github.com/super-matako/Hoshi/releases/tag/v1.1.1).
2. Simply download `Hoshi.exe`.

## Keyboard Shortcuts

### Navigation & Tree Control
* `Left Arrow` - Move back 1 move
* `Right Arrow` - Move forward 1 move
* `Shift` + `Left Arrow` / `Page Up` - Move back 15 moves
* `Shift` + `Right Arrow` / `Page Down` - Move forward 15 moves
* `Ctrl` + `Left Arrow` / `Home` - Jump to the start of the game
* `Ctrl` + `Right Arrow` / `End` - Jump to the end of the current branch
* `Up Arrow` - Switch to the previous alternative variation
* `Down Arrow` - Switch to the next alternative variation
* `Shift` + `Up Arrow` - Warp backward directly to the primary branch intersection
* `Scroll wheel` - Move in navigation tree
* `Mouse back` / `Mouse forward` - Switch between alternative variations

### Tools & Markup
* `1` - Place Black stone
* `2` - Place White stone
* `3` - Alternate colors (Black/White)
* `Q` - Triangle markup (△)
* `W` - Square markup (□)
* `E` - Circle markup (○)
* `R` - Cross markup (✕)
* `A` - Alphabet markup (Press again to toggle case)
* `S` - Number markup
* `Z` - Eraser tool
* `Shift` + `Click` - Apply active markup tool to an entire group of connected stones

### Analysis & Game Actions
* `Spacebar` - Play / Pause KataGo Analysis
* `C` - Toggle Score Estimate calculation
* `Delete` / `Backspace` - Delete current node (and all future variations)
* `Ctrl` + `Z` - Undo (Stone placement, node deletion, or stone edit)
* `Ctrl` + `Y` - Redo
* `Escape` - Cancel active stone edit

### File Operations
* `Ctrl` + `N` - New Game
* `Ctrl` + `O` - Open SGF file
* `Ctrl` + `S` - Save current game
* `Ctrl` + `Shift` + `S` - Save As...

---

# Changelog

## v1.1.1
### Auto-setup feature
No more bundling with a KataGo folder - if you don't already have KataGo, there is now a nifty `Auto-setup` button in the Options menu, allowing you to easily download the appropriate KataGo version for your system.

### Wide-screen support
Space is used to a better extent, depending on monitor size.

### Hotkey editability
You can now set your own hotkeys.

## v1.1.0
### Moved to Tauri
Changing framework from Electron to Tauri has many advantages.
* **Reduced filesize:** The application has now shrunk from **75.2 MB** down to **11.5 MB!**
* **Truly portable .exe:** The application is a single, lightweight .exe that can be placed anywhere and be associated with .sgf files with no problems.
* **Increased cross-platform compatibility:** This will help bring about Mac, Linux and mobile versions in the near future.

### Interactive Editing
You can now move stones around after they have been placed.
* **Drag-and-drop stone editing:** Right-click any placed stone (or right-click its node in the tree) to enter Edit Mode. The stone will lift off the board and be able to be moved on any other intersection without messing up the rest of the tree.
* **Time-travelling stone editing:** You can edit a stone's placement from any place on the navigation tree. This makes it easier to edit earlier misplaced stones if you are transcribing a game and made a mistake.

### Global Action Engine
* **Undo/Redo:** `Ctrl+Z` and `Ctrl+Y` now undo/redo stone placements, branch deletions, and drag-and-drop spatial edits.

### Visual Update
* Slight visual changes to make things easier on the eyes.

## License
This project is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE) - see the LICENSE file for details.
