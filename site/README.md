# site/ — the public demo

One self-contained file. No server, no build step, no network calls.

**This site runs a synthetic corpus.** The real one (`exports/ship-happens/boundary.json`)
contains one person's actual browsing history and must never be deployed.

## Rebuild

```bash
python3 tools/build_site.py
```

## Deploy (Render, static site)

- Build command: *(none)*
- Publish directory: `site`
- Custom domain: `demo.<yourdomain>.xyz` — see DEPLOY.md
