# Phase 4 Test Plan: Professional Application Integration

## 1. Introduction

The purpose of Phase 4 testing is to validate the custom Wayland compositor's compatibility, performance, and stability when running real-world professional applications. This phase focuses on ensuring that the compositor can handle demanding graphical workloads and provides a seamless user experience for common desktop and professional software.

This test plan adheres to the overall testing philosophy outlined in `livetest/SYSTEMATIC_TESTING_GUIDE.md`, emphasizing methodical validation and comprehensive coverage. Successful completion of Phase 4 will demonstrate the compositor's readiness for daily use with a variety of professional-grade applications.

## 2. Test Environment Setup

### Compositor Launch

-   **Primary Method:** As per the `README.md`, the compositor can be launched in a nested session for development and testing (e.g., from within an existing X11 or Wayland session) or as a native TTY session for full control. For initial Phase 4 testing, a nested session is recommended for ease of debugging.
    -   Example for nested launch:
        ```bash
        export WAYLAND_DISPLAY=wayland-1 # Or another available display name
        export XDG_RUNTIME_DIR=/tmp/custom-compositor-runtime-$$
        mkdir -p "$XDG_RUNTIME_DIR"
        chmod 700 "$XDG_RUNTIME_DIR"
        # Launch compositor executable
        ./target/debug/custom-compositor
        ```
-   **Environment Variables:**
    -   `WAYLAND_DISPLAY`: Set to a unique value for nested sessions (e.g., `wayland-1`, `wayland-test`).
    -   `XDG_RUNTIME_DIR`: Path to a directory for Wayland runtime files (sockets, etc.).
    -   Ensure other relevant environment variables (e.g., `LIBSEAT_BACKEND`, `WLR_BACKENDS`) are set according to compositor configuration if running natively.

### Application Installation

Selected professional applications must be installed within the test environment. Prefer official distributions or reputable repositories (e.g., Debian packages, Flatpaks if portal support is mature).

### Test Assets

For thorough testing, relevant test files and projects are required for each application:
-   **Blender:** Complex 3D scenes, high-poly models, scenes with intricate textures and lighting.
-   **VS Code:** Medium to large code projects, preferably with various file types and language features.
-   **GIMP:** High-resolution images (e.g., >12MP), multi-layer PSD or XCF files.
-   **LibreOffice:** Large documents, complex spreadsheets, presentations with embedded objects.
-   **Unity/Unreal Engine:** Sample projects or small demo scenes provided by the engine or community.
-   **Video Files:** 4K resolution video clips (e.g., H.264, VP9, AV1 if supported by the browser/player).

### Hardware Considerations

Refer to `livetest/README.md` for detailed hardware recommendations. Key considerations for Phase 4 include:
-   **GPU:** Vulkan 1.3+ compatible GPU (NVIDIA, AMD, Intel).
-   **VRAM:** Minimum 8GB VRAM recommended for 4K rendering and professional applications.
-   **Display:** 4K monitor for testing high-resolution rendering and scaling.
-   **Input Devices:** Standard keyboard, mouse. Graphics tablet if testing `zwp-tablet-v2` related features with GIMP.

## 3. Selected Professional Applications & Test Cases

The following applications, many of which are mentioned in `livetest/phase4_professional_apps.sh` and `README.md`, will be prioritized.

### Firefox (Web Browser)

-   **Basic Browsing:** Navigate various websites (news, blogs, e-commerce).
-   **Complex Web Page Rendering:** Test sites known for heavy use of HTML5, CSS3, and JavaScript (e.g., web apps, interactive demos).
-   **Video Playback:** Stream videos from platforms like YouTube and Vimeo at 1080p and 4K resolutions. Verify smooth playback and audio sync.
-   **Smooth Scrolling:** Assess scrolling performance on long pages with mixed content.
-   **Window Management:**
    -   Resize the browser window frequently.
    -   Enter and exit fullscreen mode (e.g., for video).
    -   Open and manage multiple browser windows.
-   **Text Input:** Test text input fields, forms, and address bar. Verify text clarity and rendering at 4K.

### LibreOffice Suite (Writer, Calc, Impress)

-   **Document Creation/Editing:** Create new documents, spreadsheets, and presentations. Edit existing complex documents.
-   **UI Rendering & Responsiveness:** Interact with menus, toolbars, sidebars, and dialog boxes. Ensure they render correctly and respond promptly.
-   **Copy-Paste:**
    -   Copy and paste text, images, and objects within the same LibreOffice application.
    -   Copy and paste between different LibreOffice applications (e.g., Calc to Writer).
    -   Copy from LibreOffice and paste into another application (e.g., a text editor or Firefox), and vice-versa.
-   **Menu and Dialog Behavior:** Ensure menus open correctly, dialogs are modal and positioned appropriately.
-   **Print Preview / Export:** Test print preview functionality and export documents to PDF. Verify output accuracy.

### GIMP (GNU Image Manipulation Program)

-   **Image Operations:** Load various image formats (JPEG, PNG, TIFF, XCF, PSD). Edit images using common tools. Save images in different formats.
-   **Tool Responsiveness:** Test performance of brushes, selection tools (rectangle, ellipse, free select), healing tools, etc.
-   **Layer Management:** Create, delete, reorder, and modify layers and layer masks.
-   **Color Accuracy:** Visually inspect color representation (requires a well-calibrated display for precise assessment).
-   **Performance with Large Images:** Work with images > 4000x3000 pixels at 4K display resolution. Monitor responsiveness.
-   **Graphics Tablet Input:** If `zwp-tablet-v2` support is considered mature and a tablet is available, test pressure sensitivity, eraser input, and button mapping.

### VS Code (Visual Studio Code)

-   **Code Editing:** Open, edit, and save code files (e.g., Rust, Python, JavaScript). Test syntax highlighting, auto-completion, and code folding.
-   **Integrated Terminal:** Use the integrated terminal for basic commands. Check rendering and input.
-   **Debugger UI:** If a simple project is set up, step through code and observe debugger UI responsiveness.
-   **High-DPI Text Rendering:** Verify clarity and sharpness of text at 4K resolution, especially with different font sizes and themes.
-   **Extension Compatibility:** Install and test a few popular UI-affecting extensions (e.g., themes, file icon packs).
-   **File Explorer & Sidebars:** Interact with the file explorer, search panel, source control view, and other sidebars.

### Blender

-   **Scene Navigation:** Load and navigate complex 3D scenes (e.g., Blender demo files). Test orbiting, panning, and zooming performance.
-   **Viewport Rendering:**
    -   Test Solid, Wireframe, and Material Preview modes.
    -   If the Wayland display server allows direct GPU access for applications (typical), test Eevee real-time rendering performance. Cycles testing might be limited by Wayland's current capabilities for such intensive tasks unless it's purely offscreen rendering.
-   **UI Responsiveness:** Interact with menus, properties panels, and editor type dropdowns.
-   **Animation Playback:** Play back simple to moderately complex animations within the viewport.
-   **Object Manipulation:** Select, move, rotate, and scale objects. Test snapping and precision controls.
-   **Zero-Copy Buffer Sharing:** While difficult to directly measure without specific tools, look for overall smoothness in viewport updates that might indicate efficient DMA-BUF usage.

### Unity or Unreal Engine (or Godot as an open-source alternative)

Choose one based on ease of setup and availability of test projects. Godot Engine is a strong open-source candidate that runs natively on Linux.
-   **Project Launch:** Open a sample project or a new empty project.
-   **Editor UI Responsiveness:** Interact with the editor's interface: scene view, hierarchy, inspector, asset browser.
-   **Play-Mode Rendering:** Run a simple scene in play-mode and observe rendering performance and stability at 4K.
-   **Input Handling:** Test keyboard and mouse input in play-mode (e.g., character movement, camera control).
-   **Window Management:** Test how the main editor window and any game preview windows are managed by the compositor.

### Video Conferencing Application

This could be a WebRTC client in Firefox (see Firefox tests) or a native Linux application if one is readily available and Wayland-compatible (e.g., a test build of an Electron app with Wayland flags).
-   **Camera Feed Display:** If a camera is connected, test if the application can access and display the camera feed.
-   **Screen Sharing:**
    -   This heavily depends on `xdg-desktop-portal` integration. If this protocol and its backend are implemented and configured for the compositor, test screen sharing functionality (sharing entire screen or specific windows).
    -   If not fully implemented, note this as a limitation.
-   **Audio/Video Synchronization:** Check for sync issues during a test call (if applicable).
-   **UI Element Rendering:** Ensure buttons, participant lists, and chat windows render correctly.

### Gaming Application

Select a game known to have good Wayland support, either natively or via compatibility layers like Proton/Wine if this is a target use case for the compositor.
-   **Launch & Gameplay:** Launch the game and play for a period. Assess overall performance and smoothness at 4K.
-   **Input Responsiveness:** Test keyboard, mouse, and gamepad (if supported by the game and compositor input stack) responsiveness.
-   **Fullscreen Behavior:** Test entering and exiting fullscreen mode. Verify correct display and capture of input.
-   **Graphics Settings:** Adjust in-game graphics settings (resolution, texture quality, etc.) and observe their impact and stability.

## 4. General Test Areas & Success Criteria

These criteria are drawn from `livetest/SYSTEMATIC_TESTING_GUIDE.md` and apply across all tested applications.

-   **Application Launch & Operation:**
    -   Applications launch successfully from a terminal or application launcher (if applicable).
    -   Core functionalities of each application operate as expected without Wayland-specific errors.
-   **4K Scaling & Visual Fidelity:**
    -   Applications render correctly at 4K resolution.
    -   UI elements are appropriately scaled (respecting fractional scaling if `wp-fractional-scale-v1` is active and configured).
    -   Text is clear, sharp, and legible.
    -   No visual artifacts such as tearing, flickering, missing elements, or incorrect colors attributable to the compositor.
-   **Performance & Stability:**
    -   The compositor and the host system remain stable during extended usage (e.g., >30 minutes per application).
    -   The compositor maintains responsiveness, targeting <16ms input-to-photon latency and a consistent 60 FPS at 4K.
    -   No crashes of the compositor itself.
    -   No application crashes that can be clearly attributed to Wayland protocol interactions or compositor behavior.
-   **Memory Management:**
    -   Compositor memory usage remains within acceptable limits (e.g., monitor for significant leaks over time).
    -   Overall system memory usage with multiple professional applications running is manageable.
-   **Window Management:**
    -   Resizing windows is smooth and content reflows correctly.
    -   Minimizing, maximizing, and restoring windows functions as expected.
    -   Fullscreen mode works correctly for applications that support it.
    -   Window decorations (if server-side via `xdg-decoration`) are displayed correctly and consistently.
    -   Popups, context menus, and application dialogs are positioned correctly and behave as expected (e.g., modality).
-   **Input Handling:**
    -   Keyboard input (including modifiers like Shift, Ctrl, Alt) works correctly in all applications.
    -   Mouse input (clicks, movement, scrolling) is accurately processed.
    -   If tablet support is tested, its specific features (pressure, tilt) are correctly relayed.
-   **Copy/Paste:**
    -   Clipboard operations (copy, cut, paste) using `wl_data_device_manager` work reliably:
        -   Within the same application.
        -   Between different Wayland native applications.
-   **Protocol-Specific Features:**
    -   **`wp-presentation-time`:** Observe smoothness in video playback and animations, indicating correct frame presentation timing.
    -   **`zwp-linux-explicit-sync-v1` (or similar like `wp-linux-drm-syncobj-v1`):** Look for tear-free rendering in demanding graphical applications and games.
    -   **`xdg-activation-v1`:** Ensure windows receive focus correctly upon launch or when requested by the application.
    -   **`wlr-layer-shell-unstable-v1`:** If desktop shell components (like an app bar) are part of the compositor, ensure they interact correctly with fullscreen applications.

## 5. Reporting & Issue Tracking

-   Maintain a systematic log for each test session and application.
-   For any identified failures or significant issues, capture detailed information:
    -   **Application:** Name and version (e.g., Firefox 118.0.1, Blender 3.6.2).
    -   **Steps to Reproduce:** Clear, numbered steps that reliably trigger the issue.
    -   **Expected Behavior:** What should have happened.
    -   **Actual Behavior:** What actually happened, including error messages.
    -   **Compositor Logs:** Relevant output from the compositor's standard output/error or dedicated log files. Include Wayland debug logs if enabled (e.g., `WAYLAND_DEBUG=1`).
    -   **Application Logs:** If the application produces its own logs, include relevant sections.
    -   **Visual Evidence:** Screenshots clearly showing visual artifacts or incorrect UI behavior. Screen recordings (.mp4, .webm) are highly valuable for demonstrating performance issues, animation glitches, or complex interaction failures.
-   All bugs, unexpected behaviors, and performance issues will be tracked using the project's designated issue tracker (e.g., GitHub Issues). Each issue should be tagged appropriately (e.g., "Phase 4", "Firefox", "Crash", "Visual Artifact").

This test plan provides a comprehensive framework for Phase 4 testing. Flexibility will be maintained to adapt to unforeseen issues or to explore specific areas of concern as they arise.
