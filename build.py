#!/usr/bin/env python3
"""
Markdown-to-HTML static site builder for the Learn Rust & egui course.

Compiles:
  - `index.md` -> `index.html`
  - `lessons/*.md` -> `lessons/*.html`
  - `reference/*.md` -> `reference/*.html` (with `GLOSSARY.md` -> `glossary.html`)

Usage:
  # Build HTML in-place at the repository root for local previewing:
  .venv/bin/python build.py --check

  # Build a standalone `_site/` directory for GitHub Pages deployment:
  .venv/bin/python build.py --out-dir _site --check
"""

from __future__ import annotations

import argparse
import html
import json
import re
import shutil
import sys
from pathlib import Path
from typing import Any

import yaml
from markdown_it import MarkdownIt

ROOT_DIR = Path(__file__).resolve().parent

REFERENCE_ORDER = [
    ("GLOSSARY.md", "glossary.html", "Glossary"),
    ("immediate-mode-cheatsheet.md", "immediate-mode-cheatsheet.html", "IMGUI Cheat Sheet"),
    ("testing-and-mutation-cheatsheet.md", "testing-and-mutation-cheatsheet.html", "Mutation Cheat Sheet"),
]

FRONTMATTER_RE = re.compile(r"\A---\r?\n(.*?)\r?\n---\r?\n", re.DOTALL)
FENCE_ATTR_RE = re.compile(r'(\w+)=(?:"([^"]*)"|(\S+))')
BLANK_TOKEN_RE = re.compile(r"\{\{blank:([\s\S]*?)\}\}")
SIDENOTE_TOKEN_RE = re.compile(r"\{\{sidenote:([\s\S]*?)\}\}")
CALLOUT_BLOCK_RE = re.compile(
    r"^:::(callout|teacher-prompt)(?:[ \t]+([^\n]+))?\n(.*?)^:::[ \t]*$",
    re.MULTILINE | re.DOTALL,
)
VERIFY_BLOCK_RE = re.compile(
    r"^:::verify[ \t]+([^\n]+)\n(.*?)^:::[ \t]*$",
    re.MULTILINE | re.DOTALL,
)
EXERCISE_BLOCK_RE = re.compile(
    r"^:::exercise[ \t]+([^\n]+)\n(.*?(?:^:::prompts[ \t]*\n.*?^:::[ \t]*\n.*?)?)^:::[ \t]*$",
    re.MULTILINE | re.DOTALL,
)
PROMPTS_BLOCK_RE = re.compile(
    r"^:::prompts[ \t]*\n(.*?)^:::[ \t]*$",
    re.MULTILINE | re.DOTALL,
)
QUIZ_BLOCK_RE = re.compile(
    r"^:::quiz[ \t]+([^\n]+)\n(.*?)^:::[ \t]*$",
    re.MULTILINE | re.DOTALL,
)
FENCED_CODE_RE = re.compile(
    r"^```([^\n]*)\n(.*?)^```[ \t]*$",
    re.MULTILINE | re.DOTALL,
)
CHECKLIST_ITEM_RE = re.compile(r"^- \[[ xX]\] (.*)$", re.MULTILINE)
TABLE_CAPTION_RE = re.compile(
    r"<p>Table:\s*(.*?)</p>\s*<table>",
    re.DOTALL,
)
HREF_MD_RE = re.compile(r'href="([^":#]+?)\.md(#[^"]*)?"')


def create_md_parser() -> MarkdownIt:
    """Create a MarkdownIt parser with GFM tables enabled."""
    return MarkdownIt("commonmark", {"html": True, "typographer": False}).enable("table")


MD = create_md_parser()


def render_inline_md(text: str) -> str:
    """Render inline Markdown (such as `code`, **bold**, *em*, links) without <p> tags."""
    rendered = MD.renderInline(str(text).strip())
    return rewrite_md_links(rendered)


def rewrite_md_links(html_text: str) -> str:
    """Rewrite relative `.md` links (including `GLOSSARY.md`) to `.html`."""

    def _replace(match: re.Match[str]) -> str:
        stem_path = match.group(1)
        anchor = match.group(2) or ""
        if stem_path.endswith("GLOSSARY"):
            stem_path = stem_path[: -len("GLOSSARY")] + "glossary"
        return f'href="{stem_path}.html{anchor}"'

    return HREF_MD_RE.sub(_replace, html_text)


def expand_sidenotes(md_text: str) -> str:
    """Expand `{{sidenote:...}}` into Tufte `<span class="sidenote-number"></span><span class="sidenote">...</span>`."""

    def _sub(match: re.Match[str]) -> str:
        inner = render_inline_md(match.group(1).strip())
        return f'<span class="sidenote-number"></span><span class="sidenote">{inner}</span>'

    return SIDENOTE_TOKEN_RE.sub(_sub, md_text)


def parse_kv_attrs(attr_str: str) -> dict[str, str]:
    """Parse `key="value" flag=false` attribute strings on directives or code fences."""
    attrs: dict[str, str] = {}
    for match in FENCE_ATTR_RE.finditer(attr_str):
        key = match.group(1)
        val = match.group(2) if match.group(2) is not None else match.group(3)
        attrs[key] = val
    return attrs


def parse_frontmatter(path: Path) -> tuple[dict[str, Any], str]:
    """Extract YAML frontmatter and Markdown body from a file."""
    raw = path.read_text(encoding="utf-8")
    match = FRONTMATTER_RE.match(raw)
    if not match:
        return {}, raw
    meta = yaml.safe_load(match.group(1)) or {}
    body = raw[match.end() :]
    return meta, body


def render_code_figure(
    info_line: str,
    code_body: str,
    *,
    ex_id: str | None = None,
    blanks_out: list[dict[str, Any]] | None = None,
) -> str:
    """Render a fenced code block or Mermaid diagram into course HTML."""
    stripped_info = info_line.strip()
    parts = stripped_info.split(None, 1)
    lang = parts[0] if parts else "text"
    attr_str = parts[1] if len(parts) > 1 else ""
    attrs = parse_kv_attrs(attr_str)

    if lang == "mermaid":
        caption_raw = attrs.get("caption", "")
        escaped_mermaid = html.escape(code_body.rstrip("\n"), quote=False)
        caption_html = (
            f"\n  <figcaption>{render_inline_md(caption_raw)}</figcaption>"
            if caption_raw
            else ""
        )
        return (
            f'<figure class="mermaid-figure">\n'
            f'  <pre class="mermaid">\n{escaped_mermaid}\n  </pre>{caption_html}\n'
            f"</figure>"
        )

    caption_raw = attrs.get("caption", attrs.get("title", ""))
    escaped_code = html.escape(code_body.rstrip("\n"), quote=False)

    pre_class = ""
    if ex_id is not None and blanks_out is not None:
        pre_class = ' class="keep-markup"'

        def _sub_blank(match: re.Match[str]) -> str:
            raw_spec = html.unescape(match.group(1))
            # Split from right so answers containing `|ui|` work cleanly
            b_parts = [p.strip() for p in raw_spec.rsplit("|", 3)]
            ans_raw = b_parts[0] if len(b_parts) > 0 else ""
            err_hint = b_parts[1] if len(b_parts) > 1 else "Check this expression."
            placeholder = b_parts[2] if len(b_parts) > 2 else "/* fill in */"
            width = b_parts[3] if len(b_parts) > 3 else "140px"

            blank_idx = len(blanks_out) + 1
            input_id = f"{ex_id}-blank-{blank_idx}"
            accepted = [a.strip() for a in re.split(r";;|(?<!;)\|\|", ans_raw) if a.strip()]
            blanks_out.append(
                {
                    "inputId": input_id,
                    "accepted": accepted,
                    "blankIndex": blank_idx,
                    "errorHint": render_inline_md(err_hint),
                }
            )
            style_attr = f' style="width:{html.escape(width, quote=True)};"' if width else ""
            return (
                f'<input id="{html.escape(input_id, quote=True)}" '
                f'class="code-challenge-input"{style_attr} '
                f'placeholder="{html.escape(placeholder, quote=True)}" '
                f'autocomplete="off">'
            )

        escaped_code = BLANK_TOKEN_RE.sub(_sub_blank, escaped_code)

    pre_html = (
        f'<pre{pre_class}><code class="language-{html.escape(lang)}">'
        f"{escaped_code}</code></pre>"
    )
    if caption_raw:
        return (
            f"<figure>\n"
            f"  <figcaption>{render_inline_md(caption_raw)}</figcaption>\n"
            f"  {pre_html}\n"
            f"</figure>"
        )
    return pre_html


def render_callout_block(kind: str, title_raw: str, body_md: str) -> str:
    """Render `:::callout` or `:::teacher-prompt`."""
    cls = "teacher-prompt-box" if kind == "teacher-prompt" else "callout"
    inner_md = expand_sidenotes(body_md.strip())
    if title_raw:
        inner_md = f"**{title_raw.strip()}** {inner_md}"
    rendered = rewrite_md_links(MD.renderInline(inner_md.replace("\n\n", "<br>")))
    return f'<div class="{cls}">\n  {rendered}\n</div>'


def render_verify_block(attr_line: str, inner_md: str) -> str:
    """Render `:::verify` into a full-width Cargo Verification & Mutation Coverage Gate."""
    attrs = parse_kv_attrs(attr_line)
    v_id = attrs.get("id", "verify")
    title_html = render_inline_md(attrs.get("title", "Cargo Verification Gate"))
    badge_html = html.escape(attrs.get("badge", "cargo test + cargo mutants"))
    tier_html = html.escape(attrs.get("tier", "Tier 2: Unit Tests"))
    caught_html = html.escape(attrs.get("caught", "100% Caught"))
    missed_html = html.escape(attrs.get("missed", "0 Missed"))

    code_blocks: list[str] = []

    def _stash_code(match: re.Match[str]) -> str:
        idx = len(code_blocks)
        code_blocks.append(render_code_figure(match.group(1), match.group(2)))
        return f"\n\n@@VERIFY_CODE_{idx}@@\n\n"

    working = FENCED_CODE_RE.sub(_stash_code, inner_md.strip())

    checklist_items: list[str] = []

    def _extract_check(match: re.Match[str]) -> str:
        idx = len(checklist_items) + 1
        item_id = f"{v_id}-{idx}"
        label_html = render_inline_md(match.group(1).strip())
        checklist_items.append(
            f"        <li>\n"
            f'          <input type="checkbox" id="{html.escape(item_id, quote=True)}">\n'
            f'          <label for="{html.escape(item_id, quote=True)}">\n'
            f"            {label_html}\n"
            f"          </label>\n"
            f"        </li>"
        )
        return ""

    prose_md = CHECKLIST_ITEM_RE.sub(_extract_check, working).strip()
    prose_html = rewrite_md_links(MD.render(expand_sidenotes(prose_md)).strip())
    for idx, c_html in enumerate(code_blocks):
        prose_html = prose_html.replace(f"<p>@@VERIFY_CODE_{idx}@@</p>", c_html)
        prose_html = prose_html.replace(f"@@VERIFY_CODE_{idx}@@", c_html)

    checklist_html = ""
    if checklist_items:
        checklist_html = (
            '\n      <ul class="step-checklist">\n'
            + "\n".join(checklist_items)
            + "\n      </ul>"
        )

    return (
        f'<section class="full-width-panel">\n'
        f'  <div class="panel-header">\n'
        f'    <span class="panel-title">{title_html}</span>\n'
        f'    <span class="panel-badge">{badge_html}</span>\n'
        f"  </div>\n"
        f"  {prose_html}\n"
        f'  <div class="mutation-scoreboard">\n'
        f'    <div class="mutation-stat tier">\n'
        f'      <div class="stat-label">Verification Tier</div>\n'
        f'      <div class="stat-value">{tier_html}</div>\n'
        f"    </div>\n"
        f'    <div class="mutation-stat caught">\n'
        f'      <div class="stat-label">Target Caught Mutants</div>\n'
        f'      <div class="stat-value">{caught_html}</div>\n'
        f"    </div>\n"
        f'    <div class="mutation-stat missed">\n'
        f'      <div class="stat-label">Target Missed Mutants</div>\n'
        f'      <div class="stat-value">{missed_html}</div>\n'
        f"    </div>\n"
        f"  </div>{checklist_html}\n"
        f"</section>"
    )


def render_exercise_block(
    attr_line: str,
    inner_md: str,
    *,
    source_name: str = "",
    errors: list[str] | None = None,
    scripts_out: list[str],
) -> str:
    """Render `:::exercise` into a full-width interactive code completion lab."""
    attrs = parse_kv_attrs(attr_line)
    ex_id = attrs.get("id", "ex")
    title_html = render_inline_md(attrs.get("title", "Interactive Code Lab"))
    badge_html = html.escape(attrs.get("badge", "Skill Practice • Tight Feedback Loop"))

    prompts_data: dict[str, str] = {}

    def _extract_prompts(match: re.Match[str]) -> str:
        raw_prompts = match.group(1).strip()
        for raw_line in raw_prompts.splitlines():
            line = raw_line.strip()
            if line.startswith("hint:"):
                prompts_data["hint"] = line[5:].strip()
            elif line.startswith("success:"):
                prompts_data["success"] = line[8:].strip()
        return ""

    working = PROMPTS_BLOCK_RE.sub(_extract_prompts, inner_md.strip())

    blanks: list[dict[str, Any]] = []
    code_figures: list[str] = []

    def _extract_code(match: re.Match[str]) -> str:
        idx = len(code_figures)
        code_figures.append(
            render_code_figure(
                match.group(1), match.group(2), ex_id=ex_id, blanks_out=blanks
            )
        )
        return f"\n\n@@EXERCISE_CODE_{idx}@@\n\n"

    processed_md = FENCED_CODE_RE.sub(_extract_code, working.strip())
    if errors is not None and not blanks:
        errors.append(
            f"{source_name}: Exercise '{ex_id}' contains 0 `{{{{blank:...}}}}` inputs."
        )
    rendered_inner = rewrite_md_links(MD.render(expand_sidenotes(processed_md)).strip())
    for idx, fig_html in enumerate(code_figures):
        rendered_inner = rendered_inner.replace(f"<p>@@EXERCISE_CODE_{idx}@@</p>", fig_html)
        rendered_inner = rendered_inner.replace(f"@@EXERCISE_CODE_{idx}@@", fig_html)

    check_btn_id = f"btn-check-{ex_id}"
    hint_btn_id = f"btn-hint-{ex_id}"
    feedback_id = f"{ex_id}-feedback"

    hint_html = render_inline_md(prompts_data.get("hint", "**Hint:** Review the code listing above."))
    success_html = render_inline_md(prompts_data.get("success", "**All correct!**"))

    blanks_js_items: list[str] = []
    for b in blanks:
        acc_json = json.dumps(b["accepted"])
        err_msg_json = json.dumps(b["errorHint"])
        b_idx = b["blankIndex"]
        blanks_js_items.append(
            f"        {{\n"
            f'          inputId: {json.dumps(b["inputId"])},\n'
            f"          accepted: {acc_json},\n"
            f"          getErrorHint: (val) =>\n"
            f'            `<strong>Blank {b_idx}:</strong> You wrote <code>${{val || "(empty)"}}</code>. ` + {err_msg_json},\n'
            f"        }}"
        )

    scripts_out.append(
        f"    initCodeExercise({{\n"
        f"      checkBtnId: {json.dumps(check_btn_id)},\n"
        f"      hintBtnId: {json.dumps(hint_btn_id)},\n"
        f"      feedbackId: {json.dumps(feedback_id)},\n"
        f"      blanks: [\n" + ",\n".join(blanks_js_items) + "\n      ],\n"
        f"      hintMessage: {json.dumps(hint_html)},\n"
        f"      successMessage: {json.dumps(success_html)},\n"
        f"    }});"
    )

    return (
        f'<section class="full-width-panel">\n'
        f'  <div class="panel-header">\n'
        f'    <span class="panel-title">{title_html}</span>\n'
        f'    <span class="panel-badge">{badge_html}</span>\n'
        f"  </div>\n"
        f"  {rendered_inner}\n"
        f'  <div style="display: flex; gap: 0.75rem; align-items: center;">\n'
        f'    <button type="button" class="btn-primary" id="{html.escape(check_btn_id, quote=True)}">Check Rust Code</button>\n'
        f'    <button type="button" class="btn-secondary" id="{html.escape(hint_btn_id, quote=True)}">Show Hint</button>\n'
        f"  </div>\n"
        f'  <div id="{html.escape(feedback_id, quote=True)}" class="feedback-box"></div>\n'
        f"</section>"
    )


def parse_quiz_dsl(body: str) -> tuple[str, list[dict[str, Any]]]:
    """Parse `Q: ...` / `- [ ]` / `- [x]` / `why:` / `hint:` DSL or YAML inside `:::quiz`."""
    code_match = FENCED_CODE_RE.search(body)
    if code_match:
        intro_md = body[: code_match.start()].strip()
        yaml_text = code_match.group(2).strip()
        return intro_md, yaml.safe_load(yaml_text) or []

    lines = body.strip().splitlines()
    intro_lines: list[str] = []
    questions: list[dict[str, Any]] = []
    current_q: dict[str, Any] | None = None

    for raw_line in lines:
        line = raw_line.strip()
        if not line:
            continue
        if line.startswith("Q:"):
            if current_q is not None:
                questions.append(current_q)
            current_q = {
                "question": line[2:].strip(),
                "options": [],
                "wrong_hint": "Review the section above and try again.",
            }
        elif current_q is not None and (
            line.startswith("- [ ] ") or line.startswith("- [x] ") or line.startswith("- [X] ")
        ):
            is_correct = line[3].lower() == "x"
            opt_text = line[6:].strip()
            current_q["options"].append(
                {"text": opt_text, "correct": is_correct, "explain": ""}
            )
        elif current_q is not None and line.startswith("why:"):
            explain_text = line[4:].strip()
            for opt in current_q["options"]:
                if opt.get("correct"):
                    opt["explain"] = explain_text
        elif current_q is not None and line.startswith("hint:"):
            current_q["wrong_hint"] = line[5:].strip()
        elif current_q is None:
            intro_lines.append(raw_line)

    if current_q is not None:
        questions.append(current_q)

    if not questions and body.strip():
        return "", yaml.safe_load(body.strip()) or []

    return "\n".join(intro_lines).strip(), questions


def render_quiz_block(
    attr_line: str,
    inner_body: str,
    *,
    source_name: str,
    errors: list[str],
    scripts_out: list[str],
) -> str:
    """Render `:::quiz` into a full-width retrieval quiz and validate equal word counts."""
    attrs = parse_kv_attrs(attr_line)
    quiz_id = attrs.get("id", "quiz-container")
    title_html = render_inline_md(attrs.get("title", "Retrieval Practice"))
    badge_html = html.escape(attrs.get("badge", "3 Questions • Equal-Length Options"))

    intro_md, questions = parse_quiz_dsl(inner_body)
    js_questions: list[dict[str, Any]] = []

    for q_idx, q_item in enumerate(questions, start=1):
        q_text = str(q_item.get("question", "")).strip()
        wrong_hint = str(q_item.get("wrong_hint", "Review the section above and try again.")).strip()
        options = q_item.get("options", [])
        if len(options) != 4:
            errors.append(
                f"{source_name}: Quiz question {q_idx} has {len(options)} options (expected 4)."
            )

        word_counts: list[int] = []
        correct_indices: list[int] = []
        opt_strings: list[str] = []
        explanation = ""

        for opt_idx, opt in enumerate(options):
            text_raw = str(opt.get("text", "")).strip()
            is_correct = bool(opt.get("correct", False))
            if is_correct:
                correct_indices.append(opt_idx)
                explanation = str(opt.get("explain", "")).strip()
            word_counts.append(len(text_raw.split()))
            opt_strings.append(text_raw)

        if len(set(word_counts)) > 1:
            errors.append(
                f"{source_name}: Quiz question {q_idx} option word counts are unequal: {word_counts}"
            )
        if len(correct_indices) != 1:
            errors.append(
                f"{source_name}: Quiz question {q_idx} has {len(correct_indices)} correct options (expected 1)."
            )

        js_questions.append(
            {
                "question": q_text,
                "options": opt_strings,
                "correctIndex": correct_indices[0] if correct_indices else 0,
                "explanation": explanation,
                "wrongHint": wrong_hint,
            }
        )

    scripts_out.append(
        f"    initQuiz({json.dumps(quiz_id)}, {json.dumps(js_questions, indent=6)});"
    )

    intro_html = ""
    if intro_md:
        intro_html = "\n  " + rewrite_md_links(MD.render(expand_sidenotes(intro_md)).strip())

    return (
        f'<section class="full-width-panel">\n'
        f'  <div class="panel-header">\n'
        f'    <span class="panel-title">{title_html}</span>\n'
        f'    <span class="panel-badge">{badge_html}</span>\n'
        f"  </div>{intro_html}\n"
        f'  <div id="{html.escape(quiz_id, quote=True)}"></div>\n'
        f"</section>"
    )


# ==============================================================================
# Reusable Course Interactive Simulators (Widgets)
# ==============================================================================


def widget_pcb_layer_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initPcbLayerSimulator("pcb-layer-sim");')
    return """<section class="full-width-panel" id="pcb-layer-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Trace the Immediate-Mode Frame Loop</span>
    <span class="panel-badge" data-role="frame-badge">Frame #1 • update(&amp;mut self, ctx, frame)</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Toggle the checkboxes or click the preset buttons inside the simulated <code>egui</code> window below. Watch how each interaction immediately mutates <code>self</code> via <code>&amp;mut</code> and repaints the 2D PCB viewport in the very same frame.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Persistent Rust State (<code>self</code>)</h4>
      <pre class="language-rust" style="margin: 0; font-size: 0.78rem; padding: 0.85rem;"><code class="language-rust" data-role="rust-state"></code></pre>
    </div>

    <div class="egui-mock-window">
      <div class="egui-side-panel">
        <div class="egui-heading">PCB Layers</div>
        <label class="egui-checkbox">
          <input type="checkbox" data-layer="top" checked>
          <span style="color: #ff6b5b;">■</span> Top Cu (F.Cu)
        </label>
        <label class="egui-checkbox">
          <input type="checkbox" data-layer="bottom" checked>
          <span style="color: #4da6ff;">■</span> Bottom Cu (B.Cu)
        </label>
        <label class="egui-checkbox">
          <input type="checkbox" data-layer="silk" checked>
          <span style="color: #f0e68c;">■</span> Silkscreen
        </label>
        <label class="egui-checkbox">
          <input type="checkbox" data-layer="rats">
          <span style="color: #7fe5f0;">---</span> Ratsnest
        </label>
        <button type="button" class="egui-btn" data-action="copper-only">Isolate Top Copper</button>
        <button type="button" class="egui-btn" data-action="all-on">Show All Layers</button>
      </div>

      <div class="egui-central-canvas">
        <svg viewBox="0 0 320 210" width="100%" height="210" aria-label="Simulated PCB Viewport">
          <rect x="16" y="14" width="288" height="182" rx="8" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
          <circle cx="32" cy="30" r="6" fill="#0b1410" stroke="#c5a449" stroke-width="2.5"/>
          <circle cx="288" cy="30" r="6" fill="#0b1410" stroke="#c5a449" stroke-width="2.5"/>
          <circle cx="32" cy="180" r="6" fill="#0b1410" stroke="#c5a449" stroke-width="2.5"/>
          <circle cx="288" cy="180" r="6" fill="#0b1410" stroke="#c5a449" stroke-width="2.5"/>
          <g id="svg-layer-bottom" style="transition: opacity 0.15s ease;">
            <path d="M 75 145 L 135 145 L 165 115 L 245 115" fill="none" stroke="#3b8eea" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M 95 70 L 95 125 L 145 165 L 220 165" fill="none" stroke="#3b8eea" stroke-width="4" stroke-linecap="round" stroke-linejoin="round"/>
          </g>
          <g id="svg-layer-top" style="transition: opacity 0.15s ease;">
            <path d="M 75 65 L 145 65 L 175 95 L 245 95" fill="none" stroke="#e05243" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M 75 95 L 125 95 L 145 115" fill="none" stroke="#e05243" stroke-width="4" stroke-linecap="round"/>
            <rect x="65" y="58" width="16" height="10" rx="2" fill="#e05243"/>
            <rect x="65" y="88" width="16" height="10" rx="2" fill="#e05243"/>
            <rect x="65" y="138" width="16" height="10" rx="2" fill="#e05243"/>
            <rect x="238" y="88" width="16" height="12" rx="2" fill="#e05243"/>
            <rect x="238" y="108" width="16" height="12" rx="2" fill="#e05243"/>
            <circle cx="145" cy="115" r="5" fill="#10281e" stroke="#e6c15a" stroke-width="3"/>
          </g>
          <g id="svg-layer-silk" style="transition: opacity 0.15s ease;">
            <rect x="58" y="48" width="38" height="108" fill="none" stroke="#f0e68c" stroke-width="1.5" stroke-dasharray="4 2"/>
            <text x="64" y="43" fill="#f0e68c" font-family="monospace" font-size="11">U1: MCU</text>
            <rect x="232" y="78" width="28" height="52" fill="none" stroke="#f0e68c" stroke-width="1.5"/>
            <text x="230" y="72" fill="#f0e68c" font-family="monospace" font-size="11">J1: USB</text>
          </g>
          <g id="svg-layer-rats" style="transition: opacity 0.15s ease;">
            <line x1="75" y1="143" x2="245" y2="94" stroke="#6ff7ff" stroke-width="1.6" stroke-dasharray="4 4"/>
            <line x1="75" y1="63" x2="220" y2="165" stroke="#6ff7ff" stroke-width="1.6" stroke-dasharray="4 4"/>
          </g>
        </svg>
      </div>
    </div>

    <div class="sim-box">
      <h4>2. What Happened This Frame</h4>
      <div data-role="frame-trace" style="font-size: 0.85rem; line-height: 1.55;"></div>
    </div>
  </div>
</section>"""


def widget_painter_canvas_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initPainterCanvasSimulator("painter-canvas-sim");')
    return """<section class="full-width-panel" id="painter-canvas-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Explore <code>Painter</code> Primitives &amp; <code>Rect</code> Bounds</span>
    <span class="panel-badge">Pos2 • Vec2 • Rect • Stroke</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Adjust the board margin, SMD pad width/height, and via radius below. Watch how <code>centered_board_rect</code> and <code>smd_pad_rect</code> recompute <code>min</code> and <code>max</code> coordinates for <code>painter.rect</code>, <code>painter.rect_filled</code>, <code>painter.line_segment</code>, and <code>painter.circle</code>.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Geometry Parameters</h4>
      <label style="display:block; font-size:0.82rem; margin-bottom:0.6rem;">
        Board Margin (<code>f32</code>):
        <input type="range" data-param="margin" min="10" max="48" value="24" style="width:100%;">
      </label>
      <label style="display:block; font-size:0.82rem; margin-bottom:0.6rem;">
        SMD Pad Width (<code>size.x</code>):
        <input type="range" data-param="padW" min="12" max="40" value="22" style="width:100%;">
      </label>
      <label style="display:block; font-size:0.82rem; margin-bottom:0.6rem;">
        SMD Pad Height (<code>size.y</code>):
        <input type="range" data-param="padH" min="8" max="28" value="14" style="width:100%;">
      </label>
      <label style="display:block; font-size:0.82rem; margin-bottom:0.6rem;">
        Via Radius (<code>f32</code>):
        <input type="range" data-param="viaRadius" min="4" max="14" value="7" style="width:100%;">
      </label>
      <div data-role="painter-trace" style="margin-top:0.75rem;"></div>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:230px;">
      <svg viewBox="0 0 320 210" width="100%" height="210" aria-label="2D Painter Primitives Preview">
        <rect data-svg="board-rect" x="24" y="24" width="272" height="162" rx="6" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
        <line data-svg="trace-1" x1="88" y1="80" x2="178" y2="80" stroke="#e05243" stroke-width="5" stroke-linecap="round"/>
        <line data-svg="trace-2" x1="178" y1="80" x2="214" y2="110" stroke="#e05243" stroke-width="5" stroke-linecap="round"/>
        <rect data-svg="pad-rect" x="77" y="73" width="22" height="14" rx="2" fill="#e05243"/>
        <circle data-svg="via-circle" cx="214" cy="110" r="7" fill="#10281e" stroke="#e6c15a" stroke-width="3.5"/>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. Computed <code>Rect</code> Values</h4>
      <pre class="language-rust" style="margin:0; font-size:0.76rem; padding:0.8rem;"><code class="language-rust" data-role="painter-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_viewport_transform_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initViewportTransformSimulator("viewport-transform-sim");')
    return """<section class="full-width-panel" id="viewport-transform-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Pan &amp; Cursor-Anchored Zoom Simulator</span>
    <span class="panel-badge" data-role="zoom-badge">Zoom: 3.00 px/mm</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    <strong>Click and drag</strong> inside the PCB canvas below to pan (<code>pan_by</code>), move your cursor over a pad, and click <strong>Zoom In / Out at Cursor</strong> (<code>zoom_around</code>). Watch how <code>screen_to_world</code> under the crosshair stays stationary when zooming!
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Camera State (<code>self.viewport</code>)</h4>
      <pre class="language-rust" style="margin:0 0 0.75rem 0; font-size:0.76rem; padding:0.75rem;"><code class="language-rust" data-role="transform-state"></code></pre>
      <div style="display:flex; gap:0.4rem; flex-wrap:wrap;">
        <button type="button" class="egui-btn" data-action="zoom-in">+ Zoom In at Cursor</button>
        <button type="button" class="egui-btn" data-action="zoom-out">- Zoom Out at Cursor</button>
        <button type="button" class="egui-btn" data-action="zoom-reset">Reset Camera</button>
      </div>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:230px; cursor:grab;">
      <svg data-role="viewport-svg" viewBox="0 0 320 210" width="100%" height="210" aria-label="Interactive Pan and Zoom PCB Canvas">
        <g data-svg="world-group" transform="translate(24, 20) scale(3)">
          <rect x="0" y="0" width="88" height="56" rx="2" fill="#10281e" stroke="#2f6f52" stroke-width="0.8"/>
          <path d="M 20 42 L 52 42 L 64 30" fill="none" stroke="#3b8eea" stroke-width="1.6" stroke-linecap="round"/>
          <path d="M 20 22 L 48 22 L 64 30" fill="none" stroke="#e05243" stroke-width="1.6" stroke-linecap="round"/>
          <rect x="17" y="20" width="6" height="4" rx="0.6" fill="#e05243"/>
          <rect x="17" y="40" width="6" height="4" rx="0.6" fill="#3b8eea"/>
          <circle cx="64" cy="30" r="2.2" fill="#10281e" stroke="#e6c15a" stroke-width="1.1"/>
          <rect x="13" y="15" width="14" height="33" fill="none" stroke="#f0e68c" stroke-width="0.5" stroke-dasharray="1.5 1"/>
        </g>
        <g data-svg="cursor-crosshair" transform="translate(160, 105)" style="pointer-events:none;">
          <circle cx="0" cy="0" r="5" fill="none" stroke="#3dd68c" stroke-width="1.2"/>
          <line x1="-9" y1="0" x2="9" y2="0" stroke="#3dd68c" stroke-width="1"/>
          <line x1="0" y1="-9" x2="0" y2="9" stroke="#3dd68c" stroke-width="1"/>
        </g>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. Live Coordinate Math</h4>
      <div data-role="transform-math"></div>
    </div>
  </div>
</section>"""


def widget_layer_zorder_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initLayerZOrderSimulator("layer-zorder-sim");')
    return """<section class="full-width-panel" id="layer-zorder-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Compare Sorted <code>z_rank()</code> vs. Unsorted File Order</span>
    <span class="panel-badge">PcbLayer • Vec&lt;TraceSegment&gt; • sort_by_key</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Uncheck <strong>Sort by <code>z_rank()</code></strong> below to see what happens when <code>BottomCopper</code> happens to sit at the end of <code>self.traces</code>: it paints right on top of <code>TopCopper</code> and <code>Silkscreen</code>! Re-enable <code>z_rank()</code> sorting to restore proper back-to-front CAD layer ordering.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Visibility &amp; Z-Sort Controls</h4>
      <label class="egui-checkbox">
        <input type="checkbox" data-z="bottom" checked>
        <span style="color:#3b8eea;">■</span> BottomCopper (z=0)
      </label>
      <label class="egui-checkbox">
        <input type="checkbox" data-z="top" checked>
        <span style="color:#e05243;">■</span> TopCopper (z=1)
      </label>
      <label class="egui-checkbox">
        <input type="checkbox" data-z="silk" checked>
        <span style="color:#f0e68c;">■</span> Silkscreen (z=2)
      </label>
      <label class="egui-checkbox">
        <input type="checkbox" data-z="rats" checked>
        <span style="color:#6ff7ff;">---</span> Ratsnest (z=3)
      </label>
      <hr style="border:none; border-top:1px solid var(--border-subtle); margin:0.6rem 0;">
      <label class="egui-checkbox" style="font-weight:600; color:var(--accent-pcb);">
        <input type="checkbox" data-z="sort" checked>
        Sort by <code>layer.z_rank()</code>
      </label>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:220px;">
      <svg viewBox="0 0 310 210" width="100%" height="210" aria-label="Z-Order Layer Overlap Comparison">
        <rect x="20" y="16" width="270" height="178" rx="6" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
        <g data-svg="z-layers"></g>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. Resulting Paint Sequence</h4>
      <pre class="language-rust" style="margin:0; font-size:0.78rem; padding:0.8rem;"><code class="language-rust" data-role="zorder-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_hittest_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initHitTestCrossProbeSimulator("hittest-sim");')
    return """<section class="full-width-panel" id="hittest-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Hover &amp; Click Pads to Cross-Probe Nets</span>
    <span class="panel-badge">Option&lt;NetId&gt; • Sense::click_and_drag()</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    <strong>Hover</strong> over the SMD pads on the simulated PCB canvas below to preview <code>hovered_net</code>, or <strong>click</strong> a pad to lock <code>selected_net</code>. Watch how all traces and pads sharing that <code>NetId</code> highlight across layers and update the <code>SidePanel</code> Net Inspector.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Net Inspector (<code>SidePanel</code>)</h4>
      <div data-role="net-inspector" style="font-size:0.86rem; line-height:1.55; margin-bottom:0.85rem;"></div>
      <button type="button" class="egui-btn" data-action="clear-net">Clear Selection (<code>selected_net = None</code>)</button>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:225px;">
      <svg viewBox="0 0 320 210" width="100%" height="210" aria-label="Interactive Net Cross-Probing Canvas">
        <rect x="18" y="14" width="284" height="182" rx="6" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
        <path data-net="2" data-base-color="#3b8eea" d="M 80 150 L 210 150 L 245 120" fill="none" stroke="#3b8eea" stroke-width="5" stroke-linecap="round"/>
        <line data-net="2" data-base-color="#6ff7ff" x1="80" y1="105" x2="210" y2="150" stroke="#6ff7ff" stroke-width="2" stroke-dasharray="4 4"/>
        <path data-net="1" data-base-color="#3b8eea" d="M 195 95 L 245 95" fill="none" stroke="#3b8eea" stroke-width="5" stroke-linecap="round"/>
        <path data-net="1" data-base-color="#e05243" d="M 80 65 L 165 65 L 195 95" fill="none" stroke="#e05243" stroke-width="5" stroke-linecap="round"/>
        <circle cx="195" cy="95" r="6" fill="#10281e" stroke="#e6c15a" stroke-width="3"/>
        <rect data-net="1" data-hit-net="1" data-base-color="#e05243" x="66" y="57" width="28" height="16" rx="2" fill="#e05243" style="cursor:pointer;"/>
        <rect data-net="1" data-hit-net="1" data-base-color="#e05243" x="234" y="87" width="24" height="16" rx="2" fill="#e05243" style="cursor:pointer;"/>
        <rect data-net="2" data-hit-net="2" data-base-color="#e05243" x="66" y="97" width="28" height="16" rx="2" fill="#e05243" style="cursor:pointer;"/>
        <rect data-net="2" data-hit-net="2" data-base-color="#3b8eea" x="66" y="142" width="28" height="16" rx="2" fill="#3b8eea" style="cursor:pointer;"/>
        <rect data-net="2" data-hit-net="2" data-base-color="#3b8eea" x="234" y="112" width="24" height="16" rx="2" fill="#3b8eea" style="cursor:pointer;"/>
        <text x="64" y="48" fill="#f0e68c" font-family="monospace" font-size="11">U1 (Hover/Click Pads)</text>
        <text x="232" y="78" fill="#f0e68c" font-family="monospace" font-size="11">J1</text>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. Live <code>Option&lt;NetId&gt;</code> State</h4>
      <pre class="language-rust" style="margin:0; font-size:0.78rem; padding:0.8rem;"><code class="language-rust" data-role="hittest-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_sch_text_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initSchematicTextSimulator("sch-text-sim");')
    return """<section class="full-width-panel" id="sch-text-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: <code>Painter::text</code> Anchor Alignment &amp; <code>egui_kittest</code> Node Tree</span>
    <span class="panel-badge">Align2 • FontId • Harness::new_eframe</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Adjust the <code>Align2</code> anchor mode, font size, and <code>show_silkscreen</code> toggle below. Observe how <code>Painter::text</code> repositions the reference designators on the canvas while <code>egui_kittest::Harness</code> inspects the live accessibility tree in the <code>SidePanel</code>.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Text &amp; Layer Controls</h4>
      <label style="display:block; font-size:0.82rem; margin-bottom:0.6rem;">
        <code>Align2</code> Anchor:<br>
        <select data-param="align" style="width:100%; margin-top:0.25rem; padding:0.35rem; background:var(--bg-code); color:var(--text-main); border:1px solid var(--border-strong); border-radius:4px;">
          <option value="CENTER_CENTER">Align2::CENTER_CENTER</option>
          <option value="LEFT_TOP">Align2::LEFT_TOP</option>
          <option value="RIGHT_BOTTOM">Align2::RIGHT_BOTTOM</option>
        </select>
      </label>
      <label style="display:block; font-size:0.82rem; margin-bottom:0.6rem;">
        <code>FontId::monospace(size)</code>:<br>
        <input type="range" data-param="fontSize" min="10" max="18" step="1" value="13" style="width:100%;">
      </label>
      <label class="egui-checkbox" style="margin-bottom:0.6rem;">
        <input type="checkbox" data-param="silk" checked>
        <span>Silkscreen (<code>show_silkscreen</code>)</span>
      </label>
      <div style="display:flex; gap:0.4rem; flex-wrap:wrap;">
        <button type="button" class="egui-btn" data-select-net="1">Select NET_1</button>
        <button type="button" class="egui-btn" data-select-net="2">Select NET_2</button>
        <button type="button" class="egui-btn" data-select-net="none">Clear Net</button>
      </div>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:225px;">
      <svg viewBox="0 0 320 210" width="100%" height="210" aria-label="Silkscreen Component &amp; Text Alignment Canvas">
        <rect x="18" y="14" width="284" height="182" rx="6" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
        <path d="M 80 75 L 160 75 L 240 75" fill="none" stroke="#e05243" stroke-width="4" stroke-linecap="round"/>
        <path d="M 80 135 L 160 135 L 240 135" fill="none" stroke="#3b8eea" stroke-width="4" stroke-linecap="round"/>
        <g data-svg="silk-outlines">
          <rect x="48" y="55" width="64" height="96" rx="3" fill="rgba(240,230,140,0.06)" stroke="#f0e68c" stroke-width="1.8"/>
          <rect x="134" y="88" width="52" height="32" rx="2" fill="rgba(240,230,140,0.06)" stroke="#f0e68c" stroke-width="1.8"/>
          <rect x="214" y="58" width="52" height="90" rx="3" fill="rgba(240,230,140,0.06)" stroke="#f0e68c" stroke-width="1.8"/>
          <circle cx="80" cy="103" r="2.5" fill="#ff8b6b"/>
          <circle cx="160" cy="104" r="2.5" fill="#ff8b6b"/>
          <circle cx="240" cy="103" r="2.5" fill="#ff8b6b"/>
        </g>
        <text data-svg="refdes-label" x="80" y="103" fill="#f0e68c" font-family="monospace" font-weight="bold" font-size="13" text-anchor="middle" dy="0.35em">U1</text>
        <text data-svg="refdes-label" x="160" y="104" fill="#f0e68c" font-family="monospace" font-weight="bold" font-size="13" text-anchor="middle" dy="0.35em">R12</text>
        <text data-svg="refdes-label" x="240" y="103" fill="#f0e68c" font-family="monospace" font-weight="bold" font-size="13" text-anchor="middle" dy="0.35em">J1</text>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. Live Code &amp; <code>egui_kittest</code> Tree</h4>
      <pre class="language-rust" style="margin:0 0 0.65rem 0; font-size:0.74rem; padding:0.65rem;"><code class="language-rust" data-role="text-painter-code"></code></pre>
      <pre class="language-bash" style="margin:0; font-size:0.72rem; padding:0.65rem;"><code class="language-bash" data-role="kittest-tree"></code></pre>
    </div>
  </div>
</section>"""


def widget_dual_scene_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initDualSceneCrossProbeSimulator("dual-scene-sim");')
    return """<section class="full-width-panel" id="dual-scene-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Schematic-to-PCB Cross-Probing &amp; Split Borrowing</span>
    <span class="panel-badge">TopBottomPanel • egui::Scene • &amp;BoardModel + &amp;mut ViewState</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Click any schematic symbol button (<strong>Probe U1</strong>, <strong>Probe R12</strong>, or <strong>Probe J1</strong>) below. Watch how <code>self.view.focus_component(&amp;self.board, ref_des)</code> reads the component's physical coordinates from <code>&amp;self.board</code> and updates <code>self.view.pcb_scene_rect</code> to centre the PCB camera on that footprint.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Schematic Symbol Bar</h4>
      <p style="font-size:0.8rem; color:var(--text-muted); margin-bottom:0.6rem;">
        Click a schematic symbol to cross-probe its footprint on the PCB:
      </p>
      <div style="display:flex; flex-direction:column; gap:0.45rem;">
        <button type="button" class="egui-btn" data-probe-ref="U1">Probe U1 (LDO_3V3) → (20, 27) mm</button>
        <button type="button" class="egui-btn" data-probe-ref="R12">Probe R12 (10k) → (52, 35) mm</button>
        <button type="button" class="egui-btn" data-probe-ref="J1">Probe J1 (CONN_2P) → (68, 35) mm</button>
      </div>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:225px;">
      <svg viewBox="0 0 320 210" width="100%" height="210" aria-label="Dual Schematic and PCB Cross-Probe Viewport">
        <rect x="12" y="10" width="296" height="58" rx="4" fill="#161d26" stroke="#334458" stroke-width="1.5"/>
        <text x="20" y="24" fill="#9da6b0" font-family="sans-serif" font-size="9">SCHEMATIC VIEW (TopBottomPanel)</text>
        <line x1="70" y1="45" x2="250" y2="45" stroke="#6cb6ff" stroke-width="1.5"/>
        <rect data-sch-comp="U1" data-probe-ref="U1" x="52" y="30" width="38" height="28" rx="2" fill="#1e2936" stroke="#ffe664" stroke-width="3" style="cursor:pointer;"/>
        <text x="71" y="48" fill="#e8e6df" font-family="monospace" font-size="10" text-anchor="middle" style="pointer-events:none;">U1</text>
        <rect data-sch-comp="R12" data-probe-ref="R12" x="142" y="33" width="38" height="22" rx="2" fill="#1e2936" stroke="#6cb6ff" stroke-width="1.8" style="cursor:pointer;"/>
        <text x="161" y="48" fill="#e8e6df" font-family="monospace" font-size="10" text-anchor="middle" style="pointer-events:none;">R12</text>
        <rect data-sch-comp="J1" data-probe-ref="J1" x="226" y="30" width="36" height="28" rx="2" fill="#1e2936" stroke="#6cb6ff" stroke-width="1.8" style="cursor:pointer;"/>
        <text x="244" y="48" fill="#e8e6df" font-family="monospace" font-size="10" text-anchor="middle" style="pointer-events:none;">J1</text>

        <rect x="12" y="76" width="296" height="124" rx="5" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
        <path d="M 78 88 L 162 112 L 242 102" fill="none" stroke="#e05243" stroke-width="4" stroke-linecap="round"/>
        <rect data-pcb-comp="U1" x="58" y="64" width="40" height="32" rx="2" fill="rgba(240,230,140,0.08)" stroke="#ffe664" stroke-width="3"/>
        <text x="78" y="84" fill="#f0e68c" font-family="monospace" font-size="10" text-anchor="middle">U1</text>
        <rect data-pcb-comp="R12" x="144" y="98" width="36" height="26" rx="2" fill="rgba(240,230,140,0.08)" stroke="#f0e68c" stroke-width="1.6"/>
        <text x="162" y="115" fill="#f0e68c" font-family="monospace" font-size="10" text-anchor="middle">R12</text>
        <rect data-pcb-comp="J1" x="224" y="86" width="36" height="32" rx="2" fill="rgba(240,230,140,0.08)" stroke="#f0e68c" stroke-width="1.6"/>
        <text x="242" y="106" fill="#f0e68c" font-family="monospace" font-size="10" text-anchor="middle">J1</text>
        <rect data-svg="pcb-camera-box" x="42" y="52" width="72" height="56" rx="3" fill="none" stroke="#3dd68c" stroke-width="2" stroke-dasharray="4 3"/>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. Split Borrow &amp; Camera State</h4>
      <pre class="language-rust" style="margin:0; font-size:0.75rem; padding:0.75rem;"><code class="language-rust" data-role="scene-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_bom_scroll_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initBomVirtualScrollSimulator("bom-scroll-sim");')
    return """<section class="full-width-panel" id="bom-scroll-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Live BOM Filter &amp; Virtualised <code>show_rows</code> Inspector</span>
    <span class="panel-badge">TextEdit::singleline • .filter() • ScrollArea::show_rows</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Type a query like <code>u1</code>, <code>0603</code>, <code>vcc</code>, or <code>spi</code> into the simulated <code>TextEdit::singleline</code> box below, and click any row in the virtualised <code>Grid</code> to cross-probe that component:
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. <code>TextEdit::singleline</code> Filter</h4>
      <label style="display:block; font-size:0.82rem; margin-bottom:0.6rem;">
        <code>&amp;mut self.view.bom_search_query</code>:<br>
        <input type="text" data-role="bom-query-input" placeholder="e.g. u1, 10k, vcc_3v3..." style="width:100%; margin-top:0.3rem; padding:0.4rem 0.55rem; background:var(--bg-code); color:var(--text-main); border:1px solid var(--border-strong); border-radius:4px; font-family:var(--font-mono); font-size:0.82rem;">
      </label>
      <button type="button" class="egui-btn" data-action="clear-bom-query">Clear Filter (<code>clear_bom_query()</code>)</button>
    </div>

    <div class="sim-box" style="overflow-x:auto;">
      <h4>2. Virtualised <code>egui::Grid</code> (<code>&amp;matching[row_range]</code>)</h4>
      <table style="width:100%; border-collapse:collapse; font-size:0.8rem;">
        <thead>
          <tr style="border-bottom:1px solid var(--border-strong); color:var(--text-muted); text-align:left;">
            <th style="padding:0.3rem 0.5rem;">Idx</th>
            <th style="padding:0.3rem 0.5rem;">RefDes</th>
            <th style="padding:0.3rem 0.5rem;">Value</th>
            <th style="padding:0.3rem 0.5rem;">Primary Net</th>
          </tr>
        </thead>
        <tbody data-role="bom-tbody"></tbody>
      </table>
    </div>

    <div class="sim-box">
      <h4>3. Iterator &amp; Slice State</h4>
      <pre class="language-rust" style="margin:0; font-size:0.75rem; padding:0.75rem;"><code class="language-rust" data-role="bom-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_hotkey_ruler_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initHotkeyRulerSimulator("hotkey-ruler-sim");')
    return """<section class="full-width-panel" id="hotkey-ruler-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: CAD Hotkeys (<code>F</code> / <code>M</code> / <code>Esc</code>) &amp; Euclidean Ruler</span>
    <span class="panel-badge">consume_key • ActiveTool::Measure • hypot()</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Click the simulated hotkey buttons (<code>Key::M</code>, <code>Key::Escape</code>, <code>Key::F</code>) or select different measurement target pads below to inspect how <code>ActiveTool</code> and <code>RulerMeasurement::between</code> compute Δx, Δy, and Euclidean distance in millimetres:
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Simulated Hotkeys &amp; Targets</h4>
      <div style="display:flex; gap:0.45rem; flex-wrap:wrap; margin-bottom:0.75rem;">
        <button type="button" class="egui-btn" data-key="M">Press <code>M</code> (Measure)</button>
        <button type="button" class="egui-btn" data-key="Escape">Press <code>Esc</code> (Select)</button>
        <button type="button" class="egui-btn" data-key="F">Press <code>F</code> (Fit Board)</button>
      </div>
      <p style="font-size:0.78rem; color:var(--text-muted); margin-bottom:0.4rem;">
        Snap Ruler Endpoint (from <code>(20.0, 22.0) mm</code>):
      </p>
      <div style="display:flex; flex-direction:column; gap:0.35rem;">
        <button type="button" class="egui-btn" data-ruler-preset="68,58">To J1 Corner (68, 58) → 3-4-5 Triangle (60 mm)</button>
        <button type="button" class="egui-btn" data-ruler-preset="52,22">To R12 Pad (52, 22) → Horizontal (32 mm)</button>
        <button type="button" class="egui-btn" data-ruler-preset="20,48">To B.Cu Pad (20, 48) → Vertical (26 mm)</button>
      </div>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:225px;">
      <svg viewBox="0 0 320 210" width="100%" height="210" aria-label="Point-to-Point CAD Ruler Canvas">
        <rect x="18" y="14" width="284" height="182" rx="6" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
        <path d="M 76 71 L 165.6 71 L 210.4 103.5" fill="none" stroke="#e05243" stroke-width="4" stroke-linecap="round"/>
        <rect x="66" y="65" width="20" height="12" rx="2" fill="#e05243"/>
        <rect x="155" y="65" width="20" height="12" rx="2" fill="#e05243"/>
        <rect x="66" y="130" width="20" height="12" rx="2" fill="#3b8eea"/>
        <rect x="200" y="155" width="20" height="12" rx="2" fill="#3b8eea"/>

        <g data-svg="ruler-overlay">
          <line data-svg="ruler-dx" x1="76" y1="71" x2="210.4" y2="71" stroke="#6cb6ff" stroke-width="1.5" stroke-dasharray="4 3"/>
          <line data-svg="ruler-dy" x1="210.4" y1="71" x2="210.4" y2="161" stroke="#6cb6ff" stroke-width="1.5" stroke-dasharray="4 3"/>
          <line data-svg="ruler-hyp" x1="76" y1="71" x2="210.4" y2="161" stroke="#ffe664" stroke-width="2.5"/>
          <circle cx="76" cy="71" r="4.5" fill="#3dd68c" stroke="#0c0e11" stroke-width="1.5"/>
          <circle data-svg="ruler-end-dot" cx="210.4" cy="161" r="4.5" fill="#ffe664" stroke="#0c0e11" stroke-width="1.5"/>
          <text data-svg="ruler-label" x="143" y="108" fill="#ffe664" font-family="monospace" font-size="10.5" font-weight="bold" text-anchor="middle">Δx=48.0 Δy=36.0 d=60.0mm</text>
        </g>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. Live <code>ActiveTool</code> State</h4>
      <pre class="language-rust" style="margin:0; font-size:0.75rem; padding:0.75rem;"><code class="language-rust" data-role="ruler-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_review_pins_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initReviewPinsSimulator("review-pins-sim");')
    return """<section class="full-width-panel" id="review-pins-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Anchored Review Pins, <code>HashMap::entry</code> &amp; Callout Window</span>
    <span class="panel-badge">HashMap&lt;PinId, ReviewNote&gt; • Entry::Vacant • egui::Window</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Click any review pin in the list below or test <code>insert_review_pin</code> at <code>R12 (52, 35) mm</code>. Try clicking the insert button twice to see how <code>Entry::Occupied</code> protects existing notes from silent overwrites!
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Sorted Review Pins (<code>SidePanel</code>)</h4>
      <div data-role="pin-list" style="display:flex; flex-direction:column; margin-bottom:0.6rem;"></div>
      <div style="display:flex; gap:0.4rem; flex-wrap:wrap;">
        <button type="button" class="egui-btn" data-action="add-pin-r12">+ Insert Pin #3 at R12</button>
        <button type="button" class="egui-btn" data-action="close-callout">Close Callout</button>
      </div>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:225px;">
      <svg viewBox="0 0 320 210" width="100%" height="210" aria-label="Review Pin Callouts on PCB Canvas">
        <rect x="18" y="14" width="284" height="182" rx="6" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
        <path d="M 80 82 L 170 102 L 214 102" fill="none" stroke="#e05243" stroke-width="4" stroke-linecap="round"/>
        <g data-svg="pin-markers"></g>
        <g data-svg="callout-box">
          <rect x="44" y="128" width="232" height="54" rx="5" fill="#182029" stroke="#ffe664" stroke-width="1.5"/>
          <text data-svg="callout-title" x="54" y="147" fill="#ffe664" font-family="monospace" font-size="10.5" font-weight="bold">Pin #1: Check U1 thermal relief</text>
          <text data-svg="callout-coord" x="54" y="166" fill="#e8e6df" font-family="monospace" font-size="9.5">Anchor: (20.0, 27.0) mm (@alice)</text>
        </g>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. <code>HashMap</code> &amp; Window ID State</h4>
      <pre class="language-rust" style="margin:0; font-size:0.75rem; padding:0.75rem;"><code class="language-rust" data-role="pins-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_review_triage_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initReviewTriageSimulator("review-triage-sim");')
    return """<section class="full-width-panel" id="review-triage-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Review Sign-Off State Machine &amp; Multi-Criteria Filter</span>
    <span class="panel-badge">ReviewStatus • PinSeverity::ALL • can_transition_to</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Try clicking <strong>Mark Verified</strong> while Pin #1 is still <code>Open</code>—notice how <code>can_transition_to</code> rejects skipping <code>Resolved</code>! Then adjust the minimum severity and open-only filter controls.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Triage Controls &amp; Filters</h4>
      <div style="display:flex; gap:0.4rem; flex-wrap:wrap; margin-bottom:0.75rem;">
        <button type="button" class="egui-btn" data-transition="Resolved">Mark Resolved</button>
        <button type="button" class="egui-btn" data-transition="Verified">Mark Verified</button>
        <button type="button" class="egui-btn" data-transition="Open">Reopen Pin</button>
      </div>
      <label style="display:block; font-size:0.82rem; margin-bottom:0.5rem;">
        Min Severity (<code>PinSeverity::ALL</code>):
        <select data-param="minSev" style="width:100%; margin-top:0.25rem; padding:0.35rem; background:var(--bg-code); color:var(--text-main); border:1px solid var(--border-strong); border-radius:4px;">
          <option value="Info">Info (rank &gt;= 0)</option>
          <option value="Warning">Warning (rank &gt;= 1)</option>
          <option value="Critical">Critical (rank &gt;= 2)</option>
        </select>
      </label>
      <label class="egui-checkbox">
        <input type="checkbox" data-param="onlyOpen">
        <span>Open Pins Only (<code>filter_only_open_pins</code>)</span>
      </label>
    </div>

    <div class="sim-box">
      <h4>2. Filtered Review List</h4>
      <div data-role="triage-list"></div>
    </div>

    <div class="sim-box">
      <h4>3. Workflow &amp; Filter State</h4>
      <pre class="language-rust" style="margin:0; font-size:0.75rem; padding:0.75rem;"><code class="language-rust" data-role="triage-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_drc_minimap_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initDrcMinimapSimulator("drc-minimap-sim");')
    return """<section class="full-width-panel" id="drc-minimap-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: DRC Clearance Thresholds &amp; Minimap Camera Clamping</span>
    <span class="panel-badge">pad_edge_gap_mm • Iterator::reduce • f32::clamp</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Slide <code>min_clearance_mm</code> across the exact <code>7.6 mm</code> pad gap boundary to see when the red DRC halo triggers, and pan the camera X coordinate past the board edge to observe <code>clamped_minimap_camera_rect</code> clamping at <code>0..100 mm</code>.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Clearance &amp; Camera Controls</h4>
      <label style="display:block; font-size:0.82rem; margin-bottom:0.65rem;">
        Required Clearance (<code>min_clearance_mm</code>):
        <input type="range" data-param="clearance" min="6.0" max="10.0" step="0.2" value="8.0" style="width:100%;">
      </label>
      <label style="display:block; font-size:0.82rem; margin-bottom:0.65rem;">
        Unclamped Camera Min X (<code>-30..+90 mm</code>):
        <input type="range" data-param="camX" min="-30" max="90" step="5" value="0" style="width:100%;">
      </label>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:225px;">
      <svg viewBox="0 0 320 210" width="100%" height="210" aria-label="DRC Clearance and Minimap Viewport">
        <rect x="24" y="20" width="250" height="154" rx="4" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
        <rect x="68" y="64" width="20" height="12" rx="2" fill="#e05243"/>
        <rect x="68" y="88" width="20" height="12" rx="2" fill="#e05243"/>
        <g data-svg="drc-halo">
          <circle cx="78" cy="82" r="18" fill="rgba(255,85,85,0.18)" stroke="#ff5555" stroke-width="2" stroke-dasharray="4 2"/>
          <text x="104" y="86" fill="#ff6b6b" font-family="monospace" font-size="9.5" font-weight="bold">DRC: gap 7.6mm</text>
        </g>
        <rect data-svg="minimap-cam" x="24" y="46" width="100" height="66" fill="rgba(61,214,140,0.08)" stroke="#3dd68c" stroke-width="2"/>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. DRC &amp; Clamped Minimap State</h4>
      <pre class="language-rust" style="margin:0; font-size:0.75rem; padding:0.75rem;"><code class="language-rust" data-role="drc-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_workspace_layers_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initWorkspaceLayersSimulator("workspace-layers-sim");')
    return """<section class="full-width-panel" id="workspace-layers-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Multi-Crate <code>L1</code>–<code>L6</code> Dependency Gate</span>
    <span class="panel-badge">pcb-doc (L1) • pcb-reviewer (L6) • validate_l1_manifest_no_ui_deps</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Toggle dependencies in <code>crates/pcb-doc/Cargo.toml</code> below to test how the Tier 1 architecture gate enforces a one-way dependency DAG and keeps domain compilation decoupled from <code>egui</code> / <code>eframe</code>.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. <code>crates/pcb-doc/Cargo.toml</code></h4>
      <label class="egui-checkbox" style="margin-bottom:0.45rem;">
        <input type="checkbox" checked disabled>
        <code>emath = "0.33"</code> (L0 math — allowed)
      </label>
      <label class="egui-checkbox" style="margin-bottom:0.45rem;">
        <input type="checkbox" data-dep="egui">
        Inject <code>egui = "0.33"</code> (L6 UI — forbidden!)
      </label>
      <label class="egui-checkbox" style="margin-bottom:0.45rem;">
        <input type="checkbox" data-dep="eframe">
        Inject <code>eframe = "0.33"</code> (L7 Shell — forbidden!)
      </label>
      <label class="egui-checkbox">
        <input type="checkbox" data-dep="rfd">
        Inject <code>rfd = "0.15"</code> (Native OS Dialog — forbidden!)
      </label>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:225px;">
      <svg viewBox="0 0 320 210" width="100%" height="210" aria-label="Workspace Crate Layer Graph">
        <rect x="24" y="20" width="272" height="52" rx="6" fill="#14231c" stroke="#3dd68c" stroke-width="2"/>
        <text x="160" y="42" text-anchor="middle" fill="#e6edf3" font-family="monospace" font-size="12" font-weight="bold">pcb-reviewer (L6 UI / L7 App)</text>
        <text x="160" y="59" text-anchor="middle" fill="#8b949e" font-family="monospace" font-size="10">eframe + egui + egui_kittest</text>

        <line x1="160" y1="74" x2="160" y2="122" stroke="#3dd68c" stroke-width="2.5"/>
        <polygon points="155,118 165,118 160,126" fill="#3dd68c"/>

        <rect data-svg="l1-box" x="24" y="128" width="272" height="60" rx="6" fill="#141a21" stroke="#3b8eea" stroke-width="2"/>
        <text x="160" y="152" text-anchor="middle" fill="#e6edf3" font-family="monospace" font-size="12" font-weight="bold">crates/pcb-doc (L1 Domain)</text>
        <text data-svg="l1-status" x="160" y="172" text-anchor="middle" fill="#3dd68c" font-family="monospace" font-size="10">PASS: 0 UI crates in L1</text>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. Tier 1 Manifest Gate Output</h4>
      <pre class="language-rust" style="margin:0; font-size:0.75rem; padding:0.75rem;"><code class="language-rust" data-role="layers-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_serde_session_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initSerdeSessionSimulator("serde-session-sim");')
    return """<section class="full-width-panel" id="serde-session-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Versioned <code>serde_json</code> Envelope &amp; Typed Error Recovery</span>
    <span class="panel-badge">ReviewSessionEnvelope • thiserror • SessionIoError</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Switch between a valid v1 session payload, an unsupported v99 schema, an empty <code>board_id</code>, or truncated JSON below. Observe how <code>BoardModel::from_session_json</code> returns a typed <code>Result</code> without ever panicking or clobbering the active board.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Session Payload Scenario</h4>
      <div class="btn-row" style="flex-direction:column; align-items:stretch; gap:0.45rem;">
        <button class="action-btn" type="button" data-scenario="valid">1. Valid v1 Envelope (Load OK)</button>
        <button class="action-btn" type="button" data-scenario="version99">2. schema_version: 99 (Reject)</button>
        <button class="action-btn" type="button" data-scenario="emptyid">3. board_id: "   " (Reject)</button>
        <button class="action-btn" type="button" data-scenario="corrupt">4. Truncated "{corrupt" (Reject)</button>
      </div>
    </div>

    <div class="sim-box">
      <h4>2. Raw JSON Input</h4>
      <pre class="language-rust" style="margin:0; font-size:0.74rem; padding:0.75rem;"><code class="language-rust" data-role="serde-json-preview"></code></pre>
    </div>

    <div class="sim-box">
      <h4>3. <code>from_session_json</code> &amp; <code>ViewState</code></h4>
      <pre class="language-rust" style="margin:0; font-size:0.74rem; padding:0.75rem;"><code class="language-rust" data-role="serde-result-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_cow_arc_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initCowArcSimulator("cow-arc-sim");')
    return """<section class="full-width-panel" id="cow-arc-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Copy-on-Write <code>Arc::make_mut</code> &amp; <code>Arc::ptr_eq</code> Visualizer</span>
    <span class="panel-badge">Arc&lt;Vec&lt;T&gt;&gt; • Arc::make_mut • shared_arc_buffers</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Mutate individual collections on <code>working: BoardModel</code> or take/restore a baseline snapshot. Watch how <code>Arc::make_mut</code> clones <em>only</em> the touched collection when <code>strong_count &gt; 1</code>, leaving untouched buffers pointer-shared (<code>Arc::ptr_eq == true</code>).
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Document Operations</h4>
      <div class="btn-row" style="flex-direction:column; align-items:stretch; gap:0.45rem;">
        <button class="action-btn" type="button" data-cow="pin">Mutate <code>review_pins</code> (<code>Arc::make_mut</code>)</button>
        <button class="action-btn" type="button" data-cow="pad">Mutate <code>pads</code> (<code>Arc::make_mut</code>)</button>
        <button class="action-btn" type="button" data-cow="snapshot">Take Baseline Snapshot (<code>O(1) clone</code>)</button>
        <button class="action-btn" type="button" data-cow="restore">Restore Snapshot (<code>O(1) clone</code>)</button>
      </div>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:225px; padding:0.75rem;">
      <div data-role="cow-bars" style="font-family:var(--font-mono); font-size:0.78rem; width:100%;"></div>
    </div>

    <div class="sim-box">
      <h4>2. Pointer Sharing Telemetry</h4>
      <pre class="language-rust" style="margin:0; font-size:0.75rem; padding:0.75rem;"><code class="language-rust" data-role="cow-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_undo_coalesce_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initUndoCoalesceSimulator("undo-coalesce-sim");')
    return """<section class="full-width-panel" id="undo-coalesce-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: <code>BoardHistory</code> Undo/Redo &amp; Slider Drag Coalescing</span>
    <span class="panel-badge">VecDeque&lt;HistoryEntry&gt; • coalesce_key • commit_coalesce</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Drag the <strong>Pin #1 X (mm)</strong> slider across multiple intermediate values, then click <strong>Release Drag</strong> or <strong>Undo (Z)</strong>. Notice how 60 intermediate slider frames sharing <code>Some("drag_pin_1_x")</code> coalesce into a single undo entry that restores the exact pre-drag coordinate in one step!
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Continuous vs. Discrete Edits</h4>
      <label style="display:block; font-family:var(--font-mono); font-size:0.78rem; margin-bottom:0.25rem;">
        Slider: Pin #1 X (<span data-role="pin-x-label">20.0</span> mm)
      </label>
      <input type="range" data-param="pinX" min="10" max="85" step="2.5" value="20" style="width:100%; margin-bottom:0.55rem;">
      <div class="btn-row" style="flex-wrap:wrap; gap:0.4rem;">
        <button class="action-btn" type="button" data-hist="commit">Release Drag (<code>commit_coalesce</code>)</button>
        <button class="action-btn" type="button" data-hist="discrete">Discrete Edit (+Pin)</button>
        <button class="action-btn" type="button" data-hist="undo">Undo (Z)</button>
        <button class="action-btn" type="button" data-hist="redo">Redo (Y)</button>
      </div>
    </div>

    <div class="sim-box">
      <h4>2. <code>undo_stack</code> &amp; <code>redo_stack</code></h4>
      <div data-role="undo-stack-list" style="font-family:var(--font-mono); font-size:0.76rem;"></div>
    </div>

    <div class="sim-box">
      <h4>3. <code>BoardHistory</code> State</h4>
      <pre class="language-rust" style="margin:0; font-size:0.74rem; padding:0.75rem;"><code class="language-rust" data-role="undo-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_command_palette_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initCommandPaletteSimulator("command-palette-sim");')
    return """<section class="full-width-panel" id="command-palette-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Unified <code>CommandSpec</code> Registry &amp; <code>panic_hunt</code> Fuzzer</span>
    <span class="panel-badge">CommandSpec • fn(&amp;mut BoardModel, ...) • panic_hunt</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Filter the command palette below, dispatch valid JSON commands (<code>review.addPin</code>, <code>review.clearPins</code>, <code>history.undo</code>, <code>session.inspect</code>), or click <strong>Run <code>panic_hunt</code> Fuzzer</strong> to bombard every registered <code>CommandSpec</code> with malformed/out-of-range JSON payloads.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Command Palette (<code>Key::K</code>)</h4>
      <input type="text" data-role="cmd-filter" placeholder="Filter commands (e.g. review, undo)..." style="width:100%; padding:0.4rem 0.55rem; margin-bottom:0.55rem; background:#0d1117; color:#e6edf3; border:1px solid #30363d; border-radius:4px; font-family:var(--font-mono); font-size:0.78rem;">
      <div data-role="cmd-list" style="display:flex; flex-direction:column; gap:0.35rem; margin-bottom:0.65rem;"></div>
      <button type="button" class="egui-btn" data-cmd-action="fuzz" style="width:100%;">Run <code>panic_hunt</code> Adversarial Fuzzer</button>
    </div>

    <div class="sim-box">
      <h4>2. Precheck &amp; Enabled Guard</h4>
      <div data-role="cmd-precheck" style="font-family:var(--font-mono); font-size:0.76rem; line-height:1.5;"></div>
    </div>

    <div class="sim-box">
      <h4>3. Dispatch Result (<code>Result&lt;Value, EngineError&gt;</code>)</h4>
      <pre class="language-rust" style="margin:0; font-size:0.74rem; padding:0.75rem;"><code class="language-rust" data-role="cmd-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_viewport_cull_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initViewportCullSimulator("viewport-cull-sim");')
    return """<section class="full-width-panel" id="viewport-cull-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Spatial Viewport Culling (<code>Rect::intersects</code>) &amp; Frame Budget</span>
    <span class="panel-badge">Rect::intersects • Vec::with_capacity • PerfBudget</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Drag the camera X/Y and viewport width sliders below. Watch how <code>culled_pads_in_rect</code> and <code>culled_traces_in_rect</code> test each primitive's axis-aligned bounding box (<code>bounds_mm().intersects(viewport_mm)</code>) so off-screen geometry is skipped before <code>Painter</code> tessellation.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Camera Frustum (<code>pcb_scene_rect</code>)</h4>
      <label style="display:block; font-size:0.8rem; margin-bottom:0.45rem;">
        Camera Min X (<code>mm</code>):
        <input type="range" data-cull="minX" min="0" max="60" value="10" style="width:100%;">
      </label>
      <label style="display:block; font-size:0.8rem; margin-bottom:0.45rem;">
        Camera Width (<code>mm</code>):
        <input type="range" data-cull="width" min="20" max="90" value="42" style="width:100%;">
      </label>
      <label class="egui-checkbox" style="margin-top:0.4rem;">
        <input type="checkbox" data-cull="enabled" checked>
        Enable Viewport Culling
      </label>
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:215px;">
      <svg viewBox="0 0 320 200" width="100%" height="200" aria-label="Spatial Viewport Culling Preview">
        <rect x="16" y="14" width="288" height="172" rx="6" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
        <g data-svg="cull-items"></g>
        <rect data-svg="cull-box" x="45" y="26" width="120" height="148" fill="rgba(61, 214, 140, 0.08)" stroke="#3dd68c" stroke-width="2" stroke-dasharray="5 3"/>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. <code>ViewportCullStats</code> &amp; Budget</h4>
      <pre class="language-rust" style="margin:0; font-size:0.74rem; padding:0.75rem;"><code class="language-rust" data-role="cull-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_async_import_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initAsyncImportSimulator("async-import-sim");')
    return """<section class="full-width-panel" id="async-import-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Non-Blocking Background Import (<code>mpsc::channel</code>)</span>
    <span class="panel-badge">std::thread::spawn • mpsc::Receiver::try_recv • ProgressBar</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Start a background session import below and step through non-blocking <code>try_recv()</code> frames—or cancel mid-flight by dropping the <code>Receiver&lt;ImportJobMessage&gt;</code>. Notice how the UI thread stays 100% responsive and requests repaints via <code>ctx.request_repaint()</code> while <code>ImportJobState::Running</code> is active.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Worker &amp; Channel Controls</h4>
      <div style="display:flex; flex-direction:column; gap:0.4rem;">
        <button type="button" class="egui-btn" data-import="start-valid">Spawn Valid JSON Import</button>
        <button type="button" class="egui-btn" data-import="start-corrupt">Spawn Corrupt JSON Import</button>
        <button type="button" class="egui-btn" data-import="step">Poll Next Frame (<code>try_recv</code>)</button>
        <button type="button" class="egui-btn" data-import="cancel">Cancel Import (Drop <code>rx</code>)</button>
      </div>
    </div>

    <div class="sim-box">
      <h4>2. Simulated <code>egui::ProgressBar</code></h4>
      <div style="background:#0d1117; border:1px solid #30363d; border-radius:4px; height:24px; overflow:hidden; margin-bottom:0.55rem; position:relative;">
        <div data-role="import-bar" style="background:#2ea043; height:100%; width:0%; transition:width 0.15s ease;"></div>
        <span data-role="import-bar-label" style="position:absolute; inset:0; display:flex; align-items:center; justify-content:center; font-family:var(--font-mono); font-size:0.75rem; color:#e6edf3;">Idle</span>
      </div>
      <div data-role="import-queue" style="font-family:var(--font-mono); font-size:0.75rem; line-height:1.45;"></div>
    </div>

    <div class="sim-box">
      <h4>3. <code>ImportJobState</code> on UI Thread</h4>
      <pre class="language-rust" style="margin:0; font-size:0.74rem; padding:0.75rem;"><code class="language-rust" data-role="import-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_snapshot_wasm_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initSnapshotWasmSimulator("snapshot-wasm-sim");')
    return """<section class="full-width-panel" id="snapshot-wasm-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Offscreen Snapshot Digest (<code>gpu_lock</code>) &amp; <code>wasm32</code> Gate</span>
    <span class="panel-badge">gpu_lock() • FrameSnapshotDigest • cfg(target_arch = "wasm32")</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Toggle layers or DRC clearance below and compare the live 64-bit FNV-1a <code>FrameSnapshotDigest</code> against the pinned golden baseline. Any visual regression in visible layers, pad/trace counts, or DRC halos immediately flips the snapshot check to <code>MISMATCH</code>.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. Offscreen Render Controls</h4>
      <label class="egui-checkbox"><input type="checkbox" data-snap="top" checked> Top Copper (bit 0)</label>
      <label class="egui-checkbox"><input type="checkbox" data-snap="bot" checked> Bottom Copper (bit 1)</label>
      <label class="egui-checkbox"><input type="checkbox" data-snap="silk" checked> Silkscreen (bit 2)</label>
      <label class="egui-checkbox"><input type="checkbox" data-snap="rats"> Ratsnest (bit 3)</label>
      <div style="display:flex; gap:0.4rem; flex-wrap:wrap; margin-top:0.55rem;">
        <button type="button" class="egui-btn" data-snap-act="bless">Bless Current as Baseline</button>
        <button type="button" class="egui-btn" data-snap-act="target">Toggle Target (<code>native</code> / <code>wasm32</code>)</button>
      </div>
    </div>

    <div class="sim-box">
      <h4>2. Snapshot Parity &amp; <code>gpu_lock()</code></h4>
      <div data-role="snap-diff" style="font-family:var(--font-mono); font-size:0.76rem; line-height:1.5;"></div>
    </div>

    <div class="sim-box">
      <h4>3. <code>FrameSnapshotDigest</code> &amp; Scorecard</h4>
      <pre class="language-rust" style="margin:0; font-size:0.74rem; padding:0.75rem;"><code class="language-rust" data-role="snap-state"></code></pre>
    </div>
  </div>
</section>"""


def widget_layer_swatch_sim(scripts_out: list[str]) -> str:
    scripts_out.append('    initLayerSwatchSimulator("layer-swatch-sim");')
    return """<section class="full-width-panel" id="layer-swatch-sim">
  <div class="panel-header">
    <span class="panel-title">Interactive Lab 1: Custom <code>LayerSwatch&lt;'a&gt;</code> Widget &amp; Alpha Blending</span>
    <span class="panel-badge">impl&lt;'a&gt; egui::Widget • color_with_opacity • Harness::new_ui_state</span>
  </div>
  <p style="font-family: var(--font-sans); font-size: 0.9rem; margin-bottom: 1rem;">
    Toggle visibility or drag the opacity sliders in the simulated <code>LayerSwatch&lt;'a&gt;</code> rows below. Watch how each widget borrows <code>&amp;'a mut bool</code> and <code>&amp;'a mut f32</code> for the duration of the frame to blend copper layers on the canvas.
  </p>

  <div class="sim-grid">
    <div class="sim-box">
      <h4>1. <code>LayerSwatch&lt;'a&gt;</code> Rows</h4>
      <label class="egui-checkbox" style="margin-bottom:0.35rem;">
        <input type="checkbox" data-param="topVis" checked>
        <span style="color:#e05243;">■</span> Top Copper (F.Cu)
      </label>
      <input type="range" data-param="topAlpha" min="0.1" max="1.0" step="0.05" value="0.75" style="width:100%; margin-bottom:0.75rem;">

      <label class="egui-checkbox" style="margin-bottom:0.35rem;">
        <input type="checkbox" data-param="botVis" checked>
        <span style="color:#3b8eea;">■</span> Bottom Copper (B.Cu)
      </label>
      <input type="range" data-param="botAlpha" min="0.1" max="1.0" step="0.05" value="1.0" style="width:100%;">
    </div>

    <div class="egui-central-canvas" style="border-radius:6px; border:1px solid #3c3c3c; min-height:225px;">
      <svg viewBox="0 0 320 210" width="100%" height="210" aria-label="Alpha Blended PCB Layers">
        <rect x="20" y="16" width="280" height="178" rx="6" fill="#10281e" stroke="#2f6f52" stroke-width="2"/>
        <g data-svg="swatch-bottom">
          <path d="M 60 110 L 260 110" stroke="#3b8eea" stroke-width="20" stroke-linecap="round"/>
        </g>
        <g data-svg="swatch-top" style="opacity:0.75;">
          <path d="M 160 40 L 160 175" stroke="#e05243" stroke-width="20" stroke-linecap="round"/>
          <rect x="142" y="92" width="36" height="36" rx="4" fill="#e05243"/>
        </g>
      </svg>
    </div>

    <div class="sim-box">
      <h4>2. Borrowed Widget &amp; Alpha State</h4>
      <pre class="language-rust" style="margin:0; font-size:0.75rem; padding:0.75rem;"><code class="language-rust" data-role="swatch-state"></code></pre>
    </div>
  </div>
</section>"""


def filter_lessons(
    lessons: list[dict[str, Any]],
    *,
    act: str | None = None,
    range_spec: str | None = None,
) -> list[dict[str, Any]]:
    """Filter lesson records by Act or numeric range."""
    filtered = lessons
    if range_spec:
        parts = range_spec.split("..")
        if len(parts) == 2:
            try:
                start_n, end_n = int(parts[0].strip()), int(parts[1].strip())
                filtered = [
                    item
                    for item in filtered
                    if start_n <= int(item["nav_id"]) <= end_n
                ]
            except ValueError:
                pass

    if act:
        act_str = act.strip().lower()
        act_ranges = {
            "0": (0, 1),
            "foundation": (0, 1),
            "mission": (0, 1),
            "1": (2, 5),
            "i": (2, 5),
            "act-1": (2, 5),
            "act-i": (2, 5),
            "2": (6, 9),
            "ii": (6, 9),
            "act-2": (6, 9),
            "act-ii": (6, 9),
            "3": (10, 13),
            "iii": (10, 13),
            "act-3": (10, 13),
            "act-iii": (10, 13),
            "4": (14, 17),
            "iv": (14, 17),
            "act-4": (14, 17),
            "act-iv": (14, 17),
            "5": (18, 21),
            "v": (18, 21),
            "act-5": (18, 21),
            "act-v": (18, 21),
        }
        if act_str in act_ranges:
            start_n, end_n = act_ranges[act_str]
            filtered = [
                item
                for item in filtered
                if start_n <= int(item["nav_id"]) <= end_n
            ]
        else:
            filtered = [
                item
                for item in filtered
                if act_str in str(item["meta"].get("act_badge", "")).lower()
            ]

    return filtered


def render_lesson_cards(
    lessons: list[dict[str, Any]],
    *,
    label: str | None = None,
) -> str:
    """Render `.lesson-grid` cards for `index.html` from lesson frontmatter."""
    cards: list[str] = []
    for item in lessons:
        meta = item["meta"]
        card = meta.get("card", {})
        href = f"./lessons/{item['html_name']}"
        num_str = html.escape(card.get("num", f"Lesson {item['nav_id']}"))
        mutants_str = html.escape(card.get("mutants", ""))
        title_html = render_inline_md(card.get("title", meta.get("title", "")))
        summary_html = render_inline_md(card.get("summary", meta.get("subtitle", "")))
        egui_skill = render_inline_md(card.get("egui_skill", ""))
        rust_concept = render_inline_md(card.get("rust_concept", ""))
        app_dir = html.escape(card.get("app_dir", f"apps/{item['md_path'].stem}"))

        cards.append(
            f'  <article class="lesson-card">\n'
            f"    <div>\n"
            f'      <div class="lesson-card-header">\n'
            f'        <span class="lesson-card-num">{num_str}</span>\n'
            f'        <span class="lesson-card-mutants">{mutants_str}</span>\n'
            f"      </div>\n"
            f'      <h3><a href="{href}">{title_html}</a></h3>\n'
            f"      <p>{summary_html}</p>\n"
            f"    </div>\n"
            f'    <div class="lesson-card-meta">\n'
            f'      <span><strong><code>egui</code> Skill:</strong> {egui_skill}</span>\n'
            f'      <span><strong>Rust Concept:</strong> {rust_concept}</span>\n'
            f'      <span><strong>Runnable App:</strong> <code>{app_dir}</code></span>\n'
            f"    </div>\n"
            f"  </article>"
        )
    aria_label = html.escape(label or "Published lessons", quote=True)
    return (
        f'<section class="lesson-grid" aria-label="{aria_label}">\n'
        + "\n\n".join(cards)
        + "\n</section>"
    )


def render_reference_cards(references: list[dict[str, Any]]) -> str:
    """Render the reference table for `index.html` from reference frontmatter."""
    rows: list[str] = []
    for item in references:
        meta = item["meta"]
        card = meta.get("card", {})
        href = f"./reference/{item['html_name']}"
        title_html = render_inline_md(card.get("title", meta.get("title", "")))
        summary_html = render_inline_md(card.get("summary", meta.get("subtitle", "")))
        rows.append(
            f"      <tr>\n"
            f'        <td><a href="{href}">{title_html}</a></td>\n'
            f"        <td>{summary_html}</td>\n"
            f"      </tr>"
        )
    return (
        '<table class="ref-table">\n'
        "  <thead>\n"
        "    <tr>\n"
        '      <th style="width: 34%;">Reference Sheet</th>\n'
        '      <th style="width: 66%;">What It Covers</th>\n'
        "    </tr>\n"
        "  </thead>\n"
        "  <tbody>\n"
        + "\n".join(rows)
        + "\n  </tbody>\n"
        "</table>"
    )


def enhance_tables(rendered_html: str) -> str:
    """Apply `.ref-table` styling and optional `Table: Caption` captions to Markdown tables."""
    out = TABLE_CAPTION_RE.sub(
        lambda m: f'<table class="ref-table">\n<caption>{m.group(1)}</caption>',
        rendered_html,
    )
    out = out.replace("<table>", '<table class="ref-table">')
    return out


def enhance_headings(rendered_html: str) -> str:
    """Add id attributes and support {#custom-id} on h2-h4 headings."""

    def _sub(m: re.Match[str]) -> str:
        level, attrs, inner = m.group(1), m.group(2) or "", m.group(3)
        id_match = re.search(r"\s*\{#([a-zA-Z0-9_-]+)\}\s*$", inner)
        if id_match:
            slug = id_match.group(1)
            inner = inner[: id_match.start()] + inner[id_match.end() :]
            return f'<h{level} id="{slug}"{attrs}>{inner.strip()}</h{level}>'
        if "id=" not in attrs:
            plain = re.sub(r"<[^>]+>", "", inner)
            slug = re.sub(r"[^a-zA-Z0-9]+", "-", plain.lower()).strip("-")
            if slug:
                return f'<h{level} id="{slug}"{attrs}>{inner}</h{level}>'
        return f'<h{level}{attrs}>{inner}</h{level}>'

    return re.sub(
        r"<h([2-4])(\s+[^>]*)?>(.*?)</h\1>",
        _sub,
        rendered_html,
        flags=re.DOTALL,
    )


def render_glossary_body(md_body: str) -> str:
    """
    Compile `reference/GLOSSARY.md` (`**Term**:\nDefinition\n_Avoid_: ...`) into
    structured `.ref-table` tables grouped by `##` section headings.
    """
    lines = md_body.strip().splitlines()
    blocks: list[str] = []
    rows: list[str] = []

    def _flush_table() -> None:
        if not rows:
            return
        blocks.append(
            '<table class="ref-table">\n'
            "  <thead>\n"
            "    <tr>\n"
            '      <th style="width: 24%;">Term &amp; Syntax</th>\n'
            '      <th style="width: 50%;">Definition</th>\n'
            '      <th style="width: 26%;">Avoid / CAD Context</th>\n'
            "    </tr>\n"
            "  </thead>\n"
            "  <tbody>\n"
            + "\n".join(rows)
            + "\n  </tbody>\n"
            "</table>"
        )
        rows.clear()

    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if not line or line.startswith("# "):
            i += 1
            continue
        if line.startswith("## "):
            _flush_table()
            heading_text = render_inline_md(line[3:].strip())
            blocks.append(f"<h2>{heading_text}</h2>")
            i += 1
            continue
        if line.startswith("**") and line.endswith("**:"):
            term_raw = line[:-1]
            term_html = render_inline_md(term_raw)
            i += 1
            def_lines: list[str] = []
            avoid_line = ""
            while i < len(lines) and lines[i].strip():
                cur = lines[i].strip()
                if cur.startswith(("_Avoid_:", "*Avoid*:")):
                    avoid_line = cur.split(":", 1)[1].strip()
                else:
                    def_lines.append(cur)
                i += 1
            def_html = render_inline_md(" ".join(def_lines))
            avoid_html = (
                f"<em>Avoid:</em> {render_inline_md(avoid_line)}"
                if avoid_line
                else "—"
            )
            rows.append(
                f"    <tr>\n"
                f"      <td>{term_html}</td>\n"
                f"      <td>{def_html}</td>\n"
                f"      <td>{avoid_html}</td>\n"
                f"    </tr>"
            )
            continue
        i += 1

    _flush_table()
    return "\n\n".join(blocks)


def compile_markdown_body(
    md_body: str,
    *,
    source_name: str,
    use_prose_col: bool,
    lessons: list[dict[str, Any]],
    references: list[dict[str, Any]],
    errors: list[str],
    scripts_out: list[str],
) -> str:
    """Compile a Markdown body with custom blocks, fences, widgets, and tables into HTML."""
    inline_placeholders: dict[str, str] = {}
    fullwidth_placeholders: dict[str, str] = {}

    def _stash_inline(html_fragment: str) -> str:
        token = f"@@INLINE_BLOCK_{len(inline_placeholders)}@@"
        inline_placeholders[token] = html_fragment
        return f"\n\n{token}\n\n"

    def _stash_fullwidth(html_fragment: str) -> str:
        token = f"@@FULLWIDTH_BLOCK_{len(fullwidth_placeholders)}@@"
        fullwidth_placeholders[token] = html_fragment
        return f"\n\n{token}\n\n"

    # 1. Extract `:::verify` blocks (full-width)
    text = VERIFY_BLOCK_RE.sub(
        lambda m: _stash_fullwidth(render_verify_block(m.group(1), m.group(2))),
        md_body,
    )

    # 2. Extract `:::exercise` blocks (full-width)
    text = EXERCISE_BLOCK_RE.sub(
        lambda m: _stash_fullwidth(
            render_exercise_block(
                m.group(1),
                m.group(2),
                source_name=source_name,
                errors=errors,
                scripts_out=scripts_out,
            )
        ),
        text,
    )

    # 3. Extract `:::quiz` blocks (full-width)
    text = QUIZ_BLOCK_RE.sub(
        lambda m: _stash_fullwidth(
            render_quiz_block(
                m.group(1),
                m.group(2),
                source_name=source_name,
                errors=errors,
                scripts_out=scripts_out,
            )
        ),
        text,
    )

    # 4. Extract `:::callout` and `:::teacher-prompt` blocks (inline prose width)
    text = CALLOUT_BLOCK_RE.sub(
        lambda m: _stash_inline(
            render_callout_block(m.group(1), m.group(2) or "", m.group(3))
        ),
        text,
    )

    # 5. Extract fenced code and Mermaid blocks (inline prose width)
    text = FENCED_CODE_RE.sub(
        lambda m: _stash_inline(render_code_figure(m.group(1), m.group(2))),
        text,
    )

    # 6. Extract `{{widget:...}}` shortcodes
    def _replace_lesson_cards(match: re.Match[str]) -> str:
        attr_str = match.group(1) or ""
        attrs = parse_kv_attrs(attr_str)
        matched = filter_lessons(
            lessons,
            act=attrs.get("act"),
            range_spec=attrs.get("range"),
        )
        label = attrs.get("label")
        return _stash_fullwidth(render_lesson_cards(matched, label=label))

    text = re.sub(
        r"\{\{widget:lesson-cards(?:[ \t]+([^\}\n]+))?\}\}",
        _replace_lesson_cards,
        text,
    )

    fullwidth_widgets = {
        "{{widget:pcb-layer-sim}}": lambda: widget_pcb_layer_sim(scripts_out),
        "{{widget:painter-canvas-sim}}": lambda: widget_painter_canvas_sim(scripts_out),
        "{{widget:viewport-transform-sim}}": lambda: widget_viewport_transform_sim(scripts_out),
        "{{widget:layer-zorder-sim}}": lambda: widget_layer_zorder_sim(scripts_out),
        "{{widget:hittest-sim}}": lambda: widget_hittest_sim(scripts_out),
        "{{widget:sch-text-sim}}": lambda: widget_sch_text_sim(scripts_out),
        "{{widget:dual-scene-sim}}": lambda: widget_dual_scene_sim(scripts_out),
        "{{widget:bom-scroll-sim}}": lambda: widget_bom_scroll_sim(scripts_out),
        "{{widget:hotkey-ruler-sim}}": lambda: widget_hotkey_ruler_sim(scripts_out),
        "{{widget:review-pins-sim}}": lambda: widget_review_pins_sim(scripts_out),
        "{{widget:review-triage-sim}}": lambda: widget_review_triage_sim(scripts_out),
        "{{widget:drc-minimap-sim}}": lambda: widget_drc_minimap_sim(scripts_out),
        "{{widget:layer-swatch-sim}}": lambda: widget_layer_swatch_sim(scripts_out),
        "{{widget:workspace-layers-sim}}": lambda: widget_workspace_layers_sim(scripts_out),
        "{{widget:serde-session-sim}}": lambda: widget_serde_session_sim(scripts_out),
        "{{widget:cow-arc-sim}}": lambda: widget_cow_arc_sim(scripts_out),
        "{{widget:undo-coalesce-sim}}": lambda: widget_undo_coalesce_sim(scripts_out),
        "{{widget:command-palette-sim}}": lambda: widget_command_palette_sim(scripts_out),
        "{{widget:viewport-cull-sim}}": lambda: widget_viewport_cull_sim(scripts_out),
        "{{widget:async-import-sim}}": lambda: widget_async_import_sim(scripts_out),
        "{{widget:snapshot-wasm-sim}}": lambda: widget_snapshot_wasm_sim(scripts_out),
    }
    for shortcode, factory in fullwidth_widgets.items():
        if shortcode in text:
            text = text.replace(shortcode, _stash_fullwidth(factory()))

    if "{{widget:reference-cards}}" in text:
        text = text.replace(
            "{{widget:reference-cards}}",
            _stash_inline(render_reference_cards(references)),
        )

    # 7. Expand Tufte `{{sidenote:...}}` tokens and render standard Markdown
    text = expand_sidenotes(text)
    rendered = MD.render(text)
    rendered = enhance_headings(rendered)
    rendered = enhance_tables(rendered)
    rendered = rewrite_md_links(rendered)

    # 8. Restore inline stashed blocks
    for token, fragment in inline_placeholders.items():
        rendered = rendered.replace(f"<p>{token}</p>", fragment)
        rendered = rendered.replace(token, fragment)

    if not use_prose_col:
        for token, fragment in fullwidth_placeholders.items():
            rendered = rendered.replace(f"<p>{token}</p>", fragment)
            rendered = rendered.replace(token, fragment)
        return rendered.strip()

    # 9. Split around full-width blocks so prose segments live inside `<div class="prose-col">`
    pattern = re.compile(
        r"(?:<p>)?(@@FULLWIDTH_BLOCK_\d+@@)(?:</p>)?"
    )
    parts = pattern.split(rendered)
    out_chunks: list[str] = []
    for idx, part in enumerate(parts):
        stripped = part.strip()
        if not stripped:
            continue
        if idx % 2 == 1 and stripped in fullwidth_placeholders:
            out_chunks.append(fullwidth_placeholders[stripped])
        else:
            out_chunks.append(f'<div class="prose-col">\n{stripped}\n</div>')

    return "\n\n".join(out_chunks)


def build_nav_html(
    *,
    active_id: str,
    rel_root: str,
    badge_text: str,
    lessons: list[dict[str, Any]],
    references: list[dict[str, Any]],
) -> str:
    """Build `<nav class="course-nav">` for any page in the course."""
    links: list[str] = []
    idx_active = ' class="active"' if active_id == "index" else ""
    idx_href = "./index.html" if rel_root == "" else f"{rel_root}index.html"
    links.append(f'      <a href="{idx_href}"{idx_active}>Index</a>')

    for les in lessons:
        lid = les["nav_id"]
        act = ' class="active"' if active_id == lid else ""
        if rel_root == "":
            href = f"./lessons/{les['html_name']}"
        elif active_id.isdigit():
            href = f"./{les['html_name']}"
        else:
            href = f"../lessons/{les['html_name']}"
        links.append(f'      <a href="{href}"{act}>{html.escape(lid)}</a>')

    for ref in references:
        rid = ref["nav_id"]
        act = ' class="active"' if active_id == rid else ""
        if rel_root == "":
            href = f"./reference/{ref['html_name']}"
        elif not active_id.isdigit() and active_id != "index":
            href = f"./{ref['html_name']}"
        else:
            href = f"../reference/{ref['html_name']}"
        links.append(f'      <a href="{href}"{act}>{html.escape(ref["nav_label"])}</a>')

    links_joined = "\n".join(links)
    return (
        f'  <nav class="course-nav" aria-label="Course navigation">\n'
        f"    <div>\n"
        f"{links_joined}\n"
        f"    </div>\n"
        f'    <span class="mission-badge">{html.escape(badge_text)}</span>\n'
        f"  </nav>"
    )


def render_full_page(
    *,
    meta: dict[str, Any],
    body_html: str,
    active_id: str,
    rel_root: str,
    use_prose_col: bool,
    lessons: list[dict[str, Any]],
    references: list[dict[str, Any]],
    scripts: list[str],
) -> str:
    """Wrap compiled article content in the shared dark-mode Tufte HTML5 document shell."""
    page_title = html.escape(
        str(meta.get("page_title", meta.get("title", "Learn Rust & egui"))).replace(
            "`", ""
        )
    )
    badge_text = str(
        meta.get(
            "badge",
            meta.get("act_badge", "Mission: Schematic & PCB Design Review System"),
        )
    )
    h1_html = render_inline_md(meta.get("title", ""))
    subtitle_html = render_inline_md(meta.get("subtitle", meta.get("lead", "")))

    nav_html = build_nav_html(
        active_id=active_id,
        rel_root=rel_root,
        badge_text=badge_text,
        lessons=lessons,
        references=references,
    )

    heading_block = f"<h1>{h1_html}</h1>\n<p class=\"subtitle\">{subtitle_html}</p>"
    if use_prose_col:
        if body_html.startswith('<div class="prose-col">\n'):
            body_combined = (
                '<div class="prose-col">\n'
                + heading_block
                + "\n\n"
                + body_html[len('<div class="prose-col">\n') :]
            )
        else:
            body_combined = (
                f'<div class="prose-col">\n{heading_block}\n</div>\n\n{body_html}'
            )
    else:
        body_combined = f"{heading_block}\n\n{body_html}"

    css_href = "./assets/style.css" if rel_root == "" else f"{rel_root}assets/style.css"
    js_src = "./assets/course.js" if rel_root == "" else f"{rel_root}assets/course.js"

    extra_script_html = ""
    if scripts:
        joined_scripts = "\n\n".join(scripts)
        extra_script_html = f"\n  <script>\n{joined_scripts}\n  </script>"

    return f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="color-scheme" content="dark">
  <title>{page_title}</title>
  <link rel="stylesheet" href="{css_href}">
</head>
<body>
  <header>
{nav_html}
  </header>

  <main>
    <article class="tufte-article">
{body_combined}
    </article>
  </main>

  <footer style="margin-top: 3.5rem; padding-top: 1rem; border-top: 1px solid var(--border-subtle); font-family: var(--font-sans); font-size: 0.8rem; color: var(--text-muted); display: flex; justify-content: space-between; flex-wrap: wrap; gap: 0.75rem;">
    <span>Licensed under the <a href="https://www.apache.org/licenses/LICENSE-2.0" target="_blank" rel="noopener">Apache License, Version 2.0</a>.</span>
    <span>This is not an official Google product.</span>
  </footer>

  <script src="{js_src}"></script>{extra_script_html}
</body>
</html>
"""


def verify_internal_links(
    out_dir: Path, html_files: list[Path], errors: list[str]
) -> None:
    """Verify that every relative `href="..."` or `src="..."` in generated HTML resolves."""
    attr_re = re.compile(r'(?:href|src)="([^"#]+)')
    for html_path in html_files:
        content = html_path.read_text(encoding="utf-8")
        for match in attr_re.finditer(content):
            target = match.group(1).strip()
            if not target or target.startswith(
                ("http://", "https://", "mailto:", "javascript:", "data:")
            ):
                continue
            resolved = (html_path.parent / target).resolve()
            if not resolved.exists():
                errors.append(
                    f"{html_path.relative_to(out_dir)}: Broken relative link '{target}'"
                )


def build_site(out_dir: Path, *, check: bool = False) -> int:
    """Compile all course Markdown files into `out_dir`."""
    errors: list[str] = []
    lessons_dir = ROOT_DIR / "lessons"
    ref_dir = ROOT_DIR / "reference"

    # 1. Discover and parse all lessons in sorted order
    lesson_records: list[dict[str, Any]] = []
    for md_path in sorted(lessons_dir.glob("[0-9][0-9][0-9][0-9]-*.md")):
        meta, body = parse_frontmatter(md_path)
        nav_id = str(meta.get("nav_id", md_path.stem.split("-", 1)[0]))
        card = meta.get("card") or {}
        for req_card_key in ("mutants", "egui_skill", "rust_concept"):
            if not str(card.get(req_card_key, "")).strip():
                errors.append(
                    f"lessons/{md_path.name}: Missing required `card.{req_card_key}` in YAML frontmatter."
                )
        lesson_records.append(
            {
                "md_path": md_path,
                "html_name": f"{md_path.stem}.html",
                "nav_id": nav_id,
                "meta": meta,
                "body": body,
            }
        )

    # 2. Discover and parse all reference sheets in canonical order
    ref_records: list[dict[str, Any]] = []
    for md_name, html_name, default_label in REFERENCE_ORDER:
        md_path = ref_dir / md_name
        if not md_path.is_file():
            errors.append(f"Missing reference Markdown file: {md_path}")
            continue
        meta, body = parse_frontmatter(md_path)
        ref_records.append(
            {
                "md_path": md_path,
                "html_name": html_name,
                "nav_id": html_name.removesuffix(".html"),
                "nav_label": meta.get("nav_label", default_label),
                "meta": meta,
                "body": body,
                "is_glossary": md_name == "GLOSSARY.md",
            }
        )

    # Prepare output directories
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "lessons").mkdir(parents=True, exist_ok=True)
    (out_dir / "reference").mkdir(parents=True, exist_ok=True)

    # Copy `.nojekyll` and `assets/` when building to an external directory (such as `_site/`)
    if out_dir.resolve() != ROOT_DIR.resolve():
        nojekyll = ROOT_DIR / ".nojekyll"
        if nojekyll.is_file():
            shutil.copy2(nojekyll, out_dir / ".nojekyll")
        out_assets = out_dir / "assets"
        if out_assets.exists():
            shutil.rmtree(out_assets)
        shutil.copytree(ROOT_DIR / "assets", out_assets)

    written_html: list[Path] = []

    # 3. Render `index.md` -> `index.html`
    index_md = ROOT_DIR / "index.md"
    if not index_md.is_file():
        errors.append(f"Missing course index file: {index_md}")
    else:
        idx_meta, idx_body = parse_frontmatter(index_md)
        idx_scripts: list[str] = []
        idx_body_html = compile_markdown_body(
            idx_body,
            source_name="index.md",
            use_prose_col=True,
            lessons=lesson_records,
            references=ref_records,
            errors=errors,
            scripts_out=idx_scripts,
        )
        idx_html = render_full_page(
            meta=idx_meta,
            body_html=idx_body_html,
            active_id="index",
            rel_root="",
            use_prose_col=True,
            lessons=lesson_records,
            references=ref_records,
            scripts=idx_scripts,
        )
        idx_out = out_dir / "index.html"
        idx_out.write_text(idx_html, encoding="utf-8")
        written_html.append(idx_out)

    # 4. Render lessons
    for les in lesson_records:
        les_scripts: list[str] = []
        body_html = compile_markdown_body(
            les["body"],
            source_name=f"lessons/{les['md_path'].name}",
            use_prose_col=True,
            lessons=lesson_records,
            references=ref_records,
            errors=errors,
            scripts_out=les_scripts,
        )
        page_html = render_full_page(
            meta=les["meta"],
            body_html=body_html,
            active_id=les["nav_id"],
            rel_root="../",
            use_prose_col=True,
            lessons=lesson_records,
            references=ref_records,
            scripts=les_scripts,
        )
        out_path = out_dir / "lessons" / les["html_name"]
        out_path.write_text(page_html, encoding="utf-8")
        written_html.append(out_path)

    # 5. Render reference pages
    for ref in ref_records:
        ref_scripts: list[str] = []
        if ref["is_glossary"]:
            body_html = render_glossary_body(ref["body"])
        else:
            body_html = compile_markdown_body(
                ref["body"],
                source_name=f"reference/{ref['md_path'].name}",
                use_prose_col=False,
                lessons=lesson_records,
                references=ref_records,
                errors=errors,
                scripts_out=ref_scripts,
            )
        page_html = render_full_page(
            meta=ref["meta"],
            body_html=body_html,
            active_id=ref["nav_id"],
            rel_root="../",
            use_prose_col=False,
            lessons=lesson_records,
            references=ref_records,
            scripts=ref_scripts,
        )
        out_path = out_dir / "reference" / ref["html_name"]
        out_path.write_text(page_html, encoding="utf-8")
        written_html.append(out_path)

    if check:
        verify_internal_links(out_dir, written_html, errors)

    print(
        f"Built {len(written_html)} HTML pages into {out_dir} "
        f"(1 index, {len(lesson_records)} lessons, {len(ref_records)} reference sheets)."
    )
    if errors:
        for err in errors:
            print(f"[ERROR] {err}", file=sys.stderr)
        return 1
    if check:
        print(
            "✓ All Markdown frontmatter, links, and equal-word-count quiz checks passed."
        )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build the Learn Rust & egui course HTML site."
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=ROOT_DIR,
        help="Destination directory for built HTML files (default: in-place at repo root).",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify internal links and quiz option word-count parity.",
    )
    args = parser.parse_args()
    return build_site(args.out_dir.resolve(), check=args.check)


if __name__ == "__main__":
    raise SystemExit(main())
