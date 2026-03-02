# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.1] - 2026-03-02

### Fixed
- prevent recursive logging by running the exporter task with `Dispatch::none()` subscriber isolation https://github.com/ymgyt/tracing-cloudwatch/pull/60

## [0.4.0] - 2026-03-01

### Breaking
- `CloudWatchLayer::with_client` now returns a tuple `(CloudWatchLayer, CloudWatchWorkerGuard)` instead of returning only the layer. https://github.com/ymgyt/tracing-cloudwatch/pull/52
  Existing call sites must keep the returned guard alive, and should call `cw_guard.shutdown().await` for graceful shutdown.

### Added
- Add `CloudWatchWorkerGuard::shutdown()` to explicitly wait for exporter drain + flush completion. https://github.com/ymgyt/tracing-cloudwatch/pull/55

### Changed
  - migrate crate edition from 2021 to 2024 (requires a newer Rust toolchain)
  https://github.com/ymgyt/tracing-cloudwatch/pull/58

## [0.3.1] - 2025-03-25
### Fixed
- remove type ambiguity of `batch_size` https://github.com/ymgyt/tracing-cloudwatch/pull/46 

## [0.3.0] - 2024-11-15
### Added
- add `ordered_logs` feature to handle chronological order in logs https://github.com/ymgyt/tracing-cloudwatch/pull/41

## [0.2.0] - 2024-10-12
### Added
- add `with_fmt_layer` method https://github.com/ymgyt/tracing-cloudwatch/pull/39

## [0.1.5] - 2024-02-06
### Changed
- update aws-sdk-cloudwatchlogs version from 0.34 to 1 https://github.com/ymgyt/tracing-cloudwatch/pull/34

## [0.1.4] - 2023-10-19
### Added
- add `rusoto_rustls` feature https://github.com/ymgyt/tracing-cloudwatch/pull/24

## [0.1.3] - 2023-10-03
### Changed
- update aws-sdk-cloudwatchlogs version from 0.28 to 0.31 https://github.com/ymgyt/tracing-cloudwatch/pull/23

## [0.1.2] - 2023-07-24
### Changed
- update aws-sdk-cloudwatchlogs version from 0.27.0 to 0.28.0 https://github.com/ymgyt/tracing-cloudwatch/pull/6

## [0.1.1] - 2023-05-07
### Added
- support AWS SDK https://github.com/ymgyt/tracing-cloudwatch/pull/3

## [0.1.0] - 2023-05-06
### Added
- crate published
