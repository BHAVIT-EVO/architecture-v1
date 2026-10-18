#!/usr/bin/env python3
"""Explicit one-time public multilingual E5 provisioning; doctor is offline.

Uses only Python's standard library and curl. No hub client, credentials,
cookies, telemetry, user text, runtime fallback, or automatic retry is involved.
Every asset is pinned to an immutable revision and independently checked digest.
"""

import argparse
import ctypes
import errno
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import tempfile


REPOSITORY = "intfloat/multilingual-e5-small"
REVISION = "614241f622f53c4eeff9890bdc4f31cfecc418b3"
LICENSE = "MIT"
PROFILE = "multilingual-e5-small"
CACHE_REPO = "models--intfloat--multilingual-e5-small"
# ONNX and tokenizer identities are upstream Git LFS SHA-256 object IDs.
# The JSON bytes at REVISION were checked against upstream Git blob IDs before
# these SHA-256 values were recorded. Never learn pins from this script's fetch.
ASSETS = {
    "onnx/model.onnx": (470268510, "ca456c06b3a9505ddfd9131408916dd79290368331e7d76bb621f1cba6bc8665"),
    "tokenizer.json": (17082730, "0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39"),
    "config.json": (655, "69137736cab8b8903a07fe8afaafdda25aac55415a12a55d1bffa9f581abf959"),
    "special_tokens_map.json": (167, "d05497f1da52c5e09554c0cd874037a083e1dc1b9cfd48034d1c717f1afc07a7"),
    "tokenizer_config.json": (443, "a1d6bc8734a6f635dc158508bef000f8e2e5a759c7d92f984b2c86e5ff53425b"),
}
DEFAULT_INSTALL = Path.home() / ".local" / "share" / "evo" / "models" / PROFILE / REVISION


class ModelError(Exception):
    pass


def require_pins():
    """Fail closed before I/O; no CLI/env override or trust-on-first-use."""
    missing = [name for name, (_, checksum) in ASSETS.items() if checksum is None]
    if missing:
        raise ModelError("missing independently audited SHA-256 pins: " + ", ".join(missing)
                         + "; no download or installation permitted")
    for name, (size, checksum) in ASSETS.items():
        if not isinstance(size, int) or size <= 0 or not isinstance(checksum, str) or not re.fullmatch(r"[a-f0-9]{64}", checksum):
            raise ModelError(f"invalid built-in size/SHA-256 pin for {name}; no download permitted")


def digest(path):
    result = hashlib.sha256()
    with path.open("rb") as asset:
        for block in iter(lambda: asset.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def verify(directory):
    """Validate every byte against built-in pins; never download or infer."""
    require_pins()
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
                raise ModelError("config.json: expected the 384-dimensional BERT E5 export")
        checked.append((name, actual_size, actual_digest))
    return checked


def configured_directory(env, root):
    """E5 cache only; a configured failure never falls back to MiniLM."""
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
    require_pins()
    if name not in ASSETS:
        raise ModelError(f"unrecognized public asset: {name}")
    curl = shutil.which("curl")
    if curl is None:
        raise ModelError("curl is required for fetch (doctor needs only Python 3)")
    url = f"https://huggingface.co/{REPOSITORY}/resolve/{REVISION}/{name}"
    # --disable first forbids ~/.curlrc; the environment excludes proxy/hub
    # credentials and cookie/config paths. HTTPS applies to every redirect.
    command = [
        curl, "--disable", "--fail", "--location", "--silent", "--show-error",
        "--proto", "=https", "--proto-redir", "=https", "--tlsv1.2",
        "--connect-timeout", "30", "--max-time", "1800",
        # Allow small CDN transfer-accounting overhead. verify() still requires
        # the exact pinned payload size and SHA-256 before installation.
        "--max-filesize", str(ASSETS[name][0] + 4096), "--output", str(destination), "--url", url,
    ]
    result = subprocess.run(
        command, env={"PATH": os.environ.get("PATH", os.defpath)},
        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, check=False,
    )
    if result.returncode != 0:
        # Never print network diagnostics, redirected/signed URLs, or headers.
        raise ModelError(f"public asset download failed for {name}: curl exit {result.returncode}; not installed")


def atomic_installer():
    """Select a no-clobber atomic rename before downloading; no TOCTOU overwrite."""
    if sys.platform == "win32":
        # Windows os.rename refuses existing destinations, including directories.
        return os.rename
    library = ctypes.CDLL(None, use_errno=True)
    if sys.platform == "darwin" and hasattr(library, "renamex_np"):
        rename = library.renamex_np
        rename.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_uint]
        arguments = lambda source, target: (os.fsencode(source), os.fsencode(target), 4)  # RENAME_EXCL
    elif sys.platform.startswith("linux") and hasattr(library, "renameat2"):
        rename = library.renameat2
        rename.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int, ctypes.c_char_p, ctypes.c_uint]
        arguments = lambda source, target: (-100, os.fsencode(source), -100, os.fsencode(target), 1)  # AT_FDCWD, RENAME_NOREPLACE
    else:
        raise ModelError("atomic no-replace rename unavailable on this platform; no download permitted")
    rename.restype = ctypes.c_int

    def install(source, target):
        if rename(*arguments(source, target)) != 0:
            error = ctypes.get_errno()
            if error in (errno.EEXIST, errno.ENOTEMPTY):
                raise ModelError(f"destination appeared during installation: {target}; not overwritten")
            raise OSError(error, os.strerror(error), str(target))

    return install


def fetch(directory):
    """Stage on the target filesystem, verify, atomically install without replace."""
    require_pins()
    directory = directory.expanduser().absolute()
    if os.path.lexists(directory):
        if directory.is_symlink():
            raise ModelError(f"refusing an install destination symlink: {directory}")
        checked = verify(directory)
        print(f"Already installed and verified: {directory}; no network used")
        return directory, checked
    install = atomic_installer()
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
            print(f"Explicit public download: {REPOSITORY}@{REVISION} ({LICENSE}, five files, no personal data)", flush=True)
            for name in ASSETS:
                destination = staged / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                print(f"Fetching {name}", flush=True)
                download(name, destination)
            checked = verify(staged)
            # RENAME_EXCL/NOREPLACE closes the final check/rename race, even if
            # an unrelated writer creates an empty directory during installation.
            install(staged, directory)
        print(f"Installed atomically: {directory}")
        return directory, checked
    finally:
        lock.unlink()


def print_report(directory, checked, source):
    print(f"[PASS] model/assets: {REPOSITORY}@{REVISION} ({LICENSE})")
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
    fetch_parser = modes.add_parser("fetch", help="explicit one-time public download; refuses unreviewed pins and existing invalid directories")
    fetch_parser.add_argument("--model-dir", type=path_argument, default=DEFAULT_INSTALL)
    doctor_parser = modes.add_parser("doctor", help="offline pinned-file integrity check; no network, history, or desktop actions")
    doctor_parser.add_argument("--model-dir", type=path_argument, help="exact directory; otherwise mirror EVO_EMBEDDING_MODEL_DIR/HF_HOME/FASTEMBED_CACHE_DIR for E5")
    args = parser.parse_args(argv)
    try:
        require_pins()
        if args.mode == "fetch":
            directory, checked = fetch(args.model_dir)
            print_report(directory, checked, "explicit install destination")
            print("For Evo, set: export EVO_EMBEDDING_PROFILE=" + shlex.quote(PROFILE))
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
