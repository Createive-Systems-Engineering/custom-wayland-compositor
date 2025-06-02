// Test client: clipboard_paster
// Purpose: To receive (paste) data from the clipboard.
// Design Doc: livetest/custom_tests/data_transfer_design.md

// TODO: Implement the Wayland client logic as per the design document.
// This includes:
// 1. Connect to Wayland, get wl_seat, then wl_data_device_manager, then wl_data_device.
// 2. Create a simple SHM-backed xdg_toplevel window.
// 3. Implement wl_data_device event handlers:
//    - data_offer: Store new wl_data_offer, implement its `offer` event handler to log MIME types.
//    - selection: If offer is Some, check for "text/plain". If available, call wl_data_offer.receive()
//                 with the write end of a pipe. Read from the read end of the pipe. Log received text.
//                 Call wl_data_offer.finish(). If offer is None, log selection cleared.
// 4. Log operations and received data.
// 5. Clean up resources on exit.

fn main() {
    println!("Test Application: clipboard_paster - Not yet implemented.");
    eprintln!("TODO: Implement Wayland client to create a window, get wl_data_device, handle wl_data_offer for 'text/plain', receive data via pipe, and log it.");

    // Placeholder for actual Wayland client logic
    /*
    use std::io::Read;
    use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd}; // For pipe and OwnedFd
    use std::os::unix::net::UnixStream; // Can use UnixStream::pair() for a pipe-like mechanism

    use wayland_client::{Connection, Dispatch, QueueHandle};
    use wayland_client::protocol::{wl_seat, wl_data_device, wl_data_device_manager, wl_data_offer};
    // ... other necessary imports for window creation ...

    struct AppState {
        // data_device: Option<wl_data_device::WlDataDevice>,
        // current_selection_offer: Option<wl_data_offer::WlDataOffer>,
        // available_mime_types_for_current_offer: Vec<String>,
        // ... other state for seat, window objects ...
    }

    impl Dispatch<wl_data_device::WlDataDevice, ()> for AppState {
        fn event(
            &mut self,
            _device: &wl_data_device::WlDataDevice,
            event: wl_data_device::Event,
            _data: &(),
            conn: &Connection,
            qh: &QueueHandle<AppState>,
        ) {
            match event {
                wl_data_device::Event::DataOffer { offer } => {
                    println!("[EVENT] wl_data_device.data_offer: new offer id={:?}", offer.id());
                    // The offer object is created, now we need to handle its events, esp. .offer for mime types
                    // This is usually done by setting the user_data for this new offer to something that
                    // can store the mime types, or by dispatching events for it with a specific handler.
                    // For simplicity in this placeholder, we assume its 'offer' events will populate mime types
                    // before 'selection' event is processed if it's the current selection.
                }
                wl_data_device::Event::Selection { id: opt_offer } => {
                    if let Some(offer) = opt_offer {
                        println!("[EVENT] wl_data_device.selection: New selection with offer id={:?}", offer.id());
                        // self.current_selection_offer = Some(offer.clone()); // Clone to keep it if needed
                        // self.available_mime_types_for_current_offer.clear();

                        // At this point, the offer's .offer events should have ideally already been dispatched
                        // telling us the mime types. If not, one might need to dispatch_pending here or ensure
                        // the offer's user_data context is setup to collect mime_types.
                        // For now, let's assume we know it offers "text/plain" from prior .offer events.

                        // Let's simulate asking for "text/plain"
                        // First, create a pipe.
                        let mut fds = [-1; 2];
                        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } == 0 {
                            let read_fd = unsafe { std::os::unix::io::OwnedFd::from_raw_fd(fds[0]) };
                            let write_fd = unsafe { std::os::unix::io::OwnedFd::from_raw_fd(fds[1]) };

                            println!("Requesting 'text/plain' from offer id={:?}", offer.id());
                            offer.receive("text/plain".to_string(), write_fd); // Pass OwnedFd
                            offer.finish(); // We are done with this offer for now after one request.
                                            // Or call finish after reading is complete.
                            conn.flush().expect("Flush failed after receive request");

                            // Spawn a thread or use async to read from read_fd to avoid blocking event loop.
                            // For this placeholder, synchronous read (which is bad):
                            std::thread::spawn(move || {
                                let mut pipe_reader = unsafe { std::fs::File::from(read_fd) }; // read_fd is OwnedFd
                                let mut received_data = String::new();
                                if let Err(e) = pipe_reader.read_to_string(&mut received_data) {
                                    eprintln!("Error reading from pipe: {}", e);
                                } else {
                                    println!("Successfully received text data:\n---\n{}\n---", received_data);
                                }
                            });

                        } else {
                            eprintln!("Failed to create pipe: {}", std::io::Error::last_os_error());
                        }
                    } else {
                        println!("[EVENT] wl_data_device.selection: Selection cleared.");
                        // self.current_selection_offer = None;
                        // self.available_mime_types_for_current_offer.clear();
                    }
                }
                // ... other wl_data_device events like Enter, Leave, Motion (for DND, not clipboard focus here)
                _ => {}
            }
        }
    }

    // Dispatch for wl_data_offer events (specifically .offer for MIME types)
    impl Dispatch<wl_data_offer::WlDataOffer, ()> for AppState {
        fn event(
            &mut self,
            _offer: &wl_data_offer::WlDataOffer,
            event: wl_data_offer::Event,
            _data: &(),
            _conn: &Connection,
            _qh: &QueueHandle<AppState>,
        ) {
            if let wl_data_offer::Event::Offer { mime_type } = event {
                println!("[EVENT] wl_data_offer.offer: available mime_type='{}'", mime_type);
                // if Some(_offer.id()) == self.current_selection_offer.as_ref().map(|o| o.id()) {
                //     self.available_mime_types_for_current_offer.push(mime_type);
                // }
            }
            // Handle other wl_data_offer events like SourceActions, Action if necessary
        }
    }
    // ... other Dispatch implementations for seat, registry, windowing objects ...

    // 1. Connect, get registry, bind globals.
    // 2. Create SHM window.
    // 3. Get wl_seat, then wl_data_device_manager, then wl_data_device.
    //    The wl_data_device event handler should then process offers/selections.
    // 4. Loop with event_queue.dispatch_pending() or blocking_dispatch().
    // 5. Cleanup.
    */

    eprintln!("Full functionality for clipboard_paster is not implemented yet.");
    std::process::exit(1); // Indicate not implemented
}
