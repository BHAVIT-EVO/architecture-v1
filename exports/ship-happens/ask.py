#!/usr/bin/env python3
"""
ask.py — query the work boundary.

The boundary is computed locally, once, and stored as boundary.json.
This tool does NOT compute it and does NOT need the network. It answers
"which body of work does this belong to, and why" by matching against
the boundary artifact.

Optional: pass --sarvam to phrase the answer in Indian languages via
Sarvam AI (STT/TTS/chat). The model never decides the boundary — it only
wording. That separation is the point: intelligence is replaceable,
the boundary is not.

Usage:
    python3 ask.py "which work was I in yesterday?"
    python3 ask.py --list
    python3 ask.py --voice "main kal kis kaam par tha"
"""
import json
import os
import sys
import re

HERE = os.path.dirname(os.path.abspath(__file__))
BOUNDARY = os.path.join(HERE, "boundary.json")

# Words that carry no locating power.
STOP = set("""a an the i my me we our you your it its is was were be been am
do did does doing have has had what which who when where why how
this that these those of in on at to for from with about into over
after before and or but not no yes can could should would will
work works working worked was were yesterday today tomorrow morning
night day days time doing get got doing again back start started
was there here then than so if as by up down out off all any some
please show tell give find me""".split())

SYN = {
    "trek": ["trek", "trekking", "annapurna", "hike", "hiking", "mountain", "nepal", "trail"],
    "gear": ["gear", "jacket", "arcteryx", "hat", "cap", "clothes", "clothing", "buy", "shopping", "boots"],
    "evo": ["evo", "code", "codebase", "repo", "rust", "crate", "engine", "build", "compile", "test", "capture"],
    "pitch": ["pitch", "deck", "presentation", "slides", "investor"],
    "freeai": ["free", "credits", "router", "agentrouter", "chatgpt", "login", "account", "api"],
    "kundli": ["kundli", "horoscope", "astrology", "birth", "chart", "remedies"],
    "comms": ["call", "facetime", "discord", "chat", "talk"],
    "noise": ["chrome", "tab", "untitled"],
}


def toks(s):
    return [t for t in re.findall(r"[a-z0-9']+", s.lower()) if t and t not in STOP]


def score(question, work):
    """Deterministic lexical score. No model. Returns (score, why)."""
    qt = set(toks(question))
    if not qt:
        return 0.0, ""
    hits = []
    # 1. the work's own name
    name_toks = set(toks(work["label"]))
    if qt & name_toks:
        hits.append(f"name match: {','.join(sorted(qt & name_toks))}")
    # 2. synonyms — how people actually describe the job
    syn = set(SYN.get(work["label"], []))
    if qt & syn:
        hits.append(f"synonym: {','.join(sorted(qt & syn))}")
    # 3. the resources themselves
    res_hits = []
    for r in work["top"]:
        rt = set(toks(r["r"]))
        if qt & rt:
            res_hits.extend(sorted(qt & rt))
    if res_hits:
        hits.append("resource text: " + ",".join(sorted(set(res_hits))[:4]))
    if not hits:
        return 0.0, ""
    # confidence = weighted evidence, normalised by how much evidence the work has
    s = 0.0
    if qt & name_toks:
        s += 3.0
    if qt & syn:
        s += 2.0
    s += min(len(set(res_hits)), 4) * 0.75
    return s, "; ".join(hits)


def answer(q, works):
    scored = []
    for w in works:
        s, why = score(q, w)
        if s > 0:
            scored.append((s, w, why))
    scored.sort(key=lambda x: -x[0])
    if not scored:
        return None, []
    best = scored[0]
    total = sum(s for s, _, _ in scored) or 1.0
    conf = best[0] / total
    return (best[1], best[2], conf), scored[:3]


def render(question, works):
    hit, ranked = answer(question, works)
    if hit is None:
        return (f'  no work matched "{question}".\n'
                f'  The boundary says: this is not a job you were doing.\n'
                f'  (Refusing is the correct answer — a boundary that\n'
                f'   guesses is not a boundary.)'), 0
    w, why, conf = hit
    pct = int(round(conf * 100))
    lines = []
    lines.append(f'  question : {question}')
    lines.append(f'  boundary : {w["label"]}  ({w["resources"]} resources, {w["events"]} events)')
    lines.append(f'  evidence : {why}')
    lines.append(f'  share    : {pct}% of all evidence for this question')
    if w.get("excluded"):
        lines.append('  flag     : EXCLUDED CLASS — never anchors a thread')
    lines.append('')
    lines.append('  the resources it can reopen:')
    for r in w["top"][:5]:
        lines.append(f'    - {r["r"]}')
    if len(ranked) > 1:
        others = ", ".join(f'{ww["label"]}({int(s)}pt)' for s, ww, _ in ranked[1:])
        lines.append(f'  also considered: {others}')
    lines.append('')
    lines.append('  exit 0 — boundary resolved, nothing fused')
    return "\n".join(lines), 0


def sarvam(question, text, lang="hi-IN"):
    """Optional: speak the answer in an Indian language.
    Requires SARVAM_API_KEY. The boundary is already decided; this only
    changes the language it is spoken in."""
    key = os.environ.get("SARVAM_API_KEY")
    if not key:
        print("  [sarvam] no SARVAM_API_KEY set — skipping voice.")
        print("  [sarvam] the boundary answer above is unchanged.")
        return
    try:
        import urllib.request
        body = json.dumps({
            "inputs": [text],
            "target_language_code": lang,
            "speaker": "anushka",
            "model": "bulbul:v2",
        }).encode()
        req = urllib.request.Request(
            "https://api.sarvam.ai/text-to-speech",
            data=body,
            headers={"api-subscription-key": key,
                     "Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=30) as r:
            out = json.loads(r.read())
        audio = out.get("audios", [None])[0]
        if audio:
            path = os.path.join(HERE, "boundary_answer.wav")
            import base64
            with open(path, "wb") as f:
                f.write(base64.b64decode(audio))
            print(f"  [sarvam] spoke the answer -> {path}")
    except Exception as e:
        print(f"  [sarvam] unavailable ({type(e).__name__}). Boundary answer stands.")


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return 0
    with open(BOUNDARY) as f:
        d = json.load(f)
    works = d["works"]

    if args[0] == "--list":
        for w in works:
            tag = "  [excluded]" if w.get("excluded") else ""
            print(f'  {w["label"]:<8} {w["resources"]:>3} resources  {w["events"]:>4} events{tag}')
        return 0

    voice = False
    if args[0] == "--voice":
        voice = True
        args = args[1:]
    q = " ".join(args).strip()
    if not q:
        print(__doc__)
        return 0

    out, code = render(q, works)
    print(out)
    if voice:
        sarvam(q, out)
    return code


if __name__ == "__main__":
    sys.exit(main())
