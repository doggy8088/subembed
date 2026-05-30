# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-05-31

### Added
- 以 Rust 重寫的跨平台字幕燒錄工具 `subembed`。
- 支援智慧中英文混排、ASS 格式字幕自動換行與樣式覆寫邏輯。
- 輕量級 npm 包裝器 `subembed`，支援自動偵測平台並從 GitHub Releases 下載、校驗及安裝對應二進位檔。
- 完善的 CI/CD 工作流：
  - `release.yml`：自動跨平台編譯並建立 GitHub Release。
  - `npm-publish.yml`：基於 OIDC Trusted Publishing 與雙重參數控制（支援 `[skip npm]` 關鍵字與手動跳過）的全自動化 npm 套件發佈工作流。
