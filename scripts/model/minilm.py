#!/usr/bin/env python3
"""Explicit one-time MiniLM provisioning; doctor is strictly offline.

Uses only Python's standard library and curl. No model hub client, credentials,
cookies, telemetry, user text, or runtime network fallback is involved.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile


REPOSITORY = "Qdrant/all-MiniLM-L6-v2-onnx"
REVISION = "d13954661f83248295ba75c1ed411eef3b7b936e"
CACHE_REPO = "models--Qdrant--all-MiniLM-L6-v2-onnx"
# Reviewed pins, not checksums supplied by the download being verified.
# The ONNX digest/size is its upstream Git LFS object identity; JSON digests
# were computed from the same immutable revision, never from mutable main.
ASSETS = {
    "model.onnx": (90387630, "bbd7b466f6d58e646fdc2bd5fd67b2f5e93c0b687011bd4548c420f7bd46f0c5"),
    "tokenizer.json": (711649, "851ca67100d372ca3ae031a6abd168f53489eebfd7d89523f35c5c9b4d372c3c"),
    "config.json": (650, "1b4d8e2a3988377ed8b519a31d8d31025a25f1c5f8606998e8014111438efcd7"),
    "special_tokens_map.json": (695, "5d5b662e421ea9fac075174bb0688ee0d9431699900b90662acd44b2a350503a"),
    "tokenizer_config.json": (1412, "abda01c8c14c5151ae498aceb30db406d6b91242c394fd88d3b6fd5a63a101e6"),
}
DEFAULT_INSTALL = Path.home() / ".local" / "share" / "evo" / "models" / "all-MiniLM-L6-v2-onnx" / REVISION


class ModelError(Exception):
    pass


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as asset:
        for block in iter(lambda: asset.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def verify(directory):
    """Validate every byte against built-in pins; never download or infer."""
    if not directory.is_dir():
        raise ModelError(f"model directory is missing or not a directory: {directory}")
    checked = []
    for name, (size, expected) in ASSETS.items():
        path = directory / name
        if not path.is_file():
            raise ModelError(f"required asset is missing or not a regular file: {path}")
        actual_size = path.stat().st_size
        if actual_size != size:
            raise ModelError(f"{name}: size mismatch (expected {size}, got {actual_size})")
        actual_digest = digest(path)
        if actual_digest != expected:
            raise ModelError(f"{name}: SHA-256 mismatch (expected {expected}, got {actual_digest})")
        if name.endswith(".json"):
            content = json.loads(path.read_text(encoding="utf-8"))
            if not isinstance(content, dict):
                raise ModelError(f"{name}: expected a JSON object")
            if name == "config.json" and (
                content.get("model_type") != "bert" or content.get("hidden_size") != 384
            ):
                raise ModelError("config.json: expected the 384-dimensional BERT export")
        checked.append((name, actual_size, actual_digest))
    return checked


def configured_directory(env, root):
    """Mirror the Rust loader's precedence; a configured failure never falls back."""
    for name in ("EVO_EMBEDDING_MODEL_DIR", "HF_HOME", "FASTEMBED_CACHE_DIR"):
        if name in env:
            if not env[name]:
                raise ModelError(f"{name} is set to an empty path; no fallback")
            path = Path(env[name])
            if not path.is_absolute():
                path = root / path
            if name == "EVO_EMBEDDING_MODEL_DIR":
                return path, name
            cache, source = path, name
            break
    else:
        cache, source = root / ".fastembed_cache", "current-directory .fastembed_cache"
    ref = cache / CACHE_REPO / "refs" / "main"
    revision = ref.read_text(encoding="utf-8").strip()
    if not re.fullmatch(r"[a-fA-F0-9]+", revision):
        raise ModelError(f"invalid cache ref {ref}; expected a hexadecimal revision; no fallback")
    return cache / CACHE_REPO / "snapshots" / revision, source


def download(name, destination):
    curl = shutil.which("curl")
    if curl is None:
        raise ModelError("curl is required for fetch (doctor needs only Python 3)")
    url = f"https://huggingface.co/{REPOSITORY}/resolve/{REVISION}/{name}"
    # --disable must be the first option: do not load ~/.curlrc. The minimal
    # environment excludes proxy credentials, hub tokens, cookie/config paths.
    command = [
        curl, "--disable", "--fail", "--location", "--silent", "--show-error",
        "--proto", "=https", "--proto-redir", "=https", "--tlsv1.2",
        "--connect-timeout", "30", "--max-time", "600",
        "--max-filesize", str(ASSETS[name][0]), "--output", str(destination), "--url", url,
    ]
    result = subprocess.run(
        command, env={"PATH": os.environ.get("PATH", os.defpath)},
        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=False,
    )
    if result.returncode != 0:
        # Do not log redirected signed CDN URLs or any network diagnostics.
        raise ModelError(f"public asset download failed for {name}: curl exit {result.returncode}; not installed")


def fetch(directory):
    """Stage beside the destination, verify, then install by atomic rename."""
    directory = directory.expanduser().absolute()
    if os.path.lexists(directory):
        if directory.is_symlink():
            raise ModelError(f"refusing an install destination symlink: {directory}")
        checked = verify(directory)
        print(f"Already installed and verified: {directory}; no network used")
        return directory, checked
    directory.parent.mkdir(parents=True, exist_ok=True)
    lock = directory.parent / f".{directory.name}.fetch.lock"
    try:
        descriptor = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    except FileExistsError as error:
        raise ModelError(f"fetch lock exists: {lock}; another fetch may be running; no retry") from error
    os.close(descriptor)
    try:
        if os.path.lexists(directory):
            raise ModelError(f"destination appeared while acquiring lock: {directory}; not overwritten")
        with tempfile.TemporaryDirectory(prefix=f".{directory.name}.stage-", dir=directory.parent) as temp:
            staged = Path(temp) / "model"
            staged.mkdir()
            print(f"Explicit public download: {REPOSITORY}@{REVISION} (five files, no personal data)", flush=True)
            for name in ASSETS:
                print(f"Fetching {name}", flush=True)
                download(name, staged / name)
            checked = verify(staged)
            if os.path.lexists(directory):
                raise ModelError(f"destination appeared during fetch: {directory}; not overwritten")
            # Same parent/filesystem: readers see no directory or all verified
            # assets, never a partial installation. The lock serializes fetches.
            os.rename(staged, directory)
        print(f"Installed atomically: {directory}")
        return directory, checked
    finally:
        lock.unlink()


def print_report(directory, checked, source):
    print(f"[PASS] model/assets: {REPOSITORY}@{REVISION}")
    print(f"Directory: {directory} ({source})")
    for name, size, checksum in checked:
        print(f"[PASS] {name}: {size} bytes; sha256={checksum}")
    print("Hashes and JSON structure verified locally; ONNX inference not tested by this command.")


def path_argument(text):
    if not text.strip():
        raise argparse.ArgumentTypeError("model directory must not be empty")
    return Path(text)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    fetch_parser = modes.add_parser("fetch", help="explicit one-time public download; never replaces an existing directory")
    fetch_parser.add_argument("--model-dir", type=path_argument, default=DEFAULT_INSTALL)
    doctor_parser = modes.add_parser("doctor", help="offline pinned-file integrity check; no network, Cargo, history, or desktop actions")
    doctor_parser.add_argument("--model-dir", type=path_argument, help="exact directory; otherwise mirror EVO_EMBEDDING_MODEL_DIR/HF_HOME/FASTEMBED_CACHE_DIR")
    args = parser.parse_args(argv)
    try:
        if args.mode == "fetch":
            directory, checked = fetch(args.model_dir)
            print_report(directory, checked, "explicit install destination")
            # repr is not shell escaping; shlex.quote handles whitespace/quotes.
            import shlex
            print("For Evo, set: export EVO_EMBEDDING_MODEL_DIR=" + shlex.quote(str(directory)))
        else:
            if args.model_dir is None:
                directory, source = configured_directory(os.environ, Path.cwd())
            else:
                directory, source = args.model_dir.expanduser().absolute(), "--model-dir"
            print_report(directory, verify(directory), source)
        return 0
    except (ModelError, OSError, ValueError) as error:
        print(f"[BLOCKED] model/assets: {error}", file=sys.stderr)
        print("No runtime download or remote fallback. See docs/model-local.md.", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
