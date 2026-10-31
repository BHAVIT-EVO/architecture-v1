# Deploying the .xyz domain

Do not use the apex (`yourname.xyz`) for a hackathon demo. Point a subdomain at
it and keep the apex free for the real company.

Use `demo.yourname.xyz`.

## Option A — Render static site (you have $50 of credits)

1. render.com → New → **Static Site**
2. Connect `BHAVIT-EVO/architecture-v1`, branch `arena/2873f4c9-architecture-v1`
3. Build command: leave **empty**
4. Publish directory: `site`
5. Deploy. You get `something.onrender.com` in ~40 seconds.
6. Settings → Custom Domains → add `demo.yourname.xyz`
7. Render shows a CNAME target. Add that CNAME at your domain registrar.
8. Wait for the certificate (2–10 minutes).

## Option B — GitHub Pages (free, no account)

Needs the file on a branch GitHub can serve. Fastest path:

1. Copy `site/index.html` to a new repo, or to a `gh-pages` branch.
2. Repo → Settings → Pages → Source: branch `gh-pages`, folder `/`
3. Custom domain: `demo.yourname.xyz` → creates a `CNAME` file
4. At your registrar: CNAME `demo` → `bhavit-evo.github.io`

## What goes in the DNS panel

The registrar gave you a control panel when you claimed the .xyz. Look for **DNS**
or **Manage DNS**, then **Add record**:

| Type | Name | Value | TTL |
|---|---|---|---|
| CNAME | `demo` | *the target your host shows you* | 3600 |

Never point a CNAME at the apex. Apex uses A/ALIAS records, which is a
different job and will break.

## Checks

```bash
dig demo.yourname.xyz +short        # should print the host's address
curl -I https://demo.yourname.xyz   # should be 200
```

DNS can take 5 minutes or an hour. It is not broken at minute six.

## The rule

The public site uses synthetic data. If you later deploy the real thing, it
ships an empty boundary and fills itself on the user's own machine — never a
seat of someone else's corpus.
