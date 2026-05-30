# iesna.eu production rollback runbook

This directory holds **byte-exact backups of the live `index.html`** before each
production deploy, plus a manifest of the bundles each index points at.

## Why this works (the deploy invariant)

The deploy script (`scripts/build-wasm-split.sh deploy`) uses
`rsync -avz` against `iesna.eu:/var/www/iesna.eu/html/` **without `--delete`**.
All bundle filenames are content-hashed (Trunk-style:
`eulumdat-wasm-<hash>.js`, `styles-<hash>.css`, `<loader>-<hash>.js`, etc.), so:

- New deploys *add* new hashed bundles next to the old ones.
- The **single file that decides which bundle is "live"** is `index.html`,
  whose `<script type="module">` and `<link>` tags name the active hashes.
- Old hashed bundles are never deleted — they sit on disk indefinitely.

**⇒ Rolling back is one file:** restore the previous `index.html` and the
previously-active hashed bundles (still on the server) become live again.
Zero rebuild, zero downtime, no need to redeploy anything.

## Pre-deploy capture (already done, before the SPD deploy)

Saved here on 2026-05-30:

```
docs/deploy-backups/
├── index-2026-05-30_064654Z.html   # the live index right before deploy
├── manifest-2026-05-30_064654Z.txt # sizes + sha256 of every bundle it references
└── ROLLBACK.md                     # this file
```

The captured index points at these 8 bundles (all verified present on
the server, with brotli siblings):

| Bundle | Size (raw) | sha256 (first 16) |
|---|---|---|
| `bevy-loader-4f7e84244e9fd835.js` | 1 723 B | `48fece304e22f098` |
| `eulumdat-wasm-b4931f3af8ca75f3.js` | 90 582 B | `47c4e3256339ca4d` |
| `eulumdat-wasm-b4931f3af8ca75f3_bg.wasm` | 18 098 469 B | `e5b1fafecfaa0469` |
| `gmaps-loader-34453207a5841135.js` | 18 757 B | `d5f4c7a374ee7451` |
| `skyglow-loader-3641b3f101275b69.js` | 4 692 B | `8762ecd7ed9b9140` |
| `street-loader-ccba7fb53f225cea.js` | 1 817 B | `05072d8229f34f3e` |
| `styles-c1c9c0abb0355731.css` | 101 414 B | `b9f44a7a89b9db1b` |
| `typst-loader-f97c8fafa6a0044a.js` | 1 375 B | `5f40af7fa8b974ed` |

The current production also carries the SRI integrity hashes baked into the
index — those are part of the saved file, so restoring the file restores the
SRI guarantees too.

## Rollback procedure (30 seconds, single command)

If the new deploy misbehaves and we need to flip back to the captured state:

```sh
# 1. Sanity-check the backed-up bundles are still present on the server.
ssh iesna.eu 'cd /var/www/iesna.eu/html && \
  for f in bevy-loader-4f7e84244e9fd835.js \
           eulumdat-wasm-b4931f3af8ca75f3.js \
           eulumdat-wasm-b4931f3af8ca75f3_bg.wasm \
           gmaps-loader-34453207a5841135.js \
           skyglow-loader-3641b3f101275b69.js \
           street-loader-ccba7fb53f225cea.js \
           styles-c1c9c0abb0355731.css \
           typst-loader-f97c8fafa6a0044a.js; do
    [ -f "$f" ] || echo "MISSING: $f"
  done'
# (no output = all present, rollback is safe)

# 2. Push the captured index.html back.
scp docs/deploy-backups/index-2026-05-30_064654Z.html \
    iesna.eu:/var/www/iesna.eu/html/index.html

# 3. Verify what the live site now serves.
curl -sL https://iesna.eu/ | grep -oE '[a-zA-Z0-9_-]+-[a-f0-9]{16}(_bg)?\.(wasm|js|css)' | sort -u
# expected: the 8 bundles listed in the table above
```

That's the whole rollback. Users hitting the site after step 2 get the old
bundle set immediately; any cached service-worker or browser-cache hits remain
valid because the hashes haven't changed.

## When the rollback would NOT work

- Someone manually `rm`'d the old hashed bundles from the server.
- The `--delete` flag got added to `rsync_flags` in `scripts/build-config.toml`
  (currently it is `-avz`, deliberately without `--delete`).
- The nginx config got changed to require SRI hashes not present in the saved
  index (none of the current loaders rely on this).

If any of these conditions apply, full rebuild from the
`deploy-baseline-2026-05-30` git tag is the fallback (see
`docs/deploy-restore.md` for code-side rollback).

## Before the NEXT deploy

Run the capture again, with a fresh timestamp, so each deploy has its own
restore point:

```sh
TS=$(date -u +%Y-%m-%d_%H%M%SZ)

# Snapshot the live index.
ssh iesna.eu 'cat /var/www/iesna.eu/html/index.html' \
  > docs/deploy-backups/index-${TS}.html

# Record what bundles it points at + their sizes/sha256.
BUNDLES=$(grep -oE '[a-zA-Z0-9_-]+-[a-f0-9]{16}(_bg)?\.(wasm|js|css)' \
  docs/deploy-backups/index-${TS}.html | sort -u)
ssh iesna.eu "cd /var/www/iesna.eu/html && for f in $BUNDLES; do \
  if [ -f \"\$f\" ]; then printf 'OK   %-55s %10s B  %s\n' \"\$f\" \$(stat -c%s \"\$f\") \$(sha256sum \"\$f\"|cut -c1-16); \
  else printf 'MISS %s\n' \"\$f\"; fi; done" \
  > docs/deploy-backups/manifest-${TS}.txt

# Commit both files so the restore point ships with the repo.
git add docs/deploy-backups/index-${TS}.html docs/deploy-backups/manifest-${TS}.txt
```

## Keep / prune policy

Old `index-*.html` + `manifest-*.txt` pairs cost a few KB each — keep them all
for a while; they're a free deployment history. Prune anything older than
~6 months when the dir grows past 50 files.
