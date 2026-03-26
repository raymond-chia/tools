#!/bin/sh
# Claude Code statusline script
# 透過 stdin 接收 JSON 格式的 session 資料，輸出狀態列
# 設定位置: ~/.claude/settings.json -> statusLine.command
# JSON 欄位文件: https://code.claude.com/docs/zh-TW/statusline#json
#
# 輸出格式:
#   第一行: <cwd> 🌿 <branch> ⌚ <HH:MM:SS> ✦ <model>
#   第二行: ▓▓▓░░ <N>% \t\t💰 $<cost> 🎨 <style> 📝 transcript
#   第三行: 5h: <N>% ⏳(<Xh Ym>)
#   第四行: 7d: <N>% ⏳(<Xd Yh Zm>) 📅 <MM/DD HH:MM>
#
# 腳本在每個 assistant 訊息後執行，更新會 300ms debounce。
# 如果新更新在腳本執行中觸發，會取消進行中的執行。
# 狀態列在本地執行，不消耗 API token。

# DEBUG_CLAUDE_STATUSLINE 設定時，會記錄原始輸入
raw_input=$(cat)

# 單次 jq 呼叫擷取所有欄位為 key=value 格式（避免多次 fork+exec）
# 預設值（讓 linter 識別變數已宣告）
cwd="" model="" usage_5h="" resets_5h="" usage_7d="" resets_7d="" cost=0 transcript_path="" output_style="" ctx_pct=0
eval "$(printf '%s' "$raw_input" | jq -r '{
  cwd: .cwd,
  model: .model.display_name,
  usage_5h: (.rate_limits.five_hour.used_percentage // ""),
  resets_5h: (.rate_limits.five_hour.resets_at // ""),
  usage_7d: (.rate_limits.seven_day.used_percentage // ""),
  resets_7d: (.rate_limits.seven_day.resets_at // ""),
  cost: (.cost.total_cost_usd // 0),
  transcript_path: (.transcript_path // ""),
  output_style: (.output_style.name // ""),
  ctx_pct: (.context_window.used_percentage // 0)
} | to_entries[] | "\(.key)=\u0027\(.value)\u0027"')"

# 印出原始 JSON（tee 效果：同時保留原始輸入供除錯）
if [ -n "$DEBUG_CLAUDE_STATUSLINE" ]; then
  echo "$raw_input" > /tmp/claude_statusline_debug_raw.json
  printf 'cwd=%s\nmodel=%s\nusage_5h=%s\nresets_5h=%s\nusage_7d=%s\nresets_7d=%s\ncost=%s\ntranscript_path=%s\noutput_style=%s\nctx_pct=%s\n' \
    "$cwd" "$model" "$usage_5h" "$resets_5h" "$usage_7d" "$resets_7d" "$cost" "$transcript_path" "$output_style" "$ctx_pct" \
    > /tmp/claude_statusline_debug_parsed.txt
fi

# GIT_OPTIONAL_LOCKS=0: 避免與其他 git 程序的鎖定衝突
git_branch=$(GIT_OPTIONAL_LOCKS=0 git -C "$cwd" symbolic-ref --short HEAD 2>/dev/null) || git_branch=""

read -r time_str now <<EOF
$(date '+%H:%M:%S %s')
EOF

# --- 輸出 ---

# ANSI 色碼: 31=紅 32=綠 33=黃 35=紫, 1;=粗體
if [ -n "$git_branch" ]; then
  printf "\033[1;32m%s\033[0m 🌿 \033[35m%s\033[0m ⌚ \033[1;31m%s\033[0m ✦ %s\n" \
    "$cwd" "$git_branch" "$time_str" "$model"
else
  printf "\033[1;32m%s\033[0m ⌚ \033[1;31m%s\033[0m ✦ %s\n" \
    "$cwd" "$time_str" "$model"
fi

ctx_pct_int=$(printf '%.0f' "${ctx_pct:-0}")
BAR_WIDTH=10
FILLED=$(( (ctx_pct_int * BAR_WIDTH + 99) / 100 ))
EMPTY=$((BAR_WIDTH - FILLED))
# 百分比上色（24-bit true color），結果存入 _pct_color
pct_color() {
  if [ "$1" -ge 90 ]; then _pct_color='\033[38;2;153;51;51m'   # #993333
  elif [ "$1" -ge 70 ]; then _pct_color='\033[38;2;255;0;102m'  # #FF0066
  elif [ "$1" -ge 50 ]; then _pct_color='\033[38;2;102;102;255m' # #6666FF
  elif [ "$1" -ge 20 ]; then _pct_color='\033[38;2;0;204;255m'  # #00CCFF
  else _pct_color='\033[38;2;0;255;0m'                           # #00FF00
  fi
}
pct_color "$ctx_pct_int"
BAR_COLOR=$_pct_color

# sh 相容的字元重複
repeat_char() { _rc=""; _i=0; while [ "$_i" -lt "$2" ]; do _rc="${_rc}$1"; _i=$((_i+1)); done; echo "$_rc"; }
BAR="$(repeat_char '▓' "$FILLED")$(repeat_char '░' "$EMPTY")"

cost_fmt=$(printf '%.3f' "$cost")
printf '%b%s\033[0m %s%%\t\t💰 $%s' "$BAR_COLOR" "$BAR" "$ctx_pct_int" "$cost_fmt"
[ -n "$output_style" ] && printf ' 🎨 %s' "$output_style"
# OSC 8 可點擊連結（需 iTerm2/Kitty/WezTerm）
if [ -n "$transcript_path" ]; then
  printf ' \033]8;;file://%s\a📝 transcript\033]8;;\a' "$transcript_path"
fi
printf '\n'

# --- Rate limits（僅 Pro/Max 訂閱者） ---

# 將剩餘秒數格式化為倒數字串，結果存入 _countdown
format_countdown() {
  _remain=$1
  if [ "$_remain" -gt 0 ]; then
    _d=$(( _remain / 86400 ))
    _h=$(( (_remain % 86400) / 3600 ))
    _m=$(( (_remain % 3600) / 60 ))
    if [ "$_d" -gt 0 ]; then
      _countdown="⏳(${_d}d${_h}h${_m}m)"
    else
      _countdown="⏳(${_h}h${_m}m)"
    fi
  else
    _countdown="⏳(now)"
  fi
}

# 印出單行 rate limit: <label>: <usage>% <countdown> [📅 <date>]
print_rate_limit() {
  _label=$1 _usage=$2 _resets_at=$3
  _usage_int=$(printf '%.0f' "$_usage")
  _remain=$(( _resets_at - now ))
  format_countdown "$_remain"
  pct_color "$_usage_int"
  if [ "$_label" = "7d" ] && [ "$_remain" -gt 0 ]; then
    # 跨平台 epoch → 可讀日期（macOS/BSD 用 -r，GNU/Linux 用 -d @）
    _date_str=$(date -r "$_resets_at" '+%m/%d %H:%M' 2>/dev/null) \
      || _date_str=$(date -d "@$_resets_at" '+%m/%d %H:%M' 2>/dev/null) \
      || _date_str=""
    printf '%s: %b%s%%\033[0m %s 📅 %s\n' "$_label" "$_pct_color" "$_usage_int" "$_countdown" "$_date_str"
  else
    printf '%s: %b%s%%\033[0m %s\n' "$_label" "$_pct_color" "$_usage_int" "$_countdown"
  fi
}

[ -n "$usage_5h" ] && print_rate_limit "5h" "$usage_5h" "$resets_5h"
[ -n "$usage_7d" ] && print_rate_limit "7d" "$usage_7d" "$resets_7d"
# 確保 exit 0：上方 [ -n "" ] 在值為空時回傳 1，會導致 statusline 不顯示
exit 0
