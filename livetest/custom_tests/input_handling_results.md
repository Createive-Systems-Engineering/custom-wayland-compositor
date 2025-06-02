# Results: Group 3 - Input Handling Test Applications

**Date:** (Placeholder for test execution date)
**Compositor Version/Commit:** (Placeholder)

## 1. Overview

This document records the results of executing custom test applications designed to validate the compositor's handling of pointer (mouse) and keyboard input events, as well as optional advanced pointer features like relative motion and pointer constraints. The detailed designs for these tests are specified in `livetest/custom_tests/input_handling_design.md`.

**Current Status:** The test applications (`pointer_events_logger`, `keyboard_events_logger`, `relative_pointer_test`, `pointer_constraints_test`) are currently skeletons. Full implementation of their Wayland client logic, including potentially `xkbcommon` for keyboard tests, is required. The results below are placeholders and will be updated by the development/testing team once the applications are developed and executed against the target compositor.

## 2. Test Application Results

### 2.1. `pointer_events_logger`
- **Objective:** Verify reception of various `wl_pointer` events (enter, leave, motion, button, axis).
- **Design Document:** `livetest/custom_tests/input_handling_design.md#31-test-app-pointer_events_logger`
- **Status:** PENDING_IMPLEMENTATION
- **Observations & Logged Events:**
    - (To be filled by developer/tester post-execution. This section should contain a summary of interactions performed and a comparison against the logged output from the client. E.g., "Moved mouse into window, `wl_pointer.enter` logged with correct surface and coordinates. Clicked left button, `wl_pointer.button` for BTN_LEFT (pressed/released) logged correctly...")
- **Success Criteria Met:** (Yes/No/Partial - To be filled by developer/tester)
- **Compositor Logs:**
    - (Relevant snippets or summary of compositor behavior - To be filled)
- **Client Logs (`pointer_events_logger` stdout):**
    - (Attach or summarize the full stdout log from the client for the test run - To be filled)

### 2.2. `keyboard_events_logger`
- **Objective:** Verify reception of `wl_keyboard` events and basic keymap/keysym interpretation.
- **Design Document:** `livetest/custom_tests/input_handling_design.md#32-test-app-keyboard_events_logger`
- **Status:** PENDING_IMPLEMENTATION
- **xkbcommon Integrated:** (Yes/No - To be filled by developer/tester based on implementation)
- **Observations & Logged Events:**
    - (To be filled by developer/tester post-execution. E.g., "Window focused, `wl_keyboard.enter` logged. `wl_keyboard.keymap` received. Typed 'Test', Shift+'A', Ctrl+'C'. Logged scancodes, states, and (if xkbcommon) keysyms (KEY_T, KEY_e, KEY_s, KEY_t, KEY_A, KEY_C) and modifiers correctly.")
- **Success Criteria Met:** (Yes/No/Partial - To be filled by developer/tester)
- **Compositor Logs:**
    - (Relevant snippets or summary - To be filled)
- **Client Logs (`keyboard_events_logger` stdout):**
    - (Attach or summarize the full stdout log from the client - To be filled)

### 2.3. `relative_pointer_test` (Optional Stretch Goal)
- **Objective:** Test `zwp_relative_pointer_v1` for unaccelerated, relative pointer motion.
- **Design Document:** `livetest/custom_tests/input_handling_design.md#33-test-app-relative_pointer_test`
- **Status:** PENDING_IMPLEMENTATION
- **Implemented & Tested:** (Yes/No - To be filled by developer/tester)
- **Observations & Logged Events:**
    - (To be filled if implemented and tested. E.g., "Relative pointer manager bound. `relative_motion` events logged with dx_unaccel/dy_unaccel values when mouse moved rapidly.")
- **Success Criteria Met:** (Yes/No/Partial/NA - To be filled by developer/tester)
- **Compositor Logs:**
    - (Relevant snippets - To be filled)
- **Client Logs (`relative_pointer_test` stdout):**
    - (Attach or summarize the full stdout log - To be filled)

### 2.4. `pointer_constraints_test` (Optional Stretch Goal)
- **Objective:** Test basic pointer locking/confinement using `zwp_pointer_constraints_v1`.
- **Design Document:** `livetest/custom_tests/input_handling_design.md#34-test-app-pointer_constraints_test`
- **Status:** PENDING_IMPLEMENTATION
- **Implemented & Tested:** (Yes/No - To be filled by developer/tester)
- **Observations & Logged Events:**
    - (To be filled if implemented and tested. E.g., "Pointer constraints manager bound. `lock_pointer` request sent. `locked` event received. Mouse movement appeared confined. `unlocked` event received after destroy request.")
- **Success Criteria Met:** (Yes/No/Partial/NA - To be filled by developer/tester)
- **Compositor Logs:**
    - (Relevant snippets - To be filled)
- **Client Logs (`pointer_constraints_test` stdout):**
    - (Attach or summarize the full stdout log - To be filled)

## 3. Summary of Issues Found (Compositor Bugs)

- **Issue #ID:** (Brief description of bug - To be filled)
  - **Test Application:** (Which test app revealed the bug)
  - **Severity:** (Critical/High/Medium/Low)
  - **Status:** (Open/In Progress/Resolved)

*(Placeholder: This section will list any bugs or protocol deviations related to input handling discovered in the compositor through these Group 3 tests.)*

## 4. Overall Assessment for Group 3

- **Overall Test Status:** PENDING_IMPLEMENTATION
- **General Comments:**
    - (To be filled by developer/tester: e.g., "These tests will provide essential validation for user interaction capabilities. `xkbcommon` integration in `keyboard_events_logger` will be key for proper key interpretation testing.")
- **Challenges Encountered (during test development or execution):**
    - (e.g., Ensuring correct focus for keyboard tests, interpreting complex modifier sequences, visual verification of pointer constraints - To be filled)

*(Placeholder: This section will provide a summary of the overall success, findings, and any specific challenges related to the Group 3 tests once they are implemented and executed.)*
