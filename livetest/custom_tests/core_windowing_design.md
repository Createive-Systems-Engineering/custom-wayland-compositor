# Design: Group 1 - Core Windowing & Basic Rendering Test Applications

## 1. Introduction

The purpose of this group of test applications is to rigorously validate the fundamental window creation, management, and rendering capabilities of the Wayland compositor, primarily focusing on interactions with Shared Memory (SHM) buffers. These tests form a baseline for verifying the compositor's core functionality before moving to more complex protocol interactions or real-world applications.

This test group primarily covers the following Wayland protocols:
-   `wl_compositor`: For creating surfaces.
-   `wl_shm`: For creating and managing shared memory pools and buffers.
-   `xdg_shell` (specifically `xdg_wm_base`, `xdg_surface`, `xdg_toplevel`): For managing toplevel window semantics.
-   `wl_surface`: The basic canvas for drawing.
-   `wp_viewporter`: For viewport-based scaling and cropping of surface content.

All test applications will be developed in Rust, leveraging the `wayland-client` crate for safe and idiomatic Wayland protocol interactions.

## 2. General Test Application Structure

Each test application within this group will adhere to the following structural conventions:

-   **Independent Executable:** Each test will be a small, self-contained Rust executable located within the `crates/test-clients/` directory.
-   **Wayland Connection:** Establish a connection to the Wayland display server upon startup.
-   **Global Instantiation:** Bind to necessary global Wayland objects, such as `wl_compositor`, `wl_shm`, `xdg_wm_base`, and `wp_viewporter` (if required by the specific test). Gracefully handle cases where a required global might be missing.
-   **Event Logging:** Log key Wayland events received from the compositor and important status updates or actions taken by the client to `stdout`. A common verbosity flag might be implemented.
-   **Exit Codes:** Exit with code `0` upon successful completion of all test steps and clean resource destruction. Exit with a non-zero code in case of errors, protocol violations, or unmet success criteria.

## 3. Test Application Designs

### 3.1. Test App: `shm_basic_window`

-   **Purpose:** To verify the most fundamental sequence of creating a simple toplevel window using an SHM buffer, displaying a solid color, and managing its basic lifecycle events.
-   **Protocols Used:** `wl_compositor`, `wl_shm`, `xdg_wm_base`, `xdg_surface`, `xdg_toplevel`, `wl_surface`.
-   **Steps:**
    1.  Connect to the Wayland display and instantiate required globals (`wl_compositor`, `wl_shm`, `xdg_wm_base`).
    2.  Create a `wl_shm_pool` from a sufficiently sized memory-mapped file.
    3.  From this pool, create a `wl_buffer` (e.g., 640x480 pixels). Fill this buffer with a solid, recognizable color (e.g., opaque red - `0xFFFF0000`).
    4.  Create a `wl_surface` using `wl_compositor.create_surface`.
    5.  Get an `xdg_surface` for the `wl_surface` using `xdg_wm_base.get_xdg_surface`.
    6.  Get an `xdg_toplevel` for the `xdg_surface`.
    7.  Set a title for the `xdg_toplevel` (e.g., "SHM Basic Window Test").
    8.  Attach the created `wl_buffer` to the `wl_surface`.
    9.  Damage the entire `wl_surface` (`wl_surface.damage_buffer` or `wl_surface.damage` with full dimensions).
    10. Commit the `wl_surface` (`wl_surface.commit`).
    11. Dispatch events. Expect to receive `xdg_surface.configure` and `xdg_toplevel.configure`. Acknowledge the `xdg_surface.configure` by sending `xdg_surface.ack_configure`.
    12. After the initial commit and configure sequence, expect a `wl_surface.frame` callback, indicating the compositor has processed the frame.
    13. Keep the window open for a short duration (e.g., 3-5 seconds by dispatching events in a loop or using a timer) to allow for visual verification.
    14. Listen for `xdg_toplevel.close` events. If received, proceed to cleanup.
    15. Gracefully destroy all created Wayland resources (`xdg_toplevel`, `xdg_surface`, `wl_buffer`, `wl_shm_pool`, `wl_surface`, globals) and disconnect from the server.
-   **Success Criteria:**
    -   The application launches, runs, and exits without any Wayland protocol errors being reported by the `wayland-client` library or the compositor.
    -   A window with the title "SHM Basic Window Test" (or as specified) appears on the display.
    -   The content of the window is a solid color as defined in the buffer (e.g., red).
    -   The application correctly receives and acknowledges the initial `xdg_surface.configure` event.
    -   At least one `wl_surface.frame` callback is received after the initial commit.
    -   The application exits cleanly with code 0.
-   **Parameters (CLI):**
    -   Optional: `--width <w>` (default e.g., 640)
    -   Optional: `--height <h>` (default e.g., 480)
    -   Optional: `--color <RRGGBBAA_hex>` (default e.g., `FFFF0000` for red)
    -   Optional: `--duration <seconds>` (default e.g., 3)

### 3.2. Test App: `xdg_shell_interactions`

-   **Purpose:** To test various `xdg_toplevel` interactions, focusing on state changes, configure events, and ping/pong mechanisms.
-   **Protocols Used:** `xdg_wm_base`, `xdg_surface`, `xdg_toplevel`, `wl_surface`. (This test will internally perform a basic SHM window setup similar to `shm_basic_window`).
-   **Steps (performed sequentially or based on CLI flags for specific tests):**
    1.  Perform initial window setup (connect, get globals, create an SHM buffer with a color/pattern, create `wl_surface`, `xdg_surface`, `xdg_toplevel`, set title, attach, commit, wait for initial configure/frame).
    2.  **Ping/Pong Test:**
        -   The application's event loop must listen for `xdg_wm_base.ping` events.
        -   Upon receiving a `ping(serial)`, immediately respond with `xdg_wm_base.pong(serial)`.
        -   Log receipt of pings and sending of pongs.
    3.  **Resize Event Handling:**
        -   This primarily tests the compositor sending resize events. The client logs received `xdg_toplevel.configure` events that suggest a new size (width/height).
        -   Upon receiving a new size, the client should acknowledge the `xdg_surface.configure`, potentially reallocate its SHM buffer to the new size, redraw content, attach, damage, and commit.
        -   (Manual interaction: User resizes the window, client logs new size).
    4.  **Maximize/Fullscreen Request (Client-Side):**
        -   **Maximize:** Call `xdg_toplevel.set_maximized()`. Dispatch events. Expect an `xdg_toplevel.configure` event indicating the `maximized` state. Log this.
        -   **Unmaximize:** Call `xdg_toplevel.unset_maximized()`. Dispatch events. Expect an `xdg_toplevel.configure` event without the `maximized` state. Log this.
        -   **Fullscreen:** Call `xdg_toplevel.set_fullscreen()`. Dispatch events. Expect an `xdg_toplevel.configure` event indicating the `fullscreen` state. Log this.
        -   **Unfullscreen:** Call `xdg_toplevel.unset_fullscreen()`. Dispatch events. Expect an `xdg_toplevel.configure` event without the `fullscreen` state. Log this.
    5.  **State Change Verification:** Within the `xdg_toplevel.configure` event handler, carefully check the `states` array argument to verify that the compositor is reporting the correct active states (e.g., maximized, fullscreen, activated). Log discrepancies.
    6.  Handle `xdg_toplevel.close` for clean shutdown.
    7.  Destroy resources and disconnect.
-   **Success Criteria:**
    -   Application correctly and promptly responds to `xdg_wm_base.ping` events.
    -   Application logs `xdg_toplevel.configure` events when its size or state changes (e.g., due to user interaction or client requests).
    -   After requesting `set_maximized` or `set_fullscreen`, subsequent `xdg_toplevel.configure` events from the compositor correctly report the new state.
    -   Visual verification confirms the window changes state (e.g., actually maximizes or fills the screen).
    -   Application exits cleanly without protocol errors.
-   **Parameters (CLI):**
    -   `--test ping` (focus on ping/pong, perhaps run for a longer duration)
    -   `--test maximize` (request maximize, then unmaximize)
    -   `--test fullscreen` (request fullscreen, then unfullscreen)
    -   `--test states` (perform a sequence of state changes and log reported states)

### 3.3. Test App: `surface_damage_frame`

-   **Purpose:** To verify the correct usage and compositor interpretation of `wl_surface.damage` (or `damage_buffer`) and the `wl_surface.frame` callback mechanism, especially for partial updates.
-   **Protocols Used:** `wl_surface`, `wl_shm`. (Assumes basic window setup like `shm_basic_window`).
-   **Steps:**
    1.  Setup a basic window (e.g., 200x200 pixels). Fill the SHM buffer with an initial solid color (e.g., blue) and commit.
    2.  Request a `wl_surface.frame` callback (`cb1`).
    3.  Upon receiving `cb1`:
        -   Modify a sub-rectangle of the SHM buffer (e.g., top-left quadrant, 0,0 to 100,100) to a new color (e.g., green).
        -   Call `wl_surface.damage(0, 0, 100, 100)`.
        -   Commit the `wl_surface`.
        -   Request another `wl_surface.frame` callback (`cb2`).
    4.  Upon receiving `cb2`:
        -   Modify another, different sub-rectangle (e.g., bottom-right quadrant, 100,100 to 100,100) to a third color (e.g., red).
        -   Call `wl_surface.damage(100, 100, 100, 100)`.
        -   Commit the `wl_surface`.
        -   Request another `wl_surface.frame` callback (`cb3`).
    5.  Repeat this pattern for a configurable number of updates, cycling through different quadrants or regions.
    6.  Keep window open for a short duration for visual check, then close.
-   **Success Criteria:**
    -   A `wl_surface.frame` callback is consistently received after each `wl_surface.commit` that includes damage.
    -   Visual verification of the window shows that only the explicitly damaged regions are updated with the new colors in the subsequent frame. (Note: some compositors might repaint larger areas, but the buffer content itself should only reflect the intended partial changes for this test to pass at the client logic level).
    -   No Wayland protocol errors occur.
    -   Application exits cleanly.
-   **Parameters (CLI):**
    -   `--num_updates <N>` (Number of damage/render cycles, default e.g., 4)
    -   `--delay_ms <ms>` (Optional delay between updates for easier visual tracking)

### 3.4. Test App: `viewporter_scaling`

-   **Purpose:** To test basic surface content scaling and cropping using the `wp_viewporter` protocol extension.
-   **Protocols Used:** `wp_viewporter`, `wl_surface`, `wl_shm`. (Assumes basic window setup).
-   **Steps:**
    1.  Connect and get globals, including `wp_viewporter`. If `wp_viewporter` is not advertised by the compositor, the test should log this and exit gracefully, possibly with a specific exit code indicating the feature is unsupported.
    2.  Setup a basic window with a fixed-size SHM buffer (e.g., 320x240 pixels). Fill this buffer with a recognizable pattern, gradient, or image that makes scaling and cropping obvious.
    3.  Get a `wp_viewport` object for the `wl_surface` using `wp_viewporter.get_viewport`.
    4.  **Scenario 1: Upscale.**
        -   Set the `wp_viewport.set_destination(640, 480)` (destination size larger than buffer).
        -   Attach buffer, damage, commit surface. Wait for frame callback.
        -   Hold for visual inspection.
    5.  **Scenario 2: Downscale.**
        -   Set `wp_viewport.set_destination(160, 120)` (destination size smaller than buffer).
        -   Commit surface. Wait for frame callback.
        -   Hold for visual inspection.
    6.  **Scenario 3: Cropping (Source rectangle).**
        -   Set `wp_viewport.set_source(x, y, width, height)` to select a sub-rectangle of the 320x240 buffer (e.g., source_x=80, source_y=60, source_w=160, source_h=120 - the center half).
        -   Set `wp_viewport.set_destination(320, 240)` (same as original buffer size, but only showing the cropped part).
        -   Commit surface. Wait for frame callback.
        -   Hold for visual inspection.
    7.  **Scenario 4: Identity (No scaling/cropping).**
        -   Set `wp_viewport.set_destination(-1, -1)` (or buffer dimensions).
        -   Set `wp_viewport.set_source(-1, -1, -1, -1)` (or full buffer dimensions).
        -   Commit surface. Wait for frame callback.
    8.  Destroy resources and disconnect.
-   **Success Criteria:**
    -   If `wp_viewporter` is supported:
        -   The rendered window on screen visually appears scaled according to the destination size set via the viewport.
        -   Upscaled content may appear pixelated (if simple scaling is used by compositor), downscaled content should appear shrunk.
        -   Cropping scenario shows only the selected source portion of the buffer, scaled to the destination size.
        -   Setting source/destination to -1 (or buffer dimensions) results in identity transform (1:1 rendering).
        -   No visual artifacts (e.g., misplaced content, incorrect stretching) beyond expected scaling behavior.
        -   No Wayland protocol errors.
    -   If `wp_viewporter` is not supported, the application logs this and exits cleanly.
-   **Parameters (CLI):**
    -   `--buffer_w <w>` (default 320)
    -   `--buffer_h <h>` (default 240)
    -   `--scenario <upscale|downscale|crop|identity>` (or run through all)
    -   For `crop`: `--source_x <sx> --source_y <sy> --source_w <sw> --source_h <sh>`
    -   `--dest_w <dw>` (used by specific scenarios)
    -   `--dest_h <dh>` (used by specific scenarios)

## 4. Common Considerations

-   **Error Handling:** All applications must implement robust error handling for Wayland connection issues, failure to bind globals, and protocol errors dispatched by the server. Errors should be logged descriptively.
-   **Logging:** A consistent logging approach should be used. Consider a simple logger that prefixes messages with the test application name and Wayland event/request details. A `--verbose` or `--debug` CLI flag could enable more detailed Wayland protocol message logging via `WAYLAND_DEBUG=1` (by setting the environment variable internally if the flag is passed) or by more verbose client-side logging.
-   **Resource Management:** Ensure that all Wayland objects are explicitly destroyed in the reverse order of their creation, especially upon successful completion or when handling an error that leads to premature exit. This is crucial for compositor stability and resource leak prevention.
-   **Build Configuration:** Each test app will be a separate binary in the `crates/test-clients/` workspace. A simple `main.rs` for each should suffice.
-   **Dependencies:** Primarily `wayland-client`. Other crates for argument parsing (e.g., `clap`), temporary files (e.g., `tempfile` for SHM pools), or image loading (if test patterns become complex) might be included.

This design document will serve as the blueprint for developing the Group 1 custom test applications. Results and detailed findings for each test will be documented in corresponding `_results.md` files in the `livetest/custom_tests/` directory.
