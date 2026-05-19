#!/usr/bin/env python3
"""Generate CRAP badge JSON and HTML report from cargo-crap JSON output.

Usage: crap_html.py <crap-full.json> <badge-out.json> <html-out.html>
"""

import json
import html
import sys
from collections import defaultdict


THRESHOLD = 30


def make_badge(entries: list) -> dict:
    total = len(entries)
    failing = sum(1 for e in entries if e["crap"] > THRESHOLD)
    passing = total - failing
    pct = round(100 * passing / total, 1) if total else 0
    if failing == 0:
        color = "brightgreen"
    elif failing <= 5:
        color = "green"
    elif failing <= 15:
        color = "yellow"
    else:
        color = "red"
    return {
        "schemaVersion": 1,
        "label": "CRAP",
        "message": f"{passing}/{total} passing ({pct}%)",
        "color": color,
    }


def cov_bar(cov: float | None, width: int = 10) -> str:
    """Render a small ASCII-style coverage bar as HTML spans."""
    if cov is None:
        return '<span class="cov-na">—</span>'
    filled = round(cov / 100 * width)
    empty = width - filled
    pct = round(cov, 1)
    if cov >= 90:
        cls = "cov-high"
    elif cov >= 60:
        cls = "cov-mid"
    else:
        cls = "cov-low"
    bar = f'<span class="cov-filled {cls}">{"█" * filled}</span>'
    bar += f'<span class="cov-empty">{"░" * empty}</span>'
    return f'{bar} <span class="cov-pct {cls}">{pct}%</span>'


def status_icon(crap: float) -> tuple[str, str]:
    """Return (icon, css class) based on CRAP score."""
    if crap > THRESHOLD:
        return "✗", "status-fail"
    elif crap >= THRESHOLD - 4:
        return "▲", "status-warn"
    else:
        return "✓", "status-pass"


def score_class(crap: float) -> str:
    if crap > 100:
        return "score-critical"
    elif crap > THRESHOLD:
        return "score-fail"
    elif crap >= THRESHOLD - 4:
        return "score-warn"
    else:
        return ""


def build_summary_cards(entries: list) -> str:
    total = len(entries)
    failing = sum(1 for e in entries if e["crap"] > THRESHOLD)
    passing = total - failing
    worst = max((e["crap"] for e in entries), default=0)
    avg_cov = sum(e.get("coverage") or 0 for e in entries) / total if total else 0
    cov_zero = sum(1 for e in entries if (e.get("coverage") or 0) == 0)

    return f"""
    <div class="summary-grid">
      <div class="summary-card">
        <div class="summary-value">{passing}<span class="summary-dim">/{total}</span></div>
        <div class="summary-label">passing</div>
      </div>
      <div class="summary-card {"summary-alert" if failing > 0 else ""}">
        <div class="summary-value">{failing}</div>
        <div class="summary-label">failing</div>
      </div>
      <div class="summary-card">
        <div class="summary-value">{worst:.0f}</div>
        <div class="summary-label">worst score</div>
      </div>
      <div class="summary-card">
        <div class="summary-value">{avg_cov:.0f}%</div>
        <div class="summary-label">avg coverage</div>
      </div>
      <div class="summary-card">
        <div class="summary-value">{cov_zero}</div>
        <div class="summary-label">uncovered fns</div>
      </div>
    </div>"""


def build_table(entries: list, section_id: str, title: str, show: bool = True) -> str:
    if not entries:
        return ""

    display = "" if show else ' style="display:none"'
    rows = []
    current_file = None

    for e in entries:
        crap = e["crap"]
        icon, icon_cls = status_icon(crap)
        s_cls = score_class(crap)
        fn_name = html.escape(e["function"])
        file_path = e["file"].removeprefix("./")
        line = e["line"]
        cc = int(e["cyclomatic"])
        cov = e.get("coverage")
        bar = cov_bar(cov)

        # File group separator
        if file_path != current_file:
            current_file = file_path
            rows.append(
                f'<tr class="file-separator">'
                f'<td colspan="6" class="file-path">{html.escape(file_path)}</td>'
                f'</tr>'
            )

        rows.append(
            f'<tr>'
            f'<td class="{icon_cls}">{icon}</td>'
            f'<td class="col-score {s_cls}">{crap:.1f}</td>'
            f'<td class="col-cc">{cc}</td>'
            f'<td class="col-cov">{bar}</td>'
            f'<td class="col-fn"><code>{fn_name}</code></td>'
            f'<td class="col-loc">:{line}</td>'
            f'</tr>'
        )

    table_rows = "\n".join(rows)
    count = len(entries)

    return f"""
    <div class="table-section" id="{section_id}"{display}>
      <h2 class="section-head">{title} <span class="section-count">({count})</span></h2>
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th class="th-status"></th>
              <th class="th-score">CRAP</th>
              <th class="th-cc">CC</th>
              <th class="th-cov">Coverage</th>
              <th class="th-fn">Function</th>
              <th class="th-loc">Line</th>
            </tr>
          </thead>
          <tbody>
            {table_rows}
          </tbody>
        </table>
      </div>
    </div>"""


def build_html(badge: dict, entries: list) -> str:
    color_map = {
        "brightgreen": "#4c1",
        "green": "#97ca00",
        "yellow": "#dfb317",
        "red": "#e05d44",
    }
    badge_color = color_map.get(badge["color"], "#999")

    # Sort by CRAP descending, then group
    entries_sorted = sorted(entries, key=lambda e: -e["crap"])

    failing = [e for e in entries_sorted if e["crap"] > THRESHOLD]
    warning = [e for e in entries_sorted if THRESHOLD - 4 <= e["crap"] <= THRESHOLD]
    passing = [e for e in entries_sorted if e["crap"] < THRESHOLD - 4]

    # Within each group, sort by file then line for readability
    for group in [failing, warning, passing]:
        group.sort(key=lambda e: (e["file"], e["line"]))

    summary = build_summary_cards(entries)
    table_failing = build_table(failing, "failing", "Exceeding threshold", show=True)
    table_warning = build_table(warning, "warning", "Near threshold", show=True)
    table_passing = build_table(passing, "passing", "Passing", show=False)

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>CRAP Report — remendo</title>
  <style>
    :root {{
      color-scheme: dark;
      --denim-dark:   #1a2a3a;
      --denim-mid:    #2c4a6e;
      --denim-light:  #4a7ab5;
      --thread-white: #e8e0d4;
      --thread-gold:  #c4a35a;
      --tux-orange:   #e8943a;
      --card-bg:      #1e3450;
      --card-border:  #5a8ab5;
      --text-primary: #e8e0d4;
      --text-muted:   #9ab0c8;
      --badge-color:  {badge_color};
    }}

    * {{ margin: 0; padding: 0; box-sizing: border-box; }}

    body {{
      font-family: 'Courier New', 'Consolas', 'Liberation Mono', monospace;
      min-height: 100vh;
      background-color: var(--denim-dark);
      background-image:
        repeating-linear-gradient(0deg, transparent, transparent 2px, rgba(74,122,181,0.08) 2px, rgba(74,122,181,0.08) 3px),
        repeating-linear-gradient(90deg, transparent, transparent 3px, rgba(90,138,197,0.06) 3px, rgba(90,138,197,0.06) 4px),
        repeating-linear-gradient(135deg, transparent, transparent 4px, rgba(44,74,110,0.15) 4px, rgba(44,74,110,0.15) 5px);
      color: var(--text-primary);
      padding: 2rem 1rem;
    }}

    .container {{ max-width: 1000px; margin: 0 auto; }}

    /* ── Header ──────────────────────────── */
    .header {{ text-align: center; margin-bottom: 1.5rem; }}
    .header a {{ color: var(--tux-orange); text-decoration: none; font-size: 0.85rem; }}
    .header a:hover {{ text-decoration: underline; }}
    h1 {{ font-size: 1.6rem; letter-spacing: 0.1em; color: var(--thread-white); margin-bottom: 0.5rem; }}
    .badge {{
      display: inline-block; padding: 0.3rem 0.8rem; border-radius: 4px;
      background: var(--badge-color); color: #fff; font-weight: 700;
      font-size: 0.9rem; margin: 0.5rem 0 1rem;
    }}

    /* ── Stitch divider ──────────────────── */
    .stitch {{
      border: none; height: 2px; margin: 1.5rem 0;
      background: repeating-linear-gradient(90deg, var(--thread-gold) 0px, var(--thread-gold) 8px, transparent 8px, transparent 14px);
      opacity: 0.6;
    }}

    /* ── Summary cards ───────────────────── */
    .summary-grid {{
      display: flex; gap: 1rem; flex-wrap: wrap;
      justify-content: center; margin: 1.5rem 0;
    }}
    .summary-card {{
      flex: 0 1 140px; padding: 1rem; text-align: center;
      background: var(--card-bg); border: 1px dashed var(--card-border);
      border-radius: 4px;
    }}
    .summary-card.summary-alert {{ border-color: #e05d44; }}
    .summary-value {{ font-size: 1.8rem; font-weight: 700; color: var(--thread-white); }}
    .summary-dim {{ font-size: 1rem; color: var(--text-muted); }}
    .summary-label {{ font-size: 0.7rem; color: var(--text-muted); margin-top: 0.3rem; text-transform: uppercase; letter-spacing: 0.08em; }}

    /* ── Section headings ────────────────── */
    .section-head {{
      font-size: 1rem; color: var(--tux-orange); margin: 1.5rem 0 0.5rem;
      cursor: pointer; user-select: none;
    }}
    .section-head::before {{ content: '▾ '; opacity: 0.5; }}
    .section-head.collapsed::before {{ content: '▸ '; }}
    .section-count {{ color: var(--text-muted); font-weight: 400; }}

    /* ── Table ───────────────────────────── */
    .table-wrap {{ overflow-x: auto; }}
    table {{ width: 100%; border-collapse: collapse; font-size: 0.8rem; }}
    thead th {{
      background: var(--denim-mid); color: var(--thread-white);
      padding: 0.5rem 0.6rem; text-align: left;
      border-bottom: 2px dashed var(--thread-gold);
      position: sticky; top: 0; white-space: nowrap;
    }}
    .th-status {{ width: 1.5rem; }}
    .th-score {{ width: 4.5rem; }}
    .th-cc {{ width: 3rem; }}
    .th-cov {{ width: 12rem; }}
    .th-fn {{ }}
    .th-loc {{ width: 4rem; }}

    tbody tr {{ border-bottom: 1px solid rgba(90,138,197,0.12); }}
    tbody tr:hover {{ background: rgba(90,138,197,0.08); }}
    td {{ padding: 0.35rem 0.6rem; vertical-align: middle; }}

    /* Status icons */
    .status-fail {{ color: #e05d44; font-weight: 700; text-align: center; }}
    .status-warn {{ color: var(--thread-gold); text-align: center; }}
    .status-pass {{ color: #4c1; text-align: center; }}

    /* Score coloring */
    .score-critical {{ color: #e05d44; font-weight: 700; }}
    .score-fail {{ color: var(--tux-orange); font-weight: 700; }}
    .score-warn {{ color: var(--thread-gold); }}

    /* Columns */
    .col-score {{ text-align: right; font-variant-numeric: tabular-nums; }}
    .col-cc {{ text-align: right; color: var(--text-muted); font-variant-numeric: tabular-nums; }}
    .col-cov {{ white-space: nowrap; }}
    .col-fn {{ }}
    .col-loc {{ color: var(--text-muted); font-variant-numeric: tabular-nums; }}

    /* Coverage bar */
    .cov-filled {{ letter-spacing: -1px; }}
    .cov-empty {{ letter-spacing: -1px; opacity: 0.3; }}
    .cov-pct {{ font-size: 0.75rem; margin-left: 0.3rem; }}
    .cov-na {{ color: var(--text-muted); }}
    .cov-high {{ color: #4c1; }}
    .cov-mid {{ color: var(--thread-gold); }}
    .cov-low {{ color: #e05d44; }}

    /* File group separator */
    .file-separator td {{
      padding: 0.6rem 0.6rem 0.2rem;
      font-size: 0.75rem; color: var(--denim-light);
      border-bottom: 1px solid rgba(90,138,197,0.2);
      letter-spacing: 0.03em;
    }}
    .file-path::before {{ content: '📁 '; }}

    code {{
      background: rgba(90,138,197,0.12); padding: 0.1rem 0.3rem;
      border-radius: 3px; font-size: 0.85em;
    }}

    /* ── Footer ──────────────────────────── */
    .footer {{
      text-align: center; margin-top: 2rem;
      font-size: 0.7rem; color: var(--text-muted); opacity: 0.5;
    }}
    .footer a {{ color: var(--text-muted); text-decoration: none; }}
    .footer a:hover {{ color: var(--tux-orange); }}

    @media (max-width: 640px) {{
      table {{ font-size: 0.7rem; }}
      td, th {{ padding: 0.25rem 0.3rem; }}
      .summary-card {{ flex: 0 1 100px; padding: 0.7rem; }}
      .summary-value {{ font-size: 1.3rem; }}
    }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>crap report</h1>
      <div class="badge">{html.escape(badge['message'])}</div>
      <br>
      <a href="../">&#8592; back to remendo docs</a>
    </div>

    <hr class="stitch">

    {summary}

    <hr class="stitch">

    {table_failing}
    {table_warning}
    {table_passing}

    <hr class="stitch">

    <div class="footer">
      Generated by <a href="https://github.com/minikin/cargo-crap">cargo-crap</a>
      | threshold: {THRESHOLD}
      | <a href="../crap-badge.json">badge JSON</a>
    </div>
  </div>

  <script>
    // Toggle table sections
    document.querySelectorAll('.section-head').forEach(h => {{
      h.addEventListener('click', () => {{
        const wrap = h.nextElementSibling;
        if (wrap && wrap.classList.contains('table-wrap')) {{
          const hidden = wrap.style.display === 'none';
          wrap.style.display = hidden ? '' : 'none';
          h.classList.toggle('collapsed', !hidden);
        }}
      }});
    }});
    // Start with passing section collapsed
    const passingHead = document.querySelector('#passing .section-head');
    if (passingHead) {{
      passingHead.classList.add('collapsed');
    }}
  </script>
</body>
</html>"""


def main():
    if len(sys.argv) != 4:
        print(f"Usage: {sys.argv[0]} <crap.json> <badge.json> <report.html>", file=sys.stderr)
        sys.exit(1)

    crap_json_path, badge_out, html_out = sys.argv[1:4]

    with open(crap_json_path) as f:
        data = json.load(f)

    entries = data.get("entries", [])
    badge = make_badge(entries)

    with open(badge_out, "w") as f:
        json.dump(badge, f, indent=2)

    html_content = build_html(badge, entries)
    with open(html_out, "w") as f:
        f.write(html_content)

    total = len(entries)
    failing = sum(1 for e in entries if e["crap"] > THRESHOLD)
    print(f"    {total - failing}/{total} functions passing (threshold {THRESHOLD})")


if __name__ == "__main__":
    main()
