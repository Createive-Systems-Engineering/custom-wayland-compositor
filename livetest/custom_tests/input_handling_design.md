# Design: Group 3 - Input Handling Test Applications

## 1. Introduction

The purpose of this group of test applications is to validate the compositor's handling and dispatch of fundamental pointer (mouse) and keyboard input events. This includes verifying correct event delivery to client surfaces, accurate coordinate reporting, keymap handling, key state and modifier interpretation, and focus management based on user interaction.

This test group primarily covers the following Wayland protocols:
-   `wl_seat`: For acquiring input devices.
-   `wl_pointer`: For pointer (mouse) events like motion, button clicks, and axis (scroll).
-   `wl_keyboard`: For keyboard events like key presses, releases, keymaps, and modifier states.
-   `wl_surface`: As the target for input events.
-   `xdg_shell` (`xdg_wm_base`, `xdg_surface`, `xdg_toplevel`): For creating a basic window to receive input.

Optionally, if time and compositor support permits, these tests could be extended or supplemented with clients for:
-   `zwp_relative_pointer_v1`: For unaccelerated, relative pointer motion.
-   `zwp_pointer_constraints_v1`: For pointer locking and confinement.

Test applications will be developed in Rust, using the `wayland-client` crate. Their primary function will be to create a simple window and meticulously log received input events to `stdout` for analysis.

## 2. General Test Application Structure

Each test application in this group will:

-   **Create a Simple Window:** Utilize SHM (Shared Memory) to create a basic `xdg_toplevel` window (e.g., a rectangle filled with a solid color). This window will serve as the target for input events.
-   **Instantiate Input Devices:** Connect to the Wayland display, bind to `wl_seat` (typically version 7 or higher for detailed events), and then get the `wl_pointer` and/or `wl_keyboard` objects.
-   **Log Events:** Implement event handlers for the relevant input objects. When an event is received, log its details to `stdout` in a clear, consistent, and potentially parseable format (e.g., `EVENT_NAME: param1=value, param2=value`).
-   **User Guidance:** Print instructions to `stdout` guiding the user on how to interact with the window to trigger specific events (e.g., "Move mouse into the window", "Press any key").
-   **Exit Conditions:** Run for a predetermined duration, until a specific number of events are captured, or until a designated quit action (e.g., pressing 'Q' or 'Esc'). Exit with code `0` on successful completion (all expected interactions logged correctly), and non-zero on error or if critical events are missed.

## 3. Test Application Designs

### 3.1. Test App: `pointer_events_logger`

-   **Purpose:** To verify the compositor's correct dispatch of various `wl_pointer` events to a client window.
-   **Protocols Used:** `wl_seat` (v7+ recommended for frame events), `wl_pointer`, `wl_surface`, `xdg_shell`.
-   **Steps:**
    1.  Create a basic SHM window (e.g., 400x300). Display its dimensions and mark a central point or quadrants visually or in logs for coordinate reference.
    2.  Bind to `wl_seat` and get the `wl_pointer` capability.
    3.  Implement and attach event handlers for `wl_pointer`:
        -   `enter(serial, surface, surface_x, surface_y)`: Log all parameters. `surface` should match the test window's `wl_surface` ID.
        -   `leave(serial, surface)`: Log all parameters. `surface` should match.
        -   `motion(time, surface_x, surface_y)`: Log all parameters. To avoid excessive output, consider logging only every Nth event, when coordinates change by a certain delta, or simply all events for a short test.
        -   `button(serial, time, button, state)`: Log all parameters. `button` codes (e.g., from `input-event-codes.h` like `BTN_LEFT 0x110`) and `state` (0 for released, 1 for pressed) should be clearly logged.
        -   `axis(time, axis, value)`: Log all parameters. `axis` (0 for vertical, 1 for horizontal) and `value` (scroll delta) are key.
        -   Optional: `frame()`: Log when received. Useful for grouping discrete events.
        -   Optional: `axis_source(axis_source)`: Log the source of axis events (e.g., wheel, finger).
        -   Optional: `axis_stop(time, axis)`: Log when an axis scroll sequence is considered stopped.
        -   Optional: `axis_discrete(axis, discrete)`: Log discrete (non-continuous) axis steps.
    4.  Print instructions to `stdout`: "Please move your mouse pointer into the test window. Click left, middle, right buttons. Scroll vertically and horizontally. Then move the pointer out of the window."
    5.  Run the Wayland event loop for a specified duration (e.g., 20-30 seconds) or until a quit signal (e.g., if a keyboard logger is also active and 'q' is pressed).
    6.  Cleanly destroy resources.
-   **Success Criteria:**
    -   `wl_pointer.enter` event is logged when the pointer enters the client window's surface, with correct surface-local coordinates.
    -   `wl_pointer.motion` events are logged with surface-local coordinates updating correctly as the mouse moves within the window.
    -   `wl_pointer.button` events are logged with the correct button codes (e.g., BTN_LEFT, BTN_RIGHT, BTN_MIDDLE) and states (pressed/released) upon mouse clicks.
    -   `wl_pointer.axis` events are logged with appropriate axis and value when the scroll wheel is used.
    -   `wl_pointer.leave` event is logged when the pointer leaves the client window's surface.
    -   (If implemented) `frame` events group related pointer events. Other optional axis events are logged as expected.
    -   No Wayland protocol errors are reported by the client or compositor. Application exits cleanly.
-   **Parameters (CLI):**
    -   `--duration <seconds>` (e.g., `30`)

### 3.2. Test App: `keyboard_events_logger`

-   **Purpose:** To verify the compositor's correct dispatch of `wl_keyboard` events, including keymap handling, key presses/releases, and modifier states.
-   **Protocols Used:** `wl_seat` (v7+ recommended), `wl_keyboard`, `wl_surface`, `xdg_shell`.
-   **Steps:**
    1.  Create a basic SHM window.
    2.  Bind to `wl_seat` and get the `wl_keyboard` capability.
    3.  Implement and attach event handlers for `wl_keyboard`:
        -   `keymap(format, fd, size)`: Log format (should be `KEYMAP_FORMAT_XKB_V1`), fd, and size. For robust testing, the client should `mmap` this `fd`, compile the keymap using `xkbcommon` (e.g., via `xkbcommon-rs`), and set up an `xkb_state`. For a minimal test, just logging the event's reception is a first step.
        -   `enter(serial, surface, keys)`: Log serial and surface ID. The `keys` array (raw scancodes of currently pressed keys) should also be logged.
        -   `leave(serial, surface)`: Log serial and surface ID.
        -   `key(serial, time, key, state)`: Log serial, time, raw `key` scancode, and `state` (0 for released, 1 for pressed). If `xkbcommon` is integrated, attempt to convert the scancode + current modifiers to a keysym (e.g., `XKB_KEY_A`, `XKB_KEY_Enter`) and UTF-8 string, and log these as well.
        -   `modifiers(serial, mods_depressed, mods_latched, mods_locked, group)`: Log all parameters. These values represent the state of modifier keys like Shift, Ctrl, Alt, Super. Update `xkb_state` with these if using `xkbcommon`.
        -   Optional: `repeat_info(rate, delay)`: Log keyboard repeat rate and delay.
    4.  Print instructions to `stdout`: "Please click on the test window to give it keyboard focus. Type various keys (e.g., 'hello', numbers, Enter, Esc). Hold Shift and type. Hold Ctrl and type. Then click outside the window to remove focus."
    5.  Run the Wayland event loop for a specified duration or until a specific quit key (e.g., 'Esc' if it can be reliably detected and not grabbed by compositor) is pressed and logged.
    6.  Cleanly destroy resources, including unmapping the keymap if applicable.
-   **Success Criteria:**
    -   `wl_keyboard.keymap` event is received shortly after keyboard creation, providing a valid fd.
    -   `wl_keyboard.enter` event is logged when the client window gains keyboard focus, along with any keys already pressed.
    -   `wl_keyboard.key` events are logged for each key press and release, with correct scancodes and states. If `xkbcommon` is used, keysyms and UTF-8 output should match expected values.
    -   `wl_keyboard.modifiers` events are logged correctly when modifier keys (Shift, Ctrl, Alt, etc.) are pressed or released, and the reported depressed/latched/locked/group values are accurate.
    -   `wl_keyboard.leave` event is logged when the client window loses keyboard focus.
    -   No Wayland protocol errors. Application exits cleanly.
-   **Parameters (CLI):**
    -   `--duration <seconds>` (e.g., `30`)
    -   `--use_xkbcommon` (optional flag, if `xkbcommon` integration is conditional)

### 3.3. Test App (Optional Stretch): `relative_pointer_test`

-   **Purpose:** To test the compositor's support for unaccelerated, relative pointer motion via `zwp_relative_pointer_v1`. This is often used by games or remote desktop applications.
-   **Protocols Used:** `zwp_relative_pointer_manager_v1`, `zwp_relative_pointer_v1`, `wl_pointer`, `wl_seat`, `wl_surface`, `xdg_shell`.
-   **Steps:**
    1.  Create a basic window.
    2.  Bind to `wl_seat` and get `wl_pointer`.
    3.  Bind to `zwp_relative_pointer_manager_v1` global. If not available, log and exit gracefully.
    4.  Get a `zwp_relative_pointer_v1` object for the `wl_pointer` using `get_relative_pointer`.
    5.  Implement and attach event handler for `zwp_relative_pointer_v1.relative_motion(utime_hi, utime_lo, dx, dy, dx_unaccel, dy_unaccel)`: Log all parameters, especially `dx_unaccel` and `dy_unaccel`.
    6.  Print instructions to `stdout`: "Move the mouse rapidly within this window. Relative motion events will be logged."
    7.  Run event loop for a duration.
-   **Success Criteria:**
    -   If the protocol is supported, `relative_motion` events are received when the mouse is moved while the pointer is over the associated surface (especially if combined with pointer lock).
    -   `dx_unaccel` and `dy_unaccel` values reflect the raw, unaccelerated device delta.
    -   No Wayland protocol errors.
-   **Parameters (CLI):** `--duration <seconds>`

### 3.4. Test App (Optional Stretch): `pointer_constraints_test`

-   **Purpose:** To test basic pointer locking and confinement capabilities using `zwp_pointer_constraints_v1`. This is also crucial for games and certain professional tools.
-   **Protocols Used:** `zwp_pointer_constraints_v1`, `zwp_locked_pointer_v1` (or `zwp_confined_pointer_v1`), `wl_pointer`, `wl_seat`, `wl_surface`, `xdg_shell`.
-   **Steps:**
    1.  Create a basic window.
    2.  Bind to `wl_seat` and get `wl_pointer`.
    3.  Bind to `zwp_pointer_constraints_v1` global. If not available, log and exit gracefully.
    4.  Request pointer lock: Call `zwp_pointer_constraints_v1.lock_pointer(...)` to get a `zwp_locked_pointer_v1` object.
    5.  Implement handlers for `zwp_locked_pointer_v1`:
        -   `locked`: Log event. Pointer is now locked.
        -   `unlocked`: Log event. Pointer is no longer locked (e.g., compositor broke lock, or client requested unlock).
    6.  Once the `locked` event is received, print instructions: "Pointer is now locked. Move the mouse. Observe if it's confined. Press 'U' to request unlock."
    7.  (If `relative_pointer_test` logic is active or combined) Observe if `relative_motion` events are now being received, as absolute motion might be suppressed or altered.
    8.  If a key press ('U') is detected (requires `wl_keyboard` setup), call `zwp_locked_pointer_v1.destroy()` to request unlock.
    9.  Run event loop.
-   **Success Criteria:**
    -   If the protocol is supported, the `locked` event is received after a lock request.
    -   Visually (or through logged pointer events), the pointer should be confined to the window boundaries or its movement significantly altered/stopped at edges.
    -   If combined with relative pointer, `relative_motion` events should be predominant during lock.
    -   The `unlocked` event is received after requesting unlock (destroying `zwp_locked_pointer_v1`) or if the compositor revokes the lock.
    -   No Wayland protocol errors.
-   **Parameters (CLI):** `--duration <seconds>`

## 4. Common Considerations

-   **Event Logging Format:** A standardized, easily parseable format for logging events is crucial for analysis. For example: `[<timestamp_ms>] <EVENT_SOURCE>.<EVENT_NAME>: serial=<val>, surface=<id>, param1=<val1>, param2=<val2_hex>`.
-   **Focus Management:** The test applications should print clear messages to `stdout` instructing the user when they need to ensure a test window has pointer or keyboard focus for the events to be correctly routed and captured.
-   **`xkbcommon` Integration:** For `keyboard_events_logger`, using a library like `xkbcommon-rs` (Rust bindings for `libxkbcommon`) is highly recommended for accurate keysym and UTF-8 string interpretation from raw scancodes and modifier states. Initial versions might skip this and just log raw scancodes, but full validation benefits from `xkbcommon`.
-   **Compositor State:** These tests assume the compositor is in a default state. Interactions with other specialized protocols (e.g., layer shell creating an overlay that might steal input) are out of scope for these specific tests unless explicitly combined.
-   **Resource Cleanup:** All Wayland objects must be properly destroyed upon test completion or error.

This design document will guide the development of the Group 3 custom test applications. Results and detailed findings will be documented in `livetest/custom_tests/input_handling_results.md`.
