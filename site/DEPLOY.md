# Deploying the site

## The live URL (already done)

```
https://bhavit-evo.github.io/evo-boundary-demo/
```

That repository — `BHAVIT-EVO/evo-boundary-demo`, public — holds five
artifacts copied from this folder: `index.html`, `demo.html`, `source.html`,
`film.html`, `fonts/`. The private repository never becomes public, because
`exports/ship-happens/boundary.json` holds one person's real browsing.

### Updating it

```bash
python3 tools/build_site_v2.py     # only if you changed site/src/
bash tools/deploy_site.sh          # clones the public repo, copies, pushes
```

Pages rebuilds in about a minute. To check:

```bash
curl -s -o /dev/null -w "%{http_code}\n" https://bhavit-evo.github.io/evo-boundary-demo/
```

If the push is refused, the public pages are behind a token that cannot write
to that repository. Do it by hand:

```bash
git clone https://github.com/BHAVIT-EVO/evo-boundary-demo && cd evo-boundary-demo
cp ~/Evo-Evolution.nosync/architecture-v1/site/{index,demo,source,film}.html .
mkdir -p fonts && cp ~/Evo-Evolution.nosync/architecture-v1/site/fonts/*.woff2 fonts/
git add -A && git commit -m "site" && git push
```

### What must be true of anything you publish

- Synthetic corpus only. Never `boundary.json`, never `boundary.html`.
- No keys in the file. The site is static and public.
- `fonts/` travels with `index.html` — the type is self-hosted, so the folder
  is part of the artifact.

---

## The .xyz domain

Do not use the apex (`yourname.xyz`) for a hackathon demo. Point a subdomain at
it and keep the apex free for the real company: `demo.yourname.xyz`.

### Option A — Render static site (you have credits)

1. render.com → New → **Static Site**
2. Connect `BHAVIT-EVO/architecture-v1`, branch `arena/2873f4c9-architecture-v1`
3. Build command: leave **empty**
4. Publish directory: `site`
5. Deploy. You get `something.onrender.com` in ~40 seconds.
6. Settings → Custom Domains → add `demo.yourname.xyz`
7. Render shows a CNAME target. Add that CNAME at your registrar.
8. Wait for the certificate (2–10 minutes).

### Option B — GitHub Pages + CNAME

1. Add the subdomain in the public repo → Settings → Pages → Custom domain.
2. At your registrar: CNAME `demo` → `bhavit-evo.github.io`
3. Wait. DNS can take 5 minutes or an hour; it is not broken at minute six.

| Type | Name | Value | TTL |
|---|---|---|---|
| CNAME | `demo` | `bhavit-evo.github.io` | 3600 |

Never point a CNAME at the apex. Apex uses A/ALIAS records, a different job.
