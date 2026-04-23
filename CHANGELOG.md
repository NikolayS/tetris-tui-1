# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed

- **Runtime RNG is `StdRng` (ChaCha12).** `rand_chacha` is only a dev dep
  now; runtime code uses `rand::rngs::StdRng`. Existing seeds produce
  different bag sequences than prior dev versions (v0.0.0-pre1–pre3).
  Acceptable at v0.1.0 since no public release carried the earlier RNG.
