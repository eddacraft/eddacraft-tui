# PORT-001: Port shared layout and display components to eddacraft-tui

## Steps

1. [x] Read Ink source: Header, Container, Divider, Spinner, StatusBadge, Confirm
2. [x] Read existing eddacraft-tui widget patterns (Select, TextInput, ProgressBar, StatusBar)
3. [x] Map Ink theme colours to EddaCraft Theme trait methods
4. [x] Implement Spinner widget with braille dot animation and SpinnerState
5. [x] Implement Header widget with separator, title, subtitle, version
6. [x] Implement Container widget with Primary/Secondary/Subtle variants
7. [x] Implement Divider widget with Heavy/Light variants
8. [x] Implement StatusBadge widget with 6 status types and icons
9. [x] Implement Confirm widget with toggle/confirm/reset state machine
10. [x] Update widgets/mod.rs with 6 new module declarations
11. [x] Update lib.rs prelude with all new exports
12. [x] Verify `cargo test` passes (31 eddacraft-tui tests), `cargo clippy` clean
