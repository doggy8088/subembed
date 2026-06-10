# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.6] - 2026-06-10

### Changed
- 新增發佈版本以驗證 npm Trusted Publishing 整合。

## [0.1.5] - 2026-06-10

### Changed
- 合併 `release.yml` 與 `npm-publish.yml`，優化 OIDC 發佈流程。

## [0.1.4] - 2026-06-10

### Added
- 整合基於 OIDC Trusted Publishing 的自動化 npm 發佈工作流。

## [0.1.3] - 2026-06-10

### Fixed
- 修復 `cargo clippy` 警告與程式碼格式化（`cargo fmt`）問題以確保 CI 通過。

## [0.1.2] - 2026-06-10

### Added
- 新增適用於 macOS/Linux 的單行安裝腳本 `install.sh` 與 Windows 的單行安裝腳本 `install.ps1`。
- 在 `README.md` 中新增單行指令安裝說明，簡化安裝流程。

## [0.1.1] - 2026-05-31

### Fixed
- 修復 GitHub Actions 在自動對齊 npm 版本號與 Release Tag 時可能因版本相同而產生的錯誤。
- 優化 `package.json` 與 `LICENSE` 中的作者詮釋資料（一致採用 "Will 保哥"）。

### Added
- 驗證基於 OIDC Trusted Publishing 的全自動、免密碼化套件發佈工作流。

## [0.1.0] - 2026-05-31

### Added
- 以 Rust 重寫的跨平台字幕燒錄工具 `subembed`。
- 支援智慧中英文混排、ASS 格式字幕自動換行與樣式覆寫邏輯。
- 輕量級 npm 包裝器 `subembed`，支援自動偵測平台並從 GitHub Releases 下載、校驗及安裝對應二進位檔。
- 完善的 CI/CD 工作流：
  - `release.yml`：自動跨平台編譯、建立 GitHub Release，並基於 OIDC Trusted Publishing 全自動化發佈 npm 套件。
