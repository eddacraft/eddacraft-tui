# TUI frameworks and libraries

Research snapshot: **2 August 2026**. This is deliberately broad, with the Rust ecosystem covered most deeply. “Framework” is used loosely in the wider catalogue; the lower-level terminal backends and widget/component libraries are separated from full application frameworks.

## Executive take

- **Safest Rust default:** [Ratatui](https://github.com/ratatui/ratatui). It is the centre of gravity, actively maintained, cross-platform, and has the deepest ecosystem. It is deliberately an immediate-mode rendering library rather than a batteries-included app framework.
- **Best higher-level Rust alternatives:** [iocraft](https://github.com/ccbrown/iocraft) for React/SwiftUI-like declarative composition; [Cursive](https://github.com/gyscos/cursive) for traditional retained widgets and a managed event loop; [tui-realm](https://github.com/veeso/tui-realm) for Elm/React architecture on Ratatui; [R3BL TUI](https://docs.rs/r3bl_tui/latest/r3bl_tui/) for async reactive apps; [AppCUI-rs](https://github.com/gdt050579/AppCUI-rs) for a complete desktop-GUI-like toolkit.
- **Best TypeScript bet:** [OpenTUI](https://github.com/anomalyco/opentui) for ambitious, high-performance applications; [Ink](https://github.com/vadimdemedes/ink) for mature React-style CLIs; [TermUI](https://www.termui.io/) and [Rezi](https://github.com/RtlZeroMemory/Rezi) are newer high-level contenders.
- **Best non-Rust references:** [Bubble Tea](https://github.com/charmbracelet/bubbletea) (Go), [Textual](https://github.com/Textualize/textual) (Python), [FTXUI](https://github.com/ArthurSonzogni/FTXUI) (C++), [Brick](https://github.com/jtdaugherty/brick) (Haskell), and [Terminal.Gui](https://github.com/gui-cs/Terminal.Gui) (.NET).

## Rust — application frameworks and substantial UI libraries

### Established / strongest candidates

1. **[Ratatui](https://github.com/ratatui/ratatui)** — immediate-mode renderer, layout, styling, widgets, multiple terminal backends. Successor to `tui-rs`; the ecosystem default.
2. **[Cursive](https://github.com/gyscos/cursive)** — retained-mode, event-driven widget toolkit; dialogs, views, focus and input handled for you; multiple backends.
3. **[iocraft](https://github.com/ccbrown/iocraft)** — declarative React/SwiftUI-like components, hooks and event handling, flexbox via Taffy, full-screen and inline output.
4. **[tui-realm](https://github.com/veeso/tui-realm)** — higher-level Ratatui framework with reusable components, properties/state, messages and Elm-style `update` routines.
5. **[R3BL TUI](https://docs.rs/r3bl_tui/latest/r3bl_tui/)** — reactive, unidirectional, async-first framework; supports full-screen, partial-screen and readline-style UI.
6. **[AppCUI-rs](https://github.com/gdt050579/AppCUI-rs)** — batteries-included traditional UI toolkit: controls, windows, menus, dialogs, themes and event routing.
7. **[Rooibos](https://github.com/maciejhirsz/rooibos)** — reactive Ratatui framework with components and a modern declarative model; worth evaluating, but younger than the leaders.
8. **[Yeehaw](https://docs.rs/yeehaw/latest/yeehaw/)** — batteries-included renderer, event routing and a broad element set; explicitly experimental in parts.
9. **[SuperLightTUI](https://github.com/subinium/SuperLightTUI)** — immediate-mode framework with a large widget set, flexbox layout and animations.
10. **[Reratui](https://docs.rs/reratui/latest/reratui/)** — React-inspired components and hooks on Ratatui, including state batching, effects and async hooks; very new.
11. **[RxTUI](https://docs.rs/rxtui/latest/rxtui/)** — declarative component tree, virtual DOM/diffing, reusable form components and async effects; very new.
12. **[termuix](https://docs.rs/termuix/latest/termuix/)** — layered low-level terminal, console API and full reactive component framework with signals, flex-style layout and mouse support; very new.
13. **[frankentui](https://github.com/Dicklesworthstone/frankentui)** — minimal high-performance TUI kernel with diff rendering, inline mode, lifecycle cleanup and Web/WASM support; young and opinionated.
14. **[rat-salsa](https://docs.rs/rat-salsa/latest/rat_salsa/)** — Ratatui widgets plus an application event queue, tasks, timers, focus and dialog handling.
15. **[tui-builder](https://github.com/joshka/tui-builder)** — batteries-included MVC framework around the Ratatui/tui-rs style stack; check current maintenance before adopting.
16. **[tui-react](https://github.com/fdehau/tui-rs/tree/master/examples)** / community `tui-react` crates — React-like widget composition for Ratatui; fragmented, so inspect the exact crate and maintainer before use.
17. **`widgetui`** — Bevy-like widget system for Ratatui/Crossterm, listed in the Ratatui ecosystem; small/experimental.
18. **`eye-declare`** — [declarative inline TUI rendering](https://eye-declare.rs/getting-started/introduction/) on Ratatui. It intentionally is not a full-screen application framework.
19. **`tuire`** — lightweight grid/layout-oriented TUI library; small and emerging.
20. **`Turbo Vision for Rust`** — [Rust port/reinterpretation](https://github.com/aovestdipaperino/turbo-vision-4-rust) of the classic desktop-like TUI model; niche/experimental.
21. **`Remui`** — micro Model–Update–View framework on Ratatui; emerging project, verify package ownership and activity before use.

### Rendering, terminal and console foundations

These can build a TUI, but they sit below a full application framework.

1. **[Crossterm](https://github.com/crossterm-rs/crossterm)** — dominant pure-Rust cross-platform terminal control/event backend; Ratatui’s default.
2. **[Termion](https://github.com/redox-os/termion)** — pure-Rust terminal control for Unix-like systems.
3. **[Termwiz](https://github.com/wezterm/wezterm/tree/main/termwiz)** — sophisticated terminal model, escape parser, surfaces/cells, hyperlinks and modern features; part of WezTerm.
4. **[Termina](https://docs.rs/termina/latest/termina/)** — modern terminal input/output backend supported by current Ratatui.
5. **[tuikit](https://github.com/lotabout/tuikit)** — low-level terminal UI toolkit originally built for `skim`.
6. **[console_engine](https://github.com/VincentFoulon80/console_engine)** — terminal drawing/game-style engine with keyboard and mouse input.
7. **[unsegen](https://github.com/ftilde/unsegen)** — composable terminal UI components and rendering primitives; older but conceptually interesting.
8. **[pancurses](https://github.com/ihalila/pancurses)** — Rust wrapper over ncurses/PDCurses.
9. **[ncurses-rs](https://github.com/jeaye/ncurses-rs)** — direct ncurses bindings.
10. **[rustbox](https://github.com/gchp/rustbox)** — Termbox bindings; largely legacy.
11. **[BearLibTerminal bindings](https://github.com/nabijaczleweli/BearLibTerminal.rs)** — cell-based terminal/window rendering, often used for roguelikes.
12. **[notcurses Rust bindings](https://github.com/dankamongmen/notcurses)** — modern terminal graphics through the C library and bindings.
13. **[Zaz](https://github.com/j-g00da/zaz)** — efficient terminal rendering library; small/niche.
14. **[ez_term](https://github.com/ddbnl/ez_term)** — XML/CSS-inspired declarative terminal UI; check maintenance.
15. **[zi](https://github.com/mcobzarenco/zi)** — declarative Elm-style terminal UI; appears dormant/legacy.

### Ratatui ecosystem pieces worth knowing

Not full frameworks individually, but these can materially change the build-vs-buy decision:

- **App structure / integration:** [awesome-ratatui](https://github.com/ratatui/awesome-ratatui), `ratatui-async`, `bevy_ratatui`, [Ratzilla](https://github.com/ratatui/ratzilla) (Ratatui-style web/WASM), `ratatui-uefi`, `ratatui-wgpu`, `soft_ratatui`, `mousefood` (embedded-graphics).
- **Input/editing:** `tui-input`, `ratatui-textarea` / `tui-textarea`, `tui-prompts`, `tui-textarea-search`.
- **Navigation/content:** `tui-tree-widget`, `tui-widget-list`, `tui-scrollview`, `tui-menu`, `tui-popup`, `tui-tabs`, `tui-nodes`, `ratatui-explorer`.
- **Presentation:** `tui-big-text`, `tui-markdown`, `tui-logger`, `tui-term`, `ratatui-image`, `ratatui-garnish`, `throbber-widgets-tui`, `tui-bar-graph`.
- **Effects:** [TachyonFX](https://github.com/junkdog/tachyonfx) for shader-like TUI effects and transitions.
- **Testing:** Ratatui’s `TestBackend`, snapshot testing with `insta`, and terminal integration tests via PTYs are the usual combination.

### Legacy Rust projects

- **[tui-rs](https://github.com/fdehau/tui-rs)** — original immediate-mode library; discontinued, use Ratatui.
- **`dioxus-tui`** — former Dioxus renderer for terminal UIs; discontinued/removed from the main project.
- **`tui-rs-revival`** — transitional fork that became Ratatui; historical only.
- **`rustbox`**, **`zi`**, **`unsegen`** — still useful as references or for existing code, but poor greenfield defaults unless activity has resumed.

## TypeScript / JavaScript — top frameworks

1. **[OpenTUI](https://github.com/anomalyco/opentui)** — native Zig renderer with TypeScript API; flexbox/Yoga, tree-sitter highlighting, code/diff/input/select/scroll components, focus, animation, React and Solid bindings. Powers OpenCode. **Best ambitious TS choice.**
2. **[Ink](https://github.com/vadimdemedes/ink)** — mature React renderer for interactive CLIs; huge ecosystem and excellent ergonomics. Better for CLI-shaped apps than very dense high-frequency dashboards.
3. **[TermUI](https://www.termui.io/)** — newer TypeScript-first framework with JSX, a large component catalogue, routing, state, themes, animation and hot reload. Promising, but validate maturity and package stability.
4. **[Rezi](https://github.com/RtlZeroMemory/Rezi)** — high-performance TS/Bun framework with fluent and JSX APIs, widgets and styling, without requiring React.
5. **[Blessed](https://github.com/chjj/blessed)** — classic retained widget toolkit for Node. Influential and widely used, but the original is old; prefer a maintained fork for greenfield work.
6. **[neo-blessed](https://github.com/embarklabs/neo-blessed)** — maintained Blessed fork; check current activity against alternatives.
7. **[Unblessed](https://www.npmjs.com/package/@unblessed/core)** — modern TypeScript rewrite/enhancement of Blessed with widgets and terminal primitives.
8. **[react-blessed](https://github.com/Yomguithereal/react-blessed)** — React renderer targeting Blessed widgets; older but useful for existing Node stacks.
9. **[Terminal Kit](https://github.com/cronvel/terminal-kit)** — capable lower-level terminal library with input, screen buffers, menus, forms and document model.
10. **[Melker](https://jsr.io/@melker/melker)** — Deno/TypeScript, HTML-like document-first apps, explicit permission policy, URL sharing and dev tools; unusual and interesting.
11. **[terminaltui](https://terminaltui.dev/)** — newer TypeScript framework for interactive terminal apps, no JSX/template language.
12. **[Convo TUI](https://github.com/convo-lang/convo-lang/tree/main/packages/tui)** — zero-dependency TS terminal component package; young.
13. **[tui](https://tui-ruby.vercel.app/)** — zero-dependency TypeScript terminal component library; small/new.
14. **[Pastel](https://github.com/vadimdemedes/pastel)** — Next.js-like framework on Ink for routing/building CLI apps; useful when the application is command-oriented.
15. **[blessed-contrib](https://github.com/yaronn/blessed-contrib)** — dashboard widgets on Blessed; primarily relevant to legacy Blessed stacks.

## Other languages — leading choices

### Go

- **[Bubble Tea](https://github.com/charmbracelet/bubbletea)** — Elm architecture; the most influential modern Go TUI framework. Pair with Bubbles and Lip Gloss.
- **[tview](https://github.com/rivo/tview)** — rich traditional widget toolkit on tcell; often faster to assemble forms, tables and trees than Bubble Tea.
- **[tcell](https://github.com/gdamore/tcell)** — lower-level portable cell/event foundation.
- **[gocui](https://github.com/jroimartin/gocui)** / [jesseduffield fork](https://github.com/jesseduffield/gocui) — view manager used by Lazygit; manual but proven.
- **[go-tui](https://go-tui.dev/)** — newer declarative templates, flexbox and reactive state.
- **[termui](https://github.com/gizak/termui)** — dashboard/chart widgets on tcell.
- **[Huh](https://github.com/charmbracelet/huh)** — forms and prompts built on Bubble Tea; not a general full-screen framework.

### Python

- **[Textual](https://github.com/Textualize/textual)** — the standout full framework: reactive components, CSS, widgets, async, testing and web serving.
- **[prompt_toolkit](https://github.com/prompt-toolkit/python-prompt-toolkit)** — exceptional for shells, REPLs, editors and complex input-driven apps.
- **[Urwid](https://github.com/urwid/urwid)** — established event-loop/widget toolkit, still maintained.
- **[pyTermTk](https://github.com/ceccopierangiolieugenio/pyTermTk)** — Qt-like, broad widget set and visual designer.
- **[PyTermGUI](https://github.com/bczsalba/pytermgui)** — modern Python TUI framework with markup and widgets.
- **[py_cui](https://github.com/jwlodek/py_cui)** — accessible grid/widget toolkit.
- **[asciimatics](https://github.com/peterbrittain/asciimatics)** — terminal UI plus animation/effects.
- **[npyscreen](https://github.com/npcole/npyscreen)** — traditional forms/widgets over curses; older.
- **[Rich](https://github.com/Textualize/rich)** — rich rendering rather than a full interactive framework; Textual is the full framework built from the same lineage.

### C and C++

- **[FTXUI](https://github.com/ArthurSonzogni/FTXUI)** (C++) — functional, component-based, portable and visually strong; the leading modern C++ choice.
- **[FINAL CUT](https://github.com/gansm/finalcut)** (C++) — complete desktop-like widget toolkit with windows and dialogs.
- **[notcurses](https://github.com/dankamongmen/notcurses)** (C) — modern terminal graphics, images, planes and high-performance rendering.
- **[ncurses](https://invisible-island.net/ncurses/)** (C) — the classic portability baseline.
- **[PDCurses](https://github.com/wmcbrine/PDCursesMod)** (C) — curses implementation with strong Windows relevance.
- **[termbox2](https://github.com/termbox/termbox2)** (C) — small single-header cell/event library.
- **[imtui](https://github.com/ggerganov/imtui)** (C++) — immediate-mode text UI inspired by Dear ImGui.
- **[cpp-terminal](https://github.com/jupyter-xeus/cpp-terminal)** (C++) — terminal control/input foundation rather than full widgets.

### .NET / C#

- **[Terminal.Gui](https://github.com/gui-cs/Terminal.Gui)** — established cross-platform full-screen widget toolkit.
- **[Spectre.Console](https://github.com/spectreconsole/spectre.console)** — polished rich console output, prompts and live displays; less of a free-form full-screen UI framework.
- **[Consolonia](https://github.com/jinek/Consolonia)** — Avalonia/XAML-inspired terminal GUI.
- **[Hex1b](https://github.com/mitchdenny/Hex1b)** — React-inspired declarative TUI library.
- **[SharpConsoleUI](https://github.com/nickprotop/ConsoleEx)** — multi-window compositor and widgets integrated with Spectre.Console.

### JVM

- **[Lanterna](https://github.com/mabe02/lanterna)** (Java) — pure-Java layered terminal, screen buffer and GUI toolkit.
- **[Jexer](https://gitlab.com/AutumnMeowMeow/jexer)** (Java) — desktop-like windows/widgets and terminal emulation.
- **[TUI4J](https://github.com/sshtools/tui4j)** (Java) — Bubble Tea/Textual-inspired framework.
- **[casciian](https://github.com/sshtools/casciian)** (Java) — Jexer-derived, GraalVM/AOT-oriented toolkit.
- **[Mordant](https://github.com/ajalt/mordant)** (Kotlin) — styled text, widgets, prompts and animation; best for rich CLI interactions rather than arbitrary desktop-style TUIs.

### Haskell

- **[Brick](https://github.com/jtdaugherty/brick)** — mature declarative framework with forms, editors, lists, tables, viewports and composable layouts.
- **[Vty](https://github.com/jtdaugherty/vty)** — lower-level terminal interface underneath Brick.
- **[HSCurses](https://hackage.haskell.org/package/hscurses)** — curses bindings; legacy-oriented.

### Other notable ecosystems

- **[Nocterm](https://github.com/jakobhoeg/nocterm)** (Dart) — Flutter-like declarative TUI with hot reload and many components.
- **[Nimwave](https://github.com/ansiwave/nimwave)** (Nim) — terminal/browser text interfaces.
- **[php-tui](https://github.com/php-tui/php-tui)** (PHP) — comprehensive Ratatui-inspired framework.
- **[Ashen](https://github.com/colinta/Ashen)** (Swift) — Elm-inspired terminal UI framework.
- **[libvaxis](https://github.com/rockorager/libvaxis)** (Zig) — modern terminal UI/event foundation used by real applications.
- **[vaxis](https://github.com/rockorager/libvaxis)** and **[zig-spoon](https://github.com/lithdew/zig-spoon)** (Zig) — strong low-level foundations; ecosystem is smaller than Rust/Go.
- **[tview](https://github.com/rivo/tview)**-style ports exist in several languages, but inspect maintenance carefully; many are thin or abandoned.

## A practical shortlist for evaluation

If the goal is to choose a foundation rather than merely catalogue the space, I would prototype the same small screen in these:

1. **Ratatui + Crossterm** — baseline for control, ecosystem and long-term safety.
2. **iocraft** — test whether declarative composition meaningfully reduces application code.
3. **Cursive** — test a batteries-included retained/widget model.
4. **AppCUI-rs** or **R3BL TUI** — choose AppCUI for desktop-like controls; R3BL for async/reactive architecture.
5. **OpenTUI** — cross-language benchmark for modern DX, layout, performance and rich code/diff components.

The key architectural choice is not Rust versus TypeScript; it is **rendering library versus application framework**. Ratatui gives maximum control and ecosystem depth, but your application owns state, event routing, focus, navigation and async orchestration. Higher-level frameworks buy those features at the cost of a smaller community and more framework-specific architecture.

## Primary research sources

- [Ratatui repository and documentation](https://github.com/ratatui/ratatui)
- [Ratatui backend comparison](https://ratatui.rs/concepts/backends/comparison/)
- [Awesome Ratatui ecosystem catalogue](https://github.com/ratatui/awesome-ratatui)
- [Awesome TUIs library catalogue](https://github.com/rothgar/awesome-tuis)
- [OpenTUI documentation](https://opentui.com/)
- [Bubble Tea repository](https://github.com/charmbracelet/bubbletea)
- [Brick package documentation](https://hackage.haskell.org/package/brick)
- [Urwid documentation](https://urwid.org/)
- [Lanterna repository](https://github.com/mabe02/lanterna)
- [notcurses documentation](https://notcurses.com/notcurses.3.html)

## Caveats

- This is a fast-moving niche. Several projects appeared in 2026 and have not yet accumulated enough production evidence to call stable.
- GitHub stars were intentionally not used as the primary ranking signal; architecture, maintenance, documentation, production use and ecosystem matter more.
- A name appearing here means it is relevant enough to investigate, not that its API, security posture or maintenance has been audited.
