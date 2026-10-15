"""Render docs/IMPROVEMENT_PLAN.txt to docs/IMPROVEMENT_PLAN.pdf.

The .txt is the source of truth; run this after editing it:
    python scripts/plan_to_pdf.py
Needs reportlab (pip install reportlab).
"""
import os
import re
from xml.sax.saxutils import escape

from reportlab.lib import colors
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import mm
from reportlab.platypus import Paragraph, Preformatted, SimpleDocTemplate, Spacer

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "docs", "IMPROVEMENT_PLAN.txt")
OUT = os.path.join(ROOT, "docs", "IMPROVEMENT_PLAN.pdf")

ss = getSampleStyleSheet()
H1 = ParagraphStyle("h1", parent=ss["Title"], fontSize=18, spaceAfter=8)
H2 = ParagraphStyle("h2", parent=ss["Heading2"], textColor=colors.HexColor("#1f3a5f"), spaceBefore=10)
H3 = ParagraphStyle("h3", parent=ss["Heading4"], spaceBefore=8, spaceAfter=2)
BODY = ParagraphStyle("body", parent=ss["BodyText"], fontSize=10, leading=13)
BULLET = ParagraphStyle("bullet", parent=BODY, leftIndent=14, bulletIndent=4)
CODE = ParagraphStyle("code", parent=ss["Code"], fontSize=9, leftIndent=14)


def blocks(lines):
    """Yield flowables. Heading = line followed by ==== or ----; '- ' = bullet;
    'I-n'/'Phase n' lines = sub-heading; 4-space indent = code."""
    i = 0
    para = []

    def flush():
        if para:
            # a block of "Key: value" lines (the header) keeps its line breaks
            sep = "<br/>" if all(re.match(r"^[A-Z][\w ]{0,20}:", l) for l in para) else " "
            out.append(Paragraph(sep.join(escape(l) for l in para), BODY))
            para.clear()

    out = []
    while i < len(lines):
        line = lines[i].rstrip()
        nxt = lines[i + 1].rstrip() if i + 1 < len(lines) else ""
        if nxt and set(nxt) <= {"="} and len(nxt) >= 3:
            flush(); out.append(Paragraph(escape(line), H1)); i += 2; continue
        if nxt and set(nxt) <= {"-"} and len(nxt) >= 3:
            flush(); out.append(Paragraph(escape(line), H2)); i += 2; continue
        if not line:
            flush(); i += 1; continue
        if line.startswith("    "):
            flush(); out.append(Preformatted(line.strip(), CODE)); i += 1; continue
        if line.startswith("- "):
            flush()
            text = escape(line[2:])
            # Helvetica has no checkbox glyphs, so state is spelled out
            text = re.sub(r"^\[x\]", '<font color="#2e7d32"><b>DONE</b></font>', text)
            text = re.sub(r"^\[ \]", '<font color="#b26a00"><b>TODO</b></font>', text)
            out.append(Paragraph(text, BULLET, bulletText="•"))
            i += 1; continue
        if re.match(r"^(I-\d+ |Phase \d|Housekeeping)", line):
            flush(); out.append(Paragraph("<b>%s</b>" % escape(line), H3)); i += 1; continue
        para.append(line.strip()); i += 1
    flush()
    return out


def main():
    with open(SRC, encoding="utf-8") as f:
        lines = f.read().splitlines()
    doc = SimpleDocTemplate(OUT, pagesize=A4, leftMargin=18 * mm, rightMargin=18 * mm,
                            topMargin=16 * mm, bottomMargin=16 * mm,
                            title="WorldForge 2 - Improvement Plan", author="WorldForge 2")
    doc.build(blocks(lines) + [Spacer(1, 6)])
    print("wrote", OUT)


if __name__ == "__main__":
    main()
