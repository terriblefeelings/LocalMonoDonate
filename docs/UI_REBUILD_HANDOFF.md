# `/notify` UI Implementation Handoff

Status: the `/notify` page uses the restored 8-cue workbench with a single tabbed
parameter window, compact joined cue controls, a sticky utility header, and Ctrl+wheel zoom of
the page content only. The palette mixes Flexoki, VSCode syntax, Discord graphite, and Monobank
cues. The preview monitor follows the global light/dark theme.

## 1. Target scope and routes

- Scope: the **Alerts Customization** page only.
- Page route: `GET /notify` (`Page::Notify` in `src/web.rs`, embedded with `include_str!`).
- Page assets:
  - `GET /notify.css`
  - `GET /notify.js`
  - `GET /notify-ui.js`
- Shared dependencies retained: `public/head.js`, `public/i18n.js`, `public/alert.js`,
  and shared `public/app.css` rules used by other pages.
- No unrelated page or backend behavior was redesigned.

## 2. Files and visual system

| File | Role |
|---|---|
| `public/notify.html` | Horizontal utility header, left cue bank, right output monitor, and one tabbed parameter window. |
| `public/notify.css` | Page-scoped design tokens, layout, controls, light/dark themes, responsive rules, motion. |
| `public/notify-ui.js` | Presentation adapter: preview fit, pointer pan, range readouts, center-media/double-click reset, inspector tabs, page zoom, compact header icons, hover notes, and wheel-over-preview media zoom. |
| `public/notify.js` | Preserved controller, now with the new UI labels and DOM hooks. |
| `src/web.rs` | Serves `/notify.css`, `/notify.js`, and `/notify-ui.js`; removed the old mixer asset route. |
| `public/app.css` | Old `/notify`-only selectors were removed; shared controls remain. |
| `AUC/_alltest.mjs` | Static asset checks updated for the new page assets. |
| `README.md`, `AGENTS.md` | HTTP test count updated to 190. |

Removed target-only assets and presentation:

- `public/assets/notify-mixer.jpg`
- `docs/ASSETS.md`
- The old poster layout, monitor bezel, sticky mini-preview, custom select wrapper, and
  page-local rat handlers.
- Old `/notify` CSS for `.testbtn`, `details.grp`, `.stage`, `.rule`, and `#ministage`.

Visual direction:

- Flexoki paper/ink and a Discord graphite dark base with deliberate IRC/VSCode accents: the
  rail uses four colours, the readout cells are blue/orange/cyan, and parameter tab numbers
  match their panel labels. Explicit exceptions: red "Alert" word, red TEST text and Reset,
  delete/pool hover states, and language flag colours. The default decorative side is ink in
  the light theme and paper in the dark theme.
- Bahnschrift/DIN-style display face, Segoe UI body face, Cascadia/Consolas technical
  numbers. All are local/system fonts.
- Industrial geometry: 2â€“6px radii, 1px rails, a vignetted output monitor, a left cue bank,
  and one tabbed parameter window with the section list on the left.
- The shared header is a sticky horizontal utility bar with the project rat SVG,
  icon-only moon/sun controls plus coloured UK/Ukraine flag icons matching the nav-icon
  buttons, the same Alerts/Sound Alerts icon order on every page, and an icon Save button
  sized like the other header buttons. The header brand reads `LMD://<page>`; the suffix uses
  the original body face and brand ink, translated with EN/UA. The red â€œAlertâ€ word uses
  Cascadia Code italic.
- A default-only profile is seeded with seven starter variations plus the default
  (eight cues total).
- Bilingual geometry: the title block, readout, meta slots, panel headers, cue rows, tabs,
  console, and button boxes stay the same between EN and UA; only text runs vary.
- Cue rows are compact 34px groups with an 8px standard gap: variation window, a TEST block
  sharing the delete/pool surface with red text and no hover/glow, and a right-side
  delete/pool column. Delete is top, pool is bottom. The default row has a static decorative
  side stack using the inverted ink/paper tone. Internal TEST/side/ghost outlines are removed.
- The amount range and Add-variation buttons are pinned to the bottom of the cue bank;
  FROM, TO, and the currency unit share one centered line.
- Ctrl+wheel zooms the page content from 0% to 200% in 1% steps; the utility header stays
  outside the zoom surface. The HUD shows the current percentage and resets to 100% on click
  or Ctrl+0.
- The parameter window height is locked to the tallest section on desktop, so switching
  tabs does not resize the console; below 900px it follows the active panel.
- The output monitor follows the global light/dark theme. Scrolling anywhere over the
  preview changes the media zoom from the Media-channel setting; double-clicking the
  preview recenters the media pan. Header theme/language controls are icon buttons matching
  the nav icons: moon/sun and coloured UK/Ukraine flags.
- No external images or new dependencies were added. The preview uses the applicationâ€™s
  real default media and the configured alert data.

## 3. Preserved feature inventory

- Rules: load, select, add, delete, pool toggle, fixed ranges, default fallback, test.
- Starter bank: when a profile contains only the default rule, seven named variations
  are generated in memory so the bank and preview start at eight cues. The user must Save
  to persist them; existing custom rule sets are never seeded.
- Rule range editing, clamped to `10..29999`. The default cue is fixed at `10..29999`, disables
  its range inputs, and shows the fixed-range note.
- Full alert-style controls: copy, fonts, sizes, weights, colors, outlines, opacities,
  widths, heights, backgrounds, radii, duration, and animation settings.
- TTS: Google UA only, read mode and speech speed (0.5–2.0, default 1.2).
  Defaults are Google UA only and Announcement + message. TTS uses the generated
  `/tts/...` WAV from the local server; there is no browser voice fallback, so an
  English duplicate cannot appear. The overlay uses one reusable audio element.
- Media: image/video/sound URLs, upload buttons, mutual locks, position, size, zoom,
  radius, persisted pan, and a Center action.
- Live preview through `Alert.build()` and `Alert.fitMessage()`.
- Save through the shared header Save button:
  - `POST /api/notify`
  - `POST /api/tts`
  - reload persisted rules after success
- Export JSON, import JSON, reset selected style to `Alert.DEFAULTS`.
- Shared theme and EN/UA language controls.
- Status feedback in the visible `#status` rail near the top of the page.
- Font loading from `GET /api/fonts`.

## 4. Hooks, actions, controllers, and state

| Interface | Purpose |
|---|---|
| `window.savePage()` | Save rules and TTS settings; wired to the shared header Save. |
| `window.refreshPreview()` | Rebuild the preview after external changes. |
| `window.setMediaPan(x, y)` | Persist media pan and rebuild the preview. |
| `window.getMediaPan()` | Read the selected ruleâ€™s pan coordinates. |
| `window.applyLang` | Reapply page i18n, rule labels, and preview copy. |
| `preview()` | Build the alert into `#stage`. |
| `renderRules()` | Rebuild the cue workbench rows from `rules`. |
| `loadStyleToUI()` | Load the selected rule into the controls and preview. |
| `readStyle()` / `writeStyle(s)` | Convert between controls and rule style objects. |
| `updateRuleOpts()` | Apply pool/default range state and notes. |
| `updateMediaLocks()` | Apply mutual image/video/sound locks. |
| `fillTtsVoices()` | Rebuild voice options for the selected TTS mode. |
| `loadTts()` | Load TTS settings and voices. |
| `testRule(r)` | Send a test alert. |
| `sampleAmount(r)` | Pick a representative amount for a test. |

Internal state remains `rules`, `sel`, `settingsLoaded`, `TTS`, `upKind`, and `lang`.
`rules`/`sel` are controller state, not a public store. Extend the controller rather than
mutating them from unrelated code.

`public/notify-ui.js` adds:

- Preview scaling through a `MutationObserver` and `ResizeObserver`.
- Pointer drag-to-pan on preview media, calling `window.setMediaPan()` only after movement.
- Range `<output>` synchronization.
- The `Center` media action and double-click recenter.
- Wheel-over-preview zoom that updates the existing `#media_zoom` setting and preview.
- Contextual hover notes for primary controls and fields, localized with the page.
- Header icon enhancement: SVG Save, UK/Ukraine flags and moon/sun controls, accessible labels.
  The Alerts / Sound Alerts icons keep the shared header order on every page.

## 5. Interaction-to-handler mapping

| Interaction | Contract |
|---|---|
| Click a cue row | Select, load style/range, refresh bank and preview. |
| `Enter`/`Space` on a focused cue row | Same as clicking the row. |
| Add variation | Insert a rule before the default, with `min/max: 100`. |
| Pool checkbox | Toggle `rule.pool`; disable range inputs. |
| `Test` | `POST /api/test` with `{ amount, name: "Test Donor", ruleId }`. |
| Delete `âœ•` | Remove a non-default rule. |
| Range inputs | Clamp and write to the selected rule; update the top readout. |
| Variation name | Edit `rule.label`; disabled for the default rule. |
| Style controls | Debounced preview after 90 ms. |
| TTS mode | Rebuild voices and Azure field visibility. |
| Upload buttons | Open `#file`, upload, and write the returned URL. |
| Media drag | Update pan via `setMediaPan()` only when the pointer moved. |
| Mouse wheel over preview | Adjust `#media_zoom` in 1% steps and rebuild the preview. |
| Preview double-click / Center | Reset pan to `0,0` and rebuild the preview. |
| Export | Download `{ rules }` as `localmonodonate-alerts.json`. |
| Import | Read a JSON object containing a `rules` array. |
| Reset | Confirm, replace the selected style with `Alert.DEFAULTS`. |
| Header Save | Call `window.savePage()`. |
| EN/UA | Shared `setLang` calls `window.applyLang()`. |
| Theme | Shared header toggles light/dark tokens. |

## 6. Form fields, validation, and submit contracts

### Rule shape

```json
{
  "id": "default",
  "label": "",
  "min": 0,
  "max": 0,
  "pool": false,
  "style": {}
}
```

- New rules use `id: "r" + random suffix`, `min/max: 100`.
- `max: 0` means infinity.
- Range editing clamps to `10..29999`; the default rule can retain `0/0`.
- Pool rules disable the range inputs.
- The default rule has no pool/delete control; it can still be tested.
- `style` is merged with `Alert.DEFAULTS` by the preview builder.

### Style keys

```text
top_text, money_text,
blue_font, blue_size, blue_weight, blue_color, blue_outline,
blue_outline_op, blue_maxw, blue_tracking, blue_line,
green_font, green_size, green_weight, green_color, green_outline,
green_outline_op, green_maxw, green_tracking, green_line,
msg_font, msg_size, msg_weight, msg_color, msg_outline,
msg_outline_op, msg_maxw, msg_maxh,
win_bg, win_bg_opacity, win_radius, win_maxh,
win_border_color, win_border_width, win_shadow, win_padding,
blue_bg, blue_bg_opacity, blue_radius,
green_bg, green_bg_opacity, green_radius,
red_bg, red_bg_opacity, red_radius,
duration_ms, wait_media,
anim_in, anim_out, anim_ms,
image_url, video_url, sound_url, media_pos,
media_w, media_h, media_zoom, media_radius,
media_pan_x, media_pan_y, max_width
```

`wait_media` and `media_pan_x/y` remain persisted style keys without dedicated visible
inputs. Pan is controlled through `setMediaPan()` and the Center button.

### Save payloads

`POST /api/notify`:

```json
{ "rules": [ /* complete rules array */ ] }
```

`POST /api/tts`:

```json
{
  "mode": "default",
  "voice": "",
  "readMode": "announcement",
  "azureKey": "",
  "azureRegion": ""
}
```

Success is `{ "ok": true }`; errors use `{ "ok": false, "error": "..." }`. Status text is
written to `#status` with `status ok` or `status err` classes.

## 7. API and routing dependencies

| Method | Route | Use |
|---|---|---|
| GET | `/notify` | Page route. |
| GET | `/notify.css` | Page design system and layout. |
| GET | `/notify.js` | Functional controller. |
| GET | `/notify-ui.js` | Presentation adapter. |
| GET | `/head.js`, `/i18n.js`, `/alert.js`, `/app.css` | Shared dependencies. |
| GET | `/assets/default.jpg` | Default preview media. |
| GET | `/api/notify` | Load rules. |
| POST | `/api/notify` | Save rules. |
| GET | `/api/tts` | Load TTS settings and voices. |
| POST | `/api/tts` | Save TTS settings. |
| GET | `/api/fonts` | Populate font selects. |
| POST | `/api/upload?name=...` | Upload media. |
| POST | `/api/test` | Fire a test alert. |
| GET/POST | `/api/theme` | Shared header theme state. |
| GET | `/api/config` | Shared header background config. |

Removed route: `/assets/notify-mixer.jpg` (now 404).

## 8. Accessibility and keyboard behavior

- Semantic headings: `notifyTitle`, `cueHeading`, `monitorHeading`, `settingsHeading`.
- Rule rows are focusable, use `role="listitem"`, expose `aria-current`, and support
  `Enter`/`Space`.
- Pool checkboxes and action buttons have accessible names; the default rowâ€™s decorative
  side stack is `aria-hidden`.
- All form controls have visible labels; media width/height inputs have explicit
  `aria-label` values.
- `#status` is a live region; `#rulenote` is a live region.
- `#stage` has an accessible label from the controller. Header theme/language/Save
  controls have titles and labels; primary fields and buttons expose contextual hover notes.
- Native `<details>`/`<summary>` controls each parameter module and the Azure help disclosure.
- Every interactive control has a `:focus-visible` outline.
- Selected cues use a border/background plus `aria-current`, not color alone.
- Preview drag has a keyboard-accessible Center action.
- Motion is limited to short transitions and respects `prefers-reduced-motion`.
- Both themes retain readable contrast; colour is limited to requested accents, flag
  colours, parameter syntax colours, and destructive/state cues.

## 9. Known limitations

- The controller remains DOM-bound. Preserve the IDs in this document or update the
  controller and UI adapter together.
- `#tts_voice` must stay inside an element with class `cell`; `fillTtsVoices()` uses
  `closest('.cell')` to show/hide the field.
- `#stage` must remain visible and sized for the preview fit adapter.
- The mobile preview is intentionally compact (16:9 stage); there is no fullscreen preview mode.
- Media panning uses pointer input; the Center button and preview double-click are resets.
  Preview zoom uses the wheel and writes the same Media-channel zoom setting.
- The parameter window keeps its section list on the left at desktop and becomes a
  horizontal tab strip on mobile. The fixed tallest-panel height applies at desktop only;
  below 900px the console follows the active panel.
- The eight-cue starter bank exists in memory until the user saves. A default-only profile
  regenerates it on load; an existing profile with custom rules is not changed.
- The preview monitor follows the global theme; there is no independent preview theme.
- The header is sticky; page scrollbars are intentionally hidden while content remains
  keyboard- and wheel-scrollable.
- `head.js` still injects the shared ratbox and header navigation. Do not duplicate those
  IDs or the Save button in the page.
- `AUC/_deeptest.mjs` cannot run in this environment because `playwright-core` is not
  installed. This is a pre-existing dependency issue.
- `cargo fmt` is unavailable because rustfmt is not installed in the active toolchain.
- Isolated HTTP tests require `LMD_DATA` to point at a temporary profile.

## 10. Verification results

Final local verification:

- `cargo test --release`: **18 passed, 0 failed** (includes every listed YouTube and
  non-YouTube host shape, plus Twitch clip slug parsing).
- `node AUC/_alltest.mjs` with `LMD_DATA` temp profile: **190 passed, 0 failed**.
- `node AUC/combotest.mjs` with `LMD_DATA` temp profile: **33 passed, 0 failed**.
- Final browser pass at 1440, 1024, and 390 widths: sticky header, hidden page scrollbar,
  fixed desktop console height, icon-only header controls, global theme switching through
  the preview, TEST sharing the delete/pool surface with red text, red delete hover, green
  pool active state, theme-inverted default side, 1% wheel-over-preview zoom, drag pan,
  and double-click recenter were exercised.
  EN/UA comparison confirmed that container boxes, cue rows, panel height, tabs, and field
  controls keep the same geometry; only text runs change width.
- Global header page suffixes and document titles now switch with EN/UA. Two previously
  malformed persisted strings (QR jar title and default alert amount copy) were repaired from
  the clean backup; `/qredit` now renders â€œÐÐ° Ð±ÐµÐ½Ñ‚ÐµÐ¶Ð½Ðµ Ð¶Ð¸Ñ‚Ñ‚Ñâ€.
- Hallmark `sloplint` on `notify.html` and `notify.css`: **0 FAIL, 1 REFLEX** (section
  IDs inside headings, waived as required technical console identifiers), **0 WARN**.
- Playwright review at 1440, 1024, and 390 widths:
  - No horizontal page overflow.
  - No console errors.
  - Light and dark themes render.
  - Header controls remain reachable on mobile.
  - Preview fit, add rule, save, status, TTS mode changes, language switch, and media
    pan/center were exercised.
  - The default-only test profile seeded eight cues in the contact sheet.
  - The eight-tab parameter window, keyboard tab navigation, save/status, preview fit,
    TTS panel, joined cue controls, and 0/100/200% page zoom were exercised.
  - Monochrome cue controls, the checked pool state, and the TEST surface were checked in
    both themes.
  - Save persisted rules and the top status rail reported success.
- `AUC/_deeptest.mjs` was not run because `playwright-core` is missing.

## 11. Requirements for future visual changes

- Keep the functional contracts and IDs intact unless the controller is updated.
- Keep the shared header Save path (`window.savePage`) as the single save action.
- Keep the preview visible through `#stage`; do not remove the fit adapter without a
  replacement.
- Keep the header rendered once by `head.js`; do not add page-local header markup, reorder
  icons, or duplicate the Save button.
- Keep all style keys represented in the UI, including the persisted pan fields.
- Keep Azure conditional fields and TTS voice rebuilding wired to `fillTtsVoices()`.
- Keep upload buttons using `data-up="image|video|sound"` and `#file`.
- Preserve the visible status rail and live rule note.
- Keep EN/UA copy in `window.I18N`/`DICT`; every visible new label needs both languages.
- Preserve the light/dark token split and the reduced-motion fallback.
- Do not reintroduce the old mixer asset or the removed page-specific CSS.

## 12. Reconnection and extension checklist

1. Keep `notify.html`, `notify.css`, `notify.js`, and `notify-ui.js` loaded in that order
   after the shared scripts.
2. If replacing the page shell, preserve the IDs listed in sections 4â€“6.
3. If replacing the rule bank, keep `#rules`, `#addrule`, `#rule_label`, `#rule_min`, `#rule_max`,
   `#rulenote`, `#cueCount`, `#activeCueName`, and `#ruleRange`.
4. If replacing the monitor, keep `#stage` visible and update the fit adapter if its
   sizing model changes.
5. If replacing inspector panels, keep the `nt-field` label/input structure and each
   panelâ€™s `role="tabpanel"`/`aria-labelledby` pair, or update the tab adapter.
6. Keep `#tts_voice` inside `.cell` or update `fillTtsVoices()`.
7. Keep `#status` visible near the top of the page; the controller writes status classes
   and text to it.
8. Keep `#file` hidden and wire upload buttons through `data-up`.
9. Run `cargo test --release`, `_alltest`, `combotest`, and browser checks at 1440, 1024,
   and 390 widths after any structural change.
10. Re-run the Hallmark static sweep if the design system changes.

## 13. Shared admin-page redesign

Scope: `/setup`, `/bitsedit`, `/donateedit`, `/qredit`, `/timeradmin`, and
`/auctionsettings`; `/notify` remains the reference design.

- The header is rendered once by `public/head.js` for every page. Page HTML contains only an
  empty `#siteHeader`; there are no page-local header copies. It uses the Alerts structure and
  dimensions: framed rat mark, `LMD://<page>`, icon-only moon/sun and UK/Ukraine controls, nav
  icons in the same order, and same-size Save. Desktop is 67px tall; mobile is 101px and
  single-row. `app.css` is now the only header stylesheet for every page, including Alerts;
  `notify.css` contains no header rules. `notify-ui.js` no longer swaps the Alerts/Sound Alerts
  icons.
- The Alerts masthead readout moved out of the hero and into the output monitor, so the hero
  uses the same title/status layout as every admin page. Each page has a unique code accent,
  none red or orange: Setup blue, Alerts violet, Sound Alerts green, Donate cyan, QR yellow,
  Timer magenta, Auction lime. The Alerts status text is â€œLocal adminâ€ like the admin pages.
  Hero intros stay on one line from 901px up.
- The status badge has a filled surface and the same position on Alerts and admin pages. Every
  window (panels, index, code/status badges, rail, previews) carries a strong, wide
  background-coloured halo, so the grid fades well around each window while remaining visible
  between/around them.
- The Alerts cue bank keeps its compact 30px rows, red TEST cell with red hover lines on both
  sides, and borderless x/+ side stack with red/green hover states. The selected row uses an
  outside white outline and the row gap is 6px. The bank panel always matches the
  output-monitor height; extra rows scroll inside the bank with the scrollbar hidden, and the
  amount range stays pinned at the bottom. Both default-row placeholder cells are light and
  the pool green is brighter. All windows use the same stronger low-radius background halo;
  the gap shield was removed. The Editing / Amount range / Preview readout is rounded, sits
  inside the output monitor with no halo, is the same width as the monitor frame, and uses
  blue/orange/cyan labels. Each normal cue has equally sized TEST and Test YT buttons aligned
  on one baseline; both use the same 1px separators, which turn red from either side on hover.
  New variations are inserted above Default. A **Test Variation** row follows Default with
  TEST and Test MD and the same white placeholder side cells. Console tab **09 Media Test**
  supplies the media URL for Test MD. Test Variation is ignored by real donation matching and
  only appears in the overlay when its test button is used. Test buttons pass their style
  directly to the overlay, so Test MD/Test YT preview the edited look without saving.
- The Auction rail uses the same 18px gap as the other pages; the extra flex gap was removed.
- Admin heroes now use the Alerts masthead geometry: the 1440px shell starts at x=32, the
  title is 68px / 760 / -.05em, and the accent uses the Alerts vermilion (`#f07a6a` dark,
  `#d14d41` light) in Cascadia Code italic. The title row spans the shell; the status pill
  sits at the bottom-right. Ukrainian titles are kept on one line from 641px up.
- Page naming: Credentials configuration, Alerts configuration, Sound Alerts configuration,
  Donate page configuration, QR widgets configuration, Marathon timer configuration, and
  Auction configuration. Hero titles use the page-name segment in the red Cascadia Code
  italic face and the remaining words in the page body face. In English the name is the
  segment before â€œconfigurationâ€; in Ukrainian the generic â€œÐÐ°Ð»Ð°ÑˆÑ‚ÑƒÐ²Ð°Ð½Ð½Ñâ€ lead stays in the
  body face and the actual name after it is red. The Alerts hero uses the same Ukrainian
  split. Header suffixes and document titles translate. The Marathon timer hero uses a 58px
  desktop size so its full name stays on one line.
- `public/app.css` uses the shared Flexoki paper/ink tokens, graphite dark surfaces, 2â€“4px
  geometry, dense controls, and a subtle grid background. Ctrl+wheel zooms page content in 1%
  steps on all admin pages with a bottom-right HUD; the header stays outside the zoom surface.
  The four-colour rail is the only page separator; the extra hero border was removed on both
  the admin pages and the Alerts hero. The page scrollbar is hidden visually while wheel,
  keyboard, and touch scrolling remain enabled.
- Setup, Donate, QR, and Timer use a two-column workbench: a left settings list and one
  selected settings panel on the right. The list is a tab selector, not bookmarks. The index
  and settings window share the same top edge; hidden panels use `display:none` even when a
  panel ID has its own display rule.
- Links is the fifth tab in Setup's settings list, not a separate panel below the workbench.
  Its note explains that each checkbox enables or disables the entire feature, not only its
  link.
- QR, Timer, and Donate keep their preview/output monitor permanently above the workbench;
  Preview is no longer a tab. Donate embeds the live donor form (`/banka`) at a 1440px design
  width and scales the whole page down to fit the monitor, so the full form remains visible.
  The preview reloads after a background upload/clear or a successful save. The settings index
  and parameter window sit below the monitor. Sound Alerts and Auction have no index list.
  Sound Alerts uses a fixed deck window sized for the largest 2Ã—2 variation, so every grid size
  keeps the same window; its editor is the same 820px width with a 16px gap. Auction keeps its
  two equal-width, equal-height settings columns.
- The public donor page (`/<nick>/donation`) has no grid background. Its configured image is
  applied with `background-size:cover`, filling the page at any source size, and the page fits
  desktop and mobile without horizontal overflow or mobile vertical overflow at 390Ã—844.
  The donor background is not applied to Credentials or any other admin page; admin pages keep
  their own grid.
- The donation alert overlay fills its browser source and centers the alert; the recommended
  OBS source size is **750×500**. The media overlay keeps a **640×360** picture-in-picture
  player in the bottom-right. Both sizes are shown next to the Copy links in Setup. Real
  media starts only after the donation alert ends: the selected duration, or 2 s after TTS
  if TTS runs longer. YouTube checks `yt_min`, `yt_views`, `yt_max_min`; other media checks
  `other_media_min` before playback. YouTube is a direct embed iframe created
  with `allow="autoplay"`; it starts muted so autoplay is always permitted, then uses the
  iframe API postMessage commands to unmute and resume if the browser pauses. YouTube input
  accepts watch, share, mobile, Shorts, embed, nocookie, `/v/`, timestamp and query variants.
  Instagram/TikTok use provider embeds; Twitch clips resolve to a signed direct MP4; X/Twitter
  video posts are resolved through the syndication endpoint, capped at the 720p MP4 rung for
  the PiP, and served through a same-origin proxy (the CDN requires an x.com referer), so the
  video itself plays instead of the tweet card. If the proxy fails, the tweet embed is shown
  instead of a blank stage. Reddit posts use the embeddable `redditmedia.com` view and `v.redd.it` links resolve
  to a direct MP4. Provider embeds get autoplay parameters where supported; controls remain
  disabled where the provider allows it.
- Donate settings use a **Media parameters** tab. The YouTube block keeps its amount, views,
  duration and playlist rules; a separate **Other media** minimum applies to Instagram,
  TikTok, Twitch clips, Reddit and X links. The donor form accepts those links and the overlay
  plays them through provider embeds or a direct video element. YouTube Shorts are already
  handled by the existing YouTube URL parser.
- Auction has no local Save button; the global header Save calls `window.savePage`.
- The default Setup jar row is always the user's jar: it has no â€œMy jarâ€ control and no
  delete button. Additional jar rows use a centred â€œMy jarâ€ button toggle (state shown by
  fill, no duplicate side label) plus delete, with a hover note. The YouTube playlist option uses an ON/OFF button with a hover
  note; ON text keeps its contrast while hovered. Add-jar/Show share one baseline and height
  with fixed intrinsic widths.
- Sound Alerts uses a player bar under the preview in this order: play/pause, volume slider,
  elapsed/total time. The media row uses the same left-label/field layout as Button image and
  is labelled simply â€œMediaâ€.
- QR widgets expose per-widget jar, background colour, style, corner radius, and accent
  colour with labelled controls. The default widget has no delete button; additional widgets
  show a right-aligned delete. Radius applies to both QR and goal cards; accent colours the
  progress bar in the preview and overlay.
- Compact action buttons keep intrinsic widths instead of stretching to the panel width.

Verification: `cargo test --release` 18 passed, `_alltest` 190 passed, `combotest` 33 passed.
Browser checks covered 1440, 1024, and 390 widths, shared-header equality (DOM order,
dimensions, surfaces, icon order), Alerts-matched hero position/size/red/font, one-line
Ukrainian titles from 641px up, tab alignment, the Setup Links tab, always-visible
QR/Timer/Donate previews (including the scaled full donor page), the Alerts hero separator,
the readout inside the output monitor, unique per-page code colours, one-line hero intros,
filled Local admin badges, the fading grid, 1% Ctrl+wheel zoom, hidden
scrollbar with working scroll, compact buttons, fixed Sound Alerts deck window across 2Ã—2â€“7Ã—7,
matching deck/editor width and gap, the play/volume/time player, ON/OFF toggles for My jar and
YouTube playlists, aligned Add-jar/Show buttons, equal Auction columns, Auction global save,
QR add/delete placement, rounded preview/overlay corners, and the Links note.

