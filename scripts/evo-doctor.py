#!/usr/bin/env python3
"""Local release preflight. Default is report-only; no downloads or app actions.

Run `python3 scripts/evo-doctor.py --help` for the explicit offline build option.
Cargo's offline flags do not sandbox arbitrary dependency build scripts; enforce
network denial separately for an offline release receipt.
This is separate from the daemon example doctor, which reads canonical history.
"""

import argparse
import ctypes
import hashlib
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys


MINILM_FILES = (
    "model.onnx",
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
)
E5_FILES = (
    "onnx/model.onnx",
    "tokenizer.json",
    "config.json",
    "special_tokens_map.json",
    "tokenizer_config.json",
)
MODEL_PROFILES = {
    "all-minilm-l6-v2": {
        "files": MINILM_FILES,
        "cache_repo": "models--Qdrant--all-MiniLM-L6-v2-onnx",
    },
    "multilingual-e5-small": {
        "files": E5_FILES,
        "cache_repo": "models--intfloat--multilingual-e5-small",
    },
}
BUILD_COMMAND = ["cargo", "build", "--workspace", "--offline", "--locked", "-j1"]
ORT_PATH_VARS = ("ORT_LIB_PATH", "ORT_LIB_LOCATION")
# Deliberately one reviewed packaging shape, not a runtime discovery/download API.
ORT_BUNDLE_ID = "@rpath/libonnxruntime.1.dylib"
ORT_BUNDLE_VERSION = "1.29.0"
ORT_BUNDLE_MIN_OS = (14, 0)
ORT_BUNDLE_RPATH = "@executable_path/../Frameworks"
ORT_BUNDLE_INPUTS = ("EVO_ORT_DYLIB", "EVO_ORT_LICENSE", "EVO_ORT_THIRD_PARTY_NOTICES")
ORT_SYSTEM_DEPENDENCIES = (
    "/System/Library/Frameworks/CoreML.framework/Versions/A/CoreML",
    "/System/Library/Frameworks/CoreFoundation.framework/Versions/A/CoreFoundation",
    "/System/Library/Frameworks/CFNetwork.framework/Versions/A/CFNetwork",
    "/System/Library/Frameworks/Network.framework/Versions/A/Network",
    "/System/Library/Frameworks/SystemConfiguration.framework/Versions/A/SystemConfiguration",
    "/System/Library/Frameworks/IOKit.framework/Versions/A/IOKit",
    "/System/Library/Frameworks/Foundation.framework/Versions/C/Foundation",
    "/usr/lib/libiconv.2.dylib",
    "/usr/lib/libc++.1.dylib",
    "/usr/lib/libSystem.B.dylib",
    "/usr/lib/libobjc.A.dylib",
)


def _env_path(root, value):
    path = Path(value)
    return path if path.is_absolute() else root / path


def file_digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def nonempty_regular_file(path):
    if path.is_symlink() or not path.is_file() or not path.stat().st_size or not os.access(path, os.R_OK):
        raise ValueError(f"expected a nonempty readable regular file, not a symlink: {path}")


def bundled_ort_inputs(root, env):
    if env.get("EVO_ORT_PROVISIONING") != "bundled":
        raise ValueError("explicit EVO_ORT_PROVISIONING=bundled is required; no fallback")
    paths = []
    for name in ORT_BUNDLE_INPUTS:
        value = env.get(name, "")
        if not value or "\n" in value or "\r" in value:
            raise ValueError(f"{name} must name an exact local file (no automatic search or download)")
        path = _env_path(root, value)
        nonempty_regular_file(path)
        paths.append(path)
    if len({path.resolve() for path in paths}) != 3:
        raise ValueError("runtime, license and third-party notices must be three distinct supplied files")
    return paths


def inspection_command(*command):
    """Read metadata only; never dlopen a runtime or launch an app."""
    tool = shutil.which(command[0])
    if not tool:
        raise ValueError(f"{command[0]} is not on PATH; metadata validation cannot run")
    result = subprocess.run(
        [tool, *map(str, command[1:])], capture_output=True, text=True,
        check=False, timeout=15,
    )
    if result.returncode:
        raise ValueError(f"{command[0]} metadata inspection failed for {command[-1]} (exit {result.returncode})")
    return result.stdout


def macho_metadata(path, filetype):
    nonempty_regular_file(path)
    if inspection_command("lipo", "-archs", path).strip().split() != ["arm64"]:
        raise ValueError(f"{path}: bundled mode requires thin arm64 Mach-O; no universal/x86 fallback")
    header = inspection_command("otool", "-hv", path)
    if not re.search(rf"^MH_MAGIC_64\s+ARM64\s+.*\b{filetype}\b", header, re.MULTILINE):
        raise ValueError(f"{path}: expected arm64 Mach-O {filetype}")
    linked = inspection_command("otool", "-L", path).splitlines()[1:]
    dependencies = []
    for line in linked:
        match = re.fullmatch(r"\s+(.+) \(compatibility version ([0-9.]+), current version ([0-9.]+)\)", line)
        if not match:
            raise ValueError(f"{path}: unrecognized dylib dependency metadata")
        dependencies.append(match.groups())
    loads = inspection_command("otool", "-l", path)
    build = re.search(r"cmd LC_BUILD_VERSION\s+cmdsize \d+\s+platform 1\s+minos ([0-9.]+)", loads)
    if not build:
        raise ValueError(f"{path}: macOS LC_BUILD_VERSION is required")
    min_os = tuple(map(int, build[1].split(".")))
    min_os = (min_os + (0, 0))[:2]
    rpaths = re.findall(r"cmd LC_RPATH\s+cmdsize \d+\s+path (.+) \(offset \d+\)", loads)
    return dependencies, rpaths, min_os


def validate_bundled_runtime(path):
    dependencies, rpaths, min_os = macho_metadata(path, "DYLIB")
    identity = inspection_command("otool", "-D", path).splitlines()[1:]
    if identity != [ORT_BUNDLE_ID] or not dependencies or dependencies[0] != (ORT_BUNDLE_ID, "1.0.0", ORT_BUNDLE_VERSION):
        raise ValueError(f"{path}: expected dylib ID {ORT_BUNDLE_ID}, compatibility 1.0.0, current {ORT_BUNDLE_VERSION}")
    # Counter-equivalent sorted lists catch missing, duplicate and extra deps.
    if sorted(item[0] for item in dependencies[1:]) != sorted(ORT_SYSTEM_DEPENDENCIES):
        raise ValueError(f"{path}: dependency closure differs from the eleven reviewed Apple/system libraries")
    if min_os != ORT_BUNDLE_MIN_OS or rpaths != ["@loader_path"]:
        raise ValueError(f"{path}: expected macOS 14.0 minimum and only @loader_path runtime rpath")
    exports = inspection_command("nm", "-gU", path)
    if not re.search(r"^\S+\s+T\s+_OrtGetApiBase$", exports, re.MULTILINE):
        raise ValueError(f"{path}: OrtGetApiBase C ABI export missing")


def bundled_ort_input_status(root, env, platform=sys.platform):
    try:
        runtime, license_file, notices = bundled_ort_inputs(root, env)
        if platform != "darwin":
            raise ValueError("bundled packaging requires macOS metadata tools")
        # This contract must be re-reviewed on a binding upgrade. rc.13 exposes
        # APIs through 28; runtime 1.29 is not an older/impossible version.
        lock = (root / "Cargo.lock").read_text(encoding="utf-8")
        for name in ("ort", "ort-sys"):
            versions = re.findall(rf'^name = "{name}"\nversion = "([^"]+)"$', lock, re.MULTILINE)
            if versions != ["2.0.0-rc.13"]:
                raise ValueError(f"bundled contract requires {name} 2.0.0-rc.13 in Cargo.lock; re-review ABI before upgrading")
        validate_bundled_runtime(runtime)
        digests = "; ".join(f"{name} sha256={file_digest(path)}" for name, path in zip(ORT_BUNDLE_INPUTS, (runtime, license_file, notices)))
        return "PASS", f"pinned arm64 1.29.0 metadata/C ABI export and system-only closure pass; {digests}; supplied notices only, not license approval or loading/API negotiation proof"
    except (OSError, UnicodeError, ValueError, subprocess.TimeoutExpired) as error:
        return "BLOCKED", f"bundled ONNX Runtime inputs: {error}; no copy, signing or download attempted"


def bundled_ort_bundle_status(app, platform=sys.platform):
    """Static closure/notice inspection; independent of the source cache."""
    try:
        if platform != "darwin":
            raise ValueError("bundled packaging requires macOS metadata tools")
        if app.is_symlink() or not app.is_dir():
            raise ValueError(f"expected an explicit regular .app directory: {app}")
        contents = app / "Contents"
        resources = contents / "Resources"
        frameworks = app / "Contents/Frameworks"
        macos = app / "Contents/MacOS"
        notice_dir = app / "Contents/Resources/onnxruntime"
        for directory in (contents, resources, frameworks, macos, notice_dir):
            if directory.is_symlink() or not directory.is_dir():
                raise ValueError(f"missing or symlinked bundle directory: {directory}")
        if {path.name for path in contents.iterdir()} - {"Frameworks", "MacOS", "Resources", "Info.plist", "_CodeSignature"}:
            raise ValueError("Contents contains unreviewed nested content/code")
        # Voice wake assets are ordinary model data, not executable code, but
        # they must live in the sealed bundle so the always-on agent can start
        # without depending on the checkout.
        allowed_resources = {
            "AppIcon.icns", "onnxruntime", "whisper-base.en.bin",
            "melspectrogram.onnx", "embedding_model.onnx", "hey_evoh.onnx",
            "model.onnx", "tokenizer.json", "config.json",
            "special_tokens_map.json", "tokenizer_config.json",
        }
        if {path.name for path in resources.iterdir()} - allowed_resources:
            raise ValueError("Resources contains unreviewed nested content/code")
        signature_dir = contents / "_CodeSignature"
        if signature_dir.is_symlink() or (signature_dir.exists() and not signature_dir.is_dir()):
            raise ValueError("bundle signature metadata directory must not be a symlink or file")
        icon = resources / "AppIcon.icns"
        if icon.exists() or icon.is_symlink():
            nonempty_regular_file(icon)
        if sorted(path.name for path in frameworks.iterdir()) != ["libonnxruntime.1.dylib"]:
            raise ValueError("Contents/Frameworks must contain exactly libonnxruntime.1.dylib; no unreviewed nested code")
        actual_macos = sorted(path.name for path in macos.iterdir())
        expected_macos = {
            # The default bundle is passive: no microphone-capable agent is
            # shipped, so the app cannot accidentally start the notch/voice
            # runtime when the user only asked for focused-window capture.
            ("Evo", "evo-daemon", "evo-ledger-engine"),
            # Voice is an explicit opt-in. The installer travels with the
            # matching agent so a bundle never points at a stale copy in
            # Application Support.
            ("Evo", "evo-agent", "evo-agent-install", "evo-daemon", "evo-ledger-engine"),
        }
        if tuple(actual_macos) not in expected_macos:
            raise ValueError("Contents/MacOS contains an unexpected executable")
        for name in ("LICENSE", "ThirdPartyNotices.txt"):
            nonempty_regular_file(notice_dir / name)
        if sorted(path.name for path in notice_dir.iterdir()) != ["LICENSE", "ThirdPartyNotices.txt"]:
            raise ValueError("runtime notice directory contains unexpected files")
        plist = app / "Contents/Info.plist"
        nonempty_regular_file(plist)
        info = plistlib.loads(plist.read_bytes())
        if not isinstance(info, dict) or info.get("LSMinimumSystemVersion") != "14.0":
            raise ValueError("bundled ONNX Runtime requires LSMinimumSystemVersion=14.0 (not the external mode's 13.0)")
        if info.get("CFBundleExecutable") != "Evo" or info.get("CFBundleIdentifier") != "dev.evo.desktop":
            raise ValueError("expected Evo main executable and dev.evo.desktop bundle identifier")
        validate_bundled_runtime(frameworks / "libonnxruntime.1.dylib")
        binaries = ["Evo", "evo-daemon", "evo-ledger-engine"]
        if "evo-agent" in actual_macos:
            binaries.append("evo-agent")
        for name in binaries:
            dependencies, rpaths, min_os = macho_metadata(macos / name, "EXECUTE")
            if min_os > ORT_BUNDLE_MIN_OS or rpaths != [ORT_BUNDLE_RPATH]:
                raise ValueError(f"{name}: impossible deployment ABI or non-bundle-only rpath; expected {ORT_BUNDLE_RPATH}")
            for dependency, compatibility, version in dependencies:
                if dependency == ORT_BUNDLE_ID:
                    if compatibility != "1.0.0" or version != ORT_BUNDLE_VERSION:
                        raise ValueError(f"{name}: ONNX Runtime link version differs from the bundled contract")
                elif not re.fullmatch(r"/(?:usr/lib|System/Library/(?:Frameworks|PrivateFrameworks))/[^\s]+", dependency) or "/../" in dependency:
                    raise ValueError(f"{name}: unresolved non-system dependency: {dependency}")
            if name != "evo-ledger-engine" and sum(item[0] == ORT_BUNDLE_ID for item in dependencies) != 1:
                raise ValueError(f"{name}: expected exactly one direct ONNX Runtime dylib link; dynamic discovery is not this contract")
        return "PASS", "bundled runtime/notices, arm64 executables, macOS 14.0 and exact bundle-only dependency closure pass; no app/dylib execution or license approval"
    except (OSError, UnicodeError, ValueError, plistlib.InvalidFileException, subprocess.TimeoutExpired) as error:
        return "BLOCKED", f"bundled ONNX Runtime bundle: {error}; no signing, repair or launch attempted"


def workspace_status(root):
    """Check presence only; Cargo --locked is the freshness check during build."""
    if not (root / "Cargo.toml").is_file():
        return "BLOCKED", "workspace Cargo.toml is missing"
    try:
        manifest = (root / "Cargo.toml").read_text(encoding="utf-8")
        lock = (root / "Cargo.lock").read_text(encoding="utf-8")
        digest = file_digest(root / "Cargo.lock")
    except (OSError, UnicodeError) as error:
        return "BLOCKED", f"cannot read workspace manifest/lock: {error}"
    if not re.search(r"^\[workspace\]\s*$", manifest, re.MULTILINE):
        return "BLOCKED", "Cargo.toml does not declare [workspace]"
    if "[[package]]" not in lock or not re.search(
        r"^version\s*=\s*[1-9][0-9]*\s*$", lock, re.MULTILINE
    ):
        return "BLOCKED", "Cargo.lock is empty or lacks lockfile/package markers"
    return "PASS", f"workspace Cargo.lock present; sha256={digest}; freshness requires --build"


def toolchain_status():
    """Locate tools only; report mode never runs Cargo, rustc, or rustup."""
    missing = [name for name in ("cargo", "rustc") if shutil.which(name) is None]
    if missing:
        return "BLOCKED", f"not found on PATH: {', '.join(missing)}; no install attempted"
    return "PASS", "cargo and rustc found on PATH; versions/toolchain readiness are not verified"


def model_profile(env):
    raw = env.get("EVO_EMBEDDING_PROFILE", "all-MiniLM-L6-v2")
    normalized = raw.strip().lower().replace("_", "-").replace(" ", "-")
    aliases = {
        "default": "all-minilm-l6-v2",
        "minilm": "all-minilm-l6-v2",
        "allminlml6v2": "all-minilm-l6-v2",
        "e5-small": "multilingual-e5-small",
        "multilinguale5small": "multilingual-e5-small",
    }
    normalized = aliases.get(normalized, normalized)
    if normalized not in MODEL_PROFILES:
        raise ValueError(
            f"unsupported EVO_EMBEDDING_PROFILE={raw!r}; supported profiles: "
            f"{', '.join(MODEL_PROFILES)}"
        )
    return normalized, MODEL_PROFILES[normalized]


def model_directory(root, env):
    """Mirror FastEmbedEmbedder::load; never search history or download assets."""
    _, profile = model_profile(env)
    for name in ("EVO_EMBEDDING_MODEL_DIR", "HF_HOME", "FASTEMBED_CACHE_DIR"):
        if name in env:
            value = env[name]
            if not value:
                raise ValueError(f"{name} is set to an empty path; no fallback")
            path = Path(value)
            if not path.is_absolute():
                path = root / path
            if name == "EVO_EMBEDDING_MODEL_DIR":
                return path, name
            cache = path
            source = name
            break
    else:
        cache = root / ".fastembed_cache"
        source = "workspace .fastembed_cache"
    repo = cache / profile["cache_repo"]
    ref = repo / "refs" / "main"
    if ref.is_symlink():
        raise ValueError(f"cache ref {ref} is a symlink; refusing to read outside the selected cache")
    revision = ref.read_text(encoding="utf-8").strip()
    if not re.fullmatch(r"[a-fA-F0-9]+", revision):
        raise ValueError(f"invalid cache ref {ref}: expected a hexadecimal revision; no fallback")
    return repo / "snapshots" / revision, source


def model_status(root, env):
    try:
        profile_name, profile = model_profile(env)
        directory, source = model_directory(root, env)
        if not directory.is_dir():
            return "BLOCKED", f"model directory missing: {directory} ({source}); no download attempted"
        invalid = []
        for name in profile["files"]:
            asset = directory / name
            if not asset.is_file() or asset.stat().st_size == 0 or not os.access(asset, os.R_OK):
                invalid.append(name)
        if invalid:
            return "BLOCKED", f"{directory}: missing/empty/unreadable assets: {', '.join(invalid)}"
        return "PASS", f"five nonempty local assets for {profile_name} in {directory} ({source}); presence is not inference"
    except (OSError, UnicodeError, ValueError) as error:
        return "BLOCKED", f"model assets unavailable: {error}; no download attempted"


def onnx_runtime_status(root, env, platform=sys.platform):
    """Report explicit ONNX Runtime provisioning without probing or downloading.

    fastembed's default feature can make ort-sys download a prebuilt runtime at
    build time. That is useful for development, but it is not evidence that a
    shipped app has a reviewed, portable runtime. Keep that distinction visible
    and require an explicit provisioning mode for the packaging script.
    """
    mode = env.get("EVO_ORT_PROVISIONING", "")
    if not mode:
        return (
            "NOT_RUN",
            "ONNX Runtime provisioning mode is unset; no portable runtime claim is made; "
            "set EVO_ORT_PROVISIONING=external with ORT_LIB_PATH for explicit development linking",
        )
    if mode == "bundled":
        try:
            bundled_ort_inputs(root, env)
            return "DIAGNOSTIC", "three exact local bundled inputs present; use --check-ort-inputs for pinned Mach-O/closure checks; no license approval, copy or loading proof"
        except (OSError, ValueError) as error:
            return "BLOCKED", f"bundled ONNX Runtime inputs: {error}; no search, copy or download attempted"
    if mode != "external":
        return "BLOCKED", f"unsupported EVO_ORT_PROVISIONING={mode!r}; no fallback"

    runtime_dir = None
    for name in ORT_PATH_VARS:
        if name in env:
            if not env[name]:
                return "BLOCKED", f"{name} is set to an empty path"
            runtime_dir = _env_path(root, env[name])
            break

    if runtime_dir is None:
        return "BLOCKED", "external mode requires ORT_LIB_PATH (or ORT_LIB_LOCATION)"
    if not runtime_dir.is_dir():
        return "BLOCKED", f"ONNX Runtime link directory is missing: {runtime_dir}"

    dynamic = env.get("ORT_PREFER_DYNAMIC_LINK", "").lower() in {"1", "true"}
    suffix = (".dylib" if platform == "darwin" else ".so") if dynamic else ".a"
    try:
        candidates = sorted(
            path
            for path in runtime_dir.iterdir()
            if path.is_file()
            and path.name.lower().startswith(("libonnxruntime", "onnxruntime"))
            and path.suffix.lower() == suffix
            and path.stat().st_size > 0
        )
    except OSError as error:
        return "BLOCKED", f"cannot inspect ONNX Runtime directory {runtime_dir}: {error}"
    if not candidates:
        return "BLOCKED", f"no nonempty ONNX Runtime {suffix} link candidate in {runtime_dir}; set ORT_PREFER_DYNAMIC_LINK=1 for a dylib"
    return "DIAGNOSTIC", f"explicit link candidate: {candidates[0]}; presence only, license/ABI/loading not verified; external dynamic app is not portable"


def bundle_signature_status(root, platform=sys.platform):
    """Verify an existing bundle without modifying or signing it."""
    app = Path(os.environ.get("EVO_APP_PATH", root / "dist" / "Evo.app"))
    if not app.is_dir():
        return "NOT_RUN", f"no bundle at {app}; packaging verification was not run"
    if platform != "darwin":
        return "UNAVAILABLE", "macOS codesign verification is unavailable on this platform"
    codesign = shutil.which("codesign")
    if codesign is None:
        return "BLOCKED", "codesign is not on PATH; bundle signature could not be verified"
    try:
        result = subprocess.run(
            [codesign, "--verify", "--deep", "--strict", str(app)],
            capture_output=True,
            text=True,
            check=False,
            timeout=15,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return "BLOCKED", f"codesign verification did not finish: {error}"
    if result.returncode == 0:
        return "PASS", "existing Evo.app passes codesign --verify --deep --strict"
    detail = (result.stderr or result.stdout or "").strip().splitlines()
    reason = detail[-1] if detail else f"codesign exited {result.returncode}"
    return "BLOCKED", f"existing Evo.app failed strict signature verification: {reason}"


def permission_status(platform=sys.platform):
    """Only no-prompt APIs; no AX window reads, event tap, TCC DB or requests."""
    if platform != "darwin":
        return [("permissions", "UNAVAILABLE", "macOS diagnostic APIs are unavailable on this platform")]
    results = []
    probes = (
        ("Accessibility", "/System/Library/Frameworks/ApplicationServices.framework/ApplicationServices", "AXIsProcessTrusted"),
        ("Input Monitoring", "/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics", "CGPreflightListenEventAccess"),
    )
    for name, framework, symbol in probes:
        try:
            probe = getattr(ctypes.CDLL(framework), symbol)
            probe.argtypes = []
            probe.restype = ctypes.c_bool
            state = "allowed" if probe() else "not allowed"
            results.append((name, "DIAGNOSTIC", f"{state} for this Python/responsible launch chain; not Evo's grant"))
        except (OSError, AttributeError) as error:
            results.append((name, "UNAVAILABLE", f"nonprompting API unavailable: {error}"))
    return results


def build_status(root):
    if shutil.which("cargo") is None:
        return "BLOCKED", "cargo is not installed; offline build was not run"
    try:
        before = file_digest(root / "Cargo.lock")
        print("Running explicit build: " + " ".join(BUILD_COMMAND), flush=True)
        env = dict(os.environ, CARGO_NET_OFFLINE="true")
        result = subprocess.run(BUILD_COMMAND, cwd=root, env=env, check=False)
        if file_digest(root / "Cargo.lock") != before:
            return "BLOCKED", "Cargo.lock changed during the build; inspect it (doctor does not rewrite or restore it)"
        if result.returncode != 0:
            return "BLOCKED", f"offline locked workspace build exited {result.returncode}; Cargo.lock unchanged"
        return "PASS", "offline locked workspace build succeeded; Cargo.lock unchanged"
    except (OSError, KeyboardInterrupt) as error:
        return "BLOCKED", f"offline build did not finish: {error}; no automatic retry"


def report(root, env, build=False, permissions=True, verify_bundle=False):
    print("== Evo local doctor ==", flush=True)
    print(f"Workspace: {root}")
    print("Mode: " + ("explicit offline build" if build else "report-only (no Cargo subprocess)"))
    checks = [
        ("workspace/lock", *workspace_status(root)),
        ("toolchain/PATH", *toolchain_status()),
        ("model/assets", *model_status(root, env)),
    ]
    if build and checks[0][1] == "PASS":
        checks.append(("workspace/build", *build_status(root)))
    else:
        reason = "workspace/lock blocked" if build else "use --build to verify Cargo.lock freshness and compile offline"
        checks.append(("workspace/build", "NOT_RUN", reason))
    if verify_bundle:
        checks.append(("bundle/signature", *bundle_signature_status(root)))
        app = _env_path(root, env.get("EVO_APP_PATH", "dist/Evo.app"))
        if env.get("EVO_ORT_PROVISIONING") == "bundled" or (app / "Contents/Frameworks/libonnxruntime.1.dylib").exists():
            checks.append(("bundle/ORT-closure", *bundled_ort_bundle_status(app)))
    for name, state, detail in checks:
        print(f"[{state}] {name}: {detail}")
    if permissions:
        for name, state, detail in permission_status():
            print(f"[{state}] {name}: {detail}")
    else:
        print("[NOT_RUN] permissions: --skip-permissions")
    runtime_state, runtime_detail = onnx_runtime_status(root, env)
    print(f"[{runtime_state}] PORTABLE runtime/linking: {runtime_detail}")
    if env.get("EVO_ORT_PROVISIONING") == "bundled":
        checks.append(("runtime/bundled-inputs", runtime_state, runtime_detail))
    if not verify_bundle:
        print("[NOT_RUN] PORTABLE bundle/signature: --verify-bundle opts in to read-only codesign verification; no signing or metadata repair")
    print("[NOT_RUN] PRIVACY gate: deletion/exclusion/retention/revocation must invalidate all derived views; storage tests alone are not an end-to-end receipt")
    print("[NOT_RUN] SEMANTIC gate: run real-weight synthetic inference offline; assets/build alone do not pass it")
    print("[NOT_RUN] LIVE gate: separate opt-in fixture and capture→daemon→UI receipts; isolated native pass is not end-to-end")
    print("[BLOCKED] BROWSER gate: local adapter exists, but no real capture→restore→verified-tab receipt; unsupported constraints must refuse")
    print("[BLOCKED] COLD AX gate: fixture reproduced system-wide -25204 and app-scoped success; production fallback still needs a live receipt")
    failed = any(state == "BLOCKED" for _, state, _ in checks)
    print("Result: " + ("local prerequisites blocked" if failed else "local checks passed") + "; release gates remain separate.")
    print("Exit 0 = requested local checks passed, 1 = local prerequisite/build blocked; neither means release-ready.")
    return int(failed)


def syntax_check():
    """Compile this script's syntax without executing it or writing bytecode."""
    path = Path(__file__)
    try:
        compile(path.read_text(encoding="utf-8"), str(path), "exec")
    except (OSError, UnicodeError, SyntaxError) as error:
        print(f"[BLOCKED] script/syntax: {error}")
        return 1
    print("[PASS] script/syntax: compiled without execution or bytecode writes")
    return 0


def self_test():
    """Synthetic local files only. No Cargo, frameworks, downloads or history."""
    import contextlib
    import io
    import tempfile
    import unittest
    from unittest.mock import patch

    class DoctorTests(unittest.TestCase):
        def setUp(self):
            self.temp = tempfile.TemporaryDirectory(prefix="evo-doctor-fixture-")
            self.addCleanup(self.temp.cleanup)
            self.root = Path(self.temp.name)
            (self.root / "Cargo.toml").write_text("[workspace]\nmembers = []\n", encoding="utf-8")
            (self.root / "Cargo.lock").write_text('version = 4\n[[package]]\nname = "fixture"\nversion = "0.1.0"\n', encoding="utf-8")
            self.model = self.root / "model"
            self.model.mkdir()
            for name in MINILM_FILES:
                (self.model / name).write_bytes(b"synthetic, not model weights")
            self.env = {"EVO_EMBEDDING_MODEL_DIR": "model"}

        def test_valid_presence_is_not_inference(self):
            self.assertEqual(workspace_status(self.root)[0], "PASS")
            state, detail = model_status(self.root, self.env)
            self.assertEqual(state, "PASS")
            self.assertIn("not inference", detail)

        def test_script_syntax_without_bytecode(self):
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(syntax_check(), 0)

        def test_toolchain_presence_does_not_run_it(self):
            with patch.object(subprocess, "run", side_effect=AssertionError("unexpected subprocess")):
                with patch.object(shutil, "which", return_value="fixture-tool"):
                    state, detail = toolchain_status()
                    self.assertEqual(state, "PASS")
                    self.assertIn("not verified", detail)
                with patch.object(shutil, "which", return_value=None):
                    self.assertEqual(toolchain_status()[0], "BLOCKED")

        def test_missing_workspace_lock_blocks(self):
            (self.root / "Cargo.lock").unlink()
            self.assertEqual(workspace_status(self.root)[0], "BLOCKED")

        def test_each_required_asset_and_directories_block(self):
            for name in MINILM_FILES:
                with self.subTest(name=name):
                    asset = self.model / name
                    asset.write_bytes(b"")
                    self.assertEqual(model_status(self.root, self.env)[0], "BLOCKED")
                    asset.unlink()
                    asset.mkdir()
                    self.assertEqual(model_status(self.root, self.env)[0], "BLOCKED")
                    asset.rmdir()
                    asset.write_bytes(b"synthetic")

        def test_explicit_empty_or_missing_directory_does_not_fall_back(self):
            for directory in ("", "absent"):
                self.assertEqual(model_status(self.root, {"EVO_EMBEDDING_MODEL_DIR": directory, "HF_HOME": str(self.root)})[0], "BLOCKED")

        def test_cache_ref_and_precedence(self):
            repo = self.root / MODEL_PROFILES["all-minilm-l6-v2"]["cache_repo"]
            (repo / "refs").mkdir(parents=True)
            ref = repo / "refs" / "main"
            ref.write_text("abc123\n", encoding="utf-8")
            directory, source = model_directory(self.root, {"HF_HOME": str(self.root), "FASTEMBED_CACHE_DIR": "absent"})
            self.assertEqual(directory, repo / "snapshots" / "abc123")
            self.assertEqual(source, "HF_HOME")
            for revision in ("", "../model", "/model", "abc/def"):
                ref.write_text(revision, encoding="utf-8")
                self.assertEqual(model_status(self.root, {"HF_HOME": str(self.root)})[0], "BLOCKED")

            ref.unlink()
            ref.symlink_to(self.model / "model.onnx")
            state, detail = model_status(self.root, {"HF_HOME": str(self.root)})
            self.assertEqual(state, "BLOCKED")
            self.assertIn("symlink", detail)

        def test_multilingual_profile_uses_its_own_nested_assets_and_cache(self):
            e5 = self.root / "e5"
            for name in E5_FILES:
                path = e5 / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"synthetic, not model weights")
            env = {
                "EVO_EMBEDDING_PROFILE": "multilingual-e5-small",
                "EVO_EMBEDDING_MODEL_DIR": "e5",
            }
            state, detail = model_status(self.root, env)
            self.assertEqual(state, "PASS")
            self.assertIn("multilingual-e5-small", detail)
            self.assertEqual(model_profile(env)[0], "multilingual-e5-small")

            repo = self.root / MODEL_PROFILES["multilingual-e5-small"]["cache_repo"]
            (repo / "refs").mkdir(parents=True)
            (repo / "refs/main").write_text("abc123\n", encoding="utf-8")
            directory, source = model_directory(
                self.root,
                {"EVO_EMBEDDING_PROFILE": "e5-small", "HF_HOME": str(self.root)},
            )
            self.assertEqual(directory, repo / "snapshots/abc123")
            self.assertEqual(source, "HF_HOME")

        def test_unknown_embedding_profile_blocks_without_fallback(self):
            state, detail = model_status(
                self.root,
                {"EVO_EMBEDDING_PROFILE": "unknown", "EVO_EMBEDDING_MODEL_DIR": "model"},
            )
            self.assertEqual(state, "BLOCKED")
            self.assertIn("unsupported EVO_EMBEDDING_PROFILE", detail)

        def test_report_only_has_no_subprocess_or_permission_request(self):
            with patch.object(shutil, "which", return_value="fixture-tool"), patch.object(subprocess, "run", side_effect=AssertionError("unexpected subprocess")), patch.object(ctypes, "CDLL", side_effect=AssertionError("unexpected framework")), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(report(self.root, self.env, permissions=False), 0)

        def test_permission_probes_are_no_prompt_and_not_app_grants(self):
            class Probe:
                def __call__(self):
                    return True

            class Framework:
                AXIsProcessTrusted = Probe()
                CGPreflightListenEventAccess = Probe()

            with patch.object(ctypes, "CDLL", return_value=Framework()) as load:
                results = permission_status("darwin")
                self.assertEqual(load.call_count, 2)
                self.assertEqual([name for name, _, _ in results], ["Accessibility", "Input Monitoring"])
                self.assertTrue(all(state == "DIAGNOSTIC" and "not Evo's grant" in detail for _, state, detail in results))
            with patch.object(ctypes, "CDLL", side_effect=OSError("fixture: unavailable")):
                self.assertTrue(all(state == "UNAVAILABLE" for _, state, _ in permission_status("darwin")))
            with patch.object(ctypes, "CDLL", side_effect=AssertionError("unexpected framework")):
                self.assertEqual(permission_status("linux")[0][1], "UNAVAILABLE")

        def test_report_gates_stay_separate_and_missing_assets_block(self):
            output = io.StringIO()
            with patch.object(shutil, "which", return_value="fixture-tool"), contextlib.redirect_stdout(output):
                self.assertEqual(report(self.root, self.env, permissions=False), 0)
            for gate in ("SEMANTIC", "LIVE", "BROWSER", "COLD AX"):
                self.assertIn(f"{gate} gate", output.getvalue())
            self.assertIn("[NOT_RUN] workspace/build", output.getvalue())
            (self.model / "model.onnx").unlink()
            with patch.object(shutil, "which", return_value="fixture-tool"), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(report(self.root, self.env, permissions=False), 1)

        def test_runtime_requires_explicit_provisioning_and_checks_local_candidate(self):
            self.assertEqual(onnx_runtime_status(self.root, self.env)[0], "NOT_RUN")
            self.assertEqual(
                onnx_runtime_status(
                    self.root,
                    {"EVO_ORT_PROVISIONING": "external", "ORT_LIB_PATH": "missing"},
                )[0],
                "BLOCKED",
            )
            runtime = self.root / "ort"
            runtime.mkdir()
            (runtime / "libonnxruntime.1.29.0.dylib").write_bytes(b"synthetic runtime")
            state, detail = onnx_runtime_status(
                self.root,
                {
                    "EVO_ORT_PROVISIONING": "external",
                    "ORT_LIB_PATH": "ort",
                    "ORT_PREFER_DYNAMIC_LINK": "1",
                },
            )
            self.assertEqual(state, "DIAGNOSTIC")
            self.assertIn("not portable", detail)

        def bundled_inputs(self):
            env = {"EVO_ORT_PROVISIONING": "bundled"}
            for name in ORT_BUNDLE_INPUTS:
                path = self.root / name
                path.write_bytes(b"synthetic supplied file, not a runtime/license receipt")
                env[name] = str(path)
            return env

        def test_bundled_report_presence_never_executes_tools_or_loads_library(self):
            env = self.bundled_inputs()
            with patch.object(subprocess, "run", side_effect=AssertionError("unexpected tool")), patch.object(ctypes, "CDLL", side_effect=AssertionError("unexpected dylib load")):
                state, detail = onnx_runtime_status(self.root, env, "darwin")
            self.assertEqual(state, "DIAGNOSTIC")
            self.assertIn("no license approval", detail)

        def test_bundled_each_exact_input_is_required_no_directory_fallback(self):
            env = self.bundled_inputs()
            for name in ORT_BUNDLE_INPUTS:
                with self.subTest(name=name):
                    for value in ("", "missing", str(self.root)):
                        changed = dict(env, **{name: value}, ORT_LIB_PATH=str(self.root))
                        with patch.object(subprocess, "run", side_effect=AssertionError("unexpected tool")):
                            self.assertEqual(bundled_ort_input_status(self.root, changed, "darwin")[0], "BLOCKED")

        def test_bundled_missing_inputs_are_local_exit_failure(self):
            env = dict(self.env, EVO_ORT_PROVISIONING="bundled")
            with patch.object(shutil, "which", return_value="fixture-tool"), patch.object(subprocess, "run", side_effect=AssertionError("unexpected subprocess")), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(report(self.root, env, permissions=False), 1)

        def test_bundled_preflight_requires_pinned_binding_and_platform_before_tools(self):
            env = self.bundled_inputs()
            with patch.object(subprocess, "run", side_effect=AssertionError("unexpected tool")):
                self.assertEqual(bundled_ort_input_status(self.root, env, "linux")[0], "BLOCKED")
                state, detail = bundled_ort_input_status(self.root, env, "darwin")
                self.assertEqual(state, "BLOCKED")
                self.assertIn("requires ort", detail)

        def test_bundled_static_pass_is_not_loading_or_license_approval(self):
            env = self.bundled_inputs()
            (self.root / "Cargo.lock").write_text(
                'version = 4\n[[package]]\nname = "ort"\nversion = "2.0.0-rc.13"\n'
                '[[package]]\nname = "ort-sys"\nversion = "2.0.0-rc.13"\n', encoding="utf-8",
            )
            with patch(f"{__name__}.validate_bundled_runtime") as validate, patch.object(ctypes, "CDLL", side_effect=AssertionError("unexpected dylib load")):
                state, detail = bundled_ort_input_status(self.root, env, "darwin")
            self.assertEqual(state, "PASS")
            validate.assert_called_once_with(Path(env["EVO_ORT_DYLIB"]))
            self.assertIn("not license approval or loading/API negotiation proof", detail)
            self.assertEqual(detail.count("sha256="), 3)

        def test_bundle_signature_status_does_not_probe_missing_bundle(self):
            with patch.object(subprocess, "run", side_effect=AssertionError("unexpected codesign")):
                state, detail = bundle_signature_status(self.root)
            self.assertEqual(state, "NOT_RUN")
            self.assertIn("not run", detail)

        def test_explicit_build_offline_locked_and_lock_mutation(self):
            class Result:
                returncode = 0

            def fake_build(command, **kwargs):
                self.assertEqual(command, BUILD_COMMAND)
                self.assertEqual(kwargs["env"]["CARGO_NET_OFFLINE"], "true")
                return Result()

            with patch.object(shutil, "which", return_value="cargo"), patch.object(subprocess, "run", side_effect=fake_build), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(build_status(self.root)[0], "PASS")
                (self.root / "Cargo.lock").write_bytes(b"before")

                def mutate_lock(*args, **kwargs):
                    (self.root / "Cargo.lock").write_bytes(b"after")
                    return Result()

                with patch.object(subprocess, "run", side_effect=mutate_lock):
                    self.assertEqual(build_status(self.root)[0], "BLOCKED")

        def test_build_failure_or_interruption_never_retries(self):
            class Result:
                returncode = 17

            for result in (Result(), OSError("fixture: failed to start"), KeyboardInterrupt()):
                with self.subTest(result=type(result).__name__):
                    kwargs = {"side_effect": result} if isinstance(result, BaseException) else {"return_value": result}
                    with patch.object(shutil, "which", return_value="fixture-tool"), patch.object(subprocess, "run", **kwargs) as run, contextlib.redirect_stdout(io.StringIO()):
                        self.assertEqual(build_status(self.root)[0], "BLOCKED")
                        self.assertEqual(run.call_count, 1)
            with patch.object(shutil, "which", return_value=None), patch.object(subprocess, "run", side_effect=AssertionError("unexpected subprocess")):
                self.assertEqual(build_status(self.root)[0], "BLOCKED")

    result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(DoctorTests))
    return 0 if result.wasSuccessful() else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--report-only", action="store_true", help="default: presence + nonprompting permission diagnostics only")
    mode.add_argument("--build", action="store_true", help="explicitly run cargo build --workspace --offline --locked -j1; writes build artifacts, launches no apps")
    mode.add_argument("--syntax-check", action="store_true", help="compile script syntax only; no execution or bytecode writes")
    mode.add_argument("--self-test", action="store_true", help="run synthetic script fixtures only")
    mode.add_argument("--check-ort-inputs", action="store_true", help="explicit read-only pinned bundled runtime/notice/Mach-O checks; no dlopen, signing or downloads")
    mode.add_argument("--check-ort-bundle", type=Path, metavar="APP", help="read-only bundled runtime/notice/executable closure checks on an explicit app; no signing or launch")
    parser.add_argument("--workspace", type=Path, default=Path(__file__).resolve().parent.parent, help="workspace root; relative model paths use this root")
    parser.add_argument("--skip-permissions", action="store_true", help="omit read-only macOS permission preflights")
    parser.add_argument("--verify-bundle", action="store_true", help="explicit read-only strict codesign and bundled ORT closure checks on dist/Evo.app (or EVO_APP_PATH); no signing, repair or launch")
    args = parser.parse_args()
    if args.syntax_check:
        return syntax_check()
    if args.self_test:
        return self_test()
    if args.check_ort_inputs or args.check_ort_bundle is not None:
        state, detail = (
            bundled_ort_input_status(args.workspace.resolve(), os.environ)
            if args.check_ort_inputs else bundled_ort_bundle_status(args.check_ort_bundle)
        )
        print(f"[{state}] PORTABLE runtime/metadata: {detail}")
        return int(state != "PASS")
    return report(args.workspace.resolve(), os.environ, args.build, not args.skip_permissions, args.verify_bundle)


if __name__ == "__main__":
    sys.exit(main())
