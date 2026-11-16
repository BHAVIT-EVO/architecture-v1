#!/usr/bin/env python3
"""
serve.py — the demo server. Standard library only, no pip install.

    python3 serve.py

Then open http://localhost:8765

Serves the boundary explorer and one endpoint:

    GET /ask?q=<question>   -> JSON: which work, the evidence, the share,
                               what it rejected, and the resources.

If SARVAM_API_KEY is set, /ask also returns spoken audio of the answer in
an Indian language. The boundary is decided locally by the same rules as
ask.py; Sarvam only changes the language it is spoken in. Nothing about
the boundary is sent to a model.

    export SARVAM_API_KEY=...        # optional
    python3 serve.py --port 8765
"""
import base64
import json
import os
import re
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse, parse_qs

HERE = os.path.dirname(os.path.abspath(__file__))

STOP = set("""a an the i my me we our you your it its is was were be been am
do did does doing have has had what which who when where why how
this that these those of in on at to for from with about into over
after before and or but not no yes can could should would will
work works working worked yesterday today tomorrow morning
night day days time get got again back start started
was there here then than so if as by up down out off all any some
please show tell give find""".split())

SYN = {
    "trek": ["trek", "trekking", "annapurna", "hike", "hiking", "mountain", "nepal", "trail", "youtube"],
    "gear": ["gear", "jacket", "arcteryx", "hat", "cap", "clothes", "clothing", "buy", "shopping", "boots", "nimsdai"],
    "evo": ["evo", "code", "codebase", "repo", "rust", "crate", "engine", "build", "compile", "test", "capture", "zcode"],
    "pitch": ["pitch", "deck", "presentation", "slides", "investor"],
    "freeai": ["free", "credits", "router", "agentrouter", "chatgpt", "login", "account", "api"],
    "kundli": ["kundli", "horoscope", "astrology", "birth", "chart", "remedies", "claude"],
    "comms": ["call", "facetime", "discord", "chat", "talk"],
    "noise": ["chrome", "tab", "untitled"],
}


def toks(s):
    return [t for t in re.findall(r"[a-z0-9']+", s.lower()) if t and t not in STOP]


def score(q, w):
    qt = set(toks(q))
    if not qt:
        return 0.0, ""
    hits, s = [], 0.0
    name = set(toks(w["label"]))
    if qt & name:
        hits.append("name match: " + ",".join(sorted(qt & name)))
        s += 3.0
    syn = set(SYN.get(w["label"], []))
    if qt & syn:
        hits.append("synonym: " + ",".join(sorted(qt & syn)))
        s += 2.0
    rh = []
    for r in w["top"]:
        if qt & set(toks(r["r"])):
            rh += sorted(qt & set(toks(r["r"])))
    if rh:
        hits.append("resource text: " + ",".join(sorted(set(rh))[:4]))
        s += min(len(set(rh)), 4) * 0.75
    return s, "; ".join(hits)


def ask(q, works, lang=None):
    scored = [(s, w, why) for w in works for s, why in [score(q, w)] if s > 0]
    scored.sort(key=lambda x: -x[0])
    if not scored:
        return {"matched": False, "question": q,
                "message": "This is not a job you were doing.",
                "detail": "The boundary refuses rather than guesses."}
    total = sum(s for s, _, _ in scored) or 1.0
    s, w, why = scored[0]
    out = {
        "matched": True,
        "question": q,
        "work": w["label"],
        "resources": w["resources"],
        "events": w["events"],
        "evidence": why,
        "share": round(s / total * 100),
        "excluded": bool(w.get("excluded")),
        "reopen": [r["r"] for r in w["top"][:5]],
        "rejected": [{"work": ww["label"], "points": round(ss)} for ss, ww, _ in scored[1:]],
    }
    if lang and os.environ.get("SARVAM_API_KEY"):
        audio = speak(f'{w["label"]} — {why}', lang)
        if audio:
            out["audio"] = audio
    return out


def speak(text, lang="hi-IN"):
    """Sarvam TTS. Returns base64 audio or None. Never raises."""
    key = os.environ.get("SARVAM_API_KEY")
    if not key:
        return None
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
        with urllib.request.urlopen(req, timeout=25) as r:
            audios = json.loads(r.read()).get("audios") or []
        return audios[0] if audios else None
    except Exception as e:
        print(f"  [sarvam] {type(e).__name__}: {e}", file=sys.stderr)
        return None


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def _send(self, code, ctype, body):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        u = urlparse(self.path)
        if u.path in ("/", "/index.html", "/boundary.html"):
            try:
                with open(os.path.join(HERE, "boundary.html"), "rb") as f:
                    self._send(200, "text/html; charset=utf-8", f.read())
            except FileNotFoundError:
                self._send(404, "text/plain", b"boundary.html not found")
            return
        if u.path == "/ask":
            qs = parse_qs(u.query)
            q = (qs.get("q") or [""])[0].strip()
            lang = (qs.get("lang") or [None])[0]
            if not q:
                self._send(400, "application/json", b'{"error":"empty question"}')
                return
            with open(os.path.join(HERE, "boundary.json")) as f:
                works = json.load(f)["works"]
            body = json.dumps(ask(q, works, lang)).encode()
            self._send(200, "application/json", body)
            return
        self._send(404, "text/plain", b"not found")


def main():
    port = 8765
    if "--port" in sys.argv:
        port = int(sys.argv[sys.argv.index("--port") + 1])
    srv = ThreadingHTTPServer(("0.0.0.0", port), Handler)
    voice = "on" if os.environ.get("SARVAM_API_KEY") else "off (no SARVAM_API_KEY)"
    print(f"\n  Evo boundary explorer  ->  http://localhost:{port}")
    print(f"  Sarvam voice           ->  {voice}")
    print("  Ctrl-C to stop\n")
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\n  stopped\n")


if __name__ == "__main__":
    main()
