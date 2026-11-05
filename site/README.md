# site/ — the public site

Three pages and seven font files. No server, no build step, no network calls.

```
index.html    the field report — ten chapters, scroll-driven   (~160 KB)
demo.html     the full console — eight works, two claimed        (~108 KB)
film.html     the cross-device concept film, HTML + CSS only     (~374 KB)
fonts/        7 woff2 files, latin subsets, self-hosted          (~156 KB)
src/          the three fragments index.html is assembled from
```

**Every page runs a synthetic corpus.** The real one
(`exports/ship-happens/boundary.json`) contains one person's actual browsing
and must never be deployed.

## Rebuild

```bash
python3 tools/build_site_v2.py
```

The corpus is injected from `tools/build_site.py`, so the story site and the
console can never disagree about the eight invented works. `src/00-head.html`
holds the design system, `src/10-body.html` the chapters, `src/20-js.html` the
scroll engine and scenes.

## Design rules (do not break these)

- Bone paper `#EFEAE1`, ink `#14120F`, one signal `#CE451F`, moss `#6B7565`.
- Instrument Serif for display, IBM Plex Mono for labels and numbers, IBM Plex
  Sans for prose. No fourth family, no Inter.
- Square corners. Hairline rules. No gradients, no glass, no shadows, no cards.
- Reveals on marginalia only. Headlines get a line mask, never a fade-and-slide.
- Every animation driven by one `requestAnimationFrame` loop reading scroll.
  No libraries, no CDN.
- `prefers-reduced-motion` draws every scene in its final state and skips the
  line masks.
- If the site ever needs an external request, the answer is no.

## Deploy

See `DEPLOY.md`. The short version:

```bash
bash tools/deploy_site.sh
```
