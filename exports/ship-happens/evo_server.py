#!/usr/bin/env python3
"""
evo_server.py — the local boundary service.

Two ways to run it:

    python3 evo_server.py serve            # HTTP on :8765 (the demo UI)
    python3 evo_server.py mcp              # MCP over stdio (any agent can call it)

It reads a boundary file produced either by the real engine
(`cargo run -p evo-threads --bin boundary_dump`) or, as a fallback, the
pre-computed `boundary.json`. Nothing is inferred here: this process is the
mouth, not the brain.

Optional: export SARVAM_API_KEY to speak answers in Indian languages.
"""
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


# ---------------------------------------------------------------- boundary

def _key(s):
    """Loose identity for matching engine output against hand labels."""
    return s.rstrip("…").strip().lower()


def _overlaps(a, b):
    a, b = _key(a), _key(b)
    if not a or not b:
        return False
    return a == b or a.startswith(b) or b.startswith(a)


def load_boundary(path=None):
    """Load the boundary, preferring the engine's own claim.

    If both files exist the hand-labelled works are shown, but each is marked
    `claimed` when the engine independently arrived at it. That gap is the
    product: the engine asserts little and refuses the rest.
    """
    def read(name):
        p = name if os.path.isabs(name) else os.path.join(HERE, name)
        if not os.path.exists(p):
            return None
        with open(p) as f:
            return json.load(f)

    if path:
        d = read(path)
        if d is None:
            raise SystemExit(f"no boundary file at {path}")
        return normalise(d, os.path.basename(path))

    eng, lab = read("threads.json"), read("boundary.json")

    if eng is not None and lab is not None:
        engine = normalise(eng, "threads.json")
        labels = normalise(lab, "boundary.json")
        strings = []
        for w in engine["works"]:
            strings += w["anchors"] + w["restore_set"]
        for w in labels["works"]:
            w["claimed"] = any(_overlaps(a, e) for a in w["anchors"] for e in strings)
        labels["source"] = "engine+labels"
        labels["claimed"] = sum(1 for w in labels["works"] if w["claimed"])
        labels["engine_works"] = len(engine["works"])
        return labels

    if eng is not None:
        b = normalise(eng, "threads.json")
        for w in b["works"]:
            w["claimed"] = True
        b["claimed"] = len(b["works"])
        return b

    if lab is not None:
        b = normalise(lab, "boundary.json")
        for w in b["works"]:
            w["claimed"] = None
        return b

    raise SystemExit("no boundary file found (threads.json or boundary.json)")


def normalise(d, fname):
    if "works" in d and d.get("source") == "engine":
        works = []
        for w in d["works"]:
            name = short_name(w.get("name") or "")
            works.append({
                "id": w.get("id"),
                "name": name,
                "status": str(w.get("status", "Active")),
                "episodes": w.get("episodes", 0),
                "anchors": w.get("anchors", []),
                "companions": w.get("companions", []),
                "contested": w.get("contested", []),
                "restore_set": w.get("restore_set", []),
                "resume_point": w.get("resume_point", ""),
                "resume_reason": w.get("resume_reason", ""),
                "fork": w.get("fork"),
                "claimed": w.get("claimed"),
            })
        return {"source": "engine", "events": d.get("events", 0), "works": works}

    # label-derived fallback: boundary.json
    works = []
    for w in d.get("works", []):
        anchors = [r["r"] for r in w.get("top", [])]
        works.append({
            "id": w.get("id"),
            "name": w.get("label", "work"),
            "status": "Active",
            "episodes": w.get("events", 0),
            "anchors": anchors,
            "companions": [],
            "contested": [],
            "restore_set": anchors[:5],
            "resume_point": anchors[0] if anchors else "",
            "resume_reason": f'Last seen: {anchors[0]}' if anchors else "",
            "fork": None,
            "claimed": None,
        })
    return {"source": "labels", "events": d.get("totalEvents", 0), "works": works}


def short_name(s, n=44):
    s = s.replace("/Users/bhavitsaini/", "~/").replace("https://", "").replace("http://", "")
    s = re.split(r"[|–—]", s)[0].strip()
    return s if len(s) <= n else s[: n - 1] + "…"


# ------------------------------------------------------------------ recall


# Query expansion. This is recall, not inference: it widens what a question is
# allowed to match. It never decides the boundary between two works; the
# engine already did that. Each entry reads "a question about X, asked of
# resources that look like Y".
CONCEPTS = [
    {"q": ["build", "built", "code", "coding", "repo", "crate", "rust",
           "engine", "compile", "test", "program", "bug"],
     "w": ["evo", "zcode", "crates", "rust", "src", "code", "cargo"]},
    {"q": ["trek", "trekking", "hike", "hiking", "mountain", "annapurna",
           "nepal", "trail", "mountains"],
     "w": ["trek", "annapurna", "himalaya", "trail", "descent", "region"]},
    {"q": ["jacket", "buy", "shopping", "gear", "clothes", "clothing", "hat",
           "cap", "store", "boots", "wear"],
     "w": ["arcteryx", "hats", "caps", "gear", "clothes", "clothing", "store"]},
    {"q": ["pitch", "deck", "slides", "presentation", "investor", "fundraise"],
     "w": ["pitch", "presentation", "deck", "investor"]},
    {"q": ["credits", "free", "api", "login", "account", "router", "key"],
     "w": ["agentrouter", "chatgpt", "login", "console", "auth"]},
    {"q": ["kundli", "astrology", "horoscope", "birth", "chart", "remedies"],
     "w": ["kundli", "claude", "horoscope", "astrology"]},
]


def toks(s):
    return [t for t in re.findall(r"[a-z0-9']+", s.lower()) if t and t not in STOP]


def score(q_tokens, work):
    """Deterministic. No model. Identity carriers weigh more than context."""
    if not q_tokens:
        return 0.0, ""
    hits, s = [], 0.0
    name = set(toks(work["name"]))
    nm = sorted(q_tokens & name)
    if nm:
        s += 3.0
        hits.append("name: " + ",".join(nm))
    anchor_hits = set()
    for a in work["anchors"]:
        anchor_hits |= (q_tokens & set(toks(a)))
    if anchor_hits:
        s += min(len(anchor_hits), 4) * 1.0
        hits.append("anchor: " + ",".join(sorted(anchor_hits)[:4]))
    comp_hits = set()
    for c in work["companions"]:
        comp_hits |= (q_tokens & set(toks(c)))
    if comp_hits:
        s += min(len(comp_hits), 3) * 0.35   # context corroborates, never identifies
        hits.append("companion: " + ",".join(sorted(comp_hits)[:3]))

    # query expansion: only fires when BOTH sides of a concept agree
    bag = set(toks(work["name"]))
    for a in work["anchors"]:
        bag |= set(toks(a))
    for c in work["companions"]:
        bag |= set(toks(c))
    for concept in CONCEPTS:
        asked = q_tokens & set(concept["q"])
        looks = bag & set(concept["w"])
        if asked and looks:
            s += 1.6
            hits.append("concept: " + sorted(asked)[0] + "->" + sorted(looks)[0])
            break
    return s, "; ".join(hits)


def find(boundary, q):
    qt = set(toks(q))
    scored = []
    for w in boundary["works"]:
        s, why = score(qt, w)
        if s > 0:
            scored.append((s, w, why))
    scored.sort(key=lambda x: -x[0])
    if not scored:
        return {"matched": False, "question": q,
                "answer": "That is not a body of work on this machine.",
                "why": "The boundary refuses rather than guesses."}
    total = sum(s for s, _, _ in scored) or 1.0
    s, w, why = scored[0]
    return {
        "matched": True,
        "question": q,
        "work": {"id": w["id"], "name": w["name"], "status": w["status"],
                 "episodes": w["episodes"]},
        "evidence": why,
        "confidence": round(s / total * 100),
        "resume_point": w["resume_point"],
        "resume_reason": w["resume_reason"],
        "reopen": w["restore_set"][:6],
        "rejected": [{"work": x["name"], "points": round(ss)} for ss, x, _ in scored[1:4]],
    }


# ---------------------------------------------------------------- sarvam

def speak(text, lang="hi-IN"):
    key = os.environ.get("SARVAM_API_KEY")
    if not key:
        return None
    try:
        import urllib.request
        body = json.dumps({"inputs": [text], "target_language_code": lang,
                           "speaker": "anushka", "model": "bulbul:v2"}).encode()
        req = urllib.request.Request(
            "https://api.sarvam.ai/text-to-speech", data=body,
            headers={"api-subscription-key": key, "Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=25) as r:
            a = json.loads(r.read()).get("audios") or []
        return a[0] if a else None
    except Exception as e:
        print(f"[sarvam] {type(e).__name__}", file=sys.stderr)
        return None


# ------------------------------------------------------------------- HTTP

class Handler(BaseHTTPRequestHandler):
    boundary = None

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
        b = self.boundary
        if u.path in ("/", "/index.html"):
            p = os.path.join(HERE, "evo_ui.html")
            if not os.path.exists(p):
                p = os.path.join(HERE, "boundary.html")
            self._send(200, "text/html; charset=utf-8", open(p, "rb").read())
            return
        if u.path == "/works":
            self._send(200, "application/json", json.dumps({
                "source": b["source"], "events": b["events"],
                "claimed": b.get("claimed"),
                "engine_works": b.get("engine_works"),
                "works": [{k: w[k] for k in ("id", "name", "status", "episodes",
                                             "resume_point", "resume_reason", "claimed")}
                          for w in b["works"]]}).encode())
            return
        if u.path.startswith("/works/"):
            wid = u.path.rsplit("/", 1)[-1]
            for w in b["works"]:
                if str(w["id"]) == wid:
                    self._send(200, "application/json", json.dumps(w).encode())
                    return
            self._send(404, "application/json", b'{"error":"no such work"}')
            return
        if u.path == "/ask":
            q = (parse_qs(u.query).get("q") or [""])[0].strip()
            if not q:
                self._send(400, "application/json", b'{"error":"empty"}')
                return
            r = find(b, q)
            lang = (parse_qs(u.query).get("lang") or [None])[0]
            if r.get("matched") and lang:
                audio = speak(f'{r["work"]["name"]}. {r["evidence"]}', lang)
                if audio:
                    r["audio"] = audio
            self._send(200, "application/json", json.dumps(r).encode())
            return
        if u.path == "/health":
            self._send(200, "application/json", json.dumps({
                "ok": True, "source": b["source"], "works": len(b["works"]),
                "claimed": b.get("claimed"),
                "sarvam": bool(os.environ.get("SARVAM_API_KEY"))}).encode())
            return
        self._send(404, "text/plain", b"not found")


# -------------------------------------------------------------------- MCP

TOOLS = [
    {
        "name": "list_works",
        "description": "List the bodies of work Evo derived on this machine. "
                       "Each is a separate job with its own boundary. Use this "
                       "before acting on anything, to see what exists.",
        "inputSchema": {"type": "object", "properties": {}},
    },
    {
        "name": "find_work",
        "description": "Given a resource, path, URL or phrase, say which body "
                       "of work it belongs to, the evidence, and which other "
                       "works were considered and rejected. Refuses when "
                       "nothing matches.",
        "inputSchema": {"type": "object",
                        "properties": {"query": {"type": "string"}},
                        "required": ["query"]},
    },
    {
        "name": "resume_work",
        "description": "Get the exact surfaces to reopen for a work: where the "
                       "person left off and the neighborhood, not the whole "
                       "history.",
        "inputSchema": {"type": "object",
                        "properties": {"id": {"type": "string"}},
                        "required": ["id"]},
    },
]


def mcp_tool_call(b, name, args):
    if name == "list_works":
        lines = [f'{w["id"]}: {w["name"]} ({w["episodes"]} episodes, {w["status"]})'
                 for w in b["works"]]
        return "\n".join(lines) or "no works"
    if name == "find_work":
        r = find(b, args.get("query", ""))
        if not r["matched"]:
            return f'REFUSED. {r["answer"]} {r["why"]}'
        out = [f'work: {r["work"]["name"]} (id {r["work"]["id"]})',
               f'evidence: {r["evidence"]}',
               f'confidence: {r["confidence"]}%']
        if r["rejected"]:
            out.append("rejected: " + ", ".join(f'{x["work"]}({x["points"]})'
                                                for x in r["rejected"]))
        out.append(f'resume: {r["resume_reason"]}')
        return "\n".join(out)
    if name == "resume_work":
        for w in b["works"]:
            if str(w["id"]) == str(args.get("id")):
                out = [f'work: {w["name"]}', f'resume: {w["resume_reason"]}',
                       "reopen:"]
                out += [f'  - {r}' for r in w["restore_set"][:8]]
                return "\n".join(out)
        return f"no work with id {args.get('id')}"
    return f"unknown tool {name}"


def run_mcp():
    b = load_boundary()
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except Exception:
            continue
        mid = msg.get("id")
        method = msg.get("method")
        if method == "initialize":
            res = {"protocolVersion": "2024-11-05",
                   "capabilities": {"tools": {}},
                   "serverInfo": {"name": "evo-boundary", "version": "0.1.0"}}
        elif method == "tools/list":
            res = {"tools": TOOLS}
        elif method == "tools/call":
            p = msg.get("params") or {}
            text = mcp_tool_call(b, p.get("name"), p.get("arguments") or {})
            res = {"content": [{"type": "text", "text": text}]}
        elif method in ("notifications/initialized", "notifications/cancelled"):
            continue
        else:
            if mid is None:
                continue
            out = {"jsonrpc": "2.0", "id": mid,
                   "error": {"code": -32601, "message": f"unknown method {method}"}}
            print(json.dumps(out), flush=True)
            continue
        if mid is not None:
            print(json.dumps({"jsonrpc": "2.0", "id": mid, "result": res}), flush=True)


# ------------------------------------------------------------------- main

def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "serve"
    if mode == "mcp":
        run_mcp()
        return
    Handler.boundary = load_boundary()
    # --port beats $PORT (Render and most hosts inject $PORT)
    port = int(os.environ.get("PORT", 8765))
    if "--port" in sys.argv:
        port = int(sys.argv[sys.argv.index("--port") + 1])
    print(f'\n  Evo boundary service  ->  http://localhost:{port}')
    print(f'  source                ->  {Handler.boundary["source"]} '
          f'({len(Handler.boundary["works"])} works)')
    print(f'  sarvam                ->  '
          f'{"on" if os.environ.get("SARVAM_API_KEY") else "off (no key)"}')
    print(f'  MCP                   ->  python3 evo_server.py mcp\n')
    ThreadingHTTPServer(("0.0.0.0", port), Handler).serve_forever()


if __name__ == "__main__":
    main()
