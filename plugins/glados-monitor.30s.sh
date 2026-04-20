#!/usr/bin/env bash

set -euo pipefail

api_url="${GLADOS_MONITOR_URL:-https://termometer.glados.internal/temperature}"
cpu_warn="${GLADOS_CPU_WARN:-75}"
gpu_warn="${GLADOS_GPU_WARN:-80}"
icon_base64='PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCA5NiA5NiI+CiAgPGRlZnM+CiAgICA8bGluZWFyR3JhZGllbnQgaWQ9ImJvZHkiIHgxPSIwIiB5MT0iMCIgeDI9IjEiIHkyPSIxIj4KICAgICAgPHN0b3Agb2Zmc2V0PSIwIiBzdG9wLWNvbG9yPSIjMmYzMjM4Ii8+CiAgICAgIDxzdG9wIG9mZnNldD0iMSIgc3RvcC1jb2xvcj0iIzE3MWExZiIvPgogICAgPC9saW5lYXJHcmFkaWVudD4KICAgIDxsaW5lYXJHcmFkaWVudCBpZD0ibWV0YWwiIHgxPSIwIiB5MT0iMCIgeDI9IjEiIHkyPSIxIj4KICAgICAgPHN0b3Agb2Zmc2V0PSIwIiBzdG9wLWNvbG9yPSIjZjJmM2Y1Ii8+CiAgICAgIDxzdG9wIG9mZnNldD0iMSIgc3RvcC1jb2xvcj0iIzhlOTQ5ZCIvPgogICAgPC9saW5lYXJHcmFkaWVudD4KICAgIDxsaW5lYXJHcmFkaWVudCBpZD0iZXllUmluZyIgeDE9IjAiIHkxPSIwIiB4Mj0iMSIgeTI9IjEiPgogICAgICA8c3RvcCBvZmZzZXQ9IjAiIHN0b3AtY29sb3I9IiNmZmQzNmQiLz4KICAgICAgPHN0b3Agb2Zmc2V0PSIxIiBzdG9wLWNvbG9yPSIjZDE3YjEyIi8+CiAgICA8L2xpbmVhckdyYWRpZW50PgogIDwvZGVmcz4KICA8cGF0aCBkPSJNMTUgMThjOC03IDIzLTEwIDM5LTggNyAxIDE0IDQgMTggOSAzIDUgNCAxMSAzIDE3LTEgNi00IDEwLTggMTQgNSA0IDggMTAgNyAxOC0xIDgtNyAxNC0xNSAxNS05IDItMjAgMi0zMiAwLTEwLTItMTctOC0xOC0xOC0xLTcgMi0xNCA4LTE5LTMtMy02LTgtNi0xNCAwLTYgMS0xMSA0LTE0eiIgZmlsbD0idXJsKCNib2R5KSIvPgogIDxwYXRoIGQ9Ik0yMCA1M2M3LTUgMTctNyAzMC02IDggMCAxNSAyIDIxIDctMiAxMi0xMCAyMC0yMSAyMi05IDItMjIgMS0zMS0zLTctMy0xMS0xMC0xMC0yMHoiIGZpbGw9InVybCgjbWV0YWwpIi8+CiAgPHBhdGggZD0iTTI4IDE3YzUtNCAxMy02IDIyLTYgNSAwIDExIDEgMTUgMy0yIDQtNSA4LTkgMTEtNSAzLTEyIDUtMjEgNS01IDAtMTAtMS0xNS0zIDItNCA0LTcgOC0xMHoiIGZpbGw9IiMwZjExMTUiLz4KICA8Y2lyY2xlIGN4PSIyNyIgY3k9IjU5IiByPSI3IiBmaWxsPSIjMTExMzE4Ii8+CiAgPGNpcmNsZSBjeD0iMjciIGN5PSI1OSIgcj0iMyIgZmlsbD0iI2Q5M2IyYSIvPgogIDxnIHRyYW5zZm9ybT0idHJhbnNsYXRlKDUyIDI2KSI+CiAgICA8Y2lyY2xlIGN4PSIxNCIgY3k9IjE1IiByPSIxNiIgZmlsbD0iIzIwMjQyYSIvPgogICAgPGNpcmNsZSBjeD0iMTQiIGN5PSIxNSIgcj0iMTIiIGZpbGw9InVybCgjZXllUmluZykiLz4KICAgIDxjaXJjbGUgY3g9IjE0IiBjeT0iMTUiIHI9IjciIGZpbGw9IiNmZmYxYTUiLz4KICAgIDxjaXJjbGUgY3g9IjE0IiBjeT0iMTUiIHI9IjQiIGZpbGw9IiNkZjZkMDAiLz4KICA8L2c+CiAgPHBhdGggZD0iTTc2IDM3YzYgMCAxMSAyIDE0IDcgMiA0IDIgOCAxIDEyLTEgNC00IDctOCA5bDUgMTEtNiAzLTgtMTJjLTQtMS03LTItOS01LTMtNC0zLTEwLTEtMTUgMi02IDYtMTAgMTItMTB6IiBmaWxsPSJ1cmwoI21ldGFsKSIvPgogIDxwYXRoIGQ9Ik03NyAzOGMzIDEgNiA0IDYgOCAxIDQgMCA4LTMgMTEtMyAyLTYgMi05IDEtNC0yLTUtNi00LTEwIDEtNSA1LTEwIDEwLTEweiIgZmlsbD0iI2VlZjFmNCIvPgogIDxwYXRoIGQ9Ik0yMCA3NWM1IDMgMTAgNSAxNiA2LTQgNS05IDgtMTUgOC02IDAtMTEtMi0xNC03IDMtNCA3LTYgMTMtN3oiIGZpbGw9IiMwZDBmMTMiLz4KPC9zdmc+'

if ! json="$(curl -fsSL --max-time 5 "${api_url}")"; then
  echo "GLaDOS ! | image=${icon_base64}"
  echo "---"
  echo "Failed to fetch ${api_url}"
  exit 0
fi

cpu="$(printf '%s' "${json}" | sed -n 's/.*"cpu":[[:space:]]*\([0-9.]\+\).*/\1/p')"
gpu="$(printf '%s' "${json}" | sed -n 's/.*"gpu":[[:space:]]*\([0-9.]\+\).*/\1/p')"

if [[ -z "${cpu}" || -z "${gpu}" ]]; then
  echo "GLaDOS ? | image=${icon_base64}"
  echo "---"
  echo "Unexpected response"
  echo "${json}"
  exit 0
fi

cpu_label="CPU ${cpu}C"
gpu_label="GPU ${gpu}C"

if (( ${cpu%.*} >= cpu_warn )); then
  cpu_label="CPU ${cpu}C !"
fi

if (( ${gpu%.*} >= gpu_warn )); then
  gpu_label="GPU ${gpu}C !"
fi

timestamp="$(date '+%H:%M:%S')"

echo "${cpu_label} - ${gpu_label} | image=${icon_base64}"
echo "---"
echo "Updated: ${timestamp}"
echo "CPU: ${cpu} C"
echo "GPU: ${gpu} C"
echo "CPU warning threshold: ${cpu_warn} C"
echo "GPU warning threshold: ${gpu_warn} C"
echo "Refresh | refresh=true"
