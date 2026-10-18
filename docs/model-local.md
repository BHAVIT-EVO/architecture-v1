# Local embedding assets

Evo defaults to **all-MiniLM-L6-v2**, the non-quantized 384-dimensional,
mean-pooled `Qdrant/all-MiniLM-L6-v2-onnx` export used by fastembed 7.1.0.
For multilingual retrieval, it also supports the explicit
**multilingual-e5-small** profile (`intfloat/multilingual-e5-small`), which is
also 384-dimensional and uses mean pooling. Inference is local for both.
The loader never downloads, constructs a model hub client, reads hub credentials,
or falls back to a remote service. Missing/invalid assets and unsupported
profiles are errors.

## Selecting a profile

Existing deployments remain unchanged: omit `EVO_EMBEDDING_PROFILE` and
all-MiniLM-L6-v2 is selected. To select multilingual E5, set:

```sh
export EVO_EMBEDDING_PROFILE='multilingual-e5-small'
```

The E5 model is asymmetric by design: indexed documents are embedded as
`passage: <text>` and user queries as `query: <text>`. The semantic core exposes
role-aware hooks while keeping the old symmetric fixture-embedder API
backward-compatible. Do not mix vectors from different profiles in one index.
The existing automatic voice-restore thresholds were measured only for MiniLM.
With E5, a similarity match is therefore shown as a candidate requiring an
explicit `CHOOSE`; fuzzy `SCOPE` refuses, while an exact eligible locator still
works. This is an intentional safety boundary until locally held-out E5 scores
are calibrated; selecting a model profile alone does not calibrate it.

## One-time explicit download

Requires Python 3 and curl; no pip packages, API key, paid service, or login.
From the repository root, run:

```sh
python3 scripts/model/minilm.py fetch
```

The public Apache-2.0 model is pinned to immutable revision
`d13954661f83248295ba75c1ed411eef3b7b936e`. The command downloads only five public
files (91,102,036 bytes total, about 87 MiB) over HTTPS. It sends no user text,
history, cookies, credentials, or telemetry; the hosting/CDN services necessarily
see the request and network IP. Curl configuration and proxy/token environment
variables are not used. There are no automatic retries.

The default destination is outside Git:
`~/.local/share/evo/models/all-MiniLM-L6-v2-onnx/<revision>/`. An explicit destination
is also supported:

```sh
python3 scripts/model/minilm.py fetch --model-dir /absolute/path/to/minilm
```

All assets are staged on the destination filesystem, checked for exact size and
SHA-256 against reviewed pins in `scripts/model/minilm.py`, and JSON files parsed
before a single atomic directory rename makes the model visible. A lock prevents
simultaneous fetch commands for the same destination. Failure cleans up that
command's staging files; an existing directory is **never repaired or replaced**.
A valid existing installation is rechecked without networking. A stale lock
after interruption must be investigated manually before removing that exact lock.
Keep the destination's parent directory private to the user; the lock coordinates
this tool, not unrelated programs writing the same path.

Copy the `export EVO_EMBEDDING_MODEL_DIR=...` line printed by fetch into the
environment that launches Evo. Installation does not silently change settings,
shell profiles, daemon configuration, or desktop state. Do not commit the assets.

### Provisioning multilingual E5

`scripts/model/e5.py` implements the explicit public E5 fetch and offline doctor.
All five file sizes and SHA-256 digests are pinned. The ONNX and tokenizer
digests are their upstream Git LFS object IDs. The three JSON files were read
at the immutable revision and verified against upstream Git blob IDs before
recording SHA-256. There is no checksum override or update flag.

The MIT-licensed `intfloat/multilingual-e5-small` export is fixed to revision
`614241f622f53c4eeff9890bdc4f31cfecc418b3`. JSON Git blob IDs are
`60a2a84020f1d74cc53ea9e8c4e91cf4af6c2b68`,
`e0b1d18ecd0ae4ff1d47bd297d910c0cf83e504b`, and
`059214673d9d6d2ee319411e2ffec8c024b816d5`, respectively. The immutable
revision's separately downloaded bytes matched each ID.

Explicitly run (never automatic):

```sh
python3 scripts/model/e5.py fetch
# Optional exact install destination:
python3 scripts/model/e5.py fetch --model-dir /absolute/path/to/e5
```

The command requests only these five files (487,352,505 bytes, about 465 MiB),
preserving the nested ONNX path during staging and installation:

```text
onnx/model.onnx
tokenizer.json
config.json
special_tokens_map.json
tokenizer_config.json
```

The default destination is
`~/.local/share/evo/models/multilingual-e5-small/<revision>/`, outside Git. Fetch
uses Python 3 and curl, HTTPS-only redirects, no hub client, credentials, cookies,
proxy/token environment, telemetry, user text/history, or automatic retries. The
public host/CDN necessarily sees the network IP and asset requests. All bytes and
JSON objects (including a 384-dimensional BERT config) are checked before
an atomic no-replace directory rename; existing invalid installs and symlink
destinations are refused, never repaired. macOS/Linux use exclusive rename
primitives, Windows uses no-replace `os.rename`; unavailable support fails closed.
The same destination lock/staging cleanup policy as MiniLM applies. Keep the
destination's parent private, and investigate stale locks manually.

Fetch prints both exports; it does not modify global settings or shell profiles.
Point `EVO_EMBEDDING_MODEL_DIR` at the install and set
`EVO_EMBEDDING_PROFILE=multilingual-e5-small`. The Rust loader expects the ONNX
file at `onnx/model.onnx`; it does not fall back to MiniLM or search another
profile's cache. A provisioned model is still required for real inference and
multilingual quality validation. An explicitly authorized install at
`~/.local/share/evo/models/multilingual-e5-small/614241f622f53c4eeff9890bdc4f31cfecc418b3/`
passed all five pinned hashes. The real-weight Rust smoke passed with synthetic
English, Arabic, Japanese and Hindi queries. This is not cross-person accuracy
or similarity-threshold calibration.

## Offline verification / doctor

```sh
export EVO_EMBEDDING_MODEL_DIR='/absolute/path/to/minilm'
python3 scripts/model/minilm.py doctor
# Or check a directory explicitly:
python3 scripts/model/minilm.py doctor --model-dir /absolute/path/to/minilm
# E5: requires the separately provisioned exact local assets.
python3 scripts/model/e5.py doctor --model-dir /absolute/path/to/e5
# Or, after setting EVO_EMBEDDING_MODEL_DIR to the E5 directory:
python3 scripts/model/e5.py doctor
```

Doctor prints the directory, pin, every size and SHA-256, and exits 0 only if all
five match. It makes no network calls, reads no personal history, runs no Cargo
subprocess, and performs no desktop/permission actions. Hash verification is not
proof of successful ONNX inference or semantic quality. The general
`scripts/evo-doctor.py` presence check is also not a substitute for this pin check.

Rust callers can use `FastEmbedEmbedder::doctor()` or
`FastEmbedEmbedder::doctor_from_model_dir(path)` to check asset presence/readability
and report the selected directory without initializing ONNX. These explicitly
report that hashes and inference are **not checked**; use the command above for
hash integrity. Loader errors point to this provisioning guide.

### MiniLM asset pins

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| model.onnx | 90,387,630 | `bbd7b466f6d58e646fdc2bd5fd67b2f5e93c0b687011bd4548c420f7bd46f0c5` |
| tokenizer.json | 711,649 | `851ca67100d372ca3ae031a6abd168f53489eebfd7d89523f35c5c9b4d372c3c` |
| config.json | 650 | `1b4d8e2a3988377ed8b519a31d8d31025a25f1c5f8606998e8014111438efcd7` |
| special_tokens_map.json | 695 | `5d5b662e421ea9fac075174bb0688ee0d9431699900b90662acd44b2a350503a` |
| tokenizer_config.json | 1,412 | `abda01c8c14c5151ae498aceb30db406d6b91242c394fd88d3b6fd5a63a101e6` |

The ONNX pin comes from upstream Git LFS metadata; JSON pins were independently
computed from files resolved at that same revision. No mutable `main` reference
or server-provided checksum is trusted during installation. Pin changes require
reviewed source edits and renewed real-model validation, not an update flag.

### E5 asset pins

All sizes below belong to revision `614241f622f53c4eeff9890bdc4f31cfecc418b3`.
The first two SHA-256 values are supplied upstream Git LFS object identities;
the remaining three are deliberately unset in `scripts/model/e5.py`. An unset
pin blocks the **entire** fetch/doctor, including the already-pinned files.

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| onnx/model.onnx | 470,268,510 | `ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665` |
| tokenizer.json | 17,082,730 | `0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39` |
| config.json | 655 | **Missing independently audited digest — blocked** |
| special_tokens_map.json | 167 | **Missing independently audited digest — blocked** |
| tokenizer_config.json | 443 | **Missing independently audited digest — blocked** |

Do not use a checksum returned alongside a model download as an independent pin.
The small JSON hashes must come from the immutable-revision git blobs or audited
fixtures, never mutable `main` or the synthetic tests.

## Existing offline loader paths

Selection remains unchanged: `EVO_EMBEDDING_MODEL_DIR` names an exact flat/snapshot
directory; otherwise `HF_HOME`, then `FASTEMBED_CACHE_DIR`, then current-directory
`.fastembed_cache` names a cache root. `HF_HOME` is used directly, without `hub`.
For the default profile, cache loads read
`models--Qdrant--all-MiniLM-L6-v2-onnx/refs/main`; multilingual E5 reads
`models--intfloat--multilingual-e5-small/refs/main`. Each loader then uses only
that profile's hexadecimal snapshot under `snapshots/`. Local blob symlinks are
supported.
Configured empty, missing, or invalid locations never trigger a lower-priority
fallback. Rust does not expand `~`; use an expanded absolute path in environment
variables. Explicit loader method paths override environment variables.

## Offline tests

```sh
python3 -m unittest discover -s scripts/model -p 'test_*.py' -v
# Focused E5 provisioning tests: synthetic bytes, mocked curl, no network:
python3 -B -m unittest discover -s scripts/model -p 'test_e5.py' -v
cargo test -p evo-voice --offline --locked embed::tests
# Explicit E5 real-weight smoke: synthetic text only, no download:
EVO_TEST_E5_MODEL_DIR='/absolute/path/to/e5' \
  cargo test -p evo-voice --offline --locked real_multilingual_e5_offline_smoke -- --ignored
# Explicit real-weight smoke test: synthetic text only, no download:
EVO_TEST_EMBEDDING_MODEL_DIR="$EVO_EMBEDDING_MODEL_DIR" \
  cargo test -p evo-voice --offline --locked real_model_offline_smoke -- --ignored
```

Default tests need no real weights. The ignored MiniLM smoke test refuses to run
without an explicitly selected local model directory, checks 384 finite normalized
values, repeatability, and rejection of empty input. Multilingual E5 inference and
cross-language quality still require a provisioned, verified E5 model; the default
tests otherwise stop at profile selection, asset refusal, dimensions, and
role-prefix behavior; the ignored real-weight smoke requires explicit local
assets.

The E5 script tests cover nested staging, exact file sizes/digests, JSON/profile
validation, no-retry failures and cleanup, existing-install preservation,
destination symlink refusal, atomic no-clobber races, profile-specific cache
precedence, credential-free HTTPS command construction, and the missing-pin gate.
Their synthetic digests are patched only inside tests and cannot enable real E5
provisioning or prove inference/quality.
