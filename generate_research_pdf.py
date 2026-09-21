#!/usr/bin/env python3
"""
Render a Markdown research document to PDF.

Replaces an earlier version of this script that hardcoded only sections 1-2 of the
report and would silently produce a 2-section PDF. This one reads the Markdown as
the single source of truth, so the PDF can never drift from the document.

Usage:
    python3 generate_research_pdf.py [INPUT.md] [OUTPUT.pdf]

Defaults to EVO_INVESTOR_RESEARCH_FOUNDATION.md -> EVO_Investor_Research_Foundation.pdf
in the same directory as this script.

Requires: pip install reportlab
"""

import os
import re
import sys
from datetime import datetime

from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER, TA_LEFT
from reportlab.lib.pagesizes import letter
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import inch
from reportlab.platypus import (
    KeepTogether,
    PageBreak,
    Paragraph,
    SimpleDocTemplate,
    Spacer,
    Table,
    TableStyle,
)

INK = colors.HexColor("#1a1a1a")
SLATE = colors.HexColor("#2c3e50")
GREY = colors.HexColor("#7f8c8d")
RULE = colors.HexColor("#d5dbdb")
BAND = colors.HexColor("#f4f6f7")


def build_styles():
    ss = getSampleStyleSheet()
    add = ss.add
    add(ParagraphStyle(name="CoverTitle", parent=ss["Heading1"], fontSize=30,
                       leading=34, textColor=INK, alignment=TA_CENTER,
                       fontName="Helvetica-Bold", spaceAfter=6))
    add(ParagraphStyle(name="CoverSub", parent=ss["Normal"], fontSize=13,
                       leading=17, textColor=GREY, alignment=TA_CENTER))
    add(ParagraphStyle(name="H1", parent=ss["Heading1"], fontSize=16, leading=20,
                       textColor=SLATE, fontName="Helvetica-Bold",
                       spaceBefore=20, spaceAfter=10))
    add(ParagraphStyle(name="H2", parent=ss["Heading2"], fontSize=12, leading=16,
                       textColor=SLATE, fontName="Helvetica-Bold",
                       spaceBefore=13, spaceAfter=6))
    add(ParagraphStyle(name="H3", parent=ss["Heading3"], fontSize=10.5, leading=14,
                       textColor=SLATE, fontName="Helvetica-Bold",
                       spaceBefore=10, spaceAfter=4))
    add(ParagraphStyle(name="Prose", parent=ss["Normal"], fontSize=9.5, leading=14,
                       alignment=TA_LEFT, spaceAfter=8))
    add(ParagraphStyle(name="Bullet", parent=ss["Normal"], fontSize=9.5, leading=13.5,
                       leftIndent=16, bulletIndent=5, spaceAfter=3))
    add(ParagraphStyle(name="Quote", parent=ss["Normal"], fontSize=9, leading=13,
                       leftIndent=16, textColor=SLATE, spaceAfter=8,
                       fontName="Helvetica-Oblique"))
    add(ParagraphStyle(name="Cell", parent=ss["Normal"], fontSize=7.5, leading=10))
    add(ParagraphStyle(name="CellHead", parent=ss["Normal"], fontSize=7.5, leading=10,
                       fontName="Helvetica-Bold"))
    return ss


def inline(text):
    """Convert inline Markdown to reportlab markup, escaping XML first."""
    text = text.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")
    text = re.sub(r"\*\*\*(.+?)\*\*\*", r"<b><i>\1</i></b>", text)
    text = re.sub(r"\*\*(.+?)\*\*", r"<b>\1</b>", text)
    text = re.sub(r"(?<!\*)\*([^*]+?)\*(?!\*)", r"<i>\1</i>", text)
    text = re.sub(r"~~(.+?)~~", r"<strike>\1</strike>", text)
    text = re.sub(r"`(.+?)`", r'<font face="Courier">\1</font>', text)
    text = re.sub(r"\[(.+?)\]\((.+?)\)", r'<link href="\2" color="#2c3e50">\1</link>', text)
    return text


def flush_table(rows, styles):
    """Turn collected Markdown table rows into a reportlab Table."""
    if not rows:
        return None
    header, body = rows[0], rows[1:]
    ncols = max(len(r) for r in rows)
    data = []
    for i, row in enumerate(rows):
        padded = row + [""] * (ncols - len(row))
        style = styles["CellHead"] if i == 0 else styles["Cell"]
        data.append([Paragraph(inline(c), style) for c in padded])
    avail = letter[0] - 120
    tbl = Table(data, colWidths=[avail / ncols] * ncols, repeatRows=1)
    tbl.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, 0), BAND),
        ("GRID", (0, 0), (-1, -1), 0.4, RULE),
        ("VALIGN", (0, 0), (-1, -1), "TOP"),
        ("TOPPADDING", (0, 0), (-1, -1), 4),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 4),
        ("LEFTPADDING", (0, 0), (-1, -1), 5),
        ("RIGHTPADDING", (0, 0), (-1, -1), 5),
    ]))
    return tbl


def parse_markdown(md, styles):
    story = []
    lines = md.split("\n")
    table_rows = []
    in_code = False
    code_buf = []

    def close_table():
        if table_rows:
            tbl = flush_table(table_rows, styles)
            if tbl is not None:
                story.append(Spacer(1, 4))
                story.append(tbl)
                story.append(Spacer(1, 10))
            table_rows.clear()

    for raw in lines:
        line = raw.rstrip()

        if line.strip().startswith("```"):
            close_table()
            if in_code:
                body = "\n".join(code_buf).replace("&", "&amp;")
                body = body.replace("<", "&lt;").replace(">", "&gt;")
                body = body.replace("\n", "<br/>").replace("  ", "&nbsp;&nbsp;")
                story.append(Paragraph(
                    f'<font face="Courier" size="7.5">{body}</font>', styles["Prose"]))
                code_buf.clear()
            in_code = not in_code
            continue
        if in_code:
            code_buf.append(raw)
            continue

        stripped = line.strip()

        # Table rows
        if stripped.startswith("|") and stripped.endswith("|"):
            cells = [c.strip() for c in stripped.strip("|").split("|")]
            if all(re.fullmatch(r":?-{2,}:?", c) for c in cells if c):
                continue  # separator row
            table_rows.append(cells)
            continue
        close_table()

        if not stripped:
            continue

        if re.fullmatch(r"-{3,}|\*{3,}|_{3,}", stripped):
            story.append(Spacer(1, 6))
            continue

        m = re.match(r"^(#{1,6})\s+(.*)", stripped)
        if m:
            level, text = len(m.group(1)), m.group(2)
            if level == 1:
                story.append(PageBreak())
                story.append(Paragraph(inline(text), styles["H1"]))
            elif level == 2:
                story.append(Paragraph(inline(text), styles["H1"]))
            elif level == 3:
                story.append(Paragraph(inline(text), styles["H2"]))
            else:
                story.append(Paragraph(inline(text), styles["H3"]))
            continue

        if stripped.startswith(">"):
            story.append(Paragraph(inline(stripped.lstrip("> ").strip()), styles["Quote"]))
            continue

        m = re.match(r"^[-*+]\s+(.*)", stripped)
        if m:
            story.append(Paragraph(inline(m.group(1)), styles["Bullet"], bulletText="•"))
            continue

        m = re.match(r"^(\d+)[.)]\s+(.*)", stripped)
        if m:
            story.append(Paragraph(inline(m.group(2)), styles["Bullet"],
                                   bulletText=f"{m.group(1)}."))
            continue

        story.append(Paragraph(inline(stripped), styles["Prose"]))

    close_table()
    return story


def footer(canvas, doc):
    canvas.saveState()
    canvas.setFont("Helvetica", 7.5)
    canvas.setFillColor(GREY)
    canvas.drawCentredString(letter[0] / 2, 22, str(canvas.getPageNumber()))
    canvas.restoreState()


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    src = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        here, "EVO_INVESTOR_RESEARCH_FOUNDATION.md")
    out = sys.argv[2] if len(sys.argv) > 2 else os.path.join(
        here, "EVO_Investor_Research_Foundation.pdf")

    if not os.path.exists(src):
        sys.exit(f"Input not found: {src}")

    with open(src, encoding="utf-8") as f:
        md = f.read()

    styles = build_styles()
    story = [Spacer(1, 2.2 * inch),
             Paragraph("EVO", styles["CoverTitle"]),
             Paragraph("Investor Deck Research Foundation", styles["CoverTitle"]),
             Spacer(1, 0.35 * inch),
             Paragraph("Pre-Seed Fundraising Intelligence Report", styles["CoverSub"]),
             Spacer(1, 0.12 * inch),
             Paragraph(f"Rendered {datetime.now().strftime('%B %d, %Y')} from "
                       f"{os.path.basename(src)}", styles["CoverSub"])]

    body = parse_markdown(md, styles)
    # Drop a leading PageBreak so the cover isn't followed by a blank page.
    while body and isinstance(body[0], PageBreak):
        body.pop(0)
    story.append(PageBreak())
    story.extend(body)

    doc = SimpleDocTemplate(out, pagesize=letter, title="EVO Investor Research Foundation",
                            author="Evo", rightMargin=60, leftMargin=60,
                            topMargin=58, bottomMargin=42)
    doc.build(story, onFirstPage=footer, onLaterPages=footer)
    print(f"Wrote {out} ({os.path.getsize(out) / 1024:.0f} KB)")


if __name__ == "__main__":
    main()
