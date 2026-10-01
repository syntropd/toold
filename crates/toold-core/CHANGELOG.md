# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.3](https://github.com/syntropd/toold/compare/v0.3.1...v0.3.3) - 2026-10-01

### Added

- *(actuator)* implement virtual HID actuator, self-correction loop, and netlink socket diag
- *(policy)* add allowed exit codes, systemd socket mounts, and network namespace selection
- *(toold)* implement Bubblewrap/Landlock agent sandbox execution and bump v0.3.3
- *(toold)* Landlock confinement plus runner fixes

### Fixed

- *(sandbox)* probe bwrap execution capability and fall back to Landlock if bwrap fails
- *(policy)* pass --man=no in syntax.verify to avoid manpage lookup under sandbox
- *(qa)* use journald service fallback in syntax verify test when routerd is absent

### Other

- apply cargo fmt across workspace
- *(runner)* add explicit exit code 3 verification and expand zero-shell coverage
