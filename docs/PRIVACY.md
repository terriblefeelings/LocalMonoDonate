# Privacy policy — LocalMonoDonate

**Short version:** LocalMonoDonate runs entirely on your own Windows PC. There is **no backend
server**, no account, no cloud sync and no telemetry. Everything it knows is stored in a folder on your
computer, and the only network traffic is the requests needed to fetch your donations, deliver alerts
and (optionally) talk to Twitch, StreamElements, Tailscale or a TTS provider.

Last updated: 2026-10-21

---

## 1. Local by design

- The application is a single executable. It starts a web server bound to `127.0.0.1` (and optionally a
  public port through Tailscale) on your machine.
- All state is written to `%APPDATA%\LocalMonoDonate\` on the same computer.
- Nothing is uploaded to the author, to GitHub, or to any “LocalMonoDonate cloud”. There is no such
  service.
- Uninstalling = closing the app and deleting that folder.

## 2. What is stored, where, and why

| Data | Location | Why it is needed |
|---|---|---|
| Nickname, overlay name, jar codes/names, feature switches, alert rules, timer/auction/bits settings, TTS mode, StreamElements account id and on/off state | `config.json` | to run the app the way you configured it |
| Donation history: donor nickname (as typed or bank sender name), amount, message, matched code, timestamps | `data.json` | alerts, stats, top donors, replays |
| Pending donations and their codes | `data.json` | to match a bank payment to the form it belongs to |
| Timer and auction state (bids, lots, history) | `data.json` | to resume after a restart |
| Images/videos you upload in the editors | `media\` | your own overlay/background assets |
| Monobank API token | `<port>\token` | read-only access to your jars’ statements |
| Twitch client id/secret + OAuth tokens | `<port>\twitch.json` | subs/channel-point events for the timer and auction |
| StreamElements JWT | `<port>\streamelements.json` | posting tips/activities when enabled |
| Operational log | `LocalMonoDonate.log` | debugging failed requests; contains donation events (name/amount) and HTTP errors |

There is no database of “users”, no address book, no payment card data and no bank login. Monobank’s
API only exposes statements for the jars you explicitly add, through the token you create.

## 3. What leaves your computer

The app contacts a service only for the feature that needs it:

| Destination | When | What is sent |
|---|---|---|
| **Monobank API** (`api.monobank.ua`) | always (polling) | your `X-Token`, jar/statement requests |
| **Tailscale** (`tailscale.com`, local CLI) | only if you press *Connect Tailscale* | device login; the app runs `tailscale up`/`funnel` locally |
| **Twitch** (`id.twitch.tv`, `api.twitch.tv`, EventSub WebSocket) | only if Twitch is configured and enabled | client id/secret + OAuth token, EventSub subscriptions |
| **StreamElements** (`api.streamelements.com`) | only if the SE switch is on | JWT, donation/sound-alert tip and activity data |
| **Exchange-rate API** (`open.er-api.com`) | only if StreamElements is on | a plain rate request (no personal data) to convert UAH → USD for SE totals |
| **Google Translate TTS** (via `gtts`) | only when a Google voice is used | the text to be spoken (donor name/amount/message per your read mode) |
| **Azure Speech** (`*.tts.speech.microsoft.com`) | only if you configured Azure voices | text + your Azure key/region |
| **Microsoft WebView2 bootstrapper** | only if WebView2 is missing on first start | a request to download the WebView2 runtime installer |

All requests use HTTPS (except the local Tailscale CLI). No request contains anything that is not
listed above.

## 4. Tokens and credentials

- **Are they stored?** Yes — a credential has to be kept so the app can poll the bank and refresh Twitch
  tokens after a restart. It is saved **only** in `%APPDATA%\LocalMonoDonate\<port>\`
  (`token`, `twitch.json`, `streamelements.json`), per instance/port, on this computer.
- **Where they are never stored:** `config.json` (it is written with an empty `token` field by design),
  the project folder, the release zip, HTML/JS/CSS files, generated links, logs, or any server.
- They are shown in the local admin page (`/setup`) as blurred text with a Show/Hide button. That page is
  bound to `127.0.0.1` by default and is never part of the public listener.
- They are sent **only** to the service they belong to (Monobank/Twitch/StreamElements/Azure), over
  HTTPS, and are not forwarded anywhere else.
- Deleting the matching file removes the app’s copy; revoke the token itself in the provider’s dashboard
  if you want to invalidate it completely.

## 5. What is never collected

- No analytics, telemetry, crash reporting, advertising IDs or fingerprinting.
- No accounts, email addresses or phone numbers.
- No browsing history, clipboard content, keystrokes or screenshots.
- No card numbers, bank credentials, passwords or Monobank login.
- No usage data sent to the author or to GitHub.

## 6. Public exposure

If you connect Tailscale, the public listener serves **only** the donation page and the sound-alert
deck. Admin pages are bound to `127.0.0.1` by default; overlays, the stats page and every credential
stay local. The stats page is protected by a random code in its URL and is also not part of the public
listener.

## 7. Retention and deletion

- Data stays until you delete it. There is no remote copy.
- Reset: close the app and delete `%APPDATA%\LocalMonoDonate\`.
- Delete a single credential: remove the matching file in `<port>\`.
- Logs: `LocalMonoDonate.log` is a plain text file you can delete at any time.

## 8. Third-party services

Monobank, Twitch, StreamElements, Tailscale, Google, Microsoft and the exchange-rate service process
requests under their own privacy policies. LocalMonoDonate cannot control what those providers log;
it only sends what is described above, and only when the corresponding feature is used.

## 9. Changes

This document lives in the repository. Material changes to what the app stores or sends will be
reflected here and noted in the release notes.
