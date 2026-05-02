# Brainstorm: Anvil Sentinel — The "Just Install and Use" Experience

**Date:** 2026-05-02
**Author:** Gemini CLI
**Status:** Pitch / Brainstorm
**Target Persona:** Senior Influencers, Platform Engineers, "Low-Friction" Senior Developers

---

## 1. The Problem: The "Last Mile" Gap

As of `v0.5.0-beta`, Anvil has a fast Rust kernel and a functional MCP bridge. However, a senior developer installing it today faces three points of friction that prevent a "wow" moment:

1.  **AI Passivity:** Even after `anvil mcp install`, tools like Cursor or Claude Code don't "know" they are governed. They often bypass the validation tool unless the user explicitly instructs them to use it in every session.
2.  **Configuration Blindness:** `anvil mcp install` handles the plumbing but doesn't fix the "path problem" for all clients (e.g., Claude Code's config location mismatch in #1195).
3.  **Static Onboarding:** The `anvil start` screen is a menu of things to *learn*, rather than a display of things Anvil is *already doing*.

## 2. The Solution: "Anvil Sentinel"

We move from "Anvil is a tool you run" to **"Anvil is a sentinel that protects."**

### A. Sentinel Auto-Rules (The "Brain" Injection)
Modify `anvil mcp install` to automatically detect and update editor-specific rule files (`.cursorrules`, `.clauderules`, or global instructions).

*   **Change:** Inject a mandatory governance directive:
    > "CRITICAL: You are governed by Anvil. You MUST call `anvil_validate_write` for every file modification. If Anvil returns a block, you MUST explain the violation to the user and stop. Do not attempt to bypass this tool."
*   **Result:** The AI starts respecting Anvil the moment the editor is opened. No user prompting required.

### B. The "Anvil Pulse" (Live Governance Hub)
Transform the `anvil start` (Welcome) surface from a static list of commands into a live **Sentinel Dashboard**.

*   **Live Metrics:** Show "Active Protection" status for detected editors (Cursor: Active, Claude: Active).
*   **Proof of Speed:** Display the "10µs deterministic scan" metric live as they save files in the background.
*   **Activity Feed:** A scrolling list of recent "Sentinel Decisions":
    *   `[14:02:31] Cursor Proposal: src/auth.ts -> APPROVED`
    *   `[14:03:05] Claude Proposal: .env -> BLOCKED (Secret Leak)`
*   **Result:** The user sees Anvil working in real-time. It feels like a platform, not just a binary.

### C. The "Speedrun" Interactive Installer
Enhance `install.sh` and the post-install flow to be proactive.

*   **Proactive Detection:** "I've detected Cursor and Claude Code on your machine. Enable Sentinel Protection? [Y/n]"
*   **Unified Setup:** One command runs the binary install, MCP config, and Rule injection.
*   **Result:** A "Zero-to-Protected" time of under 30 seconds.

## 3. Implementation Slices (Suggested)

| Slice | Work Item | Complexity | Value |
| :--- | :--- | :--- | :--- |
| **S1** | `anvil mcp install --inject-rules` | Medium | **High** (Solves AI bypass) |
| **S2** | `anvil start` -> Pulse Dashboard | Medium | **High** (Visual "Wow") |
| **S3** | Fix Claude Code config path (#1195) | Low | Medium (Correctness) |
| **S4** | Interactive `install.sh` | Low | High (Onboarding) |

## 4. The "Wow" Moment
The developer runs the install script. They open Cursor. They try to paste a hardcoded API key or an architectural anti-pattern. **Before they even hit save**, Cursor tells them: *"I cannot apply this change; Anvil Sentinel has blocked it due to a security policy."*

That is the version people can "just install and use."
