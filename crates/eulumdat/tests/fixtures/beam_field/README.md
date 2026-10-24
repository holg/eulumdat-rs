# Beam/field angle regression fixtures

Reference values come from a comparison against the commercial macOS viewer
LumaScope (C0–C180 / C90–C270 beam and field angles, peak intensity), which an
independent implementation of the CIE/IES section definition reproduced on
all 18 files. Used by `tests/beam_field_sections.rs`.

| Files | Source | In git |
|---|---|---|
| `4058075580…_FL_MAX_LUM_*.ldt` (3) | LEDVANCE downloads, also shipped as templates in `crates/eulumdat-wasm*/templates/` | yes |
| `ITALO 1 5P5 S05 3.140-3M.IES` | AEC, also a template in `crates/eulumdat-wasm-templates/templates/` | yes |
| `1.IES` – `4.IES`, `sample1.ldt` – `sample10.ldt` | Manufacturer files (BEGA, Tulux, Waldmann, Regent, Zumtobel, ERCO, Trilux, LEDiL, Signify) from the public GitHub repo `serhii-chechui/LumaScope`, which carries **no licence** | no (`.gitignore`) |

The 14 third-party files are not redistributed. Put them here locally to run
the full comparison; without them the test checks the committed files plus
synthetic cases that cover the same edge cases (up-lights, a field wider than
180°, a global peak outside the first stored plane, alternative lamp sets).

The folder and the test are also excluded from the published crate
(`exclude` in `Cargo.toml`).
