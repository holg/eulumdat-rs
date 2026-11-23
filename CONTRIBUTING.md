# Contributing to eulumdat-rs / Candelabra

Thank you for your interest. Feedback from people who work with photometric
data is what moves this project forward.

## What helps most

- **Issues**: wrong results, files that do not open, differences to a standard
  or to another tool. A sample file and the expected value make it quick to
  check. Please do not attach files you may not share.
- **Suggestions**: standards, file formats or tools you are missing.
- **Reports like "this entry is stale" or "this dependency has an advisory"**:
  very welcome as an issue; the maintainer makes the change and credits you in
  the commit message.

## Code contributions

The project is published under AGPL-3.0-or-later, and the maintainer keeps the
option to offer it under other licences as well. That only works while the
rights to the code are in one hand, so:

- Small fixes are usually applied by the maintainer, with credit to whoever
  reported or proposed them, instead of being merged as a pull request.
- A larger contribution needs a contributor licence agreement (you keep your
  copyright and grant the maintainer the right to license your contribution
  under other terms too). Please open an issue first so we can agree on the
  approach before you invest time.

This is no judgement on any contribution; it keeps the licensing simple for
everyone who uses the code.

## Before proposing a change

`./scripts/test-ci-locally.sh` runs what CI runs (format, clippy, build,
tests, docs).
