#!/usr/bin/env python3
"""Record the project's download total and render it as a small SVG chart.

GitHub only exposes the current count per asset, so the history has to be kept
here: every run appends today's total to docs/downloads.json and redraws
docs/downloads.svg from it. The scheduled workflow runs this once a day.

Usage: python3 scripts/download-chart.py [--repo OWNER/NAME]
Set GITHUB_TOKEN to raise the API rate limit (the workflow does).
"""
from __future__ import annotations

import json
import os
import pathlib
import sys
import urllib.request
from datetime import datetime, timezone

ROOT = pathlib.Path(__file__).resolve().parents[1]
HISTORY = ROOT / "docs/downloads.json"
CHART = ROOT / "docs/downloads.svg"
DEFAULT_REPO = "AdminZachary/WatchBridge"


def releases(repo: str) -> list[dict]:
    out: list[dict] = []
    url = f"https://api.github.com/repos/{repo}/releases?per_page=100"
    while url:
        request = urllib.request.Request(url)
        request.add_header("Accept", "application/vnd.github+json")
        request.add_header("User-Agent", "watchbridge-download-chart")
        token = os.environ.get("GITHUB_TOKEN")
        if token:
            request.add_header("Authorization", f"Bearer {token}")
        with urllib.request.urlopen(request, timeout=60) as response:
            out.extend(json.loads(response.read()))
            link = response.headers.get("Link", "")
        url = ""
        for part in link.split(","):
            if 'rel="next"' in part:
                url = part.split(";")[0].strip().strip("<>")
    return out


def render(history: list[dict]) -> str:
    width, height = 720, 240
    left, right, top, bottom = 56, 16, 24, 34
    plot_w = width - left - right
    plot_h = height - top - bottom
    totals = [point["total"] for point in history]
    peak = max(totals) if totals else 0
    ceiling = max(peak, 1)
    # Round the ceiling up so the axis label is a round number.
    magnitude = 10 ** max(len(str(ceiling)) - 2, 0)
    ceiling = ((ceiling // magnitude) + 1) * magnitude

    def x_at(index: int) -> float:
        if len(history) == 1:
            return left + plot_w / 2
        return left + plot_w * index / (len(history) - 1)

    def y_at(total: int) -> float:
        return top + plot_h * (1 - total / ceiling)

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}" role="img" aria-label="Downloads over time">',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#5b6c63}'
        '.value{fill:#246756;font-weight:600}</style>',
        f'<rect width="{width}" height="{height}" fill="#ffffff"/>',
    ]

    for step in range(4):
        value = ceiling * step / 3
        y = y_at(value)
        parts.append(
            f'<line x1="{left}" y1="{y:.1f}" x2="{width - right}" y2="{y:.1f}" '
            'stroke="#e3ebe4" stroke-width="1"/>'
        )
        parts.append(
            f'<text x="{left - 8}" y="{y + 4:.1f}" font-size="11" text-anchor="end">'
            f"{int(value):,}</text>"
        )

    if history:
        points = " ".join(f"{x_at(i):.1f},{y_at(p['total']):.1f}" for i, p in enumerate(history))
        # A single snapshot has no area to fill; it would read as history that
        # does not exist yet.
        if len(history) > 1:
            area = f"{left},{top + plot_h} {points} {x_at(len(history) - 1):.1f},{top + plot_h}"
            parts.append(f'<polygon points="{area}" fill="#246756" opacity="0.10"/>')
        parts.append(
            f'<polyline points="{points}" fill="none" stroke="#246756" stroke-width="2.5" '
            'stroke-linejoin="round" stroke-linecap="round"/>'
        )
        for i, point in enumerate(history):
            parts.append(
                f'<circle cx="{x_at(i):.1f}" cy="{y_at(point["total"]):.1f}" r="3.5" fill="#246756"/>'
            )
        last = history[-1]
        parts.append(
            f'<text class="value" x="{x_at(len(history) - 1):.1f}" y="{y_at(last["total"]) - 10:.1f}" '
            f'font-size="13" text-anchor="end">{last["total"]:,}</text>'
        )
        parts.append(
            f'<text x="{left}" y="{height - 12}" font-size="11">{history[0]["date"]}</text>'
        )
        if len(history) > 1:
            parts.append(
                f'<text x="{width - right}" y="{height - 12}" font-size="11" '
                f'text-anchor="end">{history[-1]["date"]}</text>'
            )

    parts.append(
        f'<text x="{left}" y="{top - 8}" font-size="11">total downloads (all releases)</text>'
    )
    parts.append("</svg>")
    return "\n".join(parts) + "\n"


def main() -> int:
    repo = DEFAULT_REPO
    if "--repo" in sys.argv:
        repo = sys.argv[sys.argv.index("--repo") + 1]

    history = json.loads(HISTORY.read_text(encoding="utf-8")) if HISTORY.exists() else []
    data = releases(repo)
    total = sum(asset.get("download_count", 0) for release in data for asset in release.get("assets", []))
    today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    entry = {"date": today, "total": total}
    if history and history[-1]["date"] == today:
        history[-1] = entry
    else:
        history.append(entry)

    HISTORY.parent.mkdir(parents=True, exist_ok=True)
    HISTORY.write_text(json.dumps(history, indent=2) + "\n", encoding="utf-8")
    CHART.write_text(render(history), encoding="utf-8")
    print(f"{repo}: {total} downloads across {len(data)} releases, {len(history)} snapshot(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
