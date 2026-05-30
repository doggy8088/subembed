#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

DEFAULT_VIDEO="${WORKSPACE_DIR}/OpenAI_2026-05-14_2055016850849993072.mp4"
DEFAULT_SUBTITLE="${WORKSPACE_DIR}/OpenAI_2026-05-29_2060428604727771421_zh-tw.vtt"
DEFAULT_OUTPUT="${WORKSPACE_DIR}/OpenAI_2026-05-14_2055016850849993072.zh-tw.burned.mp4"

FONT_NAME="${FONT_NAME:-LINE Seed TW_OTF}"
FONT_DIR="${FONT_DIR:-/Library/Fonts}"
FORCE_OVERWRITE=0

usage() {
  cat <<'EOF'
用法：
  ./burn_subtitles.sh [--force] [影片.mp4 字幕.vtt [輸出.mp4]]

不帶參數時，會直接使用這兩個範例檔：
  ../OpenAI_2026-05-14_2055016850849993072.mp4
  ../OpenAI_2026-05-29_2060428604727771421_zh-tw.vtt

可用環境變數：
  FONT_NAME   字型家族名稱，預設為 LINE Seed TW_OTF
  FONT_DIR    字型所在資料夾，預設為 /Library/Fonts
EOF
}

fail() {
  printf '錯誤：%s\n' "$*" >&2
  exit 1
}

abspath() {
  local path="$1"
  local dir
  dir="$(cd "$(dirname "$path")" && pwd)"
  printf '%s/%s\n' "$dir" "$(basename "$path")"
}

pick_ffmpeg() {
  local brew_prefix

  if command -v brew >/dev/null 2>&1; then
    brew_prefix="$(brew --prefix ffmpeg-full 2>/dev/null || true)"
    if [[ -n "${brew_prefix}" && -x "${brew_prefix}/bin/ffmpeg" ]]; then
      printf '%s\n' "${brew_prefix}/bin/ffmpeg"
      return 0
    fi
  fi

  if command -v ffmpeg >/dev/null 2>&1; then
    command -v ffmpeg
    return 0
  fi

  return 1
}

has_filter() {
  local ffmpeg_bin="$1"
  local filter_name="$2"
  local filters_output

  filters_output="$("${ffmpeg_bin}" -hide_banner -filters 2>/dev/null)"
  grep -Eq "[[:space:]]${filter_name}[[:space:]]" <<<"${filters_output}"
}

ensure_burn_filter() {
  local ffmpeg_bin="$1"

  if has_filter "${ffmpeg_bin}" ass; then
    printf 'ass\n'
    return 0
  fi

  if has_filter "${ffmpeg_bin}" subtitles; then
    printf 'subtitles\n'
    return 0
  fi

  cat >&2 <<EOF
找不到可燒錄字幕的 ffmpeg 濾鏡（ass/subtitles）。

目前偵測到的 ffmpeg：
  ${ffmpeg_bin}

這通常代表 ffmpeg 沒有編入 libass。
如果你用 Homebrew，安裝這個版本即可：
  brew install ffmpeg-full

安裝完成後，腳本會自動優先使用 ffmpeg-full。
EOF
  exit 1
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    -f|--force)
      FORCE_OVERWRITE=1
      shift
      ;;
    --)
      shift
      break
      ;;
    -*)
      fail "不支援的參數：$1"
      ;;
    *)
      break
      ;;
  esac
done

if [[ $# -eq 0 ]]; then
  VIDEO_PATH="${DEFAULT_VIDEO}"
  SUBTITLE_PATH="${DEFAULT_SUBTITLE}"
  OUTPUT_PATH="${DEFAULT_OUTPUT}"
elif [[ $# -eq 2 ]]; then
  VIDEO_PATH="$1"
  SUBTITLE_PATH="$2"
  OUTPUT_PATH="${1%.*}.zh-tw.burned.mp4"
elif [[ $# -eq 3 ]]; then
  VIDEO_PATH="$1"
  SUBTITLE_PATH="$2"
  OUTPUT_PATH="$3"
else
  usage >&2
  exit 1
fi

[[ -f "${VIDEO_PATH}" ]] || fail "找不到影片：${VIDEO_PATH}"
[[ -f "${SUBTITLE_PATH}" ]] || fail "找不到字幕：${SUBTITLE_PATH}"
[[ -d "${FONT_DIR}" ]] || fail "找不到字型資料夾：${FONT_DIR}"

VIDEO_PATH="$(abspath "${VIDEO_PATH}")"
SUBTITLE_PATH="$(abspath "${SUBTITLE_PATH}")"

mkdir -p "$(dirname "${OUTPUT_PATH}")"
OUTPUT_DIR="$(cd "$(dirname "${OUTPUT_PATH}")" && pwd)"
OUTPUT_FILE="${OUTPUT_DIR}/$(basename "${OUTPUT_PATH}")"

if [[ -e "${OUTPUT_FILE}" && "${FORCE_OVERWRITE}" -ne 1 ]]; then
  fail "輸出檔已存在：${OUTPUT_FILE}（如要覆蓋請加 --force）"
fi

FFMPEG_BIN="$(pick_ffmpeg)" || fail "系統上找不到 ffmpeg"
FFPROBE_BIN="$(dirname "${FFMPEG_BIN}")/ffprobe"
[[ -x "${FFPROBE_BIN}" ]] || FFPROBE_BIN="$(command -v ffprobe || true)"
[[ -n "${FFPROBE_BIN}" && -x "${FFPROBE_BIN}" ]] || fail "系統上找不到 ffprobe"

RENDER_FILTER="$(ensure_burn_filter "${FFMPEG_BIN}")"

IFS=x read -r VIDEO_WIDTH VIDEO_HEIGHT < <(
  "${FFPROBE_BIN}" \
    -v error \
    -select_streams v:0 \
    -show_entries stream=width,height \
    -of csv=s=x:p=0 \
    "${VIDEO_PATH}"
)

[[ -n "${VIDEO_WIDTH}" && -n "${VIDEO_HEIGHT}" ]] || fail "無法讀取影片解析度"

FONT_SIZE=$(( VIDEO_HEIGHT * 34 / 1000 ))
if (( FONT_SIZE < 26 )); then
  FONT_SIZE=26
elif (( FONT_SIZE > 42 )); then
  FONT_SIZE=42
fi

MARGIN_V=$(( VIDEO_HEIGHT * 35 / 1000 ))
if (( MARGIN_V < 28 )); then
  MARGIN_V=28
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

RAW_ASS="${TMP_DIR}/raw.ass"
STYLED_ASS="${TMP_DIR}/styled.ass"

"${FFMPEG_BIN}" -hide_banner -loglevel error -y -i "${SUBTITLE_PATH}" "${RAW_ASS}"

{
  cat <<EOF
[Script Info]
ScriptType: v4.00+
WrapStyle: 2
ScaledBorderAndShadow: yes
PlayResX: ${VIDEO_WIDTH}
PlayResY: ${VIDEO_HEIGHT}

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,${FONT_NAME},${FONT_SIZE},&H0000FFFF,&H0000FFFF,&H00000000,&H64000000,0,0,0,0,100,100,0,0,1,3.2,1.6,2,48,48,${MARGIN_V},1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
EOF

  awk '
    /^\[Events\]/ { in_events = 1; next }
    in_events && /^Dialogue:/ { print }
  ' "${RAW_ASS}"
} > "${STYLED_ASS}"

ASS_FILTER_PATH="${STYLED_ASS//\'/\\\'}"
ASS_FILTER_FONT_DIR="${FONT_DIR//\'/\\\'}"

if [[ "${RENDER_FILTER}" == "ass" ]]; then
  VIDEO_FILTER="ass='${ASS_FILTER_PATH}':fontsdir='${ASS_FILTER_FONT_DIR}'"
else
  VIDEO_FILTER="subtitles='${ASS_FILTER_PATH}':fontsdir='${ASS_FILTER_FONT_DIR}'"
fi

"${FFMPEG_BIN}" \
  -hide_banner \
  -loglevel info \
  $([[ "${FORCE_OVERWRITE}" -eq 1 ]] && printf '%s' "-y" || printf '%s' "-n") \
  -i "${VIDEO_PATH}" \
  -vf "${VIDEO_FILTER}" \
  -map 0:v:0 \
  -map 0:a? \
  -c:v libx264 \
  -preset medium \
  -crf 18 \
  -pix_fmt yuv420p \
  -c:a copy \
  -movflags +faststart \
  "${OUTPUT_FILE}"

printf '完成：%s\n' "${OUTPUT_FILE}"
