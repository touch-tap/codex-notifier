# Changelog

## 0.1.7 (not released)

> **Do not release before the Antigravity Notifier phone apps are live on the App Store and
> Google Play.** This version suggests Antigravity Notifier to people who use Google Antigravity.
> On 27 Nov 2026 both apps were still in store review, so the suggestion links to
> antigravitynotifier.com and never to a store; released earlier, it would send people to a
> website with no app to download.

- **Windows: a hook could be kept waiting by codex-notifier's own background work.** The daily
  update check and the anonymous setup events are started by the hook and outlive it, and on
  Windows they kept the hook's output open, so whatever was reading it (Codex, a wrapper script)
  waited until they had finished: seconds for a check, as long as a download for an update. Both
  now start with none of the hook's handles. macOS and Linux were not affected.
- A Scoop install is recognised: `upgrade` says `scoop update codex-notifier` instead of replacing
  Scoop's copy in place, and `uninstall` says `scoop uninstall codex-notifier`. If Scoop installed
  0.1.6 or earlier and it has since updated itself, run `scoop update codex-notifier` once so
  Scoop's record matches the binary.
- If you also use Google Antigravity (`agy`) and don't have antigravity-notifier, codex-notifier
  mentions [Antigravity Notifier](https://antigravitynotifier.com), the same thing for Antigravity,
  with its one-line install: after `codex-notifier pair` (with a QR code for the website), in
  `status` and `doctor`, and at most twice ever when Codex starts. Only one app is ever suggested
  at a time, Claude Code Notifier first, and the two share the limit of twice.
  `codex-notifier config set cross_promo off` or `CODEX_NOTIFIER_NO_NUDGE=1` turns both off, as
  before. In all 17 languages.
- The README and `codex-notifier pair --help` now say what a bare `codex-notifier pair` does: in a
  terminal it asks for the 6-digit code the app shows, and pressing Enter without one shows a QR
  code to scan instead.

## 0.1.6 and earlier

See the [releases on GitHub](https://github.com/touch-tap/codex-notifier/releases).
