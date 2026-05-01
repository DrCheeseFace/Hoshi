# Hoshi

A simple SGF viewer/editor and GUI for KataGo AI to analyze one's Go games.

<img width="1736" height="1249" alt="Hoshi-Screenshot" src="https://github.com/user-attachments/assets/7ef0ec09-87d2-40cf-b88f-eacf38c32c20" />


## Features
* **Markup & Analysis:** Full suite of SGF markup tools, local KataGo evaluation bubbles, and score estimation graphs.
* **Interactive Stone Editing:** Right-click a stone on the board (or a node in the tree) to pick it up and move it. Hoshi will automatically recalculate the timelines and board states for all future variations. 

## KataGo Engine
* Comes bundled with **KataGo OpenCL**, which should run on most devices with no problems.
* Alternatively, you can download the standalone `.exe` version and provide your own KataGo engine, network, and configuration files via the Options menu.

## How to Download
1. Go to the [Releases page](https://github.com/super-matako/Hoshi/releases/tag/v1.1.0).
2. Download the zip file titled `Hoshi-KataGo.zip` for the version bundled with KataGo, or `Hoshi.exe` for the standalone version.

## Keyboard Shortcuts

### Navigation & Tree Control
* `Left Arrow` - Move back 1 move
* `Right Arrow` - Move forward 1 move
* `Shift` + `Left Arrow` / `Page Up` - Move back 15 moves
* `Shift` + `Right Arrow` / `Page Down` - Move forward 15 moves
* `Ctrl` + `Left Arrow` / `Home` - Jump to the start of the game (Root)
* `Ctrl` + `Right Arrow` / `End` - Jump to the very end of the current branch
* `Up Arrow` - Switch to the previous alternative variation
* `Down Arrow` - Switch to the next alternative variation
* `Shift` + `Up Arrow` - Warp backward directly to the primary branch intersection

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

## Changelog (v1.1.0)

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
