# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.3](https://github.com/syntropd/toold/compare/v0.3.1...v0.3.3) - 2026-10-01

### Added

- *(actuator)* implement virtual HID actuator, self-correction loop, and netlink socket diag
- *(toold)* implement Bubblewrap/Landlock agent sandbox execution and bump v0.3.3

### Fixed

- *(actuator)* add MoveMouseAbs dispatch, finite coord clamping, model completers, and robust sandbox reflection
- *(qa)* use journald service fallback in syntax verify test when routerd is absent

### Other

- apply cargo fmt across workspace
