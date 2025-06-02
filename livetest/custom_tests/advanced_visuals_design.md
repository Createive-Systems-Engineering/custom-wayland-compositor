# Design: Group 5 (Part 1) - Fractional Scaling & Server-Side Decorations

## 1. Introduction

This document outlines the design for the first set of custom Rust test applications within "Group 5: Advanced Visuals & Session Test Apps." These initial tests focus on validating two critical features for modern high-resolution displays (like 4K) and contemporary desktop aesthetics: fractional scaling of surfaces and server-side window decorations.

The tests primarily cover the following Wayland protocols:
-   `wp_fractional_scale_v1` (via `wp_fractional_scale_manager_v1`): For allowing clients to request a preferred scaling factor for their surfaces, which the compositor may then apply.
-   `xdg_decoration_unstable_v1` (via `xdg_decoration_manager_v1`): For negotiating between client-side and server-side window decorations on `xdg_toplevel` surfaces.
-   Supporting protocols: `wl_surface`, `xdg_shell` (`xdg_wm_base`, `xdg_surface`, `xdg_toplevel`), `wl_shm`, `wl_compositor`, `wl_seat`.

Test applications will be developed in Rust, using the `wayland-client` crate and relevant bindings from `wayland-protocols` for these specific extensions.

## 2. General Test Application Structure

Each test application in this group will:

-   **Create a Simple Window:** Utilize SHM (Shared Memory) to create a basic `xdg_toplevel` window. The content of this window will be designed to make the effects of scaling or decoration changes visually apparent.
-   **Instantiate Necessary Globals:** Connect to the Wayland display and bind to all required global Wayland objects, including those for the specific protocols being tested (e.g., `wp_fractional_scale_manager_v1`, `xdg_decoration_manager_v1`). The tests should handle cases where optional protocols might not be supported by the compositor.
-   **Log Key Information:** Log relevant Wayland events received, requested state changes (e.g., preferred scale, decoration mode), compositor acknowledgments, and visual parameters to `stdout`.
-   **Exit Codes:** Standard exit code conventions (0 for success, non-zero for failure, error, or if a required protocol is missing and essential for the test).

## 3. Test Application Designs

### 3.1. Test App: `fractional_scale_tester`

-   **Purpose:** To verify the compositor's support for and handling of client requests for fractional scaling of a surface via the `wp_fractional_scale_v1` protocol.
-   **Protocols Used:** `wp_fractional_scale_manager_v1`, `wp_fractional_scale_v1`, `wl_surface`, `xdg_shell`, `wl_shm`, `wl_compositor`.
-   **Steps:**
    1.  Connect to Wayland and instantiate required globals. Crucially, bind to `wp_fractional_scale_manager_v1`. If the manager is not available, log this and exit gracefully (test cannot run).
    2.  Create a basic SHM-backed `xdg_toplevel` window. The buffer should have a fixed physical pixel size (e.g., 300x200 pixels).
    3.  Fill the buffer with a visually distinct pattern that makes scaling effects (e.g., blurriness, sharpness, correct proportions) and potential distortions easy to observe. Examples: a grid of fine lines, a circle perfectly inscribed within a square, text with sharp edges.
    4.  Obtain a `wp_fractional_scale_v1` object for the `wl_surface` using `wp_fractional_scale_manager_v1.get_fractional_scale`.
    5.  Implement the `wp_fractional_scale_v1.preferred_scale(factor)` event handler. When this event is received, log the `factor` (numerator) acknowledged by the compositor. This is the scale the compositor intends to apply (actual_scale = factor / 120.0).
    6.  **Test Scenario 1: Single Preferred Scale Request.**
        a.  The client calls `wp_fractional_scale_v1.set_preferred_scale(numerator)`, where `numerator` corresponds to a desired scale (e.g., `120` for 1.0x, `150` for 1.25x, `180` for 1.5x). The denominator for this protocol is fixed at 120.
        b.  Commit the `wl_surface`.
        c.  The client's event loop should dispatch events. The `preferred_scale` event (from step 5) from the compositor is expected. Log the received factor.
        d.  The client itself will *not* change its buffer size or content based on this scale for this test; it relies on the compositor to perform the scaling. The purpose is to observe the compositor's scaling of the fixed-size client buffer.
        e.  Keep the window open for a few seconds for visual inspection.
    7.  **Test Scenario 2: Cycle Through Multiple Scales (Optional, via CLI flag).**
        a.  Programmatically iterate through a list of common scale numerators (e.g., 120, 135 (for 1.125), 150, 165 (for 1.375), 180, 240).
        b.  For each numerator: call `set_preferred_scale`, `wl_surface.commit()`, wait for and log the `preferred_scale` event, and pause briefly (e.g., 2-3 seconds) for visual inspection.
    8.  (Optional Advanced) Correlate with `wl_surface.enter` event's output transform and scale factor, if available and relevant, to see how fractional scaling interacts with integer output scaling.
    9.  Clean up resources.
-   **Success Criteria:**
    -   The application runs without Wayland protocol errors.
    -   If `wp_fractional_scale_manager_v1` is supported, the `wp_fractional_scale_v1.preferred_scale` event is received from the compositor after the client calls `set_preferred_scale` and commits.
    -   The `factor` reported by the compositor in the event is logged. This factor might be the same as requested or a different one if the compositor has limitations or preferences.
    -   **Visual Inspection:** The rendered window content on screen appears scaled by the compositor according to the `factor` acknowledged in the `preferred_scale` event. For common scales (e.g., 1.25x, 1.5x), the rendering should be reasonably sharp and free of egregious artifacts, assuming the compositor implements a quality scaling algorithm.
    -   The client correctly interprets and logs the scale factor from the event.
-   **Parameters (CLI):**
    -   `--scale <numerator>` (e.g., `150` for 1.25x). If not provided, might test a default like 1.0x or a specific common fractional scale.
    -   `--cycle-scales` (Boolean flag to trigger Scenario 2).
    -   `--buffer_w <width>` `--buffer_h <height>` (Optional, for physical buffer size).

### 3.2. Test App: `ssd_decoration_tester`

-   **Purpose:** To verify the compositor's handling of server-side window decorations (SSD) versus client-side decorations (CSD) negotiation via the `xdg_decoration_unstable_v1` protocol.
-   **Protocols Used:** `xdg_decoration_manager_v1`, `xdg_toplevel_decoration_v1`, `xdg_shell` (`xdg_wm_base`, `xdg_surface`, `xdg_toplevel`), `wl_surface`, `wl_shm`, `wl_compositor`.
-   **Steps:**
    1.  Connect to Wayland and instantiate required globals. Crucially, bind to `xdg_decoration_manager_v1`. If the manager is not available, log this (compositor does not support the protocol) and exit gracefully or proceed without decoration negotiation (window will appear undecorated or with default compositor behavior).
    2.  Create a basic SHM-backed `xdg_toplevel` window. Content can be a simple colored rectangle.
    3.  Before the initial `wl_surface.commit` for the `xdg_surface` with the `xdg_toplevel` role, get an `xdg_toplevel_decoration_v1` object for the `xdg_toplevel` using `xdg_decoration_manager_v1.get_toplevel_decoration`.
    4.  Implement the `xdg_toplevel_decoration_v1.configure(mode)` event handler. When this event is received, log the `mode` (e.g., `ServerSide` or `ClientSide`) chosen/acknowledged by the compositor.
    5.  **Test Scenario 1: Request Server-Side Decorations.**
        a.  Call `xdg_toplevel_decoration_v1.set_mode(ServerSide)`.
        b.  Perform the initial `wl_surface.commit`.
        c.  The client's event loop should dispatch events. The `configure` event (from step 4) from the decoration object is expected. Log the received mode.
        d.  The client should *not* draw its own decorations if this mode is requested.
        e.  Keep window open for visual inspection.
    6.  **Test Scenario 2: Request Client-Side Decorations (if compositor allows/prefers).**
        a.  (Can be a separate run or a subsequent request if compositor allows mode changes after initialization, though this is uncommon for this protocol).
        b.  Call `xdg_toplevel_decoration_v1.set_mode(ClientSide)`.
        c.  Commit the `wl_surface`.
        d.  Expect and log the `configure` event with the acknowledged mode.
        e.  If this mode is active, the client would typically be responsible for drawing decorations. This test client might simply draw a thin border within its SHM buffer to indicate it believes it's in CSD mode, or nothing special.
        f.  Keep window open for visual inspection.
    7.  Clean up resources.
-   **Success Criteria:**
    -   The application runs without Wayland protocol errors.
    -   If `xdg_decoration_manager_v1` is supported, the `xdg_toplevel_decoration_v1.configure` event is received after setting a mode and committing. The logged mode indicates the compositor's choice.
    -   **Visual Inspection is Key:**
        -   If `ServerSide` mode is requested and acknowledged by the compositor: The compositor should draw window decorations (e.g., a title bar, borders, minimize/maximize/close buttons) around the client's surface. The client's rendered content should appear correctly within these server-drawn decorations.
        -   If `ClientSide` mode is requested and acknowledged: The compositor should *not* draw substantial decorations (perhaps only a minimal border or shadow). The client would be responsible for drawing its own, but this test client might just show its raw content without any client-drawn decorations to make the absence of server decorations clear.
    -   The mode received in the `configure` event matches what is visually presented.
-   **Parameters (CLI):**
    -   `--mode <server|client>`: Allows requesting a specific decoration mode at startup. If not provided, might default to requesting server-side.

## 4. Common Considerations

-   **Visual Verification:** These tests, particularly `fractional_scale_tester` and `ssd_decoration_tester`, will heavily rely on visual inspection by the human tester to confirm that the compositor's rendering matches the protocol interactions and expected outcomes (e.g., is the content actually scaled? Are decorations present/absent as expected?). Screenshots should be taken to document visual results.
-   **Compositor Configuration & Capabilities:** The behavior and success of these tests can be significantly influenced by the compositor's own configuration and capabilities. For example:
    -   A compositor might only support a specific set of fractional scales (e.g., only 1.25x and 1.5x, but not 1.375x).
    -   A compositor might always force server-side decorations or always client-side, regardless of client preference.
    -   The test results should ideally note the compositor's behavior and whether it aligns with its documented capabilities if known.
-   **Logging:** Clear logging of requested parameters (e.g., scale numerator, decoration mode) and parameters/modes received back from the compositor via events is essential for correlating client actions with compositor responses.
-   **Simplicity of Client Rendering:** The client applications should keep their own rendering extremely simple (e.g., basic patterns, solid colors, simple borders for CSD indication) so that the focus remains on the compositor's handling of the protocols, not on complex client-side drawing logic.

This design document will guide the development of the Group 5 (Part 1) custom test applications. Results and detailed findings will be documented in `livetest/custom_tests/advanced_visuals_results.md` (or a similarly named file).
