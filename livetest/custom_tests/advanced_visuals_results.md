# Results: Group 5 (Part 1) - Fractional Scaling & Server-Side Decorations

**Date:** (Placeholder for test execution date)
**Compositor Version/Commit:** (Placeholder)
**Compositor Configuration Notes (if relevant for scaling/decorations):** (e.g., "Fractional scaling enabled via config", "Default decoration mode set to server-side" - Placeholder)

## 1. Overview

This document records the results of executing custom test applications designed to validate advanced visual features of the compositor, specifically focusing on fractional surface scaling via `wp_fractional_scale_v1` and server-side window decorations via `xdg_decoration_unstable_v1`. The detailed designs for these tests are specified in `livetest/custom_tests/advanced_visuals_design.md`.

**Current Status:** The test applications (`fractional_scale_tester` and `ssd_decoration_tester`) are currently skeletons. Full implementation of their Wayland client logic is required. The results documented below will be placeholders until these applications are developed and executed. These tests inherently rely heavily on **visual verification** by the tester, supplemented by client and compositor logs.

## 2. Test Application Results

### 2.1. `fractional_scale_tester`
- **Objective:** Verify the compositor's handling of fractional scaling requests via `wp_fractional_scale_v1` and the visual quality of the scaled output.
- **Design Document:** `livetest/custom_tests/advanced_visuals_design.md#31-test-app-fractional_scale_tester`
- **Status:** PENDING_IMPLEMENTATION
- **Test Scenarios Executed:** (e.g., Requested scale 1.0 (120/120), 1.25 (150/120), 1.5 (180/120), 1.75 (210/120), 2.0 (240/120). Cycle mode tested. - To be filled by developer/tester)
- **Visual Observations (Screenshots should be linked or referenced):**
    - **Scale 1.0x (120/120):** (Description of sharpness, any unexpected scaling - To be filled)
    - **Scale 1.25x (150/120):** (Description of sharpness, clarity of pattern, any visible artifacts like blurriness or shimmering - To be filled)
    - **Scale 1.5x (180/120):** (Description - To be filled)
    - **Scale [Other Tested]:** (Description - To be filled)
- **Logged Scale Factors (Requested Numerator vs. Acknowledged Numerator by Compositor via `preferred_scale` event):**
    - Scenario 1 (e.g., Request: 150, Acknowledged: 150) - (To be filled)
    - Scenario 2 (e.g., Request: 180, Acknowledged: 180) - (To be filled)
    - Scenario 3 (e.g., Request: 210, Acknowledged: 200 - if compositor chose a different scale) - (To be filled)
- **Success Criteria Met:** (Yes/No/Partial - Based on logs and visual inspection - To be filled by developer/tester)
- **Compositor Logs:**
    - (Relevant snippets regarding fractional scaling operations or errors - To be filled)
- **Client Logs (`fractional_scale_tester` stdout):**
    - (Attach or summarize relevant logs, especially requested and acknowledged scale factors - To be filled)

### 2.2. `ssd_decoration_tester`
- **Objective:** Verify the compositor's handling of server-side decorations versus client-side decorations via `xdg_decoration_unstable_v1`.
- **Design Document:** `livetest/custom_tests/advanced_visuals_design.md#32-test-app-ssd_decoration_tester`
- **Status:** PENDING_IMPLEMENTATION
- **Test Scenarios Executed:** (e.g., Requested Server-Side mode, Requested Client-Side mode - To be filled by developer/tester)
- **Visual Observations (Screenshots should be linked or referenced):**
    - **Requested Server-Side:** (Description of window decorations: Are they present? Do they look like native compositor decorations? Is client content correctly placed? - To be filled)
    - **Requested Client-Side:** (Description: Are server decorations absent or minimal? If client drew a border, is it visible? - To be filled)
- **Logged Decoration Modes (Requested Mode vs. Acknowledged Mode by Compositor via `configure` event):**
    - Scenario 1 (e.g., Requested: ServerSide, Acknowledged: ServerSide) - (To be filled)
    - Scenario 2 (e.g., Requested: ClientSide, Acknowledged: ClientSide or ServerSide if compositor overrides) - (To be filled)
- **Success Criteria Met:** (Yes/No/Partial - Based on logs and visual inspection - To be filled by developer/tester)
- **Compositor Logs:**
    - (Relevant snippets regarding decoration mode negotiation or errors - To be filled)
- **Client Logs (`ssd_decoration_tester` stdout):**
    - (Attach or summarize relevant logs, especially requested and acknowledged decoration modes - To be filled)

## 3. Summary of Issues Found (Compositor Bugs, Visual Artifacts, or Protocol Deviations)

- **Issue #ID:** (Brief description of issue - To be filled)
  - **Test Application:** (`fractional_scale_tester` or `ssd_decoration_tester`)
  - **Affected Feature:** (Fractional Scaling, Server-Side Decorations)
  - **Severity:** (Critical/High/Medium/Low - e.g., visual artifact, wrong behavior, crash)
  - **Status:** (Open/In Progress/Resolved)

*(Placeholder: This section will list any bugs, unexpected behaviors, visual rendering issues, or protocol handling deviations discovered in the compositor through these Group 5 (Part 1) tests.)*

## 4. Overall Assessment for Group 5 (Part 1)

- **Overall Test Status:** PENDING_IMPLEMENTATION
- **General Comments:**
    - (To be filled by developer/tester: e.g., "These tests are crucial for HiDPI display usability and modern look-and-feel. Visual inspection is paramount.")
- **Challenges Encountered (during test development or execution):**
    - (e.g., Ambiguity in interpreting visual correctness for certain fractional scales, compositor overriding decoration modes unexpectedly - To be filled)

*(Placeholder: This section will provide a summary of the overall success, findings, and any specific challenges related to the Group 5 (Part 1) tests once they are implemented and executed.)*
