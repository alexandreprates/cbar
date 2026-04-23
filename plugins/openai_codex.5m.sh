#!/usr/bin/env bash
#<xbar.title>OpenAI Codex Usage</xbar.title>
#<xbar.version>1.0</xbar.version>
#<xbar.author>OpenAI Codex</xbar.author>
#<xbar.desc>Display OpenAI Codex usage limits from local Codex session metadata</xbar.desc>
#<xbar.dependencies>python3</xbar.dependencies>
#<xbar.image>https://developers.openai.com/favicon.png</xbar.image>

# User variables
# ================
#<xbar.var>boolean(VAR_SHOW_7D="true"): Also show the secondary window in the title (e.g. 5h:12% week:4%).</xbar.var>
#<xbar.var>boolean(VAR_COLORS="true"): Color-code title at warning (>75%) and critical (>90%) levels.</xbar.var>
#<xbar.var>boolean(VAR_SHOW_RESET="true"): Show time-until-reset for each window in the dropdown.</xbar.var>
#<xbar.var>boolean(VAR_SHOW_BARS="true"): Show dynamic dual progress bar icon (primary top, secondary bottom).</xbar.var>

SHOW_7D="${VAR_SHOW_7D:-true}"
COLORS="${VAR_COLORS:-true}"
SHOW_RESET="${VAR_SHOW_RESET:-true}"
SHOW_BARS="${VAR_SHOW_BARS:-true}"

CODEX_ICON="iVBORw0KGgoAAAANSUhEUgAAABIAAAASCAYAAABWzo5XAAAABGdBTUEAALGPC/xhBQAAACBjSFJNAAB6JgAAgIQAAPoAAACA6AAAdTAAAOpgAAA6mAAAF3CculE8AAAAeGVYSWZNTQAqAAAACAAEARoABQAAAAEAAAA+ARsABQAAAAEAAABGASgAAwAAAAEAAgAAh2kABAAAAAEAAABOAAAAAAAAAJAAAAABAAAAkAAAAAEAA6ABAAMAAAABAAEAAKACAAQAAAABAAAAEqADAAQAAAABAAAAEgAAAABIJAr0AAAACXBIWXMAABYlAAAWJQFJUiTwAAADc0lEQVQ4EW2UW2hcVRSGv73PZWaEkqQ3xGjQTm1orFqh3mNRkajtg9AqqDG+9KKRoogP9kUd8aUPgo2CYhovL9I+aEGoaBWlqUV9iLZIUIvGVqmRCkmqtc7lzDnbf5+ZVCrdcObsM2utf631r39vw/nW9lNdlOIBCDfh0lswRGAOYswYWe0jKh2z/w8z5/xRcSGmvgVjn8AEvbgMsmbLxYbCkrvLfsaxExe9RsW0jTKdBar8uRBTHCWIN5Kl0EggaFv1iW3vQxUXyJA03oXaI/PVtYC2uoiLqruJSxtp1DAOBq8KuPUyS105D/6SMfmHo9Z0TM3I6KMKRSWr72V68n5G1yStPN31R4laINQFcnVA72LDM580+XHWsb0/4L6Vlqf7Q56/PST2UUpIWNjAhSs3+1oNnthi4Qg26iFJWN9rGb42ZPiDhKduDPnttGPf0ZRVSy2fHcsYvi5gSuC7j6jfgtpMG1NQXxNSKqzLQcTJOoEMqZqZqqOnw+DUgg8auTviwPGMgeUhM/84+pbKoA5JPY+FMhl3Wpzboicnc0NfQGW8yWxVNIqbtT2WFYvgpzmHVezIlymn1fqlnYZYxeTLT9bZzX6m/fmI5Riq91NqvUM83nCxZe/3KTv2p9gFKfsfiulbYnhJYA1nGVhh2fedQEI/UtfvaVN9QtH3VycyHlZrL37R5OZLDFeohSULTQ564i/HW+LltmWWw9MZ5a52e3mPRAIyh7DShXZ7JjPuEU/bRPZOZZ5QwKebYgavbAmqoQICxd9Rtnx7skUHXqi4QxaTjUnJebupHL1exg6nDK4O6FtsefLDhPd/SKlKQ15TQ9LXyb8d45pgLlivdmN2GSpznVCcIIjK1BO23RTQvcDw5jcpXSXDenERyvdlVTh0jeX3M/DO1+LBk231kyW/UiuulovWs2eGiS94laaYVsUPKuv13YaSEJYvMrwt0Mv1zmR84UCKistpJdJUmvXHea74Sgto64SOyKo9xFKqV+zZowhlTcpP0AtzXFrySsmPSOyPSPU9pksPMGqSFpCvyh9aiq8TxfeSqvRMw/RLVOSP9/ScB2rHaNPUOUM3RftK+Q9IPvhrhPpjWH+N2GV5+nOuEQ3FpUd1lYzgCrvOf414oPnlq7PFuwSkA+nWippEVXyu/RtUqx+zo3Nu3nX+/S9FI1GiDAig5wAAAABJRU5ErkJggg=="

show_error() {
  local message="$1"
  echo "! | image=${CODEX_ICON}"
  echo "---"
  echo "${message}"
  echo "---"
  echo "Refresh | refresh=true"
  echo "Open Codex Docs | href=https://developers.openai.com/codex"
  exit 0
}

SESSION_INFO="$(
python3 - "${HOME}/.codex/sessions" <<'PY'
import sys
from pathlib import Path

sessions_dir = Path(sys.argv[1]).expanduser()

try:
    latest = max(
        (path for path in sessions_dir.rglob("*.jsonl") if path.is_file()),
        key=lambda path: path.stat().st_mtime,
    )
except ValueError:
    sys.exit(0)

print(latest)
print(int(latest.stat().st_mtime))
PY
)"

LATEST_SESSION="$(printf '%s\n' "$SESSION_INFO" | sed -n '1p')"
SESSION_MTIME="$(printf '%s\n' "$SESSION_INFO" | sed -n '2p')"

[ -z "$LATEST_SESSION" ] && show_error "No Codex session files found in ~/.codex/sessions"
[ ! -f "$LATEST_SESSION" ] && show_error "Latest Codex session file is not accessible"

PARSED="$(
python3 - "$LATEST_SESSION" "${HOME}/.codex/auth.json" <<'PY'
import json
import sys
from pathlib import Path

session_path = Path(sys.argv[1])
auth_path = Path(sys.argv[2])

rate_limits = None
plan_type = ""
model = ""
cli_version = ""
credits = None

try:
    with session_path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                item = json.loads(line)
            except Exception:
                continue

            payload = item.get("payload") or {}
            item_type = item.get("type")

            if item_type == "session_meta":
                cli_version = payload.get("cli_version") or cli_version

            if item_type == "turn_context":
                model = payload.get("model") or model

            if item_type == "event_msg" and payload.get("type") == "token_count":
                rl = payload.get("rate_limits")
                if isinstance(rl, dict):
                    rate_limits = rl
                    plan_type = rl.get("plan_type") or plan_type
                    credits = rl.get("credits")

    if not plan_type and auth_path.exists():
        try:
            auth = json.loads(auth_path.read_text(encoding="utf-8"))
            token = (((auth.get("tokens") or {}).get("access_token")) or "")
            if token:
                parts = token.split(".")
                if len(parts) >= 2:
                    import base64
                    payload = parts[1]
                    payload += "=" * (-len(payload) % 4)
                    decoded = json.loads(base64.urlsafe_b64decode(payload).decode("utf-8"))
                    plan_type = (
                        (((decoded.get("https://api.openai.com/auth") or {}).get("chatgpt_plan_type")) or "")
                    )
        except Exception:
            pass

    if not rate_limits:
        raise RuntimeError("no token_count rate_limits found")

    primary = rate_limits.get("primary") or {}
    secondary = rate_limits.get("secondary") or {}

    def out(value):
        print("" if value is None else value)

    out(primary.get("used_percent", 0))
    out(primary.get("window_minutes", 0))
    out(primary.get("resets_at", ""))
    out(secondary.get("used_percent", 0))
    out(secondary.get("window_minutes", 0))
    out(secondary.get("resets_at", ""))
    out(plan_type)
    out(model)
    out(cli_version)
    out("yes" if isinstance(credits, dict) and credits.get("has_credits") else "no")
    out("yes" if isinstance(credits, dict) and credits.get("unlimited") else "no")
    out("" if not isinstance(credits, dict) else credits.get("balance", ""))
except Exception as exc:
    sys.stderr.write(str(exc) + "\n")
    sys.exit(1)
PY
)"

[ -z "$PARSED" ] && show_error "Could not parse Codex session rate limits"

UTIL_PRIMARY="$(     printf '%s\n' "$PARSED" | sed -n '1p')"
WINDOW_PRIMARY="$(   printf '%s\n' "$PARSED" | sed -n '2p')"
RESET_PRIMARY="$(    printf '%s\n' "$PARSED" | sed -n '3p')"
UTIL_SECONDARY="$(   printf '%s\n' "$PARSED" | sed -n '4p')"
WINDOW_SECONDARY="$( printf '%s\n' "$PARSED" | sed -n '5p')"
RESET_SECONDARY="$(  printf '%s\n' "$PARSED" | sed -n '6p')"
PLAN_TYPE="$(        printf '%s\n' "$PARSED" | sed -n '7p')"
MODEL_NAME="$(       printf '%s\n' "$PARSED" | sed -n '8p')"
CLI_VERSION="$(      printf '%s\n' "$PARSED" | sed -n '9p')"
HAS_CREDITS="$(      printf '%s\n' "$PARSED" | sed -n '10p')"
UNLIMITED_CREDITS="$(printf '%s\n' "$PARSED" | sed -n '11p')"
CREDITS_BALANCE="$(  printf '%s\n' "$PARSED" | sed -n '12p')"

format_pct() {
  python3 -c "print(round(float('${1:-0}')))" 2>/dev/null || echo "0"
}

window_label() {
  local minutes="${1:-0}"
  case "$minutes" in
    300) echo "5h" ;;
    10080) echo "7d" ;;
    1440) echo "24h" ;;
    *) python3 -c "m=int('${minutes:-0}'); print(f'{m}m' if m < 60 else (f'{m//60}h' if m % 60 == 0 else f'{m}m'))" 2>/dev/null || echo "${minutes}m" ;;
  esac
}

title_window_label() {
  local minutes="${1:-0}"
  case "$minutes" in
    1440) echo "day" ;;
    10080) echo "W" ;;
    *) window_label "$minutes" ;;
  esac
}

PLAN_LABEL="$(printf '%s' "${PLAN_TYPE:-unknown}" | tr '[:lower:]' '[:upper:]')"
PRIMARY_LABEL="$(window_label "$WINDOW_PRIMARY")"
SECONDARY_LABEL="$(window_label "$WINDOW_SECONDARY")"
TITLE_PRIMARY_LABEL="$(title_window_label "$WINDOW_PRIMARY")"
TITLE_SECONDARY_LABEL="$(title_window_label "$WINDOW_SECONDARY")"
PCT_PRIMARY="$(format_pct "$UTIL_PRIMARY")"
PCT_SECONDARY="$(format_pct "$UTIL_SECONDARY")"

time_until_epoch() {
  local ts="$1"
  [ -z "$ts" ] && echo "?" && return
  python3 -c "
from datetime import datetime, timezone
try:
    reset = datetime.fromtimestamp(float('${ts}'), tz=timezone.utc)
    now = datetime.now(timezone.utc)
    diff = reset - now
    secs = diff.total_seconds()
    if secs <= 0:
        print('now')
    else:
        days = int(secs // 86400)
        hours = int((secs % 86400) // 3600)
        mins = int((secs % 3600) // 60)
        if days > 0:
            print(f'{days}d {hours}h')
        elif hours > 0:
            print(f'{hours}h {mins}m')
        else:
            print(f'{mins}m')
except Exception:
    print('?')
" 2>/dev/null || echo "?"
}

format_reset_local() {
  local ts="$1"
  [ -z "$ts" ] && echo "?" && return
  python3 -c "
from datetime import datetime
try:
    print(datetime.fromtimestamp(float('${ts}')).strftime('%Y-%m-%d %H:%M'))
except Exception:
    print('?')
" 2>/dev/null || echo "?"
}

time_since_epoch() {
  local ts="$1"
  [ -z "$ts" ] && echo "?" && return
  python3 -c "
from datetime import datetime, timezone
try:
    then = datetime.fromtimestamp(float('${ts}'), tz=timezone.utc)
    now = datetime.now(timezone.utc)
    secs = max(int((now - then).total_seconds()), 0)
    days = secs // 86400
    hours = (secs % 86400) // 3600
    mins = (secs % 3600) // 60
    if days > 0:
        print(f'{days}d {hours}h ago')
    elif hours > 0:
        print(f'{hours}h {mins}m ago')
    else:
        print(f'{mins}m ago')
except Exception:
    print('?')
" 2>/dev/null || echo "?"
}

color_for_pct() {
  local pct=$1
  if [ "$COLORS" = "true" ]; then
    [ "$pct" -ge 90 ] 2>/dev/null && echo "#CC0000" && return
    [ "$pct" -ge 70 ] 2>/dev/null && echo "#CC8800" && return
  fi
  echo ""
}

make_bar() {
  local pct="${1:-0}"
  local width=20
  local filled
  filled=$(python3 -c "print(min(int(round(${pct} * ${width} / 100)), ${width}))" 2>/dev/null || echo "0")
  local bar=""
  local i=1
  while [ "$i" -le "$width" ]; do
    if [ "$i" -le "$filled" ]; then
      bar="${bar}#"
    else
      bar="${bar}-"
    fi
    i=$((i + 1))
  done
  echo "$bar"
}

make_window_svg() {
  local pct="${1:-0}"
  python3 -c "
import base64

pct = min(max(int(round(${pct})), 0), 100)
track_width = 27
fill = max(4, round(track_width * pct / 100)) if pct > 0 else 0

svg = f'''<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 45 16\">
<rect x=\"9\" y=\"6\" width=\"{track_width}\" height=\"4\" fill=\"#ffffff\" fill-opacity=\"0.30\"/>
<rect x=\"9\" y=\"6\" width=\"{fill}\" height=\"4\" fill=\"#ffffff\"/>
</svg>'''

print(base64.b64encode(svg.encode('utf-8')).decode())
" 2>/dev/null
}

make_usage_svg() {
  local pct1="${1:-0}" pct2="${2:-0}"
  python3 -c "
import base64

p1 = min(max(int(round(${pct1})), 0), 100)
p2 = min(max(int(round(${pct2})), 0), 100)

track_width = 28
fill1 = max(5, round(track_width * p1 / 100)) if p1 > 0 else 0
fill2 = max(5, round(track_width * p2 / 100)) if p2 > 0 else 0

svg = f'''<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 48 20\">
<rect x=\"10\" y=\"5\" width=\"{track_width}\" height=\"4\" fill=\"#ffffff\" fill-opacity=\"0.30\"/>
<rect x=\"10\" y=\"5\" width=\"{fill1}\" height=\"4\" fill=\"#ffffff\"/>
<rect x=\"10\" y=\"11\" width=\"{track_width}\" height=\"4\" fill=\"#ffffff\" fill-opacity=\"0.30\"/>
<rect x=\"10\" y=\"11\" width=\"{fill2}\" height=\"4\" fill=\"#ffffff\"/>
</svg>'''

print(base64.b64encode(svg.encode('utf-8')).decode())
" 2>/dev/null
}

COLOR_PRIMARY="$(color_for_pct "$PCT_PRIMARY")"
COLOR_SECONDARY="$(color_for_pct "$PCT_SECONDARY")"

title_color() {
  local c1="$1" c2="$2"
  [ "$c1" = "#CC0000" ] || [ "$c2" = "#CC0000" ] && echo "#CC0000" && return
  [ "$c1" = "#CC8800" ] || [ "$c2" = "#CC8800" ] && echo "#CC8800" && return
  echo ""
}

if [ "$SHOW_7D" = "true" ]; then
  TITLE_COLOR="$(title_color "$COLOR_PRIMARY" "$COLOR_SECONDARY")"
  TITLE="${TITLE_PRIMARY_LABEL}:${PCT_PRIMARY}% ${TITLE_SECONDARY_LABEL}:${PCT_SECONDARY}%"
else
  TITLE_COLOR="$COLOR_PRIMARY"
  TITLE="${TITLE_PRIMARY_LABEL}:${PCT_PRIMARY}%"
fi

if [ "$SHOW_BARS" = "true" ]; then
  BAR_ICON="$(make_usage_svg "$PCT_PRIMARY" "$PCT_SECONDARY")"
  if [ -n "$TITLE_COLOR" ]; then
    echo " | image=${BAR_ICON} color=${TITLE_COLOR}"
  else
    echo " | image=${BAR_ICON}"
  fi
else
  if [ -n "$TITLE_COLOR" ]; then
    echo "${TITLE} | image=${CODEX_ICON} color=${TITLE_COLOR}"
  else
    echo "${TITLE} | image=${CODEX_ICON}"
  fi
fi

echo "---"
echo "Plan: ${PLAN_LABEL:-UNKNOWN} | color=#888888"
[ -n "$MODEL_NAME" ] && echo "Model: ${MODEL_NAME} | color=#888888"
[ -n "$CLI_VERSION" ] && echo "CLI: ${CLI_VERSION} | color=#888888"
echo "Updated: $(time_since_epoch "$SESSION_MTIME") | color=#888888"
echo "Source: ${LATEST_SESSION##*/} | color=#888888"
echo "---"

BAR_PRIMARY="$(make_bar "$PCT_PRIMARY")"
BAR_PRIMARY_IMAGE="$(make_window_svg "$PCT_PRIMARY" true)"
echo "${PRIMARY_LABEL} window | color=#888888"
if [ -n "$BAR_PRIMARY_IMAGE" ]; then
  if [ -n "$COLOR_PRIMARY" ]; then
    echo "${PRIMARY_LABEL}: ${PCT_PRIMARY}% | image=${BAR_PRIMARY_IMAGE} color=${COLOR_PRIMARY}"
  else
    echo "${PRIMARY_LABEL}: ${PCT_PRIMARY}% | image=${BAR_PRIMARY_IMAGE}"
  fi
elif [ -n "$COLOR_PRIMARY" ]; then
  echo "${PRIMARY_LABEL}: ${PCT_PRIMARY}% ${BAR_PRIMARY} | color=${COLOR_PRIMARY}"
else
  echo "${PRIMARY_LABEL}: ${PCT_PRIMARY}% ${BAR_PRIMARY}"
fi
if [ "$SHOW_RESET" = "true" ] && [ -n "$RESET_PRIMARY" ]; then
  echo "Resets in: $(time_until_epoch "$RESET_PRIMARY") | color=#888888"
  echo "Resets at: $(format_reset_local "$RESET_PRIMARY") | color=#888888"
fi

echo "---"

BAR_SECONDARY="$(make_bar "$PCT_SECONDARY")"
BAR_SECONDARY_IMAGE="$(make_window_svg "$PCT_SECONDARY" false)"
echo "${SECONDARY_LABEL} window | color=#888888"
if [ -n "$BAR_SECONDARY_IMAGE" ]; then
  if [ -n "$COLOR_SECONDARY" ]; then
    echo "${SECONDARY_LABEL}: ${PCT_SECONDARY}% | image=${BAR_SECONDARY_IMAGE} color=${COLOR_SECONDARY}"
  else
    echo "${SECONDARY_LABEL}: ${PCT_SECONDARY}% | image=${BAR_SECONDARY_IMAGE}"
  fi
elif [ -n "$COLOR_SECONDARY" ]; then
  echo "${SECONDARY_LABEL}: ${PCT_SECONDARY}% ${BAR_SECONDARY} | color=${COLOR_SECONDARY}"
else
  echo "${SECONDARY_LABEL}: ${PCT_SECONDARY}% ${BAR_SECONDARY}"
fi
if [ "$SHOW_RESET" = "true" ] && [ -n "$RESET_SECONDARY" ]; then
  echo "Resets in: $(time_until_epoch "$RESET_SECONDARY") | color=#888888"
  echo "Resets at: $(format_reset_local "$RESET_SECONDARY") | color=#888888"
fi

echo "---"
if [ "$UNLIMITED_CREDITS" = "yes" ]; then
  echo "Credits: unlimited | color=#888888"
elif [ "$HAS_CREDITS" = "yes" ]; then
  if [ -n "$CREDITS_BALANCE" ]; then
    echo "Credits balance: ${CREDITS_BALANCE} | color=#888888"
  else
    echo "Credits: enabled | color=#888888"
  fi
else
  echo "Credits: not enabled in session data | color=#888888"
fi

echo "---"
echo "Open Codex Web | href=https://chatgpt.com/codex"
echo "Open Codex Docs | href=https://developers.openai.com/codex"
echo "Open Help: ChatGPT Plan Limits | href=https://help.openai.com/en/articles/11369540-codex-in-chatgpt"
echo "Open Help: Credits Usage | href=https://help.openai.com/en/articles/12642688"
echo "---"
echo "Refresh | refresh=true"
