# Kept lazy bundles

Files here are copied into `dist/app/` by the "Site layout" step of
`scripts/build-wasm-split.sh` after every build. They exist because a bundle
is disabled in `scripts/build-config.toml` (`bundles.skyglow = false` since
the Bevy 0.19 upgrade) while the already built, content-hashed bundle stays
on the server; without the hashed loader here the fresh `index.html` would
reference the dev loader and `?wasm=skyglow_demo` would break. The relink
script then points `index.html` at the hashed name. Replace the files when
the bundle is rebuilt and re-enabled.
