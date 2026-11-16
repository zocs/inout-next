---
name: inout
description: A household inventory notebook for local file sharing.
colors:
  paper: "#f6f4ef"
  surface: "#ffffff"
  secondary: "#e9eee8"
  ink: "#192821"
  muted: "#59645e"
  shell: "#203c33"
  shell-muted: "#cbd7cf"
  accent: "#22624e"
  accent-hover: "#174737"
  selected: "#edf5ef"
  border: "#d8ddd6"
  input-border: "#78867b"
  error: "#a3302a"
  error-bg: "#fff1ed"
  error-hover: "#7d221e"
  header-border: "#71887b"
  header-hover: "#355449"
  row-hover: "#f8faf7"
typography:
  headline: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "24px", fontWeight: 650, lineHeight: 1.25, letterSpacing: "-0.02em"}
  headline-mobile: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "22px", fontWeight: 650, lineHeight: 1.25, letterSpacing: "-0.02em"}
  dialog-title: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "21px", fontWeight: 650, lineHeight: 1.25, letterSpacing: "-0.02em"}
  brand: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "17px", fontWeight: 650, lineHeight: 1.2, letterSpacing: "-0.01em"}
  title: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "15px", fontWeight: 650, lineHeight: 1.4}
  selection-title: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "14px", fontWeight: 650, lineHeight: 1.5}
  body: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "14px", fontWeight: 400, lineHeight: 1.5}
  body-small: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "13px", fontWeight: 400, lineHeight: 1.5}
  label: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "13px", fontWeight: 600, lineHeight: 1.5}
  control: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "14px", fontWeight: 550, lineHeight: 1.5}
  caption: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "12px", fontWeight: 400, lineHeight: 1.5}
  table-header: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "11px", fontWeight: 600, lineHeight: 1.5}
  transfer-status: {fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif', fontSize: "11px", fontWeight: 400, lineHeight: 1.5}
rounded:
  control: "7px"
  container: "10px"
  dialog: "14px"
  progress: "3px"
spacing:
  unit: "4px"
  micro: "3px"
  small: "5px"
  label: "6px"
  tight: "7px"
  action: "8px"
  field: "9px"
  row: "10px"
  input-inline: "11px"
  compact: "12px"
  detail: "14px"
  standard: "16px"
  form: "18px"
  section: "20px"
  host: "22px"
  wide: "24px"
  main-top: "26px"
  shell: "28px"
  main: "30px"
  page-bottom: "36px"
  rail: "40px"
  host-columns: "48px"
  empty: "50px"
  opening: "64px"
components:
  button-primary: {backgroundColor: "{colors.accent}", textColor: "{colors.surface}", typography: "{typography.control}", rounded: "{rounded.control}", padding: "8px 12px"}
  button-primary-hover: {backgroundColor: "{colors.accent-hover}"}
  button-primary-active: {backgroundColor: "{colors.shell}"}
  button-secondary: {backgroundColor: "{colors.surface}", textColor: "{colors.ink}", typography: "{typography.control}", rounded: "{rounded.control}", padding: "8px 12px"}
  button-secondary-hover: {backgroundColor: "{colors.secondary}"}
  button-secondary-active: {backgroundColor: "{colors.border}"}
  button-danger: {backgroundColor: "{colors.error}", textColor: "{colors.surface}", typography: "{typography.control}", rounded: "{rounded.control}", padding: "8px 12px"}
  button-danger-hover: {backgroundColor: "{colors.error-hover}"}
  button-danger-active: {backgroundColor: "{colors.error-hover}"}
  button-danger-quiet: {backgroundColor: "{colors.surface}", textColor: "{colors.error}", typography: "{typography.control}", rounded: "{rounded.control}", padding: "8px 12px"}
  button-danger-quiet-hover: {backgroundColor: "{colors.error-bg}"}
  button-danger-quiet-active: {backgroundColor: "{colors.error-bg}", textColor: "{colors.error}"}
  icon-button: {backgroundColor: "transparent", textColor: "{colors.muted}", rounded: "{rounded.control}", padding: "0", width: "40px"}
  icon-button-active: {backgroundColor: "{colors.border}"}
  text-button-active: {textColor: "{colors.accent-hover}"}
  header-button-active: {backgroundColor: "{colors.shell}", textColor: "{colors.surface}"}
  field-input: {backgroundColor: "{colors.surface}", textColor: "{colors.ink}", typography: "{typography.body-small}", rounded: "{rounded.control}", padding: "9px 11px"}
  search-input: {backgroundColor: "transparent", textColor: "{colors.ink}", typography: "{typography.body-small}", padding: "6px 0"}
  rail-link: {backgroundColor: "transparent", textColor: "{colors.ink}", typography: "{typography.control}", rounded: "{rounded.control}", padding: "10px 12px", width: "100%"}
  rail-link-current: {backgroundColor: "{colors.surface}", textColor: "{colors.accent}"}
  rail-link-active: {backgroundColor: "{colors.border}"}
  file-view: {backgroundColor: "{colors.surface}", textColor: "{colors.ink}", rounded: "{rounded.container}"}
  row-selected: {backgroundColor: "{colors.selected}"}
  entry-name-active: {backgroundColor: "{colors.secondary}", textColor: "{colors.accent-hover}"}
  folder-details-active: {backgroundColor: "{colors.secondary}", textColor: "{colors.accent-hover}"}
  host-connection: {backgroundColor: "{colors.secondary}", rounded: "12px", padding: "22px"}
  selection-panel: {typography: "{typography.body}", padding: "0 0 20px"}
  transfer-panel: {typography: "{typography.body}", padding: "0 0 20px"}
  notice: {backgroundColor: "{colors.selected}", textColor: "{colors.accent}", rounded: "{rounded.control}", padding: "5px 10px 5px 14px"}
  notice-error: {backgroundColor: "{colors.error-bg}", textColor: "{colors.error}"}
  delete-dialog: {backgroundColor: "{colors.surface}", textColor: "{colors.ink}", rounded: "{rounded.dialog}", padding: "28px", width: "calc(100% - 32px)"}
---

# Design System: inout

## Overview

**Creative North Star: "Household inventory notebook"**

The household inventory notebook gives inout warm paper, dark ink and restrained evergreen accents. Familiar file controls, compact rows and quiet tonal surfaces make names, access settings and transfer states easy to scan. System typography keeps the interface local and readable.

The built system uses color and borders to separate working surfaces; controls have modest curved corners and explicit keyboard focus. Full names remain available through accessible labels, titles and wrapped detail paths. A saved check acknowledges confirmed publication, while sending and saving retain their own text states.

**Key Characteristics:**

- Warm paper, ink and evergreen with one primary accent.
- System sans text, compact file rows and aligned numeric metadata.
- Flat working surfaces, restrained controls and visible state feedback.

This document describes the implemented inventory-notebook interface, its retained visual tokens, component states, keyboard focus and responsive behavior. Browser interaction/layout checks and native-device validation have different scopes; this design reference is not a whole-WCAG or assistive-device certification.

## Colors

The palette places dark green ink on warm paper, with white working surfaces and evergreen actions. Frontmatter values are normative; names below explain their use.

### Primary

- **Evergreen action** (`accent`): primary buttons, links, selected navigation text, focus outlines, input carets, checkbox accent and progress/saved marks.
- **Deep evergreen** (`accent-hover`): primary hover state; the shell tone supplies primary active state.
- **Evergreen shell** (`shell`): the product header, with shell-muted secondary text.

### Neutral

- **Warm paper** (`paper`): application background.
- **White working surface** (`surface`): file list, enabled fields, secondary controls and modal body.
- **Quiet green layer** (`secondary`): side rail, column headers, host connection panel and secondary/icon hover. It is a neutral layer, not a second accent.
- **Dark ink / muted ink** (`ink`, `muted`): main text and file names / supporting copy, metadata, icons and placeholders.
- **Shell-muted** (`shell-muted`): access/description text and keyboard focus on the dark header.
- **Selected wash** (`selected`): selected rows, rail hover and success notices.
- **Quiet border** (`border`): list/panel separation, secondary-control borders and secondary active state.
- **Input boundary** (`input-border`): enabled field outlines and the search bottom border. Existing computed evidence measures 3.821889:1 against white fill and 3.477053:1 against paper.
- **Row hover** (`row-hover`): ordinary row hover; selected rows retain their selected wash.
- **Header border / hover** (`header-border`, `header-hover`): the transparent header action’s edge and hovered fill. These and row-hover now use their retained-value root custom properties.

### Feedback

- **Error ink / wash** (`error`, `error-bg`): destructive text, error notices and quiet Delete hover.
- **Deep error** (`error-hover`): filled destructive confirmation hover/active, now a retained-value root custom property.

**The State Meaning Rule.** Evergreen identifies actions, focus and confirmed saved state; error text and notices use the error palette. Keep the state label alongside the color.

Eight-step tonal previews are illustrative documentation, not additional project tokens, themes or hover colors.

### Selectable accent themes

The HTTP manager offers four accents over the same paper/ink surfaces. Evergreen remains the default. Each palette changes the shell, shell-muted text, action/hover, secondary surface, selected wash and header/row feedback tokens; destructive feedback retains the separate error role.

| Theme | Shell | Action | Action hover | Selected wash |
| --- | --- | --- | --- | --- |
| Evergreen | `#203c33` | `#22624e` | `#174737` | `#edf5ef` |
| Tangerine | `#492e20` | `#984414` | `#76320f` | `#fff0df` |
| Ocean | `#183c50` | `#176587` | `#104d68` | `#e9f5fb` |
| Berry | `#422d49` | `#7d3984` | `#612b67` | `#f7edf9` |

The root `data-theme` attribute carries the palette independently of locale state. Focus, selection, caret and progress use the active semantic accent. Header language/theme controls wrap with native select affordances and minimum 40px targets; the existing mobile host preview retains 48px targets. Mobile input/select text is 16px.

### Language and retained task state

The HTTP manager supports `English`, `简体中文` and `繁體中文`, plus a system preference. Controls retain their full native-language names. The effective language updates `html.lang`; explicit Chinese Hans/Hant scripts take precedence over region hints, while scriptless TW/HK/MO tags use Traditional Chinese. `Intl.Locale` supplies the actual language, script and region, keeping extension/private-use tokens out of those decisions; rejected tags fall through to the next supported system language. Ordered system-language selection retains an English fallback, and an explicit stored choice wins over system languages. Browser storage failures preserve a usable current session.

Translate complete phrases with typed text arguments, without HTML interpolation. File names, paths, addresses and custom product descriptions are data. Known API errors and retained notices render in the current language; unknown server/provider details stay available. Switching locale or theme preserves folder/search/selection, forms, open dialogs and transfers, with stable module and connection identities. Dates and numbers use cached native Intl formatters. Timestamps outside the browser's Date range show localized Unavailable metadata while browsing and file actions remain usable.

These Next.js surfaces serve the HTTP manager. The existing Tauri WebView host is a transitional preview, including shared localized controls and lazy diagnostics. An opt-in Slint Rust desktop trial now implements the native sharing controls separately. Slint is evaluated first with egui as the fallback; platform, input, assistive, distribution and matched-performance acceptance remain separate.

The integrated Chromium run passes 131 cases against the real static file service and explicitly identified native/context boundary fixtures, including 33 localization cases and five permission cases. It covers private-use script/region tokens, rejected-tag fallback, public writes and independently visible rename/delete actions. The palette samples cover 72 manager/host-preview combinations across three locales, four palettes and 320/390/1440px widths, with minimum measured text contrast 5.60:1 and enabled control-boundary contrast 3.13:1. A throttled 2 MiB upload preserves its exact published bytes while changing locale/theme; a dropped publication response retains the localized instruction to check the folder before retrying. The integrated export contains 1,084,605 bytes, including 1,011,194 bytes of JavaScript (312,634 bytes gzip, measured per file). These browser checks do not establish native toolkit/device performance or assistive-device clearance.

## Typography

**UI Font:** the system stack in frontmatter: `-apple-system`, BlinkMacSystemFont, Segoe UI, sans-serif. The interface has no separate display family or downloaded font. Compact headings use a medium-heavy weight; body copy remains regular. There is no imposed mathematical scale ratio.

### Hierarchy

- **Headline**: folder/host headings; headline-mobile is the folder override at the mobile breakpoint, and dialog-title is the modal override.
- **Brand**: compact product identification with a reusable dot-matrix wordmark. Its ordinary-text fallback uses slight negative tracking and wraps inside the same flex allocation.
- **Title / selection-title**: section headings and selected-entry names.
- **Body / body-small**: default copy and smaller context/input text. Paragraphs default to a maximum 70ch; host introduction uses 65ch and empty-state text uses 45ch.
- **Label / control**: stronger labels and button text, with mixed case retained from product copy.
- **Caption / table-header / transfer-status**: metadata, column labels and small transfer details. Entry-name text is 13px, with its name span reduced to 12px on mobile; its accessible name remains complete.

**The Numeric Alignment Rule.** Use tabular numerals for item counts, sizes, detail values and transfer status; right-align file sizes in the table.

## Layout

The browser workspace is centered with a maximum width of 1600px. Its rail is 194px beside a flexible `minmax(0, 1fr)` main area; the main inset is 26px 30px 36px. A populated tools area uses a 256px column beside the list with a 24px gap. The tools column stacks contributions with 20px gaps. Management contributes immediately after the selected-entry inspector through the generic selection-panel slot, before ordinary transfer panels; folder creation can occupy that slot without a selection.

At a maximum width of 1190px, the rail becomes 164px, the main inset becomes 24px, content collapses to one column and tools become two flexible columns with a 24px column gap. Folder heading controls wrap and the brand description is hidden.

At a maximum width of 760px, the rail is hidden, the main inset becomes 18px 16px 28px and tools stack into one column. The toolbar fills its row and refresh aligns at its end. Modified metadata is hidden; the size column narrows from 88px to 74px and download from 54px to 46px. The header inset becomes 14px 18px. Breadcrumb targets grow from a minimum 32px to 40px. The native host’s flexible settings plus 300px connection layout becomes one column; its desktop gap is 48px and mobile gap 30px.

Use the 24 retained spacing entries in frontmatter and matching `--space-*` root properties. The four-pixel unit remains, with nonuniform built gaps/insets explicitly tokenized; all prior nonzero authored padding/margin/gap geometry is preserved. Buttons and downloads have a minimum 40px target; entry-name controls have a minimum 46px height. Table cells use 10px 14px padding, while the name cell reduces vertical padding to 3px around its control. The fixed table layout, ellipsized name spans, `min-width: 0` and wrapped detail paths contain long file names. Header and footer brand text wrap inside shrinkable allocations while adjacent access/footer controls preserve their width. Both `html` and `body` use `overflow-x: clip`; the table retains its own `overflow-x: auto` region.

Earlier checks of the interface before the wordmark/startup extension preserved its recorded geometry at manager widths 320/390/760/761/1190/1440/1920 and native-browser widths 320/390/761/1440, with no page overflow. Those mobile 23- and 80-character brand fixtures retained a measured 16px header gap with no header/footer intersection. Current extension checks cover wordmark/text substitution and native setup at 320/390/1440, plus diagnostic page/control overflow at the same widths. The earlier Android capability/layout milestone passed 91 Chromium cases; the integrated source now passes 131 cases as described above. Historical release-APK accessibility checks on API36/x86_64 cover font scale 1.3, increased display density, landscape and keyboard clearance. These samples do not establish every viewport, native WebView, assistive device or compositor's behavior.

## Elevation & Depth

Depth comes from paper, white and quiet green layers with thin separators. File containers and ordinary selection, action and transfer panels have no shadows. The delete dialog is the lifted surface: its shadow is `0 20px 64px var(--color-dialog-shadow)`, retaining `rgb(25 40 33 / 20%)`; its native modal backdrop uses `--color-dialog-backdrop`, retaining `rgb(25 40 33 / 42%)`. Those color and spacing tokens preserve the recorded modal appearance/geometry values.

**The Dialog Depth Rule.** Keep working panels flat. Reserve the built shadow and dark backdrop for modal destructive confirmation.

## Shapes

Controls and notices use the shared control radius. File containers use the container radius and the existing host connection preview uses 12px; the modal uses the larger dialog radius, and progress tracks use the smaller progress radius. Ordinary panels have a bottom separator without a surrounding card. File-list clipping contains its rounded edge and table content. Enabled field borders use the dedicated input boundary rather than the quieter container divider. Native checkboxes, scrollbars and progress semantics remain recognizable; icons use local inline SVG geometry.

## Components

### Buttons

Compact controls pair a minimum 40px target, 7px icon/text gap and the control typography. Primary, secondary and destructive buttons use the frontmatter inset and radius. Primary hover deepens evergreen and active uses the shell; secondary hover uses the quiet green layer and active uses the border tone. Filled danger is reserved for destructive confirmation, while the action-grid Delete uses an error-colored quiet button. Header controls use a transparent shell-compatible variant. Icon buttons are 40px wide with no padding and muted ink; text buttons use evergreen with an underline on hover.

Eight additional authored active recipes retain the existing palette: danger deep error, quiet danger error wash/edge, icon and rail border fill, text deeper evergreen, header shell/shell-muted edge, entry-name/Details secondary fill and deeper evergreen. Every new recipe guards disabled state. Actual press checks preserve each control rectangle; selected-row state remains distinct from transient press feedback.

All keyboard-focusable controls receive the source 2px solid evergreen outline with 3px offset; header buttons substitute shell-muted for contrast on the header. Buttons and inputs use a default cursor and 0.55 opacity when disabled. Colored button/icon/header transitions use the shared 160ms state duration and `cubic-bezier(0.16, 1, 0.3, 1)` easing. Reduced-motion preference forces these transition durations to zero. Ordinary loading rows remain static; the separate startup presentation has its own bounded motion and readiness contract.

### Product Wordmark and Startup

The header, footer and startup presentation use the same memoized `Wordmark` component. Local 5×7 glyphs cover ASCII uppercase/lowercase letters and digits, with an eight-character matrix limit. Each dot is static SVG geometry filled with the inherited text color. Unsupported or longer product names use wrapped ordinary text. The SVG is decorative to assistive technology; a plain-name equivalent supplies the accessible text. The native name button retains its focus outline and diagnostic activation callback, while the HTTP manager uses a noninteractive wordmark.

Startup initially shows that wordmark on paper with a short opening status. The initial authentication screen, usable file-manager context, native setup/status or recoverable connection error releases it. Folder listing, later navigation and sharing Start/Stop do not replay startup. The opacity/transform exit follows the shared CSS state duration, currently 160ms, while pointer and status semantics are already released; controls never wait for an animation event or minimum interval. Removal reads computed transition durations and delays from the screen and wordmark, with a finite one-second ceiling; overrides beyond that ceiling can outlast the presentation. Browser cases verify actual 40/160/360ms effects and cleanup without relying on transition events. Escape dismisses the presentation while it is waiting, without stopping sharing or consuming the key event. Initial or newly enabled reduced motion completes cleanup immediately at readiness; listeners and the removal timer are scoped to the presentation's lifetime. This design adds no downloaded fonts, animated dots, filters or motion dependency. Browser checks establish these state and layout contracts, rather than native device frame times.

### Inputs / Fields

Labels form a compact grid with a 7px gap. Text inputs use white fill, ink text, the control radius, a minimum 42px height and 9px 11px inset. Hover changes the boundary to muted ink; focus retains the shared outline. Search has a minimum 36px height, transparent fill and only the dedicated bottom border. Placeholder text uses muted ink at full opacity and the caret uses evergreen. Error copy appears with an alert rather than an invented invalid-field color state. Native checkbox controls use a 17px square and evergreen accent.

### Navigation

The rail uses padded full-width links with a minimum 44px target. The current place has white fill and evergreen text; hover uses the selected wash. Breadcrumbs wrap, expose the current path via `aria-current`, and use muted text with evergreen underlined hover. The current place, folder heading, full path and loaded content must agree. A failed requested folder clears stale rows and selection, retains the requested path, and exposes a named recovery region with Retry folder; folder toolbar/selection actions wait for a successful load.

### File List / Selection

The white file container has a quiet border and clipped curved edge. A small live item count sits beside search, followed by a fixed-layout semantic table. Hover uses the row-hover tone; a selected row keeps its selected wash during hover. File selection and folder Details expose boolean pressed state; folder Open remains navigation. The stable atomic polite status contains the full selected name or announces clearing, while selection focus remains on its initiating button. Explicit inspector clearing returns focus to that file/folder button, or the current-folder breadcrumb if the origin is disconnected; passive listing reconciliation and management clearing preserve existing focus.

The selected-entry inspector wraps the full name/path and places numeric definitions beside a 68px label column. Its download action spans the panel. The adjacent management contribution uses a two-column action grid; available actions follow the server modules, permissions and capabilities. Destructive confirmation uses the native HTML dialog, a focused Cancel control, explicit permanent-delete copy and an optional recursive checkbox for folders.

New folder, Move and Copy use the general management flag. Rename and Delete follow their independent flags; responses from older servers that omit those fields retain their legacy management behavior. The contribution remains available for a rename-only or delete-only context, while unavailable actions are omitted. Explicitly enabled public upload, creation and rename work without an access key. Published deletion requires both the server permission and configured-key authentication; hidden controls never substitute for server authorization.

Explicit management cancellation returns focus to its surviving opener, including Delete Cancel/Escape. Successful operations return to the current-folder breadcrumb; real failures retain/focus the appropriate input or dialog Cancel. Task focus is captured before disabling/removal and abandons restoration when the user chooses unrelated focus while a response is pending. These task destinations are independent of selection-origin clearing and do not run on passive listing/navigation effects.

### Transfers / Notices

Transfers are a compact separated list with ellipsized visible names, full title text, tabular status values and a native 6px progress track while uploading. Queued, Uploading, Saving…, Saved, Failed and Canceled remain distinct. Saving explains that bytes were sent and publication is awaiting confirmation; cancellation is disabled during this phase. Only saved entries show the evergreen acknowledgment check. A lost upload response retains the instruction to refresh the folder and check whether the file was saved before retrying, in the current locale. Transport failure alone cannot establish whether publication succeeded. Failed/canceled context persists until Clear finished; error messages use alerts and phase labels use status semantics.

Success notices use selected wash and evergreen; error notices use error wash and error ink. Their dismiss control inherits the notice color and retains ordinary icon-button hover/focus. Empty, loading and recovery states keep named context and honest actions instead of decorative card content.

Explicit Clear finished returns task-owned focus to the surviving Transfers heading (`tabIndex=-1`); dismissing a notice returns to the current-folder breadcrumb. Programmatic removal and async completion preserve deliberately chosen outside focus. The heading remains outside the ordinary Tab order until explicitly focused.

### Native Sharing Controls

The transitional native host reuses the same forms, buttons and quiet connection container. Running settings display the sanitized effective network, bound port, upload, management and configured-key indication separately from stopped drafts. The setup permits explicitly selected public uploads/management and explains the key requirement for deletion. A configured key can be acknowledged after reload without returning or fabricating it; host setting changes still require stopping. Backend live-admission controls exist, but native live-policy IPC/UI and browser task preservation across policy/authentication changes remain pending. Successful initiated Start/Stop performs a one-shot focus handoff to the resulting enabled control and updates a stable atomic polite active/stopped status. Passive status refresh and unrelated actions preserve existing focus. This documents built UI behavior; native browser boundary fixtures are not target-device IPC evidence.

The [Android preview.2 foreground host](docs/distribution/android-preview.md) replaces folder selection with **App storage**, explains persistence/uninstall and the need to keep the app on screen, and hides the unavailable local-manager action. LAN sharing is selected for this composition. Buttons, name activation, fields and checkbox labels have a minimum 48px target; supporting copy and inputs use 16px text. The key row retains a flexible input beside a 48px visibility control. Android's native content container owns system-bar, cutout and IME clearance, so the WebView does not apply those insets again. Keyboard Back dismisses the keyboard; root Back backgrounds the single host and stops sharing. Returning leaves an explicit Start action. The existing dot-matrix wordmark, readiness-driven startup and reduced-motion behavior are retained.

An initial status failure exposes Try again after startup is released. A keyboard-owned retry hands focus to the resulting folder, Stop or retry control; deliberately chosen outside focus is preserved.

### Local Diagnostics

Five activations of the native app-name button within five seconds reveal a lazy-loaded inline panel. Keyboard activation uses the same button; the ordinary HTTP manager has no diagnostic entry or native command authority. Opening focuses the panel heading, and explicit close returns focus to the surviving name button. Sharing controls remain available while a test runs.

The panel inherits the existing fields, buttons, feedback colors and focus outlines. Its labelled buffer/connection selects and relative-path input wrap within the available width, and result values use tabular numerals. Pending, running, cancellation, stopped-service, error and closing states explain the current action. Settings are temporary, and the result is labelled as a local throughput sample with its byte count, duration and plain settings text such as “32 KiB buffer with 4 connections.” Stop/restart releases the old action's controls; delayed replies, errors and cleanup cannot overwrite or unlock a newer session/action. Diagnostic sampling does not run while the panel is inactive. Browser fixtures check the UI boundary; real native tests separately exercise the shared Rust service and cancellation.

### Slint Native Desktop Trial

The native surface has independent design authority. The browser manager supplies an aesthetic reference; its page structure and component geometry do not constrain future native layouts. Organize the host around choosing share content, starting the service, using the address and understanding transfer state. Native composition and motion may evolve while retaining configurable pixel branding, readable localized controls, explicit confirmation and service ownership. The following values describe this implemented desktop trial, rather than a required layout for every future native surface.

The independent [Slint composition](apps/native-slint/README.md) carries the same paper/ink/evergreen design through compiled native layout. Its header identifies the product with the compiled 5×7 wordmark and provides language, About and theme controls. The desktop setup and connection columns sit in one document scroll view, followed by bounded current-run transfers. The authored window prefers 980×820 logical pixels and has a 680×620 minimum; this desktop trial does not claim a mobile layout. Header/body insets are 20/24 logical pixels, the column gap is 24, the connection inset is 22 and its minimum width is 280. Wrapped text retains full relative names and effective settings; OS scale and font fallback affect actual pixels.

`NativeTheme` uses the existing paper, white, ink, muted, border and error values, with evergreen/tangerine/ocean/berry alternatives for the shell, connection surface and accent. Standard Fluent controls retain Slint's platform accent, focus ring and interaction feedback; theme selection does not promise recoloring every toolkit widget. Body/hint text uses the system font at 14 logical pixels, headings at 22 and confirmation titles at 21. No web font, browser engine or HTML/CSS runtime is used for the native screen.

The setup separates stopped drafts from the active folder, actual bound port, network and permission summary. It supports native folder selection and a masked optional key, and explains that public uploads/management can be enabled while deletion requires configured-key authentication. The actual address is selectable/copyable. Failed external-browser handoff retains a localized recoverable message and the usable address. This trial has current-run receipts, rather than durable history, resume, native file preview/safe-open or a diagnostic/live-setting editor.

Stop and normal window-close request a centered explicit confirmation; Keep sharing initially owns focus. Tab/Shift-Tab cycle within the confirmation, Escape preserves the listener/body, and accepted completion returns focus to the resulting sharing control once. About similarly contains keyboard focus, blocks background actions and restores its opener. Busy state uses accepted command IDs and a persistent completion watermark, so a passive progress snapshot cannot unlock the confirmation or take the task's focus handoff. Locale/theme rerenders preserve the active service and drafts.

The native startup covers only initial readiness and exits through bounded 160 ms opacity motion; ready controls do not wait for it. `INOUT_NATIVE_REDUCE_MOTION=1` sets zero duration. Native OS motion-policy detection and preference persistence remain pending. Render callbacks are separate from displayed-frame timing and cannot establish an achieved frame budget.

Actual Linux keyboard, semantic and physical-action checks include paused real transfer preservation, confirmation focus and About attribution. The linked AccessKit AT-SPI bridge lacks EditableText and reports disabled buttons as enabled/sensitive. Direct reactive availability and guarded activation prevent those actions from changing sharing, but the incorrect metadata remains a full assistive-acceptance gate. CJK composing input, screen readers and each target's font/input/scale behavior still need independent checks. The existing browser contrast/layout results above do not certify the Slint renderer.

Component recipes cover enabled/disabled controls, navigation, selection, transfers and notices through the shared UI and feature modules. Representative states do not introduce new application commands or component APIs. No chip/tag component is present in this implementation.

## Do's and Don'ts

### Do:

- Do use the shared paper, ink, surface and evergreen tokens for existing roles.
- Do retain visible focus, full accessible names and pressed state on selection controls.
- Do keep selected-entry actions adjacent to the inspector before transfer history.
- Do distinguish uploading, saving and saved state with the existing text and acknowledgment mark.
- Do retain constrained brand wrapping and responsive metadata compression.
- Do return focus after explicit task-owned removals while respecting user-chosen outside focus.

### Don't:

- Don't use the quiet container border as the enabled input boundary; use the dedicated input-border token.
- Don't present a previous folder’s actionable rows under a failed requested folder.
- Don't mark sent bytes as saved before publication is confirmed.
- Don't move keyboard focus during passive native status refresh.
- Don't present draft defaults as effective settings for a running sharing session.
