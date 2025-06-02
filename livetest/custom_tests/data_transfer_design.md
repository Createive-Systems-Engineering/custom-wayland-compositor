# Design: Group 4 - Data Transfer (Clipboard) Test Applications

## 1. Introduction

The purpose of this group of test applications is to validate the compositor's handling of clipboard operations (copy and paste) using the Wayland data device manager and related protocols. These tests are crucial for ensuring basic interoperability between applications for data sharing.

This initial phase of data transfer testing will focus on the `text/plain` MIME type. Future extensions could cover other types like `image/png` if necessary.

The tests primarily cover the following Wayland protocols:
-   `wl_data_device_manager`: For obtaining the data device.
-   `wl_data_device`: Represents the seat's data handling capabilities, including selections (clipboard).
-   `wl_data_source`: Used by an application to offer data for transfer.
-   `wl_data_offer`: Used by an application to learn about data being offered and to receive it.
-   `xdg_shell` (`xdg_wm_base`, `xdg_surface`, `xdg_toplevel`): For creating basic windows for the test applications.
-   `wl_seat`: To get the `wl_data_device`.

Two distinct test applications will be developed in Rust using the `wayland-client` crate:
1.  `clipboard_copier`: Offers predefined text data to the clipboard.
2.  `clipboard_paster`: Receives text data from the clipboard.

## 2. General Test Application Structure

Both applications will share some common structural elements:

-   **Simple Window:** Each application will create a basic `xdg_toplevel` window (e.g., using SHM for a simple colored background or status text). This provides a visual presence and a surface that can conceptually gain focus for clipboard operations.
-   **Data Device Initialization:** Connect to the Wayland display, bind to `wl_seat`, then get `wl_data_device_manager`, and finally obtain a `wl_data_device` for the relevant seat.
-   **Logging:** Comprehensive logging to `stdout` is essential to observe the sequence of protocol events, offered/received MIME types, data content (or summaries), and any errors.
-   **Exit Codes:** Standard exit code conventions (0 for success, non-zero for failure/error).

## 3. Test Application Designs

### 3.1. Test App: `clipboard_copier`

-   **Purpose:** To offer data (a simple text string) to the system clipboard.
-   **Protocols Used:** `wl_data_device_manager`, `wl_data_device`, `wl_data_source`, `wl_seat`, `xdg_shell`.
-   **Steps:**
    1.  Connect to Wayland, instantiate globals (`wl_seat`, `wl_data_device_manager`, `xdg_wm_base`, `wl_compositor`, `wl_shm`).
    2.  Create a simple `xdg_toplevel` window.
    3.  Obtain the `wl_data_device` from the `wl_seat`.
    4.  **Trigger Copy Action:** This can be triggered automatically on startup (for simplicity in initial tests) or by a specific user action (e.g., a key press if `wl_keyboard` is also set up, or a button click in the window if `wl_pointer` is set up). The trigger event will provide a `serial` needed for `set_selection`.
        a.  Log that the copy action is being initiated.
        b.  Create a `wl_data_source` using `wl_data_device.create_data_source()`.
        c.  Implement the `wl_data_source` event handler:
            i.  `send(mime_type, fd)`: If `mime_type` is `text/plain`, write a predefined test string (e.g., "Hello Wayland Clipboard!" or text from CLI params) to the provided file descriptor (`fd`). Close the `fd` after writing. Log the attempt to send and the data.
            ii. `cancelled()`: Log if the data source is destroyed and the data transfer cancelled.
            iii. `dnd_finished()`: Log (not primary for clipboard, but good to handle).
            iv. `action(dnd_action)`: Log (not primary for clipboard).
        d.  Call `wl_data_source.offer("text/plain")` to advertise the supported MIME type.
        e.  Call `wl_data_device.set_selection(Some(&data_source), serial)`. The `serial` must be from the user event that triggered the copy (e.g., button press serial, keyboard key press serial). If automatically triggered on startup, obtaining a valid recent serial might require careful handling (e.g., from a pointer enter event or keyboard enter event if the window is focused). For initial testing, a hardcoded or most recent serial might be attempted, though this is not robust.
    5.  Log the fact that `text/plain` has been offered with the specific content.
    6.  The application should remain running to keep the `wl_data_source` alive and serve potential requests.
    7.  Optionally, display a status in its window (e.g., "Copied: 'Hello Wayland Clipboard!'").
    8.  Clean up resources on exit (e.g., triggered by closing the window or a specific signal).
-   **Success Criteria:**
    -   The application runs without Wayland protocol errors.
    -   `wl_data_source` is successfully created.
    -   `wl_data_source.offer("text/plain")` is called.
    -   `wl_data_device.set_selection()` is called with the data source and a valid serial.
    -   When the `clipboard_paster` (or another Wayland application) attempts to paste `text/plain` data, the `wl_data_source.send` event is triggered on `clipboard_copier`.
    -   The data is successfully written to the provided `fd` in the `send` handler.
    -   Client logs clearly show these operations.
-   **Parameters (CLI):**
    -   `--text <string>`: Optional. Specifies the text to be copied. Defaults to "Hello Wayland Clipboard!".
    -   `--trigger <auto|key|click>`: Optional. Defines how the copy is triggered. `auto` might be simplest for initial tests but needs careful serial handling.

### 3.2. Test App: `clipboard_paster`

-   **Purpose:** To receive (paste) text data from the system clipboard.
-   **Protocols Used:** `wl_data_device_manager`, `wl_data_device`, `wl_data_offer`, `wl_seat`, `xdg_shell`.
-   **Steps:**
    1.  Connect to Wayland, instantiate globals.
    2.  Create a simple `xdg_toplevel` window. This window should ideally be focused to become the recipient of clipboard data.
    3.  Obtain the `wl_data_device` from the `wl_seat`.
    4.  Implement the `wl_data_device` event handler:
        a.  `data_offer(offer)`: When a new `wl_data_offer` (which is a `wl_data_offer` object) is introduced by the compositor (e.g., when data is copied by another app), this event is triggered. Store the new `wl_data_offer`. Implement its event handler:
            i.  `wl_data_offer.offer(mime_type)`: This event is emitted by the offer to announce a MIME type it can provide. Log all announced `mime_type`s for this offer.
        b.  `selection(offer)`: This event indicates that the clipboard selection has changed.
            i.  If `offer` is `Some(wl_data_offer)`, a new selection is available. This is the offer that should be used for pasting. Log that a new selection is available.
            ii. Check if this `wl_data_offer` supports the `text/plain` MIME type (by checking against the MIME types logged from its `wl_data_offer.offer` events).
            iii. **Trigger Paste Action:** If `text/plain` is supported, initiate the paste. This could be automatic upon new selection or triggered by a user action (e.g., key press like Ctrl+V, or a button in the test app window).
                1.  Create a pipe (`pipe2(O_CLOEXEC)`).
                2.  Call `wl_data_offer.receive("text/plain", write_fd)`, passing the write end of the pipe. Close the client's copy of `write_fd`.
                3.  Dispatch events in a loop while reading from the read end of the pipe (`read_fd`) until EOF. Accumulate the data.
                4.  Once all data is read, close `read_fd`.
                5.  Log the received text. Optionally, display it in the window.
            iv. After data transfer is complete (or if the offer is not used or the desired MIME type is not available), call `wl_data_offer.finish()`.
            v.  If `offer` is `None`, the clipboard selection has been cleared. Log this.
    5.  The application should run, processing events, ready to receive clipboard updates.
    6.  Clean up resources on exit.
-   **Success Criteria:**
    -   The application runs without Wayland protocol errors.
    -   The `wl_data_device.selection` event is received when `clipboard_copier` (or another app) calls `set_selection`.
    -   The associated `wl_data_offer` correctly advertises the `text/plain` MIME type via its `offer` event.
    -   Calling `wl_data_offer.receive("text/plain", fd)` results in the data being successfully written by the source (e.g., `clipboard_copier`) and read by `clipboard_paster` from the pipe.
    -   The text received by `clipboard_paster` exactly matches the text offered by `clipboard_copier`.
    -   Client logs clearly show these operations, including announced MIME types and received data.
-   **Parameters (CLI):**
    -   `--trigger <auto|key>`: Optional. Defines how the paste is triggered upon new selection. `auto` is typical for clipboard monitoring.

## 4. Test Scenario Flow

A typical test execution would involve:
1.  Start the `clipboard_paster` application. Its window appears. It should log that it's monitoring for clipboard selections.
2.  Start the `clipboard_copier` application.
    -   If on `auto` trigger, it immediately creates a data source and sets the selection.
    -   If on `key` or `click` trigger, the user performs the action in `clipboard_copier`'s window.
3.  `clipboard_copier` logs its actions (offering data, setting selection).
4.  `clipboard_paster` should then log:
    -   Receipt of `wl_data_device.data_offer` (if it's a new offer object).
    -   Receipt of `wl_data_offer.offer("text/plain")` from that offer.
    -   Receipt of `wl_data_device.selection` with the new offer.
    -   Its attempt to call `wl_data_offer.receive("text/plain", ...)`.
5.  `clipboard_copier` should log that its `wl_data_source.send` event was triggered for `text/plain` and that it wrote the data.
6.  `clipboard_paster` should log the data it read from the pipe.
7.  The tester compares the source text from `clipboard_copier`'s log/config with the received text in `clipboard_paster`'s log/display. They must match.
8.  (Optional) Test clearing selection: If `clipboard_copier` exits or calls `set_selection(None, serial)`, `clipboard_paster` should receive `wl_data_device.selection(None)`.

## 5. Common Considerations

-   **Serials:** Correctly obtaining and using serials for `wl_data_device.set_selection` is critical. The serial should originate from the user input event (key press, button press) that logically initiated the copy operation. For automated tests, this might require focusing the `clipboard_copier`'s window and simulating an input event if the compositor provides such debug capabilities, or using a recent serial from a pointer/keyboard `enter` event.
-   **Error Handling:** Applications should gracefully handle scenarios where necessary globals are missing or if `wl_data_offer.receive` fails.
-   **Resource Management:** Proper destruction of `wl_data_source` and `wl_data_offer` objects is important. File descriptors used in pipes must be closed diligently to avoid resource leaks and ensure EOF is correctly signaled.
-   **Focus:** While `set_selection` is global, the trigger for copy (and potentially paste, if not automatic) often relies on window focus to get the relevant serials and user intent. Test setup should ensure appropriate window focus.
-   **MIME Type Matching:** Be exact with MIME type strings (e.g., "text/plain").

This design document will guide the development of the Group 4 custom test applications. Results and detailed findings will be documented in `livetest/custom_tests/data_transfer_results.md`.
