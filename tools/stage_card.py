#!/usr/bin/env python3
"""Builds Evo_Stage_Card.pdf — the two pages you hold at 4:30.

Supersedes the demo and submission sections of the build plan: Sarvam
credits never landed, so the track beat is Kiro consuming Evo over MCP.

    python3 tools/stage_card.py
"""
import os
from fpdf import FPDF

A4W, A4H = 210, 297
M = 15
CW = A4W - 2 * M

INK = (24, 24, 24)
ACC = (150, 92, 38)
MUT = (108, 104, 98)
PAN = (245, 242, 236)
CODBG = (240, 236, 228)
OKC = (34, 88, 58)
BADC = (138, 50, 36)
RULE = (212, 206, 196)

F = "/usr/share/fonts/truetype/dejavu/"
SANS, SANSB, MONO = F + "DejaVuSans.ttf", F + "DejaVuSans-Bold.ttf", F + "DejaVuSansMono.ttf"


class Card(FPDF):
    def __init__(self):
        super().__init__(orientation="P", unit="mm", format="A4")
        self.set_margins(M, M, M)
        self.set_auto_page_break(True, 12)
        self.add_font("d", "", SANS)
        self.add_font("d", "B", SANSB)
        self.add_font("m", "", MONO)

    def head(self, kicker, title):
        self.set_font("d", "B", 7.4)
        self.set_text_color(*ACC)
        self.cell(0, 5, kicker.upper(), new_x="LMARGIN", new_y="NEXT")
        self.set_font("d", "B", 19)
        self.set_text_color(*INK)
        self.cell(0, 9, title, new_x="LMARGIN", new_y="NEXT")
        y = self.get_y() + 1
        self.set_draw_color(*ACC)
        self.set_line_width(0.8)
        self.line(M, y, M + 20, y)
        self.set_line_width(0.2)
        self.ln(6)

    def beat(self, t, say, do=None):
        """A time marker plus the line to say."""
        y0 = self.get_y()
        self.set_font("d", "B", 8.4)
        self.set_text_color(*ACC)
        self.cell(13, 4.4, t)
        x = self.get_x()
        self.set_font("d", "", 9.6)
        self.set_text_color(*INK)
        self.multi_cell(CW - 13, 4.6, say, new_x="LMARGIN", new_y="NEXT")
        if do:
            self.set_x(x)
            self.set_font("d", "", 8.2)
            self.set_text_color(*MUT)
            self.multi_cell(CW - 13, 4.0, do, new_x="LMARGIN", new_y="NEXT")
        self.ln(2.6)

    def prompt(self, label, body):
        lines = body.split("\n")
        h = 5 + len(lines) * 3.5
        self.need(h + 8)
        self.set_font("d", "B", 7.2)
        self.set_text_color(*ACC)
        self.cell(0, 4.6, label.upper(), new_x="LMARGIN", new_y="NEXT")
        y0 = self.get_y()
        self.set_fill_color(252, 250, 246)
        self.set_draw_color((200, 160, 118))
        self.set_line_width(0.35)
        self.rect(M, y0, CW, h, style="DF")
        self.set_line_width(0.2)
        self.set_y(y0 + 2.4)
        self.set_x(M + 3.5)
        self.set_font("m", "", 7.4)
        self.set_text_color(40, 36, 32)
        self.multi_cell(CW - 7, 3.5, body, new_x="LMARGIN", new_y="NEXT")
        self.set_y(y0 + h + 2.6)

    def note(self, title, t, color=ACC):
        self.set_font("d", "", 8.6)
        lines = self.multi_cell(CW - 11, 4.4, t, dry_run=True, output="LINES")
        h = 8.5 + len(lines) * 4.4
        self.need(h + 4)
        y0 = self.get_y()
        self.set_fill_color(*PAN)
        self.rect(M, y0, CW, h, style="F")
        self.set_fill_color(*color)
        self.rect(M, y0, 1.2, h, style="F")
        self.set_xy(M + 5, y0 + 2.4)
        self.set_font("d", "B", 7.2)
        self.set_text_color(*color)
        self.cell(0, 4, title.upper(), new_x="LMARGIN", new_y="NEXT")
        self.set_x(M + 5)
        self.set_font("d", "", 8.6)
        self.set_text_color(*INK)
        self.multi_cell(CW - 11, 4.4, t, new_x="LMARGIN", new_y="NEXT")
        self.set_y(y0 + h + 3)

    def need(self, h):
        if self.get_y() + h > A4H - 16:
            self.add_page()

    def pair(self, q, a):
        self.set_font("d", "B", 8.3)
        self.set_text_color(*MUT)
        self.multi_cell(CW, 4.3, q, new_x="LMARGIN", new_y="NEXT")
        self.set_font("d", "", 9.0)
        self.set_text_color(*INK)
        self.multi_cell(CW, 4.5, a, new_x="LMARGIN", new_y="NEXT")
        self.ln(2.8)


def page_one(c):
    c.add_page()
    c.head("the 3 minutes", "On stage")

    c.beat("0:00",
           "\"Every knowledge worker has a second job nobody pays for: rebuilding what "
           "they were doing before the last interruption. It is not a search problem. "
           "The answer is not on the web. It is the state of your own machine.\"",
           "Say nothing else. Let them look at you.")

    c.beat("0:20",
           "\"Ten thousand events, one machine, eight separate jobs. I labelled them by "
           "hand. The engine independently claimed two of the eight and refused the other "
           "six.\"",
           "evo_ui.html is open. Point at the greyed rows. Do not explain them yet.")

    c.beat("0:40",
           "\"Every agent today is told its boundary — you point it at a folder. None of "
           "them derive it. And you cannot get it from content, because two people can "
           "have identical files and different jobs.\"")

    c.beat("0:55",
           "\"So I gave the boundary a mouth. Kiro is an AWS product. It has never heard "
           "of my laptop. Watch it ask.\"",
           "Switch to Kiro. This is the track integration and the demo at once.")

    c.prompt("Kiro · prompt 1", """Use the evo-boundary MCP tool list_works. Only that.
Do not read files. Do not run commands.
Show the output exactly as returned.""")

    c.beat("1:15",
           "\"Two of eight. The other six say 'refused'. That is not a bug — a wrong merge "
           "is irreversible and a refusal costs nothing.\"")

    c.prompt("Kiro · prompt 2", """Call evo-boundary find_work once with exactly:
where did I look for hiking clothes

Reply with ONLY these three lines, nothing else:
work: <name> (<confidence>%)
evidence: <evidence>
rejected: <rejected list, or none>""")

    c.beat("1:40",
           "\"Gear. And look at the last line — it tells me it nearly picked the trek. "
           "A boundary that hides its second guess is not auditable.\"",
           "Read the rejected line out loud. That line is the product.")

    c.prompt("Kiro · prompt 3", """Call evo-boundary find_work once with exactly:
what is the capital of france

Reply with only the returned text. No commentary.""")

    c.beat("2:05",
           "\"It refuses. A boundary that guesses is not a boundary.\"")

    c.beat("2:15",
           "\"And if I try to force it—\"",
           "boundary.html: select trek + gear → Merge by inference → refused. Then "
           "Declare them the same work → verifier goes red, exit 1.")

    c.beat("2:35",
           "\"That is not a test suite. It is a verifier, and it is the part I would "
           "defend.\"",
           "Hold the red panel. Do not explain it.")

    c.beat("2:50",
           "\"Every agent shipping right now has memory. None of them have jurisdiction. "
           "That is the layer.\"",
           "Stop. Do not fill the silence.")


def page_two(c):
    c.add_page()
    c.head("if it goes sideways", "Answers, and the four sentences")

    c.note("If the laptop dies, say these — they are the whole pitch",
           "1.  Every agent today is TOLD where its work begins and ends.\n"
           "2.  None of them DERIVE it, and content cannot give it to them.\n"
           "3.  Evo derives it locally, and lets any agent ask.\n"
           "4.  A boundary that guesses is not a boundary.", OKC)

    c.set_font("d", "B", 9.4)
    c.set_text_color(*INK)
    c.cell(0, 6, "Four questions, four answers", new_x="LMARGIN", new_y="NEXT")
    c.ln(2)

    c.pair("\"Did you build this today?\"",
           "\"The engine is months old. Today I built the service, the MCP surface and the "
           "UI. I would rather say that than have you find it.\"")

    c.pair("\"Why only two works out of eight?\"",
           "\"Because it would rather be silent than fuse a trek with a jacket. The gate "
           "asserts no fusion, no leisure anchor and determinism — it never asserts "
           "coverage. Six refusals is the design, not the score.\"")

    c.pair("\"Where is the AI?\"",
           "\"Deliberately not in the decision. A boundary has to be reproducible or it is "
           "a suggestion. Models consume this; they do not author it.\"")

    c.pair("\"What would you do with ninety days?\"",
           "\"Ten people outside the company running it for thirty days. Right now the "
           "boundary is proven on exactly one person — me.\"")

    c.note("If something breaks on stage",
           "Switch to the recording. Say nothing about it. Keep narrating. Never say "
           "'sorry, it usually works' — one apology costs more than one broken demo.\n\n"
           "If the engine JSON is missing: the header says source 'labels' instead of "
           "'engine+labels'. Say 'this is the hand-labelled view' and move on.", BADC)

    c.set_font("d", "B", 9.4)
    c.set_text_color(*INK)
    c.cell(0, 6, "Submission — the lines that matter", new_x="LMARGIN", new_y="NEXT")
    c.ln(2)

    c.prompt("Track tool", """Kiro (AWS) — consumes the boundary over MCP.
The agent asks where it is before it acts.
Sarvam credits did not land; the integration moved to the tool that
composes instead of competing.""")

    c.prompt("What is new today", """The service, the MCP surface, the product UI.
Built before today: the grouping engine — 16 Rust crates, deterministic,
no HTTP client.""")

    c.prompt("Numbers you may quote", """10,392 recorded events · 164 hand labels · 8 works labelled
2 claimed by the engine, 6 refused · 0 HTTP clients in the engine
1 person the boundary is proven on

Anything else — users, revenue, retention — is not yours to quote yet.""")


def main():
    c = Card()
    c.set_title("Evo — stage card")
    page_one(c)
    page_two(c)
    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(here, "..", "exports", "ship-happens", "Evo_Stage_Card.pdf")
    out = os.path.normpath(out)
    c.output(out)
    print("wrote", out)


if __name__ == "__main__":
    main()
