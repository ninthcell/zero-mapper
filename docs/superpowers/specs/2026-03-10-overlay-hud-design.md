# Overlay HUD Design

Semi-transparent on-screen overlay that shows controller button layout with active key mappings, pressed button visualization, and dynamic combo layer switching.

## Window

- Style: `WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW` + `WS_POPUP`
- Click-through, always on top, no taskbar entry, no title bar
- Rendering: 32bpp ARGB DIB + `UpdateLayeredWindow` for per-pixel alpha (semi-transparent background)
- Position: configurable corner via `overlay_position` config key
  - Values: `top-left`, `top-right`, `bottom-left`, `bottom-right` (default: `bottom-right`)
  - Placed relative to monitor work area (`SystemParametersInfoW` SPI_GETWORKAREA) with margin

## Rendering

- GDI drawing to memory DC, `UpdateLayeredWindow` once per frame
- Only redraws when `pressed_buttons` changes from previous tick
- GDI resources (fonts, brushes, pens) cached in AppState, not recreated per frame

### Layout (Option A: labels below buttons)

```
        [LB]                    [RB]

         [U]                     [X]
    [L]       [R]           [Y]       [A]
         [D]                     [B]
        Ctrl+Z  Ctrl+Y         Spc
         B         C-Z          P
                                 E

                [Start]
                 C-S

            Clip Studio Paint
```

- Dpad (left): cross layout, 4 directional buttons
- Face buttons (right): diamond layout, ABXY
- LB/RB: top row, shoulder buttons
- Start: bottom center
- Each button: rounded rectangle with button name inside
- Key label: separate row below each button, blue color (#6af), bold
- Profile name: bottom of overlay, dimmed text

### Colors

| State | Button BG | Button text | Key label |
|-------|-----------|-------------|-----------|
| Default (mapped) | #333 | #aaa | #6af bold |
| Default (unmapped) | #333 | #aaa | (none) |
| Pressed | #3a6aba + bright border | #fff | #fff bold |
| Dimmed (no combo) | #222 | #555 | (none) |
| Modifier held | #2a4a8a + glow border | #8bf | — |

## State Machine

```
pressed_buttons.len() == 0:
  → Default view: each button shows its solo mapping

pressed_buttons.len() == 1:
  → Combo layer view:
    - The single held button = "modifier", shown highlighted
    - For every other button, look up modifier+button combo mapping
    - Buttons with a combo mapping: show combo key label
    - Buttons without a combo mapping: dim out
    - If the modifier button itself has a solo mapping, don't show it
      (user is holding it as a modifier, not using it solo)

pressed_buttons.len() >= 2:
  → Default view: combo has already been triggered, show solo mappings
  → Pressed buttons shown with glow effect
```

This generalizes modifier detection to ANY button, not just LB/RB. If a user holds Y (which might be a hold-mode mapping), the overlay switches to show Y+X combos if any exist.

## Config Changes

```toml
# New fields in root config
overlay = true                     # Show overlay (default: false)
overlay_position = "bottom-right"  # top-left / top-right / bottom-left / bottom-right
```

Added to `CompiledConfig`:
```rust
pub overlay: bool,
pub overlay_position: OverlayPosition,
```

```rust
enum OverlayPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
```

## Tray Menu

Add toggle item to existing context menu:
- "Show overlay" / "Hide overlay" depending on current state
- Runtime toggle independent of config value
- App restart restores config value

Menu item ID: `ID_TOGGLE_OVERLAY`

## Performance Constraints

- Redraw ONLY when `pressed_buttons` differs from previous tick's snapshot
- When overlay is off: `ShowWindow(SW_HIDE)` — zero rendering cost
- When no profile matched (no active app): hide overlay
- When app is suspended (sleep/lock): hide overlay
- GDI resources cached, not allocated per frame
- No additional timer — piggyback on existing controller polling tick

## File Structure

- `src/overlay.rs` — new module
  - Window creation/destruction
  - Layout calculation (button positions, label positions)
  - GDI rendering (draw buttons, labels, highlight states)
  - Show/hide/reposition logic
  - Key label formatting (e.g., "Ctrl+Z" → "C-Z" short form)
- `src/main.rs` — integration
  - AppState: add overlay window handle, previous pressed_buttons snapshot, overlay_visible flag
  - `tick_controller`: after resolving mappings, call overlay update if buttons changed
  - `WM_CREATE`: create overlay window if config.overlay is true
  - Tray menu: add overlay toggle
- `src/config.rs` — add overlay, overlay_position fields to RawConfig and CompiledConfig

## Windows Crate Features Needed

- `Win32_Graphics_Gdi` (already present) — CreateCompatibleDC, SelectObject, CreateFont, etc.
- May need additional GDI functions but the feature flag is already enabled
