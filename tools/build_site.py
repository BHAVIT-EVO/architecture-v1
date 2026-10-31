#!/usr/bin/env python3
"""Builds site/index.html — the public demo.

    python3 tools/build_site.py

The public site must never contain the real corpus. boundary.json holds one
person's actual browsing; publishing it publishes their life. So this builds a
synthetic day: eight invented bodies of work for an invented person, with the
same structure the engine produces.

Output is one self-contained file. No server, no build step, no network.
"""
import json
import os
import random

random.seed(7)

BASE = 1789200000000          # arbitrary fixed epoch; deterministic
DAY = 86_400_000
MIN = 60_000

# ---------------------------------------------------------------- corpus
# name, claims_a_work, colour, resources, event weight
WORKS = [
    {
        "label": "grant",
        "claimed": True,
        "colour": "#C89B6B",
        "resources": [
            "~/Documents/clinic-grant/application-v6.docx",
            "~/Documents/clinic-grant/budget-2027.xlsx",
            "https://funding.council.gov.uk/portal/application/88412",
            "~/Documents/clinic-grant/letters-of-support.pdf",
            "~/Documents/clinic-grant/outcomes-evidence.docx",
            "https://funding.council.gov.uk/portal/eligibility",
        ],
        "events": 1648,
    },
    {
        "label": "thesis",
        "claimed": True,
        "colour": "#6FA8A0",
        "resources": [
            "~/Documents/thesis/chapter-4-draft.md",
            "~/Documents/thesis/references.bib",
            "https://scholar.google.com/citations?user=8f2a",
            "~/Documents/thesis/figures/fig-4-2.svg",
            "https://arxiv.org/abs/2409.01188",
        ],
        "events": 1552,
    },
    {
        "label": "invoices",
        "claimed": False,
        "colour": "#7B8CDE",
        "resources": [
            "~/Documents/studio/invoices/q3-2026.xlsx",
            "https://dashboard.stripe.com/invoices",
            "~/Documents/studio/contracts/northwind.pdf",
            "https://mail.google.com/mail/u/0/#inbox/Q3",
        ],
        "events": 384,
    },
    {
        "label": "portfolio",
        "claimed": False,
        "colour": "#C77DBA",
        "resources": [
            "~/code/portfolio/src/pages/index.astro",
            "https://localhost:4321/",
            "https://fonts.google.com/specimen/Instrument+Serif",
            "~/code/portfolio/src/styles/global.css",
        ],
        "events": 572,
    },
    {
        "label": "lesson",
        "claimed": False,
        "colour": "#E0A458",
        "resources": [
            "~/Movies/lessons/recording-04.mov",
            "~/Documents/teaching/lesson-plan-04.md",
            "https://obsproject.com/open-project",
            "~/Documents/teaching/slides-04.key",
        ],
        "events": 244,
    },
    {
        "label": "flat",
        "claimed": False,
        "colour": "#8FA35A",
        "resources": [
            "https://www.rightmove.co.uk/property-for-sale/2-bed",
            "https://www.openrent.co.uk/property-to-rent/london",
            "~/Downloads/floorplan-camberwell.pdf",
        ],
        "events": 296,
    },
    {
        "label": "comms",
        "claimed": False,
        "colour": "#6B7A8F",
        "excluded": True,
        "resources": ["Zoom", "https://discord.com/channels/1180"],
        "events": 152,
    },
    {
        "label": "noise",
        "claimed": False,
        "colour": "#4A4E52",
        "excluded": True,
        "resources": ["Login", "Untitled", "System Settings", "Software Update"],
        "events": 44,
    },
]

# Query vocabulary — how a person would actually ask about each work.
VOCAB = {
    "grant": ["grant", "funding", "application", "clinic", "budget", "funder",
              "council", "bid", "supporting", "outcomes"],
    "thesis": ["thesis", "chapter", "paper", "reference", "citation", "draft",
               "figure", "writing", "phd", "research"],
    "invoices": ["invoice", "money", "paid", "stripe", "client", "contract",
                 "billing", "q3", "accounts"],
    "portfolio": ["portfolio", "website", "site", "css", "design", "astro",
                  "font", "redesign"],
    "lesson": ["lesson", "teaching", "record", "video", "course", "slides",
               "students", "class"],
    "flat": ["flat", "apartment", "rent", "house", "moving", "council tax",
             "tenancy", "property"],
    "comms": ["call", "zoom", "discord", "talking", "meeting"],
    "noise": ["login", "settings", "update", "system"],
}

STOP = set("""a an the i my me we our you your it its is was were be been am
do did does doing have has had what which who when where why how this that
these those of in on at to for from with about into over after before and or
but not no yes can could should would will work works working worked
yesterday today tomorrow morning night day days time get got again back
start started was there here then than so if as by up down out off all any
some please show tell give find""".split())


def tokens(s):
    out, cur = [], ""
    for ch in s.lower():
        if ch.isalnum() or ch == "'":
            cur += ch
        else:
            if cur:
                out.append(cur)
            cur = ""
    if cur:
        out.append(cur)
    return [t for t in out if t and t not in STOP]


def build_data():
    works, stream = [], []
    t = BASE
    for i, w in enumerate(WORKS):
        dur = w["events"] * 1500
        first = BASE + i * 420_000
        last = first + dur
        for k in range(w["events"]):
            ts = first + int(dur * (k / max(1, w["events"] - 1)))
            stream.append([ts, i])
        works.append({
            "id": i,
            "label": w["label"],
            "events": w["events"],
            "resources": len(w["resources"]),
            "first": first,
            "last": last,
            "excluded": w.get("excluded", False),
            "claimed": w["claimed"],
            "colour": w["colour"],
            "top": [{"r": r, "n": max(1, w["events"] // (j + 1))}
                    for j, r in enumerate(w["resources"])],
            "kinds": {"Typed": w["events"] // 2, "Focused": w["events"] // 3,
                      "Visited": w["events"] // 6},
        })
    stream.sort()
    step = max(1, len(stream) // 2600)
    stream = stream[::step]
    return {
        "synthetic": True,
        "works": works,
        "stream": stream,
        "t0": min(s[0] for s in stream),
        "t1": max(s[0] for s in stream),
        "totalEvents": sum(w["events"] for w in WORKS),
        "claimed": sum(1 for w in WORKS if w["claimed"]),
    }


HTML = r"""<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Evo — the boundary layer for AI agents</title>
<meta name="description" content="Evo derives where one body of work ends and the next begins, on the device, without a model, so agents can act on a real machine.">
<style>
:root{--ink:#EDE9E3;--bg:#13161A;--panel:#1B1F24;--dim:#8A8F94;--car:#C89B6B;--rule:#2A2F34;--ok:#5FBF8F;--bad:#E06A5C}
*{box-sizing:border-box;margin:0;padding:0}
body{background:var(--bg);color:var(--ink);font:14px/1.55 ui-sans-serif,-apple-system,"Segoe UI",Roboto,sans-serif;-webkit-font-smoothing:antialiased}
.wrap{max-width:1120px;margin:0 auto;padding:30px 26px 90px}
header{display:flex;align-items:flex-end;justify-content:space-between;gap:24px;border-bottom:1px solid var(--rule);padding-bottom:18px;margin-bottom:20px;flex-wrap:wrap}
.wordmark{font-size:27px;font-weight:800;letter-spacing:-.6px}
.wordmark span{color:var(--car)}
.sub{color:var(--dim);font-size:12.5px;margin-top:4px;max-width:560px}
.stats{display:flex;gap:24px;text-align:right}
.stat b{display:block;font-size:20px;font-weight:700;color:var(--car)}
.stat s{display:block;text-decoration:none;font-size:9.5px;color:var(--dim);text-transform:uppercase;letter-spacing:.1em;margin-top:2px}
.tag{display:inline-block;font-size:9.5px;font-weight:800;letter-spacing:.11em;text-transform:uppercase;padding:3px 9px;border-radius:20px;background:#2A2118;color:var(--car);border:1px solid #4A3826;margin-bottom:14px}
h2{font-size:10.5px;text-transform:uppercase;letter-spacing:.14em;color:var(--car);font-weight:700;margin:30px 0 10px}
.panel{background:var(--panel);border:1px solid var(--rule);border-radius:10px;padding:17px 19px}
#lanes{display:flex;flex-direction:column;gap:7px}
.lane{display:grid;grid-template-columns:104px 1fr 34px;gap:12px;align-items:center;cursor:pointer;padding:3px 0;border-radius:5px}
.lane:hover{background:#20262B}
.lane.on{background:#252B31}
.lname{font-size:11.5px;font-weight:700;color:var(--dim);text-align:right;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.lane.on .lname{color:var(--ink)}
.lref{font-size:9.5px;color:#5C6167;text-align:left}
.track{position:relative;height:15px;background:#171B1F;border-radius:3px;overflow:hidden}
.dot{position:absolute;top:3px;width:3px;height:9px;border-radius:1px;opacity:.85}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(172px,1fr));gap:10px}
.card{border:1px solid var(--rule);border-radius:9px;padding:12px 13px;background:var(--panel);cursor:pointer;transition:.12s}
.card:hover{border-color:#3A4147;transform:translateY(-1px)}
.card.sel{border-color:var(--car);box-shadow:0 0 0 1px var(--car) inset}
.card.refused{opacity:.55}
.cname{font-weight:700;font-size:13px;display:flex;align-items:center;gap:8px}
.sw{width:9px;height:9px;border-radius:2px;flex:none}
.cmeta{color:var(--dim);font-size:11px;margin-top:6px;line-height:1.45}
.btn{background:transparent;color:var(--ink);border:1px solid #394046;border-radius:7px;padding:9px 14px;font:inherit;font-size:12px;font-weight:600;cursor:pointer;transition:.12s}
.btn:hover:not(:disabled){border-color:var(--car);color:var(--car)}
.btn:disabled{opacity:.32;cursor:not-allowed}
.btn.primary{background:var(--car);border-color:var(--car);color:#13161A}
.row{display:flex;gap:9px;align-items:center;flex-wrap:wrap}
#q{flex:1;min-width:250px;background:#171B1F;border:1px solid var(--rule);color:var(--ink);border-radius:7px;padding:10px 12px;font:inherit;font-size:13px}
#q:focus{outline:none;border-color:var(--car)}
#say{margin-top:13px;padding:12px 14px;border-left:2px solid var(--car);background:#1A1E22;font-size:13px;border-radius:0 7px 7px 0;min-height:46px}
#say .k{color:var(--car);font-weight:700}
#ans{display:none;margin-top:13px;border-top:1px solid var(--rule);padding-top:13px}
.wname{font-size:17px;font-weight:800;color:var(--car)}
.ev{font-size:11.5px;color:var(--dim);margin-top:5px}
.bar{margin-top:9px;height:4px;background:#23282D;border-radius:3px;overflow:hidden;max-width:260px}
.bar i{display:block;height:100%;background:var(--car)}
.rej{margin-top:9px;font-size:11.5px;color:var(--dim)}
.rej b{color:#C0AE9A}
.refuse{font-size:16px;font-weight:700;color:var(--bad)}
table{width:100%;border-collapse:collapse;font-size:12px}
td{padding:8px 0;border-bottom:1px solid var(--rule);vertical-align:top}
td:first-child{width:28px}
tr:last-child td{border-bottom:none}
.pill{display:inline-block;font-size:9px;font-weight:800;letter-spacing:.08em;padding:2px 7px;border-radius:20px}
.p-ok{background:#1D3229;color:var(--ok)}
.p-bad{background:#3A201C;color:var(--bad)}
.gname{font-weight:600}
.gwhy{color:var(--dim);font-size:11px;margin-top:1px}
#banner{display:none;margin-top:13px;padding:13px 15px;border-radius:8px;background:#3A201C;border:1px solid #6B3A32;color:#F0B8AE;font-size:13px;font-weight:600}
#banner code{font-family:ui-monospace,monospace;background:#2A1714;padding:1px 5px;border-radius:3px;color:#E0A39A}
.two{display:grid;grid-template-columns:1fr 350px;gap:15px;align-items:start}
.foot{margin-top:40px;padding-top:18px;border-top:1px solid var(--rule);color:var(--dim);font-size:11.5px;line-height:1.75}
.foot b{color:#B8BABE}
.chip{display:inline-block;font-size:11px;color:var(--dim);border:1px solid var(--rule);border-radius:20px;padding:4px 11px;margin:0 6px 6px 0;cursor:pointer}
.chip:hover{color:var(--car);border-color:var(--car)}
a{color:var(--car)}
@media(max-width:860px){.two{grid-template-columns:1fr}.stats{display:none}}
</style></head><body><div class="wrap">

<header>
  <div>
    <div class="wordmark">EVO <span>BOUNDARY</span></div>
    <div class="sub">Every agent today is <i>told</i> where its work begins and ends. Evo derives it, on the device, without a model — and lets any agent ask.</div>
  </div>
  <div class="stats">
    <div class="stat"><b id="s-ev">—</b><s>events</s></div>
    <div class="stat"><b id="s-wk">—</b><s>works</s></div>
    <div class="stat"><b id="s-cl">—</b><s>claimed</s></div>
  </div>
</header>

<div class="tag">Synthetic demo corpus — no real user data</div>

<h2>Two days, one machine, as the engine sees it</h2>
<div class="panel"><div id="lanes"></div></div>

<h2>The boundary map</h2>
<div class="grid" id="cards"></div>

<h2>Ask the boundary</h2>
<div class="panel">
  <div class="row">
    <input id="q" placeholder="ask about the work — try: where did I look for flats to rent?" autocomplete="off">
    <button class="btn primary" id="go">Ask</button>
  </div>
  <div style="margin-top:11px">
    <span class="chip" data-q="where did I look for flats to rent">flats to rent</span>
    <span class="chip" data-q="which chapter was I writing">the chapter</span>
    <span class="chip" data-q="did I send the invoice">the invoice</span>
    <span class="chip" data-q="what is the capital of france">not a job</span>
  </div>
  <div id="ans"></div>
</div>

<div class="two" style="margin-top:28px">
  <div>
    <h2 style="margin-top:0">Try to break it</h2>
    <div class="panel">
      <div class="row">
        <button class="btn" id="btn-infer">Merge these by inference</button>
        <button class="btn primary" id="btn-declare" disabled>Declare them the same work</button>
        <button class="btn" id="btn-reset">Reset</button>
      </div>
      <div id="say">Select two works above, then try to merge them.</div>
      <div id="banner"></div>
    </div>
  </div>
  <div>
    <h2 style="margin-top:0">The verifier</h2>
    <div class="panel"><table id="gate"></table></div>
  </div>
</div>

<div class="foot">
  <b>What is real here.</b> The engine is real: 16 Rust crates, deterministic, offline, no HTTP client, no model in the decision path. The gate below is the real assertion set from <code>crates/evo-threads</code> — it fails the build if two bodies of work ever fuse.<br><br>
  <b>What is not.</b> This page runs a <i>synthetic</i> corpus so that publishing it does not publish anyone's browsing history. The two buttons demonstrate the gate: they simulate a fusion and show the verifier rejecting it. They do not write to the engine.<br><br>
  <b>Why so few claimed.</b> Two of eight works are claimed; six are refused. That is the design, not a defect — a wrong merge is irreversible and a refusal costs nothing. A boundary that guesses is not a boundary.
</div>

<script>
const D = __DATA__;
const W = D.works;
let sel = [], fused = false;

document.getElementById('s-ev').textContent = D.totalEvents.toLocaleString();
document.getElementById('s-wk').textContent = W.length;
document.getElementById('s-cl').textContent = D.claimed;

const span = Math.max(1, D.t1 - D.t0);
const lanes = document.getElementById('lanes');
W.forEach(w=>{
  const el = document.createElement('div');
  el.className='lane'; el.dataset.id=w.id;
  const dots = D.stream.filter(e=>e[1]===w.id)
    .map(e=>`<i class="dot" style="left:${((e[0]-D.t0)/span*100).toFixed(3)}%;background:${w.colour}"></i>`).join('');
  el.innerHTML = `<div class="lname">${w.label}</div><div class="track">${dots}</div>
    <div class="lref">${w.claimed?'':'ref'}</div>`;
  el.onclick = ()=>toggle(w.id);
  lanes.appendChild(el);
});

const cards = document.getElementById('cards');
W.forEach(w=>{
  const c = document.createElement('div');
  c.className='card'+(w.claimed?'':' refused'); c.dataset.id=w.id;
  c.innerHTML = `<div class="cname"><i class="sw" style="background:${w.claimed?w.colour:'#41474E'}"></i>${w.label}</div>
    <div class="cmeta">${w.resources} resources · ${w.events} events${w.claimed?'':'<br><b>refused: not asserted</b>'}</div>`;
  c.onclick = ()=>toggle(w.id);
  cards.appendChild(c);
});

function toggle(id){
  if(fused) return;
  if(sel.includes(id)) sel = sel.filter(x=>x!==id);
  else { sel.push(id); if(sel.length>2) sel.shift(); }
  paint();
}

function paint(){
  document.querySelectorAll('.lane,.card').forEach(e=>{
    e.classList.toggle('on', sel.includes(+e.dataset.id));
    e.classList.toggle('sel', sel.includes(+e.dataset.id));
  });
  const d = document.getElementById('btn-declare');
  d.disabled = sel.length!==2 || fused;
  d.textContent = fused ? 'Declared — merged' : 'Declare them the same work';
  const a = W.find(x=>x.id===sel[0]), b = W.find(x=>x.id===sel[1]);
  if(fused && a && b){
    say(`<span class="k">Merged.</span> "${a.label}" and "${b.label}" are now one thread. A declaration is the only path that could do this — and the gate caught it.`, '#F0B8AE');
  } else if(sel.length===2){
    say(`Selected <span class="k">${a.label}</span> and <span class="k">${b.label}</span>. Inference cannot fuse them. Try it.`);
  } else {
    say(`Select two works. Then try to break the boundary.`);
  }
  gate();
}
function say(html, col){
  const s=document.getElementById('say'); s.innerHTML=html;
  s.style.borderLeftColor = col || '#C89B6B';
}
document.getElementById('btn-infer').onclick = ()=>{
  if(sel.length!==2){ say('Select two works first.'); return; }
  say(`<span class="k">Refused.</span> No inference path exists. A body of work is a claim about relationships, and only a person can make it. <span class="k">exit 0</span>.`, '#5FBF8F');
};
document.getElementById('btn-declare').onclick = ()=>{
  if(sel.length!==2) return;
  fused = true;
  const b=document.getElementById('banner');
  b.style.display='block';
  b.innerHTML = `<code>threads fusing 2+ distinct works: 1</code><br>FUSED: {"${W.find(x=>x.id===sel[0]).label}", "${W.find(x=>x.id===sel[1]).label}"}<br><br>Investigate the fused thread above.<br><b>exit code 1</b> — the build fails.`;
  paint();
};
document.getElementById('btn-reset').onclick = ()=>{
  sel=[]; fused=false;
  document.getElementById('banner').style.display='none';
  paint();
};

/* ---- the query layer, same rules as evo_server.py ---- */
const STOP=new Set("__STOP__".split(" "));
const SYN=__SYN__;
function toks(s){ return (s.toLowerCase().match(/[a-z0-9']+/g)||[]).filter(t=>!STOP.has(t)); }
function score(q,w){
  const qt=new Set(toks(q)); if(!qt.size) return [0,''];
  let s=0, hits=[];
  const name=new Set(toks(w.label));
  const nm=[...qt].filter(t=>name.has(t));
  if(nm.length){ s+=3; hits.push('name: '+nm.join(',')); }
  const syn=new Set(SYN[w.label]||[]);
  const sy=[...qt].filter(t=>syn.has(t));
  if(sy.length){ s+=2; hits.push('term: '+sy.join(',')); }
  let rh=new Set();
  w.top.forEach(r=>{ const rt=new Set(toks(r.r)); [...qt].forEach(t=>{ if(rt.has(t)) rh.add(t); }); });
  if(rh.size){ s+=Math.min(rh.size,4)*0.75; hits.push('resource: '+[...rh].sort().slice(0,4).join(',')); }
  return [s,hits.join('; ')];
}
function askLocal(q){
  const scored=W.map(w=>{ const [s,why]=score(q,w); return {s,w,why}; }).filter(x=>x.s>0).sort((a,b)=>b.s-a.s);
  if(!scored.length) return {matched:false,question:q};
  const tot=scored.reduce((n,x)=>n+x.s,0)||1;
  const b=scored[0];
  return {matched:true,work:b.w.label,resources:b.w.resources,events:b.w.events,
          evidence:b.why,share:Math.round(b.s/tot*100),claimed:b.w.claimed,
          reopen:b.w.top.slice(0,5).map(r=>r.r),
          rejected:scored.slice(1).map(x=>({work:x.w.label,points:Math.round(x.s)}))};
}
function showAns(a){
  const box=document.getElementById('ans'); box.style.display='block';
  if(!a.matched){
    box.innerHTML=`<div class="refuse">Not a job.</div>
      <div class="ev">No work matches that. Refusing is the correct answer — a boundary that guesses is not a boundary.</div>`;
    return;
  }
  box.innerHTML=`<div class="wname">${a.work}</div>
    <div class="ev">${a.resources} resources · ${a.events} events${a.claimed?'':' · <b>refused by the engine</b>'}</div>
    <div class="ev">${a.evidence}</div>
    <div class="bar"><i style="width:${a.share}%"></i></div>
    <div class="ev" style="margin-top:5px">${a.share}% of the evidence for this question</div>
    ${a.rejected && a.rejected.length ? `<div class="rej">also considered: <b>${a.rejected.map(r=>r.work+'('+r.points+'pt)').join(', ')}</b></div>` : ''}`;
}
function ask(q){ if(!q.trim()) return; showAns(askLocal(q)); }
document.getElementById('go').onclick=()=>ask(document.getElementById('q').value);
document.getElementById('q').addEventListener('keydown',e=>{ if(e.key==='Enter') ask(e.target.value); });
document.querySelectorAll('.chip').forEach(c=>c.onclick=()=>{
  document.getElementById('q').value=c.dataset.q; ask(c.dataset.q);
});

function gate(){
  const rows = [
    ['corpus loaded', W.reduce((n,w)=>n+w.events,0)+' events parsed', true],
    ['threads minted', W.length+' works', true],
    ['no two works fuse', fused ? 'FUSED: 2 works in 1 thread' : 'fused 0', !fused],
    ['no leisure anchor', fused ? 'cannot evaluate — gate already failed' : 'leisure anchor false', !fused],
    ['determinism', fused ? 'halted' : 'same log twice = identical', !fused]
  ];
  document.getElementById('gate').innerHTML = rows.map(([n,w,ok])=>
    `<tr><td>${ok?'<span class="pill p-ok">PASS</span>':'<span class="pill p-bad">FAIL</span>'}</td>
     <td><div class="gname">${n}</div><div class="gwhy">${w}</div></td></tr>`).join('');
}
paint();
</script></div></body></html>
"""


def build():
    d = build_data()
    data = json.dumps(d, separators=(",", ":"))
    stop = " ".join(sorted(STOP))
    syn = json.dumps(VOCAB, separators=(",", ":"))
    html = HTML.replace("__DATA__", data).replace("__STOP__", stop).replace("__SYN__", syn)

    here = os.path.dirname(os.path.abspath(__file__))
    outdir = os.path.normpath(os.path.join(here, "..", "site"))
    os.makedirs(outdir, exist_ok=True)
    out = os.path.join(outdir, "index.html")
    with open(out, "w") as f:
        f.write(html)
    print("wrote", out, len(html), "bytes")
    print("works:", [(w["label"], "claimed" if w["claimed"] else "refused")
                     for w in d["works"]])
    print("events:", d["totalEvents"])


if __name__ == "__main__":
    build()
