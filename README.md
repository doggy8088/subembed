# burn-in-zh-subtitles

`burn-in-zh-subtitles` 是一個 Rust CLI，用來把繁體中文字幕直接燒錄進影片。

它會呼叫 `ffprobe` / `ffmpeg` 建立一條完整流程：探測影片尺寸、把字幕整理成 ASS、套用樣式、重新換行，最後輸出已燒字的 MP4。

## 這個工具會做什麼

- 接受明確指定的影片檔與字幕檔
- 支援常見字幕格式（例如 `.ass`、`.srt`、`.vtt`，以及其他 `ffmpeg` 可讀取的格式）
- 先把字幕整理成 ASS，再套用繁中字體與版面樣式
- 依影片高度自動推算字體大小、底部邊距、描邊與陰影
- 以 `libx264` 重新編碼影片，保留原始音訊（`-c:a copy`）
- 可在成功輸出後自動用系統預設程式開啟結果
- 先用系統上的 `ffmpeg` / `ffprobe`；找不到時再嘗試受管下載

## 安裝 Rust CLI

### 方式 1：下載 GitHub Release 二進位檔

每個正式版 tag（`vX.Y.Z`）都會由 GitHub Actions 建出對應平台的壓縮檔與 SHA-256 校驗檔。

目前 release 產物矩陣：

- macOS arm64：`aarch64-apple-darwin`
- macOS x64：`x86_64-apple-darwin`
- Linux x64：`x86_64-unknown-linux-gnu`
- Windows x64：`x86_64-pc-windows-msvc`

下載解壓後，把可執行檔 `burn-in-zh-subtitles`（Windows 為 `burn-in-zh-subtitles.exe`）放到你的 `PATH` 即可。

### 方式 2：從原始碼建置

需要 stable Rust toolchain。

```bash
git clone https://github.com/willh/burn-in-zh-subtitles.git
cd burn-in-zh-subtitles
cargo build --release
```

建置完成後，可執行：

```bash
./target/release/burn-in-zh-subtitles -v
```

如果想安裝到 Cargo bin 目錄：

```bash
cargo install --path .
```

## CLI 用法

### 基本語法

```text
burn-in-zh-subtitles [OPTIONS] <VIDEO> <SUBTITLE> [OUTPUT]
burn-in-zh-subtitles -v
burn-in-zh-subtitles --version
```

### 位置參數

- `VIDEO`：要燒錄字幕的來源影片
- `SUBTITLE`：要轉換並燒入的字幕檔
- `OUTPUT`：輸出檔路徑；若省略，預設為 `${video_stem}.zh-tw.burned.mp4`

> 例如 `movie.mp4` 的預設輸出會是 `movie.zh-tw.burned.mp4`。

### 支援的選項

| 選項 | 說明 |
| --- | --- |
| `-f`, `--force` | 若輸出檔已存在，直接覆寫 |
| `-o`, `--open` | 成功後用系統預設程式開啟輸出檔 |
| `-v`, `--version` | 顯示版本資訊後離開 |
| `-h`, `--help` | 顯示完整說明 |

### 使用範例

```bash
# 使用預設輸出名稱
burn-in-zh-subtitles movie.mp4 movie.zh-tw.vtt

# 指定輸出檔名
burn-in-zh-subtitles movie.mp4 movie.zh-tw.srt movie.final.mp4

# 允許覆寫既有檔案
burn-in-zh-subtitles --force movie.mp4 movie.zh-tw.ass

# 輸出後自動開啟
burn-in-zh-subtitles --open movie.mp4 movie.zh-tw.vtt

# 顯示版本
burn-in-zh-subtitles -v
burn-in-zh-subtitles --version
```

## 字幕樣式環境變數

CLI 目前用環境變數控制字型與樣式。

| 變數 | 預設值 | 說明 |
| --- | --- | --- |
| `FONT_NAME` | `LINE Seed TW_OTF Regular` | ASS 樣式使用的字型名稱 |
| `FONT_DIR` | `/Library/Fonts` | 提供給 `ffmpeg` 搜尋字型的資料夾 |
| `FONT_SIZE` | 依影片高度自動推算 | 字體大小 |
| `MARGIN_V` | 依影片高度自動推算 | 底部邊距 |
| `OUTLINE_SIZE` | 依字體大小自動推算 | 描邊粗細 |
| `SHADOW_SIZE` | 依描邊大小自動推算 | 陰影粗細 |

自動推算規則：

- `FONT_SIZE`：依影片高度縮放，並限制在合理範圍
- `MARGIN_V`：依影片高度與字體大小決定
- `OUTLINE_SIZE`：約為字體大小的 8%，最小值固定
- `SHADOW_SIZE`：約為描邊的一半，最小值固定

1080p 影片的預設結果大約會是：

- `FONT_SIZE = 70`
- `MARGIN_V = 35`
- `OUTLINE_SIZE = 5`
- `SHADOW_SIZE = 2`

### 樣式覆寫範例

```bash
FONT_NAME="Noto Sans CJK TC" \
FONT_DIR="/Library/Fonts" \
FONT_SIZE=64 \
MARGIN_V=48 \
OUTLINE_SIZE=5 \
SHADOW_SIZE=2 \
burn-in-zh-subtitles movie.mp4 movie.zh-tw.vtt
```

> 注意：預設 `FONT_DIR` 是 macOS 路徑。若你在 Linux 或 Windows 上執行，通常需要自行設定 `FONT_DIR`，並確認 `FONT_NAME` 對應的字型真的存在。

## ffmpeg / ffprobe 行為與回退策略

工具在執行時會依序做以下事情：

1. 先在 `PATH` 上尋找可用的系統 `ffmpeg` 與 `ffprobe`
2. 檢查 `ffmpeg -hide_banner -filters` 是否包含 `ass` 或 `subtitles` filter
3. 若系統工具不可用，則在支援平台上嘗試下載受管版本
4. 若受管下載也失敗，顯示手動安裝指引

### 受管下載支援平台

- macOS arm64
- macOS x64
- Linux x64
- Windows x64

### 受管下載快取位置

- macOS：`~/Library/Caches/burn-in-zh-subtitles/tools/ffmpeg`
- Linux：`${XDG_CACHE_HOME:-~/.cache}/burn-in-zh-subtitles/tools/ffmpeg`
- Windows：`%LOCALAPPDATA%\burn-in-zh-subtitles\tools\ffmpeg`

### 手動安裝時請確認

- `ffmpeg` 與 `ffprobe` 都能在 `PATH` 上找到
- `ffmpeg -hide_banner -filters` 的輸出裡有 `ass` 或 `subtitles`
- Linux / Windows 平台上若字型不在預設路徑，請自行設定 `FONT_DIR`

如果你想完全掌控 ffmpeg 版本，最穩定的作法仍然是先自行安裝系統 `ffmpeg` / `ffprobe`。

## npm 套件：`@willh/burn-subtitle`

此專案同時提供 npm wrapper：`@willh/burn-subtitle`。

- npm 套件名稱：`@willh/burn-subtitle`
- 安裝後的命令名稱：`burn-subtitle`
- 本質上是薄封裝：實際工作仍由 Rust 二進位檔執行

### 安裝與使用

```bash
npm install -g @willh/burn-subtitle
burn-subtitle movie.mp4 movie.zh-tw.vtt
```

或使用 `npx`：

```bash
npx @willh/burn-subtitle movie.mp4 movie.zh-tw.vtt
```

### npm wrapper 會做什麼

- `postinstall` 會依目前平台下載對應 GitHub Release 資產
- 下載後會驗證 `.sha256` 校驗檔
- 安裝完成後，`burn-subtitle` 會直接轉呼叫原生 Rust binary
- 若是在此 repo 內開發，且 `target/release/` 已經有本地建好的 binary，wrapper 會優先使用本地建置結果

### npm wrapper 支援平台

- `darwin-arm64`
- `darwin-x64`
- `linux-x64`
- `win32-x64`

不在這個矩陣內的平台，npm 安裝會直接失敗並提示不支援。

## GitHub Actions / Release 流程

專案已包含完整的 CI 與發佈流程：

- `CI`：在 push / pull request 上執行
  - `cargo check --locked`
  - `cargo fmt --all --check`
  - `cargo clippy --locked --all-targets --all-features -- -D warnings`
  - `cargo test --locked`
  - `npm test`
- `Release`：在 push `v*.*.*` tag 時建立 GitHub Release、打包各平台 Rust binary、產出 SHA-256 檔並上傳資產
- `Publish npm`：在 GitHub Release 發佈後，先確認所有 release 資產都可下載，再以 provenance 發佈到 npm

## 支援平台與注意事項

### Rust CLI

- 原始碼理論上可在更多平台建置，但「受管 ffmpeg 下載」只保證上述矩陣
- 若不在受管矩陣內，請自行安裝 `ffmpeg` / `ffprobe`
- 執行時必須明確提供 `VIDEO` 與 `SUBTITLE`
- 輸出檔若已存在，必須加 `--force`

### 轉檔與輸出行為

- 影片會重新編碼為 H.264（`libx264`）
- 音訊會直接複製（`-c:a copy`）
- 輸出容器為 MP4
- 會加上 `+faststart`

### 字型相關

- 若 `FONT_NAME` 與 `FONT_DIR` 不一致，ffmpeg 可能找不到正確字型
- 預設字型 `LINE Seed TW_OTF Regular` 若系統沒有安裝，請自行指定
- Linux / Windows 最常見的問題是忘記設定 `FONT_DIR`

## 快速檢查

```bash
burn-in-zh-subtitles --help
burn-in-zh-subtitles -v
ffmpeg -hide_banner -filters | grep -E ' ass | subtitles '
```

如果以上都正常，再執行正式燒字命令即可。
