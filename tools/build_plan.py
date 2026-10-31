#!/usr/bin/env python3
"""Builds Evo_Ship_Happens_Build_Plan.pdf — the operational document for 31 Oct.

    python3 tools/build_plan.py

Revision 3. All five blocks are present, each opening with an
Open / Run / Paste strip so there is nothing to guess. Sarvam credits
never landed, so the track integration is Kiro consuming the boundary
over MCP.
"""
import os
from fpdf import FPDF

A4W, A4H = 210, 297
M = 17
CW = A4W - 2 * M

INK = (26, 26, 26)
ACC = (158, 96, 40)
ACC2 = (196, 138, 74)
MUT = (110, 106, 100)
PAN = (245, 242, 236)
CODBG = (238, 234, 226)
OKC = (38, 92, 62)
BADC = (140, 52, 38)
RULE = (214, 208, 198)

F = "/usr/share/fonts/truetype/dejavu/"
SANS, SANSB, MONO = F + "DejaVuSans.ttf", F + "DejaVuSans-Bold.ttf", F + "DejaVuSansMono.ttf"


class Doc(FPDF):
    def __init__(self):
        super().__init__(orientation="P", unit="mm", format="A4")
        self.set_margins(M, M, M)
        self.set_auto_page_break(True, 18)
        self.add_font("d", "", SANS)
        self.add_font("d", "B", SANSB)
        self.add_font("m", "", MONO)
        self.sect = ""

    def footer(self):
        if self.page == 1:
            return
        self.set_y(-13)
        self.set_font("d", "", 7.2)
        self.set_text_color(*MUT)
        self.cell(0, 5, "Evo  ·  Ship Happens build plan  ·  31 Oct 2026  ·  rev 3")
        self.set_x(-M - 46)
        self.cell(46, 5, self.sect, align="R")

    # ---------------------------------------------------------------- layout
    def need(self, h):
        if self.get_y() + h > A4H - 20:
            self.add_page()

    def strip(self, open_what, run=None, paste=None):
        """Open / Run / Paste — what to touch before the block starts."""
        rows = [("OPEN", open_what)]
        if run:
            rows.append(("RUN", run))
        if paste:
            rows.append(("PASTE", paste))
        h = 3.2 + len(rows) * 4.9
        self.need(h + 4)
        y0 = self.get_y()
        self.set_fill_color(30, 32, 36)
        self.rect(M, y0, CW, h, style="F")
        y = y0 + 2.4
        for k, v in rows:
            self.set_xy(M + 4, y)
            self.set_font("d", "B", 6.6)
            self.set_text_color(*ACC2)
            self.cell(13, 4.2, k)
            self.set_font("m", "", 6.8)
            self.set_text_color(226, 220, 210)
            self.multi_cell(CW - 19, 4.2, v, new_x="LMARGIN", new_y="NEXT")
            y += 4.9
        self.set_y(y0 + h + 3.5)

    def h1(self, title, kicker=None):
        self.need(40)
        if self.get_y() > self.t_margin + 8:
            self.ln(4)
        if kicker:
            self.set_font("d", "B", 7.4)
            self.set_text_color(*ACC)
            self.cell(0, 5, kicker.upper(), new_x="LMARGIN", new_y="NEXT")
        self.set_font("d", "B", 17)
        self.set_text_color(*INK)
        self.multi_cell(CW, 8, title, new_x="LMARGIN", new_y="NEXT")
        y = self.get_y() + 1.5
        self.set_draw_color(*ACC)
        self.set_line_width(0.7)
        self.line(M, y, M + 16, y)
        self.set_line_width(0.2)
        self.ln(6)

    def h2(self, t):
        self.need(20)
        self.ln(2)
        self.set_font("d", "B", 10.6)
        self.set_text_color(*INK)
        self.multi_cell(CW, 5.6, t, new_x="LMARGIN", new_y="NEXT")
        self.ln(1.6)

    def h3(self, t):
        self.need(14)
        self.ln(1)
        self.set_font("d", "B", 8.6)
        self.set_text_color(*ACC)
        self.multi_cell(CW, 5, t, new_x="LMARGIN", new_y="NEXT")
        self.ln(0.8)

    def p(self, t, size=8.9, lead=4.55, color=INK, after=2.2, style=""):
        self.set_font("d", style, size)
        self.set_text_color(*color)
        self.multi_cell(CW, lead, t, new_x="LMARGIN", new_y="NEXT")
        self.ln(after)

    def bullet(self, t, size=8.7, indent=4.6):
        self.need(10)
        self.set_font("d", "", size)
        self.set_text_color(*INK)
        x = self.l_margin
        self.set_x(x + indent)
        self.cell(3.4, 4.4, "·")
        self.set_x(x + indent + 3.4)
        self.multi_cell(CW - indent - 3.4, 4.4, t, new_x="LMARGIN", new_y="NEXT")
        self.ln(0.9)

    def code(self, t, size=7.0):
        self.set_font("m", "", size)
        lh = size * 0.44 * 1.22
        wrapped = self.multi_cell(CW - 6, lh, t, dry_run=True, output="LINES")
        h = 2.6 + len(wrapped) * lh
        self.need(h + 3)
        y0 = self.get_y()
        self.set_fill_color(*CODBG)
        self.rect(M, y0, CW, h, style="F")
        self.set_y(y0 + 1.6)
        self.set_x(M + 3)
        self.set_text_color(52, 48, 44)
        self.multi_cell(CW - 6, size * 0.44 * 1.22, t, new_x="LMARGIN", new_y="NEXT")
        self.set_y(y0 + h + 2)

    def prompt(self, tool, target, t):
        lines = t.split("\n")
        h = 4.2 + len(lines) * 3.55
        self.need(h + 12)
        self.set_font("d", "B", 7.2)
        self.set_text_color(*ACC)
        head = "PASTE INTO %s" % tool.upper()
        if target:
            head += "   ·   " + target
        self.cell(0, 5, head, new_x="LMARGIN", new_y="NEXT")
        y0 = self.get_y()
        self.set_fill_color(250, 248, 244)
        self.set_draw_color(*ACC2)
        self.set_line_width(0.35)
        self.rect(M, y0, CW, h, style="DF")
        self.set_line_width(0.2)
        self.set_y(y0 + 2.4)
        self.set_x(M + 3.4)
        self.set_font("m", "", 6.9)
        self.set_text_color(40, 36, 32)
        self.multi_cell(CW - 6.8, 3.55, t, new_x="LMARGIN", new_y="NEXT")
        self.set_y(y0 + h + 2.6)

    def callout(self, title, t, color=ACC):
        self.set_font("d", "", 8.3)
        lines = self.multi_cell(CW - 10, 4.2, t, dry_run=True, output="LINES")
        h = 8 + len(lines) * 4.2
        self.need(h + 4)
        y0 = self.get_y()
        self.set_fill_color(*PAN)
        self.rect(M, y0, CW, h, style="F")
        self.set_fill_color(*color)
        self.rect(M, y0, 1.1, h, style="F")
        self.set_xy(M + 5, y0 + 2.4)
        self.set_font("d", "B", 7.4)
        self.set_text_color(*color)
        self.cell(0, 4, title.upper(), new_x="LMARGIN", new_y="NEXT")
        self.set_x(M + 5)
        self.set_font("d", "", 8.3)
        self.set_text_color(*INK)
        self.multi_cell(CW - 10, 4.2, t, new_x="LMARGIN", new_y="NEXT")
        self.set_y(y0 + h + 3)

    def table(self, head, rows, widths, size=7.6, headbg=(232, 227, 218)):
        self.need(20)
        self.set_font("d", "B", size)
        self.set_fill_color(*headbg)
        self.set_text_color(*INK)
        for h, w in zip(head, widths):
            self.cell(w, 6.4, " " + h, border=0, fill=True)
        self.ln(6.4)
        self.set_font("d", "", size)
        for r in rows:
            n = 1
            for c, w in zip(r, widths):
                lines = self.multi_cell(w - 3, 3.9, " " + str(c),
                                        dry_run=True, output="LINES")
                n = max(n, len(lines))
            rh = n * 3.9 + 1.9
            self.need(rh + 2)
            y0 = self.get_y()
            x = self.l_margin
            for i, (c, w) in enumerate(zip(r, widths)):
                self.set_xy(x, y0 + 0.7)
                self.set_text_color(*INK)
                if i == 0:
                    self.cell(w - 2, 3.9, " " + str(c))
                else:
                    self.multi_cell(w - 3, 3.9, " " + str(c), border=0,
                                    new_x="RIGHT", new_y="TOP", max_line_height=3.9)
                x += w
            self.set_y(y0 + rh)
            self.set_draw_color(*RULE)
            self.line(M, self.get_y(), M + CW, self.get_y())
            self.ln(1.6)
        self.ln(2)


# ---------------------------------------------------------------- pages

def cover(d):
    d.add_page()
    d.set_fill_color(18, 20, 24)
    d.rect(0, 0, A4W, A4H, style="F")
    d.set_fill_color(*ACC)
    d.rect(M, 46, 7, 1.6, style="F")

    d.set_xy(M, 54)
    d.set_font("d", "B", 44)
    d.set_text_color(240, 236, 229)
    d.cell(0, 18, "EVO")

    d.set_xy(M, 76)
    d.set_font("d", "B", 15)
    d.set_text_color(*ACC2)
    d.cell(0, 8, "The boundary layer for AI agents")

    d.set_xy(M, 92)
    d.set_font("d", "", 9.6)
    d.set_text_color(158, 152, 144)
    d.multi_cell(120, 5.2, "A build plan for one day, from zero, with the clock running.")

    d.set_draw_color(58, 62, 68)
    d.line(M, 126, M + CW, 126)

    rows = [("EVENT", "Ship Happens  ·  AWS Student Builder Group  ×  ADIGPS"),
            ("DATE", "Saturday 31 October 2026  ·  9:00 – 18:30 IST"),
            ("VENUE", "Juspay Technologies  ·  Gurugram"),
            ("FREEZE", "4:00 PM  ·  demos 4:30  ·  awards 6:00"),
            ("TRACK", "Kiro (AWS) — consumes Evo over MCP")]
    y = 134
    for k, v in rows:
        d.set_xy(M, y)
        d.set_font("d", "B", 8.6)
        d.set_text_color(150, 146, 138)
        d.cell(30, 6, k)
        d.set_font("d", "", 9.2)
        d.set_text_color(216, 210, 200)
        d.cell(0, 6, v)
        y += 8

    d.set_fill_color(28, 31, 36)
    d.rect(M, 196, CW, 46, style="F")
    d.set_xy(M + 8, 203)
    d.set_font("d", "", 10.2)
    d.set_text_color(216, 210, 200)
    d.multi_cell(CW - 16, 5.8,
                 "Everyone in that room will build an agent that does something.\n"
                 "You are building the thing the agents have to ask.")
    d.set_xy(M + 8, 220)
    d.set_font("d", "B", 10.2)
    d.set_text_color(*ACC2)
    d.multi_cell(CW - 16, 5.8, "That is the only category of one available today.")

    d.set_xy(M, 262)
    d.set_font("d", "", 7.6)
    d.set_text_color(120, 116, 110)
    d.multi_cell(CW, 4.4,
                 "Revision 3 — every block present. Sarvam credits did not land, so "
                 "the track integration is Kiro consuming the boundary over MCP.")


def thesis(d):
    d.sect = "The argument"
    d.h1("What you are actually entering", "part one")
    d.p("A hackathon is a compression contest: how much credible uniqueness can you put "
        "in front of a tired judge in three minutes. You already have more substance than "
        "most teams will produce today — a deterministic engine, a real corpus, a verifier "
        "that fails the build when it is wrong.")
    d.p("What you are not doing today is writing more engine. You are giving it a mouth.",
        after=3)
    d.bullet("A service. Something on a port with endpoints. Not a page.")
    d.bullet("A protocol. Any agent can ask — and one does, on stage, in an AWS IDE.")
    d.bullet("A UI. What a human sees when they ask the same question.")

    d.h2("Three things a judge decides in thirty seconds")
    d.table(["", "The question in their head", "What answers it"],
            [["1", "Do I understand this?", "One sentence, repeated three times, never varied."],
             ["2", "Is it real?", "Real events, real numbers, a failure you cause on purpose."],
             ["3", "Is it theirs?", "A refusal, a gate, an agent asking. Things you cannot fake live."]],
            [9, 62, 105])

    d.callout("The unfair advantage",
              "Every other team will demo an agent that does a task. When they are asked "
              "\"how does it know what to work on?\", they will say \"the user tells it\" or "
              "\"we pass the folder\". You are the answer to that question. Do not compete "
              "on what the agent does. Compete on what it is allowed to touch.")

    d.h2("What wins, specifically")
    d.p("Judges reward four things, in this order: a demo that does not break, a problem "
        "they recognise, evidence that it is not a wrapper, and a story they can repeat. "
        "Novelty is third. You do not need to be the most original project in the room — "
        "you need to be the one nobody can place in an existing category.", after=3)
    d.table(["Judging pressure", "How Evo answers it"],
            [["\"Is this another OpenAI wrapper?\"",
              "The decision path has no model in it. Turn off the wifi and it still works."],
             ["\"Isn't this just a folder?\"",
              "A folder is declared in advance. This is derived from behaviour, and carries its own evidence."],
             ["\"How do I know it's right?\"",
              "It refuses six of eight works rather than guessing, and shows what it rejected."],
             ["\"Why now?\"",
              "Agents just started acting on real machines. They need jurisdiction first."]],
            [54, 122])
    d.p("The most memorable thing you will do today: let a judge try to merge two works, "
        "refuse them, then let them declare it anyway and watch the build fail. People "
        "remember the product that told them no.", after=0)


def product(d):
    d.sect = "What you are building"
    d.h1("The demo product and the real product", "part two")
    d.p("These are the same thing at two distances. Say it that way and the ambition "
        "lands without sounding like a deck.", after=3)
    d.table(["", "Demo product (today)", "Real product (what you say next)"],
            [["What it is",
              "A local service on one laptop. Any program on the machine can ask which "
              "body of work a resource belongs to.",
              "The same service shipped inside the desktop app. Every agent on the "
              "machine queries it as infrastructure."],
             ["Who uses it",
              "You, in front of a judge, in three minutes.", "Knowledge workers whose agents act on their machines."],
             ["How it earns", "Nothing.",
              "Per seat, prosumer first, then team. The boundary is the moat, not the model."],
             ["Why defensible", "It is a demo.",
              "The corpus and the verifier: an API can be copied in a week, the ten "
              "defects you found by measuring cannot."]],
            [27, 71, 78])

    d.h2("The four surfaces you will show")
    d.table(["#", "Surface", "What the judge sees", "Built"],
            [["1", "Service", "curl on a terminal: works, resume bundles, a refusal. JSON, not a screenshot.", "Today"],
             ["2", "MCP", "Kiro — an AWS product that has never heard of your laptop — asking Evo where it is.", "Today, new"],
             ["3", "UI", "The same question asked by a human, with the refused works greyed out.", "Today"],
             ["4", "Engine", "A gate that fails the build when two works fuse.", "Months"]],
            [7, 24, 118, 27])

    d.callout("What is genuinely new today",
              "The repository had no MCP server and no socket listener. Today Evo becomes "
              "addressable: an HTTP surface, a protocol any agent can speak, a product UI. "
              "\"This morning Evo was a library; tonight it is addressable\" is true, and "
              "true is the point.", OKC)

    d.h2("The sentence, memorised")
    d.set_font("d", "B", 11.5)
    d.set_text_color(*ACC)
    d.multi_cell(CW, 6.4, "Every agent today is told where its work begins and ends. "
                          "Evo derives it, locally, and lets any agent ask.")
    d.ln(3)
    d.p("If you lose your place, say it again. Everything else is evidence for it.", after=0)


def arch(d):
    d.sect = "Architecture"
    d.h1("How it reads Evo", "part three")
    d.p("Be able to draw this. The question \"how does the new bit read the engine?\" is "
        "what separates you from someone who wired an API to a mock.", after=3)
    d.code("""
  sample_events.json      10,392 recorded events
        |
        |   cargo run -p evo-threads --bin boundary_dump
        v
  evo-threads ENGINE     Rust, deterministic, offline, no HTTP client
        |
        |   Engine::briefs()  ->  Vec<WorkBrief>
        v
  threads.json           the one contract between the old and the new
        |
        |   python3 evo_server.py     (one process, two mouths)
        v
  evo_server.py
        |
        +--- serve  :8765   ->  evo_ui.html   (a human asks)
        |
        +--- mcp    stdio   ->  Kiro          (an agent asks)
""")
    d.h2("Why each layer exists")
    d.table(["Layer", "Its job", "If you removed it"],
            [["Engine", "Decides the boundary. Deterministic, offline, no model.", "There is no product, only a UI."],
             ["threads.json", "The contract. One file, one shape.", "The service would have to link Rust to be useful."],
             ["Service", "Makes the boundary addressable: HTTP, JSON, a port.", "Nothing else can use it. It stays a demo."],
             ["MCP", "Makes it speakable by agents, not just by your UI.", "You are the only client. That is a toy."],
             ["Kiro", "Proves an outside agent can consume it, on the day.", "One less witness; the claim is unchanged."]],
            [24, 84, 68])

    d.callout("If a judge asks where the AI is",
              "\"Deliberately not in the decision. The boundary has to be deterministic "
              "and auditable or it is not a boundary, it is a suggestion. AI consumes "
              "this; it does not author it. That is why the answer is the same twice, and "
              "why I can show you a test that fails the build when it gets it wrong.\"", OKC)
    d.p("The engine mints two works on this corpus and refuses the other six. That is not "
        "a defect: `tests/ground_truth.rs` asserts no fusion, no leisure anchor and "
        "determinism — never coverage. Silence is a designed outcome.", after=0)


def position(d):
    d.sect = "Starting position"
    d.h1("What already exists", "part four")
    d.p("Everything below is committed. Your job is to rehearse it, not to rebuild it.",
        after=4)
    d.table(["File", "What it does", "Status"],
            [["crates/evo-threads/src/bin/boundary_dump.rs",
              "Replays the corpus through the engine, writes threads.json.", "Ran: 2 works"],
             ["exports/ship-happens/evo_server.py",
              "HTTP service and MCP server. Stdlib only.", "Tested"],
             ["exports/ship-happens/evo_ui.html", "Product surface, refused rows greyed.", "Tested"],
             ["exports/ship-happens/mcp_probe.py", "A real MCP client; backup if Kiro fails.", "Tested"],
             ["exports/ship-happens/boundary.html", "Offline explorer, the two break-it buttons.", "Tested"],
             [".kiro/settings/mcp.json", "Registers Evo with Kiro.", "Kiro connected"]],
            [58, 90, 28])
    d.code("""architecture-v1/
  .kiro/steering/product.md        the invariant Kiro must not break
  .kiro/settings/mcp.json          registers evo-boundary with Kiro
  crates/evo-threads/src/          the engine — FROZEN today
    bin/boundary_dump.rs           events -> threads.json
  exports/ship-happens/            everything you demo
    evo_server.py                  service + MCP
    evo_ui.html                    product UI
    boundary.html                  offline explorer
    threads.json                   engine output
    boundary.json                  hand-label view""")
    d.callout("Do not touch the engine today",
              "A rushed edit at 12:40 that does not compile costs the afternoon. If you "
              "find a bug, write it down and fix it on Monday.", BADC)


def sprint_table(d):
    d.sect = "Sprint 01"
    d.h1("Sprint 01: 10:10 to 14:00", "part five")
    d.p("Five blocks. Each ends with something you can show, so if you stop at any point "
        "you still have a demo. That property matters more than finishing.", after=4)
    d.table(["Block", "Time", "You produce", "If it fails"],
            [["A", "10:10 – 10:40", "Kiro open, engine verified, threads.json written.", "Use boundary.json."],
             ["B", "10:40 – 11:40", "Service on :8765, /ask answering, UI loading works.", "Fall back to boundary.html."],
             ["C", "11:40 – 12:40", "Kiro asks Evo over MCP and gets evidence back.", "Drop Kiro; use mcp_probe.py."],
             ["D", "12:40 – 13:40", "Timed handoff rehearsal + backup recording.", "Drop Render. Never drop the recording."],
             ["E", "13:40 – 14:00", "Frozen, one full run-through, committed.", "—"]],
            [12, 30, 74, 60])
    d.p("Each block has a twenty-minute overrun limit. Past it, take the failure path, "
        "say the fallback line once, and move on. A demo missing its third act still "
        "wins. A demo still compiling at 4:10 cannot be submitted.", after=0)


def block_a(d):
    d.sect = "Sprint 01 / A"
    d.h1("Block A — Setup and the engine proof", "10:10 – 10:40")
    d.strip("Terminal at the repo root. Kiro in a second window.",
            "the four commands below, in order",
            "the steering check and the A3 explain prompt")
    d.p("Goal: by 10:40 you either have a threads.json from the real engine, or a written "
        "decision to use the fallback. No ambiguity, no half-built Rust.", after=4)

    d.h3("A1  ·  Prove the engine runs, then write the boundary")
    d.code("""cd ~/Desktop/Evo-Evolution.nosync

# the corpus path, used three times below — avoids long lines
E=tools/threads-regression/sample_events.json
L=tools/threads-regression/ground_truth_labels.json

# 1. the gate — 5-8 min the first time, ~40s after
cargo run -p evo-threads --bin regression -- --events $E --labels $L

# 2. the bridge — writes the boundary the service will serve
O=exports/ship-happens/threads.json
cargo run -p evo-threads --bin boundary_dump -- --events $E --out $O

# 3. look at what it claimed
python3 -m json.tool exports/ship-happens/threads.json | head -30""")

    d.p("The regression binary prints a report and exits 0. If it exits 1, the gate is "
        "working — that is worth thirty seconds of understanding, not a rewrite.", after=3)

    d.callout("If the path gets clipped",
              "You will get a file called ship-hap instead of a folder. Fix it with "
              "`mv exports/ship-hap exports/ship-happens/threads.json`. The dump binary "
              "creates the file wherever you point it; it does not create folders.")

    d.h3("A2  ·  Confirm Kiro loaded the steering file")
    d.p("The steering file is already in the repo at `.kiro/steering/product.md`. You are "
        "not writing it — you are checking Kiro read it, because rule 4 is the only thing "
        "stopping an agent from wiring a model into the decision path.", after=2)
    d.code("""cat .kiro/steering/product.md      # if this is empty, pull the branch:

git fetch origin arena/2873f4c9-architecture-v1
git checkout origin/arena/2873f4c9-architecture-v1 -- .kiro""")
    d.prompt("Kiro", "chat, with the repo folder open", """Without reading any files, tell me the hard rules that apply to this
repository. Quote rule 4 exactly.""")

    d.p("It should recite: external services consume the boundary, they never influence how "
        "it is computed. If it cannot, quit Kiro (Cmd+Q), reopen the folder, try again. "
        "Do not spend more than five minutes on this.", after=3)

    d.h3("A3  ·  Learn to defend the design")
    d.prompt("Kiro", "chat — read-only, writes nothing", """Read crates/evo-threads/src/lib.rs and src/brief.rs. Do not write code.
Explain in under 250 words, in plain English: what a work is, what a brief
contains, how resume_point and restore_set are chosen, and why the engine
has no concept of a filename or an application name.

Then give me three questions a judge might ask about this design and the
one-sentence answer to each.""")

    d.callout("Why A3 is not optional",
              "You will be asked \"how does it decide?\". If the answer is \"the AI figures it "
              "out\", you are every other project in the room. If the answer is \"attention "
              "is serial, work is not, so it segments episodes and joins them by what you "
              "made rather than where you were\", you are the only person in the building "
              "who can say that.")


def block_b(d):
    d.sect = "Sprint 01 / B"
    d.h1("Block B — The service", "10:40 – 11:40")
    d.strip("Terminal in exports/ship-happens. A browser tab.",
            "python3 evo_server.py serve",
            "the /evidence endpoint prompt, only if you have spare time")
    d.p("Goal: something running on a port that a stranger could curl. This is the "
        "difference between a project and a page.", after=4)

    d.h3("B1  ·  Run it before you change it")
    d.code("""cd exports/ship-happens
python3 evo_server.py serve

# second terminal — four answers, in this order
curl -s localhost:8765/health
curl -s localhost:8765/works | head -c 400
curl -s "localhost:8765/ask?q=where+did+i+look+for+hiking+clothes"
curl -s 'localhost:8765/ask?q=capital+of+france'""")

    d.p("You want: a health line saying engine+labels, a list of works with claimed and "
        "refused marked, \"gear\" with evidence, and a refusal. The refusal is the one that "
        "must work — if the boundary is guessing, the thesis is broken and nothing "
        "cosmetic matters.", after=3)

    d.h3("B2  ·  Make it yours, if there is time")
    d.prompt("Kiro", "exports/ship-happens/evo_server.py", """Read exports/ship-happens/evo_server.py. Do not rewrite it.

Add one endpoint: GET /evidence?q=<question>

It returns the same answer as /ask, plus, for the winning work, the three
strongest individual pieces of evidence with their weights, in the order
the scorer produced them - not re-sorted, not summarised.

Standard library only. Do not change the /ask response shape. Do not touch
load_boundary() or normalise().""")

    d.h3("B3  ·  The line you say while it runs")
    d.callout("The service line",
              "\"That is not a screenshot. That is a service on this laptop, reading a file "
              "the Rust engine wrote twenty minutes ago. Nothing here left the machine, "
              "which is why it still works with the wifi off — and I will show you that "
              "later.\"")
    d.p("Then move on. Do not tour the code. The code is not the demo; the refusal is.",
        after=0)


def block_c(d):
    d.sect = "Sprint 01 / C"
    d.h1("Block C — The protocol, and the agent you demo with", "11:40 – 12:40")
    d.strip("Kiro (quit and reopened). One terminal for the stdio check.",
            "python3 evo_server.py mcp   —   then python3 mcp_probe.py --tools",
            "the three MCP prompts, in the order they appear on stage")
    d.p("This is the new thing. Until today nothing outside the repository could ask Evo "
        "anything. MCP is how an agent asks — and Kiro is the agent.", after=4)

    d.h3("C1  ·  Confirm the server speaks before Kiro touches it")
    d.code("""cd exports/ship-happens
python3 mcp_probe.py --tools        # initialize, then tools/list""")
    d.p("You should see the handshake and three tool names: list_works, find_work, "
        "resume_work. Then ask it something real:", after=2)
    d.code("""python3 mcp_probe.py 'where did I look for hiking clothes'""")
    d.p("It prints the answer, the evidence, and the work it rejected. If this works, "
        "everything else in this block is configuration — and you have a demo even if "
        "Kiro never connects.", after=3)

    d.h3("C2  ·  Register Evo with Kiro")
    d.p("The config is committed at `.kiro/settings/mcp.json` with your absolute path "
        "already in it. Verify it, then **fully quit Kiro** — the config is read at "
        "startup only.", after=2)
    d.code("""cat .kiro/settings/mcp.json""")
    d.p("Reopen Kiro on the repo folder. The MCP panel should show `evo-boundary` with "
        "three tools: list_works, find_work, resume_work. If it shows nothing, run the "
        "stdio check above — that separates a broken server from a broken config, and "
        "they have different fixes.", after=3)

    d.callout("The narrowness is the point",
              "A full agent will happily start exploring your repository, and the moment "
              "stops being about the boundary. Every prompt below forbids file reads and "
              "commands for a reason: you want the judge watching a tool call, not an IDE "
              "doing detective work.")

    d.h3("C3  ·  The three prompts — these go on stage")
    d.prompt("Kiro", "prompt 1 · list_works", """Use the evo-boundary MCP tool list_works. Only that.
Do not read files. Do not run commands.
Show the output exactly as returned.""")

    d.prompt("Kiro", "prompt 2 · find_work", """Call evo-boundary find_work once with exactly:
where did I look for hiking clothes

Reply with ONLY these three lines, nothing else:
work: <name> (<confidence>%)
evidence: <evidence>
rejected: <rejected list, or none>""")

    d.prompt("Kiro", "prompt 3 · the refusal", """Call evo-boundary find_work once with exactly:
what is the capital of france

Reply with only the returned text. No commentary.""")

    d.p("Write down how long each call takes. That number paces your 0:55 beat: under two "
        "seconds and you move, five to ten and you start talking a beat earlier and let "
        "the pause read as work.", after=2)

    d.h3("C4  ·  The fallback, and why it is not a downgrade")
    d.code("""python3 mcp_probe.py 'where did I look for hiking clothes'""")
    d.p("Prints initialize, tools/list, then the answer with evidence and the rejected "
        "work. If Kiro will not connect on the day, you run this instead and say: \"this is "
        "the same conversation, in a terminal, so you can see the protocol.\" The judge "
        "watches the handshake happen. It is arguably clearer.", after=0)


def block_d(d):
    d.sect = "Sprint 01 / D"
    d.h1("Block D — Rehearse the handoff, then insure it", "12:40 – 13:40")

    d.strip("Kiro, the service terminal, and QuickTime ready.",
            "nothing new — you are rehearsing what exists",
            "the three C3 prompts, timed once each")
    d.p("This block changed. Sarvam credits did not land, so there is no voice to build. "
        "What replaces it is worth more: the ten seconds where an AWS product asks your "
        "engine a question — and the recording that survives if it does not.", after=4)

    d.h3("D1  ·  The Kiro rehearsal (25 min)")

    d.p("You are not rehearsing the whole demo. You are rehearsing the handoff, because "
        "that is the only part with a dependency you do not control.", after=2)

    d.code("""# terminal 1 — the boundary must be up before Kiro asks
cd ~/Desktop/Evo-Evolution.nosync/exports/ship-happens
python3 evo_server.py serve

# terminal 2 — prove the server is fine, independently of Kiro
python3 mcp_probe.py --tools""")

    d.p("Then quit Kiro fully (Cmd+Q), reopen it, and run the three demo prompts in "
        "order. Time each one. Write the number down — you will use it to pace 0:55.",
        after=3)

    d.table(["What to watch", "What to do about it"],
            [["Tool call is instant (<2s)", "Good. Give the beat fifteen seconds and move."],
             ["Tool call lags 5–10s", "Start talking one beat earlier: say \"watch it ask\" and let it spin — the pause reads as work."],
             ["Kiro writes three paragraphs of prose", "The prompt failed. Re-paste prompt 2 verbatim; it forbids commentary."],
             ["Kiro asks permission every call", "Answer once, then re-check autoApprove in .kiro/settings/mcp.json."],
             ["MCP panel shows no server", "Use mcp_probe.py on stage instead. It is the same protocol, in a terminal."]],
            [62, 114], size=7.4)

    d.callout("The fallback is not a downgrade",
              "If Kiro will not connect, run `python3 mcp_probe.py \"where did I look for "
              "hiking clothes\"` and say: \"this is the same conversation a moment ago, in "
              "a terminal, so you can see the protocol.\" The judge sees initialize, "
              "tools/list, and the answer. It is arguably clearer.")

    d.h3("D2  ·  The backup recording (20 min, non-negotiable)")

    d.p("Your demo now depends on an IDE, a JSON config, and a child process. Record all "
        "of it at 13:20, while you are calm and everything works.", after=2)
    d.code("""Cmd-Shift-5  ->  Record Selected Portion  ->  record  ->  Desktop

Cover, in this order:
  1. the service starting, source line says engine+labels
  2. evo_ui.html: the works, the greyed refusals, 2 of 8 claimed
  3. one good question, one refusal
  4. Kiro: list_works, then find_work, then the refusal
  5. boundary.html: merge by inference refused, declare -> exit 1""")

    d.h3("D3  ·  Render — only if you are ahead (skip by default)")

    d.p("Off the critical path. It cannot touch the boundary honestly: a hosted instance "
        "either contradicts local-first or runs fake data. If you do it, it is a "
        "synthetic corpus on a public URL — never your real history.", after=2)
    d.code("""python3 evo_server.py serve      # reads $PORT, so it deploys unchanged
# say once: the public one runs synthetic data; the real one runs on this laptop.""")

    d.h3("D4  ·  Freeze at 13:40")

    d.p("Commit everything. From here you change words, not code. The most common way to "
        "lose a hackathon is a change made after the last rehearsal.", after=0)


def block_e(d):
    d.sect = "Sprint 01 / E"
    d.h1("Block E — Freeze the machine", "13:40 – 14:00")
    d.strip("Everything you will show. Nothing you will not.",
            "git status  ·  git commit -am \"hackathon\"",
            "nothing. No prompts from here.")
    d.p("Twenty minutes to make the laptop itself part of the performance. Most demos do "
        "not fail because the code is wrong — they fail because something on screen "
        "should not be there.", after=4)

    d.h3("E1  ·  Commit, then stop writing code")
    d.code("""cd ~/Desktop/Evo-Evolution.nosync
git add -A && git commit -m "ship happens submission"
git log --oneline -3""")

    d.h3("E2  ·  The screen, exactly")
    d.p("One browser window with two tabs, and Kiro behind it. Nothing else.", after=2)
    d.table(["Order", "What is open", "Why"],
            [["Tab 1", "localhost:8765 — evo_ui.html", "The product. This is what beat 0:20 shows."],
             ["Tab 2", "exports/ship-happens/boundary.html", "The break-it. Opened by file, no server, so it cannot fail."],
             ["Window 2", "Kiro, repo folder, MCP panel visible", "The agent. Beat 0:55."],
             ["Window 3", "Terminal, server running, scrolled to the startup lines", "The proof. Only used if asked."],
             ["Closed", "Slack, WhatsApp, mail, every other tab", "One notification over a demo is a lost demo."]],
            [17, 72, 87], size=7.4)

    d.h3("E3  ·  One complete run-through, out loud, timed")
    d.p("Not a mental rehearsal. Stand up, screen full, talk, click. You are looking for "
        "three things: where you stumble, where a window is in the wrong place, and "
        "whether you finish under 3:00.", after=2)
    d.code("""Target: 2:40. Cut the works tour if you are over.
Never cut: the refusal, the break-it, the closing line.""")

    d.callout("The freeze is not a suggestion",
              "After 13:40 you change words, not code. The single most common way to lose "
              "a hackathon is a small fix made at 15:50 that does not get re-tested. Your "
              "demo is good enough. Your delivery is not yet.", BADC)

    d.h3("E4  ·  The two-minute breather")
    d.p("Eat something. Walk once around the room. You are on at 4:30 and you will be "
        "speaking to people who have been sitting for seven hours. Energy is part of the "
        "demo and you cannot fake it at 4:25.", after=0)


def sprint2(d):
    d.sect = "Sprint 02"
    d.h1("Sprint 02: 14:30 to 16:00", "part six")
    d.table(["Time", "Do", "Output"],
            [["14:30 – 15:05", "Rehearse the full demo twice, out loud, timed against the stage card.", "You finish at 2:45, not 4:10."],
             ["15:05 – 15:20", "Write the submission. Paste from the next page.", "Draft, not a blank form."],
             ["15:20 – 15:40", "Hostile drill: eight questions, one at a time.", "You stop being surprised."],
             ["15:40 – 15:52", "Rehearse once more; re-record any beat that broke.", "Clean recording."],
             ["15:52 – 16:00", "Submit. Close the lid on code.", "Done."]],
            [27, 96, 53])
    d.p("Sprint 02 is where most teams lose, and not because they run out of time. They "
        "keep building past the point where building helps. With the internship prize, "
        "the marginal rehearsal is worth more than the marginal feature: they are "
        "deciding whether they want you on their team.", after=3)
    d.prompt("Claude / ChatGPT", "before 15:40", """You are a judge at an Indian student hackathon. You are technical, you have
seen forty AI projects today, and you are tired and sceptical. I have three
minutes.

Interrogate me, one question at a time. After each answer say in one line what
a sceptical judge would think of it, then ask the next question. Escalate: what
it does, then accuracy, then why it is not just a folder, then why an
incumbent will not simply build it, then the business.

Do not be encouraging. Stop after eight questions and name my two weakest
answers.""")


def submission(d):
    d.sect = "Submission"
    d.h1("The submission, field by field", "part seven")
    d.p("Write it at 15:05, not at 15:58.", after=3)
    d.h3("Title")
    d.code("""Evo — the boundary layer for AI agents""")
    d.h3("One line")
    d.code("""Evo derives where one body of work ends and the next begins, on the device,
so agents can act on a real machine without a human labelling every folder.""")

    d.h3("Description")
    d.code("""Every AI agent today is TOLD where its work begins and ends: you point it
at a folder. None of them DERIVE it. That is why agents still cannot safely
act on a real desktop - they do not know that this repository is one job and
that deck is another. Scale does not fix it, because the boundary is not in
the content. Two people can have identical files and be doing different jobs.

Evo derives the boundary locally, as a side effect of a product people use
every morning, and exposes it for agents to query.

What you can try in the demo:
- The service: curl a work, its resume bundle, and a refusal.
- The protocol: Kiro, an AWS product, asks Evo where it is over MCP and gets
  an answer with evidence - including the work it nearly picked instead.
- The refusal: six of eight works are refused rather than guessed.
- The gate: declare two works the same and the build fails, on purpose.

Built today: the service, the MCP surface, the product UI.
Built before today: the grouping engine - 16 Rust crates, deterministic, no
HTTP client. We say that plainly because the architecture is the interesting
part, not the timeline.

Track: Kiro (AWS). Sarvam credits did not land, so the integration went to
the tool that composes instead of competing: Kiro consumes the boundary.""")

    d.h3("Stack")
    d.code("""Rust (16 crates, the engine) · Python 3 standard library (service, MCP,
UI server) · Kiro / AWS (MCP client) · no cloud in the decision path ·
no model decides the boundary""")

    d.callout("Say the timeline sentence yourself",
              "\"The engine is months old; today I built the layer that lets you talk to "
              "it.\" One sentence, at the top, in your own voice. It costs nothing and it "
              "removes the only way to lose the room.")


def demo(d):
    d.sect = "The demo"
    d.h1("Three minutes, in order", "part eight")
    d.p("Fullscreen. One tab. Nothing else running. Times are targets, not rules.",
        after=4)
    d.table(["", "Say", "Do"],
            [["0:00",
              "\"Every knowledge worker has a second job nobody pays for: rebuilding what "
              "they were doing before the last interruption. It is not a search problem. "
              "The answer is not on the web. It is the state of your own machine.\"",
              "Nothing. Let them look at you."],
             ["0:20",
              "\"Ten thousand events, one machine, eight separate jobs. I labelled them by "
              "hand. The engine independently claimed two of the eight and refused the "
              "other six.\"",
              "Point at the works rail. Greyed rows are refusals."],
             ["0:40",
              "\"Every agent today is told its boundary — you point it at a folder. None "
              "derive it. And content cannot give it to you, because two people can have "
              "identical files and different jobs.\"",
              "curl /works."],
             ["0:55",
              "\"So I gave the boundary a mouth. Kiro is an AWS product. It has never heard "
              "of my laptop. Watch it ask.\"",
              "Switch to Kiro. Run prompt 1 (list_works)."],
             ["1:15",
              "\"Two of eight. The other six say refused. That is not a bug — a wrong merge "
              "is irreversible and a refusal costs nothing.\"", "Scroll the list slowly."],
             ["1:30",
              "\"Now a real question.\"",
              "Run prompt 2 (find_work: hiking clothes)."],
             ["1:45",
              "\"Gear, sixty percent. And the last line tells me it nearly picked the trek. "
              "A boundary that hides its second guess is not auditable.\"",
              "Read the rejected line out loud."],
             ["2:05",
              "\"And when it has no answer—\"", "Run prompt 3 (capital of France)."],
             ["2:15",
              "\"It refuses.\"", "Say nothing more. Then: \"Now break it.\""],
             ["2:20",
              "\"—\"", "boundary.html: merge by inference refused; declare → verifier red, exit 1."],
             ["2:40",
              "\"That is not a test suite. It is a verifier, and it is the part I would "
              "defend.\"", "Hold the red panel."],
             ["2:52",
              "\"Every agent shipping right now has memory. None of them have jurisdiction. "
              "That is the layer.\"", "Stop. Do not fill the silence."]],
            [13, 103, 57], size=7.2)

    d.callout("The four words that decide it",
              "When the verifier goes red, resist explaining it. Say the sentence, hold the "
              "screen, let the room work out what they saw. Explaining steals the "
              "realisation, and the realisation is what they repeat to each other.")
    d.h2("If you are cut to ninety seconds")
    d.p("Problem in one sentence. Kiro asks once. The refusal. The closing line. Drop the "
        "works tour and the break-it. Never drop the refusal.", after=0)


def qa(d):
    d.sect = "Questions"
    d.h1("What they will ask", "part nine")
    d.table(["They ask", "You say"],
            [["\"Did you build this today?\"",
              "\"The engine is months old. Today I built the service, the MCP surface and the UI. I would rather say that than have you find it.\""],
             ["\"Why only two works out of eight?\"",
              "\"Because it would rather be silent than fuse a trek with a jacket. The gate asserts no fusion, no leisure anchor and determinism — it never asserts coverage.\""],
             ["\"Why would anyone need this?\"",
              "\"Their agents are useless on a real machine without it. You cannot hand-label a thousand folders per employee. It has to be derived, once, passively.\""],
             ["\"Isn't this just a folder?\"",
              "\"A folder is declared in advance from a menu. This is derived from behaviour and carries its own evidence. Different artifact, different guarantees.\""],
             ["\"Where's the AI?\"",
              "\"Deliberately not in the decision. A boundary has to be reproducible or it is a suggestion. Models consume it; they do not author it.\""],
             ["\"What about Cowork / Recall?\"",
              "\"They carry context forward. This defines the boundary that context lives inside. A context engine with no jurisdiction is guessing.\""],
             ["\"Business model?\"",
              "\"Per seat, prosumer first. The wedge is people who lose an hour a day to this. I am not going to invent enterprise pricing.\""],
             ["\"What would you do with ninety days?\"",
              "\"Ten people outside the company running it for thirty days. Right now the boundary is proven on exactly one person - me.\""]],
            [46, 130], size=7.4)

    d.callout("The answer that must never leave your mouth",
              "\"We don't know, here's how we'd find out\" is strong. Every invented number "
              "is a permanent wound: one judge who knows the domain catches one, and "
              "everything else you claimed becomes suspect.", BADC)


def breaks(d):
    d.sect = "Failure modes"
    d.h1("When it breaks, and it will", "part ten")
    d.table(["Symptom", "Cause", "First minute"],
            [["Kiro shows no evo-boundary server", "Config read at startup only.",
              "Cmd+Q, reopen. Still missing: run mcp_probe.py on stage."],
             ["Kiro asks permission on every call", "autoApprove not loaded.",
              "Approve once and continue. Do not debug config mid-demo."],
             ["Kiro answers in prose, slowly", "The prompt was not restrictive enough.",
              "Re-paste prompt 2 verbatim; it forbids commentary."],
             ["Service starts but /works is empty", "Not run from exports/ship-happens.",
              "cd into it and rerun. It resolves paths from its own file."],
             ["Header says 'labels' not 'engine+labels'",
              "threads.json missing or in the wrong folder.",
              "\"This is the hand-labelled view\" — once — and continue."],
             ["Something crashes on stage", "Any of the above, live.",
              "Switch to the recording without comment. Keep narrating."],
             ["Judge does not understand it", "You explained architecture first.",
              "Stop. Problem sentence, then boundary sentence, then the refusal."]],
            [50, 53, 73], size=7.2)

    d.h2("The four sentences you must not lose")
    d.code("""1.  Every agent today is TOLD where its work begins and ends.
2.  None of them DERIVE it, and content cannot give it to them.
3.  Evo derives it locally, and lets any agent ask.
4.  A boundary that guesses is not a boundary.""")
    d.p("If you can say those four from memory, you cannot lose the room, only the prize.",
        after=0)


def appendix(d):
    d.sect = "Appendix"
    d.h1("Everything in one place", "appendix")
    d.h3("Commands")
    d.code("""# engine -> json
O=exports/ship-happens/threads.json
cargo run -p evo-threads --bin boundary_dump -- --events $E --out $O
  --events tools/threads-regression/sample_events.json \\
  --out exports/ship-happens/threads.json

# the service + UI
cd exports/ship-happens && python3 evo_server.py serve

# the protocol, without any IDE
python3 evo_server.py mcp
python3 mcp_probe.py "where did I look for hiking clothes"

# the offline explorer (no server, no install)
open boundary.html""")

    d.h3("Endpoints")
    d.code("""GET  /                 the product UI
GET  /works            all works, with claimed/refused flags
GET  /works/{id}       one work with its restore set
GET  /ask?q=...        the boundary answer
GET  /health           source, work count, claimed count""")

    d.h3("MCP tools")
    d.code("""list_works   every work, marking the ones the engine refused to assert
find_work    which work a resource or phrase belongs to, with evidence
resume_work  the exact surfaces to reopen for a work""")

    d.h3("Numbers you may quote, and only these")
    d.code("""10,392   recorded events (13-15 September 2026)
   164   resources labelled by hand
     8   bodies of work labelled
     2   claimed by the engine; the other 6 it refuses to assert
     0   HTTP clients in the engine; a test fails the build if one appears
     1   person the boundary is proven on""")
    d.p("Everything else — users, revenue, retention — is not yours to quote yet. Say "
        "\"nobody outside the company has run it\" and watch how much more they believe "
        "the rest.", after=0)


def build():
    d = Doc()
    d.set_title("Evo — Ship Happens build plan (rev 2)")
    d.set_author("Evo")
    cover(d)
    thesis(d)
    product(d)
    arch(d)
    position(d)
    sprint_table(d)
    block_a(d)
    block_b(d)
    block_c(d)
    block_d(d)
    block_e(d)
    sprint2(d)
    submission(d)
    demo(d)
    qa(d)
    breaks(d)
    appendix(d)

    here = os.path.dirname(os.path.abspath(__file__))
    out = os.path.normpath(os.path.join(here, "..", "exports", "ship-happens",
                                        "Evo_Ship_Happens_Build_Plan.pdf"))
    d.output(out)
    print("wrote", out)


if __name__ == "__main__":
    build()
