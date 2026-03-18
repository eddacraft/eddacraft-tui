# SYSTEM PROMPT: EDDACRAFT / ANVIL TUI DEVELOPMENT

## 1. PROJECT OVERVIEW

You are assisting in the development of **Anvil**, a CLI/TUI tool built in Rust
using the `ratatui` framework. Anvil is a deterministic policy engine that
governs probabilistic AI workflows. It acts as the "adult in the room"—enforcing
rules, watching file system changes, and blocking non-compliant AI agent actions
at generation time. Anvil is part of the broader **EddaCraft** foundry suite.

## 2. BRAND & AESTHETIC LAWS

- **Aesthetic:** "Nordic Brutalist" / Industrial Terminal.
- **Tone:** Strict, authoritative, structural, and quiet. It is a flight
  recorder, not a chat app.
- **Borders:** Strictly sharp. Use `BorderType::Plain`. Absolutely no rounded
  corners.
- **Spacing:** Generous and highly deliberate. Left-align text using hardcoded
  spaces rather than relying on standard terminal padding.
- **Language:** UK English spelling for all variables and comments (e.g.,
  `colour`).

## 3. DESIGN SYSTEM & COLOUR TOKENS

Do not use standard ANSI or Tailwind terminal colours. You must use these exact
RGB tuples for all Ratatui styling:

```rust
pub struct EddaTheme;
impl EddaTheme {
    // Core Backgrounds & Structural elements
    pub const VOID: Color = Color::Rgb(13, 13, 15);      // Terminal background
    pub const BORDER: Color = Color::Rgb(42, 42, 46);    // Standard panel borders

    // Typography
    pub const FG: Color = Color::Rgb(235, 235, 235);     // Primary text
    pub const MUTED: Color = Color::Rgb(133, 133, 138);  // Inactive text, comments, footers

    // Brand & Semantic Accents
    pub const EMBER: Color = Color::Rgb(204, 85, 0);     // Anvil Primary (Headers, active borders)
    pub const GROWTH: Color = Color::Rgb(46, 139, 87);   // Edda Success (Pass indicators, active status)

    // Standard CLI States (Desaturated)
    pub const ERROR: Color = Color::Rgb(201, 74, 74);    // Blocked actions, failures
    pub const WARNING: Color = Color::Rgb(208, 140, 56); // Warnings
}
```

## 4. TUI ARCHITECTURE & LAYOUT

The Ratatui layout is strictly divided into three vertical chunks, with the
middle chunk split horizontally:

1. Header (Top): Fixed Constraint::Length(9). Contains the Macro Anvil block
   logo.

2. Core (Middle): Constraint::Min(10). Split horizontally:

- Left Pane (40%): [ ≡ ] ACTIVE_POLICY. Shows the loaded Rego rules and plan
  spec. Standard BORDER colour.
- Right Pane (60%): [ > ] SIGNAL_INTERCEPTOR. The real-time watcher. Border
  coloured in EMBER to draw the eye.

3. Footer (Bottom): Fixed Constraint::Length(5). Split horizontally:

- Left (80%): [ SYSTEM_LOGS ]. Standard CLI output.
- Right (20%): EddaCraft watermark.

## 5. REQUIRED ICONOGRAPHY & LOGOS

Micro-Prefixes (For inline logs and statuses):

- Anvil (Governance/Action): [ = ]
- Edda (Memory/Context): [ ≡ ]
- EddaCraft (Parent System): [ ■ ]

The Macro Anvil Header (Must be rendered perfectly in EMBER, with text in
FG/MUTED):

```Plaintext
████     ████
██         ██
██  █████  ██
██         ██   a n v i l
██  █████  ██
██         ██
████     ████
```

The EddaCraft Footer Watermark (Bottom right of the TUI, rendered in MUTED and
BORDER):

```Plaintext
  [ ■ ] e d d a c r a f t
        v0.9.2-beta
```

## 6. INSTRUCTIONS FOR CLAUDE

When writing or refactoring Ratatui code for this project:

- Adhere strictly to the EddaTheme colours.
- Never introduce arbitrary ASCII art; stick to the provided block logos.
- Ensure all lists and log outputs are formatted with deep, deliberate
  indentation to maintain the Brutalist grid.
- Assume all code is going into a professional, heavy-duty Rust environment.
