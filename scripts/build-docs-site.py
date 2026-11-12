#!/usr/bin/env python3
"""Build a static searchable docs hub from the repository Markdown.

Requires Python 3.9+ and mistune 3.x. The generated `docs-site/` folder is
self-contained except for images, which are linked from the GitHub raw host.
"""
from __future__ import annotations

import html
import json
import re
from datetime import datetime, timezone
from pathlib import Path

import mistune

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs-site"
REPO = "https://github.com/ansarbuilds/saywhen"
RAW = "https://raw.githubusercontent.com/ansarbuilds/saywhen/main"

GROUPS = {
    "overview": "Start here",
    "faq": "Start here",
    "install-with-ai": "Start here",
    "troubleshooting": "Start here",
    "how-it-works": "Using saywhen",
    "changelog": "Using saywhen",
    "architecture": "Engineering",
    "how-we-built-it": "Engineering",
    "measurements": "Engineering",
    "risks": "Engineering",
    "roadmap": "Engineering",
    "prior-art": "Engineering",
    "idea-log": "Engineering",
    "vision": "Project",
    "contributing": "Project",
    "security": "Project",
    "code-of-conduct": "Project",
    "launch-checklist": "Project",
    "launch-template": "Project",
}

TITLES = {
    "overview": "What is saywhen",
}

DESCRIPTIONS = {
    "overview": "What saywhen is, how it feels to use, and where to start.",
    "faq": "Short answers about privacy, offline use, platforms and the preview.",
    "install-with-ai": "A step-by-step install guide written for a coding assistant.",
    "troubleshooting": "Symptom-first fixes for permissions, camera stalls, typing and model downloads.",
    "how-it-works": "The camera, microphone, speech model and preview in plain language.",
    "changelog": "What changed in each beta and what is still unreleased.",
    "architecture": "The Rust and TypeScript components, state machine and data flow.",
    "how-we-built-it": "How the prototype became a desktop app, and what was measured.",
    "measurements": "The numbers, the test corpus and the caveats.",
    "risks": "Known failure modes and the assumptions behind them.",
    "roadmap": "Implementation history and the remaining stages.",
    "prior-art": "The tools and research that came before saywhen.",
    "idea-log": "Early notes and decisions from the prototype.",
    "vision": "The problem, the idea and the seven-second demo.",
    "contributing": "How to propose, build and test a change.",
    "security": "How to report a security or privacy issue.",
    "code-of-conduct": "The standards for the project community.",
    "launch-checklist": "What has been verified, and what has not.",
    "launch-template": "Draft copy for launch day.",
}

SLUGS = {
    "README": "overview",
    "FAQ": "faq",
    "INSTALL_WITH_AI": "install-with-ai",
    "TROUBLESHOOTING": "troubleshooting",
    "HOW_IT_WORKS": "how-it-works",
    "CHANGELOG": "changelog",
    "ARCHITECTURE": "architecture",
    "HOW_WE_BUILT_IT": "how-we-built-it",
    "MEASUREMENTS": "measurements",
    "RISKS": "risks",
    "ROADMAP": "roadmap",
    "PRIOR_ART": "prior-art",
    "IDEA_LOG": "idea-log",
    "VISION": "vision",
    "CONTRIBUTING": "contributing",
    "SECURITY": "security",
    "CODE_OF_CONDUCT": "code-of-conduct",
    "LAUNCH_CHECKLIST": "launch-checklist",
    "LAUNCH_TEMPLATE": "launch-template",
}

FILES = [
    ("overview", ROOT / "README.md"),
    ("faq", ROOT / "docs" / "FAQ.md"),
    ("install-with-ai", ROOT / "INSTALL_WITH_AI.md"),
    ("troubleshooting", ROOT / "docs" / "TROUBLESHOOTING.md"),
    ("how-it-works", ROOT / "docs" / "HOW_IT_WORKS.md"),
    ("changelog", ROOT / "docs" / "CHANGELOG.md"),
    ("architecture", ROOT / "docs" / "ARCHITECTURE.md"),
    ("how-we-built-it", ROOT / "docs" / "HOW_WE_BUILT_IT.md"),
    ("measurements", ROOT / "docs" / "MEASUREMENTS.md"),
    ("risks", ROOT / "docs" / "RISKS.md"),
    ("roadmap", ROOT / "docs" / "ROADMAP.md"),
    ("prior-art", ROOT / "docs" / "PRIOR_ART.md"),
    ("idea-log", ROOT / "docs" / "IDEA_LOG.md"),
    ("vision", ROOT / "docs" / "VISION.md"),
    ("contributing", ROOT / "CONTRIBUTING.md"),
    ("security", ROOT / "SECURITY.md"),
    ("code-of-conduct", ROOT / "CODE_OF_CONDUCT.md"),
    ("launch-checklist", ROOT / "docs" / "LAUNCH_CHECKLIST.md"),
    ("launch-template", ROOT / "docs" / "LAUNCH_TEMPLATE.md"),
]

MARKDOWN = mistune.create_markdown(escape=False, plugins=["table", "strikethrough", "task_lists"])


def slugify(text: str) -> str:
    text = re.sub(r"<[^>]+>", "", text)
    text = re.sub(r"[`*_~]", "", text)
    text = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", text)
    text = html.unescape(text).lower()
    text = re.sub(r"[^\w\s-]", "", text)
    return re.sub(r"[\s-]+", "-", text).strip("-")


def first_heading(text: str) -> str:
    for line in text.splitlines():
        match = re.match(r"^#\s+(.+?)\s*$", line)
        if match:
            return re.sub(r"[`*_]", "", match.group(1)).strip()
    return "saywhen docs"


def first_paragraph(text: str) -> str:
    for block in re.split(r"\n\s*\n", text):
        block = block.strip()
        if not block or block.startswith(("#", "!", "|", "```", ">", "-", "*")):
            continue
        clean = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", block)
        clean = re.sub(r"[`*_>#]", "", clean)
        clean = " ".join(clean.split())
        if len(clean) > 20:
            return clean[:180].rstrip() + ("..." if len(clean) > 180 else "")
    return ""


def normalize_repo_path(path: str, source_rel_dir: str) -> str:
    combined = f"{source_rel_dir}/{path}" if source_rel_dir else path
    parts: list[str] = []
    for part in combined.split("/"):
        if not part or part == ".":
            continue
        if part == "..":
            if parts:
                parts.pop()
            continue
        parts.append(part)
    return "/".join(parts)


MEDIA_EXTENSIONS = (".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".mp4", ".webm")


def rewrite_target(target: str, current_slug: str, source_rel_dir: str = "") -> str:
    if not target or target.startswith(("http://", "https://", "mailto:", "#", "data:")):
        return target
    path, _, anchor = target.partition("#")
    suffix = f"#{anchor}" if anchor else ""
    if path.endswith(".html") and path in {f"{slug}.html" for slug in SLUGS.values()}:
        return target
    repo_rel = normalize_repo_path(path, source_rel_dir)
    if repo_rel.endswith(".md"):
        stem = Path(repo_rel).name[:-3]
        slug = SLUGS.get(stem)
        if slug:
            return f"{slug}.html{suffix}"
        return f"{REPO}/blob/main/{repo_rel}{suffix}"
    if repo_rel.lower().endswith(MEDIA_EXTENSIONS) or repo_rel.startswith(
        ("docs/visuals/", "docs/screenshots/", "public/")
    ):
        return f"{RAW}/{repo_rel}"
    if (
        repo_rel.startswith(("src/", "src-tauri/", "scripts/", "docs/", "film/"))
        or repo_rel.endswith((".rs", ".ts", ".css", ".toml", ".json", ".py", ".sh", ".mjs"))
    ):
        return f"{REPO}/blob/main/{repo_rel}{suffix}"
    return target


def rewrite_markdown(text: str, slug: str, source_rel_dir: str = "") -> str:
    text = re.sub(
        r"(!?)\[([^\]]*)\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)",
        lambda m: (
            f"![{m.group(2)}]({rewrite_target(m.group(3), slug, source_rel_dir)})"
            if m.group(1)
            else f"[{m.group(2)}]({rewrite_target(m.group(3), slug, source_rel_dir)})"
        ),
        text,
    )
    text = re.sub(
        r'(<img[^>]+src=")([^"]+)(")',
        lambda m: m.group(1) + rewrite_target(m.group(2), slug, source_rel_dir) + m.group(3),
        text,
    )
    text = re.sub(
        r'(<a[^>]+href=")([^"]+)(")',
        lambda m: m.group(1) + rewrite_target(m.group(2), slug, source_rel_dir) + m.group(3),
        text,
    )
    return text


def add_heading_ids(body: str):
    toc = []
    seen: dict[str, int] = {}

    def repl(match: re.Match[str]) -> str:
        level, inner = match.group(1), match.group(2)
        plain = re.sub(r"<[^>]+>", "", inner)
        base = slugify(plain)
        count = seen.get(base, 0)
        seen[base] = count + 1
        anchor = base if count == 0 else f"{base}-{count}"
        if level in ("2", "3"):
            toc.append((level, anchor, plain))
        return f'<h{level} id="{anchor}">{inner}</h{level}>'

    body = re.sub(r"<h([1-6])>(.*?)</h\1>", repl, body, flags=re.S)
    return body, toc


def plain_text(body: str) -> str:
    text = re.sub(r"<script.*?</script>", " ", body, flags=re.S)
    text = re.sub(r"<style.*?</style>", " ", text, flags=re.S)
    text = re.sub(r"<[^>]+>", " ", text)
    text = html.unescape(text)
    return re.sub(r"\s+", " ", text).strip()


def page_shell(title: str, slug: str, content: str, toc, prev_next, docs, source: str) -> str:
    nav_groups: dict[str, list[dict]] = {}
    for doc in docs:
        nav_groups.setdefault(doc["group"], []).append(doc)
    nav_html = []
    for group, items in nav_groups.items():
        nav_html.append(f'<div class="nav-group"><span>{html.escape(group)}</span>')
        for item in items:
            active = " is-active" if item["slug"] == slug else ""
            nav_html.append(
                f'<a class="nav-link{active}" href="{item["slug"]}.html">{html.escape(item["title"])}</a>'
            )
        nav_html.append("</div>")

    toc_html = ""
    if toc:
        toc_items = "".join(
            f'<a class="toc-{level}" href="#{anchor}">{html.escape(plain)}</a>'
            for level, anchor, plain in toc
        )
        toc_html = f'<aside class="toc"><span class="eyebrow">On this page</span>{toc_items}</aside>'

    pager = ""
    if prev_next:
        prev_doc, next_doc = prev_next
        links = []
        if prev_doc:
            links.append(
                f'<a class="pager-link" href="{prev_doc["slug"]}.html"><span>Previous</span><strong>{html.escape(prev_doc["title"])}</strong></a>'
            )
        if next_doc:
            links.append(
                f'<a class="pager-link pager-next" href="{next_doc["slug"]}.html"><span>Next</span><strong>{html.escape(next_doc["title"])}</strong></a>'
            )
        pager = f'<nav class="pager">{"".join(links)}</nav>'

    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="light dark">
  <title>{html.escape(title)} · saywhen docs</title>
  <link rel="stylesheet" href="docs.css">
</head>
<body>
  <header class="topbar">
    <a class="brand" href="index.html"><span class="brand-mark">S</span><span>saywhen <em>docs</em></span></a>
    <div class="top-actions">
      <label class="search"><span>Search</span><input id="docs-search" type="search" placeholder="Search the docs" autocomplete="off" spellcheck="false"></label>
      <a class="top-link" href="https://ansarbuilds.dev/p/saywhen/">Download</a>
    </div>
  </header>
  <div class="search-panel" id="search-panel" hidden></div>
  <div class="layout">
    <aside class="sidebar">
      {''.join(nav_html)}
      <div class="nav-group nav-meta">
        <span>More</span>
        <a class="nav-link" href="https://github.com/ansarbuilds/saywhen">GitHub</a>
        <a class="nav-link" href="https://ansarbuilds.dev/p/saywhen/">Project page</a>
      </div>
    </aside>
    <main class="content">
      <div class="breadcrumb"><a href="index.html">Docs</a><span>/</span><span>{html.escape(GROUPS.get(slug, 'Docs'))}</span></div>
      <article class="prose">
        {content}
      </article>
      {pager}
      <footer class="page-footer"><a href="{REPO}/blob/main/{source}">Edit this page on GitHub</a></footer>
    </main>
    {toc_html}
  </div>
  <script src="docs.js"></script>
</body>
</html>
"""


def index_shell(docs) -> str:
    groups: dict[str, list[dict]] = {}
    for doc in docs:
        groups.setdefault(doc["group"], []).append(doc)
    cards = []
    for group, items in groups.items():
        cards.append(f'<section class="doc-group"><h2>{html.escape(group)}</h2><div class="doc-grid">')
        for item in items:
            cards.append(
                f'<a class="doc-card" href="{item["slug"]}.html"><span class="doc-group-label">{html.escape(group)}</span>'
                f'<strong>{html.escape(item["title"])}</strong><span>{html.escape(item["description"])}</span></a>'
            )
        cards.append("</div></section>")
    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="light dark">
  <title>saywhen docs</title>
  <link rel="stylesheet" href="docs.css">
</head>
<body class="index-page">
  <header class="topbar">
    <a class="brand" href="index.html"><span class="brand-mark">S</span><span>saywhen <em>docs</em></span></a>
    <div class="top-actions">
      <label class="search"><span>Search</span><input id="docs-search" type="search" placeholder="Search the docs" autocomplete="off" spellcheck="false"></label>
      <a class="top-link" href="https://ansarbuilds.dev/p/saywhen/">Download</a>
    </div>
  </header>
  <div class="search-panel" id="search-panel" hidden></div>
  <main class="landing">
    <section class="hero">
      <span class="eyebrow">saywhen documentation</span>
      <h1>Look. Talk. <em>Read.</em></h1>
      <p>Everything about installing, using and trusting saywhen. Search the whole set, or start with the short answers.</p>
      <div class="hero-actions"><a class="button primary" href="faq.html">Read the FAQ</a><a class="button" href="troubleshooting.html">Fix a problem</a></div>
    </section>
    {''.join(cards)}
    <footer class="landing-footer"><span>saywhen is MIT licensed and local-first.</span><a href="https://github.com/ansarbuilds/saywhen">GitHub</a></footer>
  </main>
  <script src="docs.js"></script>
</body>
</html>
"""


CSS = r"""
:root {
  --paper: #f7f4f7;
  --surface: #ffffff;
  --ink: #2a0823;
  --muted: #6f5f6b;
  --faint: #8b7b86;
  --accent: #4a0b3e;
  --amber: #fbb303;
  --rule: rgba(42, 8, 35, 0.12);
  --rule-strong: rgba(42, 8, 35, 0.2);
  --serif: "Instrument Serif", "Iowan Old Style", Georgia, serif;
  --sans: "Plus Jakarta Sans", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --mono: "JetBrains Mono", ui-monospace, "SF Mono", Menlo, monospace;
}
@media (prefers-color-scheme: dark) {
  :root {
    --paper: #141016;
    --surface: #211a24;
    --ink: #f4eef2;
    --muted: #c3b8c1;
    --faint: #a79aa4;
    --accent: #f0b6df;
    --amber: #fbb303;
    --rule: rgba(255,255,255,0.12);
    --rule-strong: rgba(255,255,255,0.22);
  }
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; }
body {
  margin: 0;
  background: radial-gradient(900px 500px at 80% -10%, rgba(251,179,3,0.12), transparent 70%), var(--paper);
  color: var(--ink);
  font: 16px/1.65 var(--sans);
}
a { color: var(--accent); text-decoration-thickness: 1px; text-underline-offset: 2px; }
.topbar {
  position: sticky; top: 0; z-index: 20;
  display: flex; align-items: center; justify-content: space-between; gap: 20px;
  padding: 14px 28px; border-bottom: 1px solid var(--rule);
  background: color-mix(in srgb, var(--paper) 88%, transparent);
  backdrop-filter: blur(14px);
}
.brand { display: flex; align-items: center; gap: 10px; color: var(--ink); text-decoration: none; font-weight: 700; letter-spacing: -0.03em; }
.brand em { font-family: var(--serif); font-style: italic; font-weight: 400; }
.brand-mark { display: grid; place-items: center; width: 30px; height: 30px; border-radius: 9px; background: linear-gradient(145deg, #4a0b3e, #22061c); color: var(--amber); font: 700 15px/1 var(--sans); }
.top-actions { display: flex; align-items: center; gap: 18px; }
.top-link { color: var(--muted); text-decoration: none; font-size: 13px; font-weight: 650; }
.search { display: flex; align-items: center; gap: 8px; padding: 7px 12px; border: 1px solid var(--rule-strong); border-radius: 10px; background: var(--surface); }
.search span { color: var(--faint); font-size: 12px; }
.search input { width: min(280px, 34vw); border: 0; outline: 0; background: transparent; color: var(--ink); font: inherit; font-size: 13px; }
.search-panel {
  position: fixed; z-index: 30; top: 64px; right: 28px; width: min(520px, calc(100vw - 40px));
  max-height: 60vh; overflow: auto; padding: 8px; border: 1px solid var(--rule-strong); border-radius: 14px;
  background: var(--surface); box-shadow: 0 24px 60px -20px rgba(20,2,16,0.45);
}
.search-result { display: block; padding: 10px 12px; border-radius: 9px; color: var(--ink); text-decoration: none; }
.search-result:hover { background: color-mix(in srgb, var(--accent) 8%, transparent); }
.search-result strong { display: block; font-size: 13px; }
.search-result span { display: block; color: var(--muted); font-size: 12px; line-height: 1.45; }
.search-empty { padding: 14px 12px; color: var(--muted); font-size: 13px; }
.layout { display: grid; grid-template-columns: 220px minmax(0, 760px) 220px; gap: 40px; max-width: 1280px; margin: 0 auto; padding: 36px 28px 80px; align-items: start; }
.sidebar { position: sticky; top: 84px; display: flex; flex-direction: column; gap: 22px; }
.nav-group { display: flex; flex-direction: column; gap: 2px; }
.nav-group > span { padding: 0 10px 6px; color: var(--faint); font-size: 10px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; }
.nav-link { padding: 6px 10px; border-radius: 8px; color: var(--muted); text-decoration: none; font-size: 13px; }
.nav-link:hover { color: var(--ink); background: color-mix(in srgb, var(--ink) 5%, transparent); }
.nav-link.is-active { color: var(--accent); background: color-mix(in srgb, var(--accent) 8%, transparent); font-weight: 650; }
.nav-meta { border-top: 1px solid var(--rule); padding-top: 16px; }
.content { min-width: 0; }
.breadcrumb { display: flex; gap: 8px; margin-bottom: 18px; color: var(--faint); font-size: 12px; }
.breadcrumb a { color: var(--muted); text-decoration: none; }
.prose h1 { margin: 0 0 10px; font: 400 clamp(38px, 5vw, 58px)/1.02 var(--serif); letter-spacing: -0.02em; }
.prose h2 { margin: 42px 0 12px; font: 700 22px/1.2 var(--sans); letter-spacing: -0.03em; }
.prose h3 { margin: 30px 0 8px; font: 700 17px/1.25 var(--sans); }
.prose h4 { margin: 24px 0 6px; font: 700 15px/1.3 var(--sans); }
.prose p, .prose li { color: var(--ink); }
.prose p { margin: 0 0 16px; }
.prose ul, .prose ol { margin: 0 0 16px; padding-left: 22px; }
.prose li { margin: 4px 0; }
.prose blockquote { margin: 20px 0; padding: 12px 18px; border-left: 3px solid var(--amber); background: color-mix(in srgb, var(--amber) 10%, transparent); border-radius: 0 10px 10px 0; color: var(--muted); }
.prose code { padding: 2px 5px; border-radius: 5px; background: color-mix(in srgb, var(--ink) 7%, transparent); font: 0.86em var(--mono); }
.prose pre { overflow: auto; padding: 16px 18px; border: 1px solid var(--rule); border-radius: 12px; background: #22061c; color: #f4eef2; }
.prose pre code { padding: 0; background: transparent; color: inherit; font-size: 13px; }
.prose table { width: 100%; border-collapse: collapse; margin: 18px 0; font-size: 14px; }
.prose th, .prose td { padding: 9px 12px; border: 1px solid var(--rule); text-align: left; vertical-align: top; }
.prose th { background: color-mix(in srgb, var(--ink) 5%, transparent); font-size: 12px; text-transform: uppercase; letter-spacing: 0.04em; }
.prose img { max-width: 100%; height: auto; border-radius: 12px; border: 1px solid var(--rule); }
.prose hr { margin: 34px 0; border: 0; border-top: 1px solid var(--rule); }
.prose a { overflow-wrap: anywhere; }
.toc { position: sticky; top: 84px; display: flex; flex-direction: column; gap: 3px; font-size: 12px; }
.toc .eyebrow { margin-bottom: 6px; color: var(--faint); font-size: 10px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; }
.toc a { padding: 3px 0; color: var(--muted); text-decoration: none; }
.toc a:hover { color: var(--accent); }
.toc .toc-3 { padding-left: 12px; color: var(--faint); }
.pager { display: flex; justify-content: space-between; gap: 16px; margin-top: 46px; padding-top: 20px; border-top: 1px solid var(--rule); }
.pager-link { display: flex; flex-direction: column; gap: 2px; max-width: 48%; color: var(--muted); text-decoration: none; }
.pager-link span { font-size: 10px; text-transform: uppercase; letter-spacing: 0.09em; color: var(--faint); }
.pager-link strong { color: var(--ink); font-size: 13px; }
.pager-next { text-align: right; margin-left: auto; }
.page-footer { margin-top: 26px; font-size: 12px; }
.page-footer a { color: var(--faint); }
.landing { max-width: 1100px; margin: 0 auto; padding: 64px 28px 80px; }
.hero { max-width: 760px; margin-bottom: 56px; }
.eyebrow { color: var(--faint); font-size: 11px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase; }
.hero h1 { margin: 12px 0 14px; font: 400 clamp(52px, 8vw, 96px)/0.98 var(--serif); letter-spacing: -0.03em; }
.hero h1 em { color: #b0457a; }
.hero p { max-width: 60ch; color: var(--muted); font-size: 17px; }
.hero-actions { display: flex; gap: 10px; margin-top: 24px; }
.button { display: inline-flex; align-items: center; height: 42px; padding: 0 18px; border: 1px solid var(--rule-strong); border-radius: 999px; color: var(--ink); text-decoration: none; font-size: 13px; font-weight: 650; background: var(--surface); }
.button.primary { background: var(--accent); border-color: var(--accent); color: var(--paper); }
.doc-group { margin-top: 44px; }
.doc-group h2 { margin: 0 0 14px; font: 700 14px/1 var(--sans); letter-spacing: 0.08em; text-transform: uppercase; color: var(--faint); }
.doc-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 14px; }
.doc-card { display: flex; flex-direction: column; gap: 7px; padding: 20px; border: 1px solid var(--rule); border-radius: 16px; background: var(--surface); color: var(--ink); text-decoration: none; transition: transform 0.2s ease, border-color 0.2s ease; }
.doc-card:hover { transform: translateY(-2px); border-color: var(--rule-strong); }
.doc-card strong { font-size: 16px; letter-spacing: -0.02em; }
.doc-card > span:last-child { color: var(--muted); font-size: 12.5px; line-height: 1.5; }
.doc-group-label { color: var(--faint); font-size: 9px; font-weight: 700; letter-spacing: 0.1em; text-transform: uppercase; }
.landing-footer { display: flex; justify-content: space-between; gap: 16px; margin-top: 64px; padding-top: 20px; border-top: 1px solid var(--rule); color: var(--faint); font-size: 12px; }
.landing-footer a { color: var(--muted); }
@media (max-width: 1100px) {
  .layout { grid-template-columns: 200px minmax(0, 1fr); }
  .toc { display: none; }
}
@media (max-width: 800px) {
  .topbar { padding: 12px 16px; }
  .top-link, .search span { display: none; }
  .search input { width: 44vw; }
  .layout { display: block; padding: 22px 16px 60px; }
  .sidebar { position: static; flex-direction: column; flex-wrap: nowrap; gap: 14px; width: 100%; margin-bottom: 26px; }
  .nav-group { width: 100%; min-width: 0; flex-direction: row; flex-wrap: wrap; }
  .nav-group > span { width: 100%; }
  .nav-link { max-width: 100%; white-space: normal; border: 1px solid var(--rule); }
  .landing { padding: 40px 16px 60px; }
  .doc-grid { grid-template-columns: 1fr; }
  .search-panel { right: 16px; top: 60px; }
}
"""

JS = r"""
(() => {
  const input = document.getElementById('docs-search');
  const panel = document.getElementById('search-panel');
  if (!input || !panel) return;
  let docs = [];
  let loading = null;
  const load = async () => {
    if (docs.length > 0) return;
    if (!loading) {
      loading = fetch('search.json')
        .then((response) => response.json())
        .catch(() => [])
        .finally(() => { loading = null; });
    }
    docs = await loading;
  };
  const escapeHtml = (value) => String(value).replace(/[&<>"']/g, (char) => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[char]));
  const search = () => {
    const query = input.value.trim().toLowerCase();
    if (!query) { panel.hidden = true; panel.innerHTML = ''; return; }
    const results = docs.filter((doc) => doc.text.toLowerCase().includes(query)).slice(0, 8);
    panel.hidden = false;
    if (!results.length) {
      panel.innerHTML = '<div class="search-empty">No page matches that search.</div>';
      return;
    }
    panel.innerHTML = results.map((doc) => {
      const index = doc.text.toLowerCase().indexOf(query);
      let start = Math.max(0, index - 55);
      while (start > 0 && !/\s/.test(doc.text[start - 1])) start -= 1;
      let end = Math.min(doc.text.length, index + query.length + 110);
      while (end < doc.text.length && !/\s/.test(doc.text[end])) end += 1;
      const snippet = (start > 0 ? '... ' : '') + doc.text.slice(start, end).trim() + (end < doc.text.length ? ' ...' : '');
      return `<a class="search-result" href="${doc.slug}.html"><strong>${escapeHtml(doc.title)}</strong><span>${escapeHtml(doc.group)} · ${escapeHtml(snippet)}</span></a>`;
    }).join('');
  };
  input.addEventListener('focus', load);
  input.addEventListener('input', async () => { await load(); search(); });
  input.addEventListener('keydown', (event) => { if (event.key === 'Escape') { input.value = ''; search(); input.blur(); } });
  document.addEventListener('keydown', (event) => {
    if (event.key === '/' && document.activeElement !== input) { event.preventDefault(); input.focus(); }
  });
})();
"""


def write_site():
    OUT.mkdir(exist_ok=True)
    docs = []
    rendered = {}
    for slug, path in FILES:
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8")
        title = TITLES.get(slug) or first_heading(text)
        description = DESCRIPTIONS.get(slug) or first_paragraph(text)
        rel_dir = str(path.relative_to(ROOT).parent).replace("\\", "/")
        source_rel_dir = "" if rel_dir == "." else rel_dir
        rewritten = rewrite_markdown(text, slug, source_rel_dir)
        body = MARKDOWN(rewritten)
        body, toc = add_heading_ids(body)
        rendered[slug] = (body, toc)
        docs.append({
            "slug": slug,
            "order": len(docs),
            "title": title,
            "group": GROUPS.get(slug, "Docs"),
            "description": description,
            "path": str(path.relative_to(ROOT)),
        })
    docs.sort(key=lambda doc: (list(GROUPS.values()).index(doc["group"]) if doc["group"] in GROUPS.values() else 99, doc["order"]))
    search = []
    for doc in docs:
        body, _ = rendered[doc["slug"]]
        search.append({
            "slug": doc["slug"],
            "title": doc["title"],
            "group": doc["group"],
            "text": plain_text(body)[:6000],
        })
    for index, doc in enumerate(docs):
        prev_doc = docs[index - 1] if index > 0 else None
        next_doc = docs[index + 1] if index + 1 < len(docs) else None
        body, toc = rendered[doc["slug"]]
        page = page_shell(doc["title"], doc["slug"], body, toc, (prev_doc, next_doc), docs, doc["path"])
        (OUT / f"{doc['slug']}.html").write_text(page, encoding="utf-8")
    (OUT / "index.html").write_text(index_shell(docs), encoding="utf-8")
    (OUT / "docs.css").write_text(CSS, encoding="utf-8")
    (OUT / "docs.js").write_text(JS, encoding="utf-8")
    (OUT / "search.json").write_text(json.dumps(search, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    stamp = datetime.now(timezone.utc).strftime("%Y-%m-%d")
    (OUT / "build.txt").write_text(f"Built {stamp} from {len(docs)} Markdown files.\n", encoding="utf-8")
    print(f"Built {len(docs)} pages in {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    write_site()
