# Design: Group 2 - DMA-BUF & Explicit Sync Test Applications

## 1. Introduction

The purpose of this group of test applications is to validate the compositor's capability to import DMA-BUF (Direct Memory Access BUFfer) buffers and correctly handle explicit synchronization primitives. These are critical features for high-performance graphics applications, enabling zero-copy buffer sharing between clients and the compositor, and precise control over buffer access timing.

This test group primarily covers the following Wayland protocols:
-   `zwp_linux_dmabuf_v1` (and its various feedback mechanisms like `format_table` or `feedback_default` if testing specific versions/features): For importing DMA-BUFs as `wl_buffer` objects.
-   `zwp_linux_explicit_sync_v1`: For synchronized buffer submissions using explicit fence file descriptors.
-   `wl_surface`: The Wayland surface to which DMA-BUF backed buffers will be attached.
-   `xdg_shell` (specifically `xdg_wm_base`, `xdg_surface`, `xdg_toplevel`): For managing the toplevel window semantics for displaying the DMA-BUF content.

Test applications will be developed in Rust using the `wayland-client` crate. For DMA-BUF creation and sync file fd generation, a minimal approach using existing system libraries (like GBM for buffer allocation or direct ioctls for sync files) or very lightweight Vulkan/EGL interop will be preferred to keep the test clients focused and relatively simple. The primary goal is to test the Wayland protocol interactions, not the intricacies of various graphics APIs.

## 2. General Test Application Structure

Each test application within this group will follow these conventions:

-   **Independent Executable:** Each test will be a small, self-contained Rust executable located in `crates/test-clients/`.
-   **Wayland Connection:** Establish a connection to the Wayland display.
-   **Global Instantiation:** Bind to necessary global Wayland objects, including `wl_compositor`, `zwp_linux_dmabuf_v1`, `zwp_linux_explicit_sync_v1`, and `xdg_wm_base`. The tests should gracefully handle and report if optional protocols like `zwp_linux_explicit_sync_v1` are not supported by the compositor.
-   **Event Logging:** Log key Wayland events, DMA-BUF parameters, fence operations, and status updates to `stdout`.
-   **Exit Codes:** Exit with code `0` on success, non-zero on failure, error, or if a required protocol is missing.
-   **DMA-BUF Source Note:**
    -   These tests require a source of DMA-BUF file descriptors (fds) and, for explicit synchronization, sync file fds.
    -   **Option A (Preferred for Simplicity if Feasible):** Utilize a minimal, self-contained approach within the test client. For DMA-BUF, this might involve using a library like `gbm` (Generic Buffer Management) to allocate a simple graphics buffer (e.g., an ARGB8888 surface) and export its DMA-BUF fd. For sync files, direct use of `ioctl` on `/dev/syncobj` or `/dev/sw_sync` (if available and simple enough) or a minimal Vulkan timeline semaphore export.
    -   **Option B (Fallback):** If self-contained creation is too complex or adds too many dependencies for a focused Wayland test client, the application might accept pre-created DMA-BUF fds or sync file fds as command-line arguments. This would require an external utility or script to provide these resources.
    -   The design will aim for Option A where practical, abstracting the graphics API details as much as possible.

## 3. Test Application Designs

### 3.1. Test App: `dmabuf_import_render`

-   **Purpose:** To verify the compositor's ability to correctly import a DMA-BUF via `zwp_linux_dmabuf_v1` and render its content in a window.
-   **Protocols Used:** `zwp_linux_dmabuf_v1`, `wl_surface`, `xdg_shell` (`xdg_wm_base`, `xdg_surface`, `xdg_toplevel`), `wl_compositor`.
-   **Steps:**
    1.  Connect to Wayland and instantiate required globals, ensuring `zwp_linux_dmabuf_v1` is present.
    2.  **Create DMA-BUF:**
        -   Use a minimal GBM setup (e.g., find a card, create a GBM device, allocate a GBM BO - Buffer Object - of a small size like 256x256 with a common format like `DRM_FORMAT_ARGB8888` or `XRGB8888`).
        -   Map the GBM BO (if necessary for CPU write) and fill its content with a distinct, recognizable solid color or a simple pattern (e.g., colored checkerboard).
        -   Export the DMA-BUF file descriptor (fd) for the GBM BO. Query its stride and offset for plane 0.
        -   Log these parameters.
        -   *(Fallback: If GBM is too complex, this step would involve receiving fd, width, height, stride, offset, format, modifier via CLI arguments).*
    3.  Query supported formats/modifiers from the compositor using `zwp_linux_dmabuf_v1` events (e.g., `format` and `modifier` or by observing `zwp_dmabuf_feedback_v1` if a newer version of the protocol is targeted and used). Ensure the chosen format/modifier for the created buffer is supported.
    4.  Create a `zwp_linux_buffer_params_v1` object from the `zwp_linux_dmabuf_v1` global.
    5.  Add the DMA-BUF fd, plane index (0), offset, stride, buffer width, buffer height, DRM format code, and DRM modifier to the params object.
    6.  Request the creation of a `wl_buffer` from these params by calling `create_immed`. Listen for the `created` and `failed` events from the `zwp_linux_buffer_params_v1` object.
    7.  If `failed` is received, log error and exit. If `created(wl_buffer)` is received, store the new `wl_buffer`.
    8.  Create a basic `xdg_toplevel` window (similar to `shm_basic_window` but without SHM buffer creation). Set a title (e.g., "DMA-BUF Import Test").
    9.  Attach the newly created DMA-BUF-backed `wl_buffer` to the `wl_surface`.
    10. Damage the entire surface (`wl_surface.damage_buffer` or `wl_surface.damage`).
    11. Commit the `wl_surface`.
    12. Dispatch events, wait for `xdg_surface.configure`, `xdg_toplevel.configure`, and `wl_surface.frame`.
    13. Keep the window open for a few seconds for visual verification.
    14. Clean up: Destroy Wayland objects, then close the DMA-BUF fd. Disconnect.
-   **Success Criteria:**
    -   The application runs without Wayland protocol errors.
    -   `zwp_linux_dmabuf_v1` global is successfully bound.
    -   The `zwp_linux_buffer_params_v1.created` event is received, and `failed` is not.
    -   A window appears on screen displaying the content (e.g., solid color, pattern) that was put into the DMA-BUF.
    -   Compositor logs (if available and verbose) do not indicate errors during DMA-BUF import or rendering.
    -   The application exits cleanly with code 0.
-   **Parameters (CLI):**
    -   Optional: `--width <w>` (default e.g., 256)
    -   Optional: `--height <h>` (default e.g., 256)
    -   Optional: `--format <drm_format_code_hex>` (e.g., `0x34325241` for ARGB8888. Default to a common one if using internal GBM).
    -   Optional: `--modifier <modifier_code_hex>` (e.g., `0` for LINEAR. Default if using internal GBM).
    -   Optional (if not using internal GBM): `--fd <fd_num> --stride0 <s0> --offset0 <o0>`

### 3.2. Test App: `dmabuf_explicit_sync`

-   **Purpose:** To verify the compositor's handling of explicit synchronization fences (`acquire_fence` and `release_fence`) when using DMA-BUF buffers, via `zwp_linux_explicit_sync_v1`.
-   **Protocols Used:** `zwp_linux_dmabuf_v1`, `zwp_linux_explicit_sync_v1`, `wl_surface`, `xdg_shell`.
-   **Steps:**
    1.  Perform setup similar to `dmabuf_import_render` to obtain a DMA-BUF backed `wl_buffer`. This includes connecting to Wayland, getting globals, and creating/importing the DMA-BUF.
    2.  Ensure the `zwp_linux_explicit_sync_v1` global is bound. If not available, log and exit gracefully (test cannot run).
    3.  For the `wl_surface`, get a `zwp_surface_synchronization_v1` object using `zwp_linux_explicit_sync_v1.get_synchronization`.
    4.  **Loop for N frames (e.g., 3-5 frames):**
        a.  **Client-Side Rendering Simulation & Acquire Fence:**
            -   Simulate client-side rendering to the DMA-BUF. (Actual rendering is not strictly necessary for this protocol test if the buffer content is static; the key is the fence.)
            -   Create an "acquire fence" sync file fd. This fd should represent the completion of client rendering to the DMA-BUF.
                -   *Minimal Method:* Use Linux `sync_file_create` via `ioctl` on `/dev/sw_sync` or `syncobj_create` on `/dev/syncobj` if available and simple enough. Create it in an unsignaled state, then signal it (or use an already signaled fence for initial tests).
                -   Log the fd value.
        b.  Provide this acquire fence to the compositor by calling `zwp_surface_synchronization_v1.set_acquire_fence(acquire_fence_fd)`.
        c.  Close the `acquire_fence_fd` immediately after the Wayland request, as its ownership is transferred.
        d.  Attach the DMA-BUF `wl_buffer` to the `wl_surface`.
        e.  Damage the surface.
        f.  **Release Fence:** Create another sync file fd (initially unsignaled) to serve as the "release fence". This fd will be passed to the compositor, which should signal it when it's finished reading from the attached buffer. Log this fd value.
        g.  Set this release fence by calling `zwp_surface_synchronization_v1.set_release_fence(release_fence_fd)`.
        h.  Commit the `wl_surface`.
        i.  Close the client's copy of `release_fence_fd` (the actual signaling happens on the fd passed to the compositor).
        j.  **Wait for Release:** Use `poll` or a similar mechanism to wait for the original `release_fence_fd` (that the client created and kept a dup of, or a new one if the API implies it) to be signaled by the compositor. Log when it's signaled. This indicates the client is now free to reuse or modify the DMA-BUF.
        k. If testing multiple frames, slightly modify the DMA-BUF content (e.g., change color) to make frames visually distinct, then loop back to step 4a.
    5.  Keep the window open for a short duration for final visual check.
    6.  Clean up all Wayland resources, GBM objects, and close any remaining fds. Disconnect.
-   **Success Criteria:**
    -   The application runs without Wayland protocol errors.
    -   `zwp_linux_explicit_sync_v1` global is successfully bound.
    -   The compositor correctly consumes the `acquire_fence` for each frame (difficult to verify directly from client side, but absence of compositor errors or hangs is a good sign).
    -   The `release_fence` (the fd the client waits on) is signaled by the compositor after each commit in a timely manner.
    -   If dynamic content is rendered across frames, it appears correctly and without tearing, indicating proper synchronization.
    -   Application exits cleanly with code 0.
-   **Parameters (CLI):**
    -   (Similar to `dmabuf_import_render` for buffer parameters if not using internal GBM)
    -   `--num_frames <N>` (default e.g., 5)
    -   `--timeout_ms <ms>` (timeout for waiting on release fence, default e.g., 1000)

## 4. Common Considerations

-   **Error Handling:** Applications must robustly handle errors such as:
    -   Failure to bind required globals (especially `zwp_linux_dmabuf_v1`, `zwp_linux_explicit_sync_v1`).
    -   Errors during DMA-BUF creation/export (if done internally).
    -   `zwp_linux_buffer_params_v1.failed` event.
    -   Errors in creating or waiting on sync file fds.
    -   Protocol errors from the compositor.
-   **Logging:** Verbose logging is crucial:
    -   Parameters of created DMA-BUFs (fd, size, stride, offset, format, modifier).
    -   Values of acquire and release fence fds used.
    -   Timestamps or notifications when acquire fences are submitted, and when release fences are signaled.
    -   Any relevant Wayland events received.
-   **Resource Management:** Strict management of resources:
    -   Proper destruction of all Wayland objects created.
    -   Closing all file descriptors (DMA-BUF fds, sync file fds) when they are no longer needed or after ownership has been transferred (as in `set_acquire_fence`).
    -   Proper cleanup of any GBM/EGL/Vulkan objects.
-   **Minimal Graphics Stack Dependency:** The focus is on testing Wayland protocol interactions. If GBM, EGL, or Vulkan are used:
    -   Keep their usage to the absolute minimum required for buffer allocation/export and fence creation/export.
    -   Avoid complex rendering logic within these APIs in the test client. A simple filled buffer or basic pattern is sufficient.
    -   Clearly document the minimal graphics setup if it's internal to the test client.

This design document will guide the development of Group 2 custom test applications. Results and detailed findings for each test will be documented in `livetest/custom_tests/dmabuf_sync_results.md`.
