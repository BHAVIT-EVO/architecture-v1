#!/usr/bin/env python3
"""Checks the Sarvam key in ten seconds, and prints the real error if it fails.

    export SARVAM_API_KEY=...
    python3 sarvam_check.py            # hi-IN
    python3 sarvam_check.py ta-IN      # another language

Writes sarvam_test.wav on success. Standard library only.
"""
import base64
import json
import os
import sys
import urllib.error
import urllib.request

LANG = sys.argv[1] if len(sys.argv) > 1 else "hi-IN"
TEXT = "Evo derives the boundary. The answer was decided locally."

key = os.environ.get("SARVAM_API_KEY", "").strip()
if not key:
    print("no SARVAM_API_KEY in this shell.")
    print("  export SARVAM_API_KEY=...   then run this again")
    sys.exit(2)

print(f"key      : {key[:6]}…{key[-4:]}  ({len(key)} chars)")
print(f"language : {LANG}")

body = json.dumps({
    "inputs": [TEXT],
    "target_language_code": LANG,
    "speaker": "anushka",
    "model": "bulbul:v2",
}).encode()

req = urllib.request.Request(
    "https://api.sarvam.ai/text-to-speech",
    data=body,
    headers={"api-subscription-key": key, "Content-Type": "application/json"},
)

try:
    with urllib.request.urlopen(req, timeout=30) as r:
        payload = json.loads(r.read())
except urllib.error.HTTPError as e:
    detail = e.read().decode("utf-8", "replace")[:600]
    print(f"\nHTTP {e.code}")
    print(detail)
    print("\nIf this mentions an unknown field, the request shape has changed.")
    print("Paste the error into Kiro and ask it to fix ONLY the payload,")
    print("keeping the function signature and the one-attempt limit.")
    sys.exit(1)
except Exception as e:
    print(f"\n{type(e).__name__}: {e}")
    print("Network or DNS. Not a code problem.")
    sys.exit(1)

audios = payload.get("audios") or []
if not audios:
    print("\n200 but no audio in the response. Body:")
    print(json.dumps(payload)[:600])
    sys.exit(1)

raw = base64.b64decode(audios[0])
with open("sarvam_test.wav", "wb") as f:
    f.write(raw)

print(f"ok       : {len(raw) // 1024} KB of audio")
print("wrote      sarvam_test.wav  — play it with:  afplay sarvam_test.wav")
print("\nthe voice path works. Serve with the key exported and the")
print("हिंदी toggle in the UI will speak every answer.")
