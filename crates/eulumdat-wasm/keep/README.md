# Kept lazy bundles

Files here are copied into `dist/app/` by the "Site layout" step of
`scripts/build-wasm-split.sh` after every build. They exist because a bundle
is disabled in `scripts/build-config.toml` (`bundles.skyglow = false` since
the Bevy 0.19 upgrade) while the already built, content-hashed bundle stays
on the server; without the hashed loader here the fresh `index.html` would
reference the dev loader and `?wasm=skyglow_demo` would break. The relink
script then points `index.html` at the hashed name. Replace the files when
the bundle is rebuilt and re-enabled.

2026-11-01: skyglow is re-enabled (Bevy 0.19 port + Burg Vischering scene), so the
layout step skips this folder while `bundles.skyglow = true`. After deploying the
new bundle, replace the two files here with its hashed loader
(`dist/app/skyglow-loader-<hash>.js` + `.br`) in case the bundle is disabled again.
