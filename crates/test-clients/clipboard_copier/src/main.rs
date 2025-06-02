// Test client: clipboard_copier
// Purpose: To offer data (simple text) to the clipboard.
// Design Doc: livetest/custom_tests/data_transfer_design.md

// TODO: Implement the Wayland client logic as per the design document.
// This includes:
// 1. Connect to Wayland, get wl_seat, then wl_data_device_manager, then wl_data_device.
// 2. Create a simple SHM-backed xdg_toplevel window.
// 3. On a trigger (e.g., auto-start or key press):
//    a. Create a wl_data_source.
//    b. Implement wl_data_source event handlers (esp. `send` for "text/plain").
//    c. Offer "text/plain" MIME type.
//    d. Call wl_data_device.set_selection() with the source and a valid serial.
// 4. Log operations and keep running to serve the data source.
// 5. Clean up resources on exit.

fn main() {
    println!("Test Application: clipboard_copier - Not yet implemented.");
    eprintln!("TODO: Implement Wayland client to create a window, get wl_data_device, create wl_data_source, offer 'text/plain', and set selection.");

    // Placeholder for actual Wayland client logic
    /*
    use std::io::Write;
    use std::os::unix::io::AsRawFd; // For getting fd from File or OwnedFd

    use wayland_client::{Connection, Dispatch, QueueHandle};
    use wayland_client::protocol::{wl_seat, wl_data_device, wl_data_device_manager, wl_data_source};
    // ... other necessary imports for window creation (xdg_shell, wl_shm, wl_compositor) ...

    const DEFAULT_TEXT_TO_COPY: &str = "Hello Wayland Clipboard!";

    struct AppState {
        data_source: Option<wl_data_source::WlDataSource>,
        text_to_copy: String,
        // ... other state for seat, data_device, window objects ...
        // last_serial: u32, // To store a recent serial for set_selection
    }

    impl Dispatch<wl_data_source::WlDataSource, ()> for AppState {
        fn event(
            &mut self,
            _source: &wl_data_source::WlDataSource,
            event: wl_data_source::Event,
            _data: &(),
            _conn: &Connection,
            _qh: &QueueHandle<AppState>,
        ) {
            match event {
                wl_data_source::Event::Send { mime_type, fd } => {
                    println!("[EVENT] wl_data_source.send: mime_type='{}', fd={}", mime_type, fd.as_raw_fd());
                    if mime_type == "text/plain" {
                        // Write self.text_to_copy to the fd
                        // Need to wrap fd in a File or similar to write easily.
                        // The fd is an OwnedFd, so it will be closed when it goes out of scope.
                        let mut file = unsafe { std::fs::File::from(fd) }; // fd is OwnedFd
                        if let Err(e) = writeln!(file, "{}", self.text_to_copy) {
                            eprintln!("Error writing to fd in send event: {}", e);
                        } else {
                            println!("Successfully wrote text to fd for mime_type '{}'", mime_type);
                        }
                    } else {
                        eprintln!("Requested unoffered mime_type '{}', ignoring.", mime_type);
                        // Closing the fd is important if not used, as the sender is expected to.
                        // `fd` (OwnedFd) going out of scope handles this.
                    }
                }
                wl_data_source::Event::Cancelled => {
                    println!("[EVENT] wl_data_source.cancelled: Data source offering was cancelled/destroyed.");
                    // self.data_source.as_ref().map(|ds| ds.destroy()); // Should already be handled if compositor destroys it
                    self.data_source = None;
                }
                wl_data_source::Event::DndFinished => {
                    println!("[EVENT] wl_data_source.dnd_finished (Should not happen for clipboard)");
                }
                wl_data_source::Event::Action { dnd_action } => {
                     println!("[EVENT] wl_data_source.action: {:?} (Should not happen for clipboard)", dnd_action);
                }
                _ => {}
            }
        }
    }
    // ... other Dispatch implementations for seat, registry, windowing objects ...

    // 1. Connect, get registry, bind globals.
    // 2. Create SHM window.
    // 3. Get wl_seat, then wl_data_device_manager, then wl_data_device.
    // 4. Trigger copy:
    //    let data_device = app_state.data_device.as_ref().unwrap();
    //    let new_data_source = data_device.create_data_source(&qh, ());
    //    new_data_source.offer("text/plain".to_string());
    //    app_state.data_source = Some(new_data_source);
    //    // A valid serial is needed here, typically from an input event (key/button press)
    //    // For auto-copy, might need to use serial from window focus `enter` event.
    //    // let serial = app_state.last_serial;
    //    // data_device.set_selection(app_state.data_source.as_ref(), serial);
    //    // conn.flush();
    //    println!("Offered '{}' to clipboard.", app_state.text_to_copy);
    // 5. Loop with event_queue.dispatch_pending() or blocking_dispatch().
    // 6. Cleanup (destroy data_source if not None, etc.).
    */

    eprintln!("Full functionality for clipboard_copier is not implemented yet.");
    std::process::exit(1); // Indicate not implemented
}
