#!/usr/bin/env python3
"""Reproducible packaging contract tests; fake Mach-O/tools, no app execution.

Run: python3 scripts/tests/test_ort_packaging.py
All build/sign commands target disposable fixtures. No real Cargo, codesign,
runtime loading, keychain, downloads, model assets or activity history are used.
"""

import importlib.util
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile
import unittest


SCRIPTS = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("evo_doctor", SCRIPTS / "evo-doctor.py")
DOCTOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DOCTOR)

FAKE_TOOL = r'''
import json, os, pathlib, plistlib, sys
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ["FIXTURE_LOG"], "a") as log:
    log.write(json.dumps({"tool": name, "args": args,
        "ort_path": os.environ.get("ORT_LIB_PATH"),
        "dynamic": os.environ.get("ORT_PREFER_DYNAMIC_LINK"),
        "offline": os.environ.get("CARGO_NET_OFFLINE")}) + "\n")
if name == "uname":
    print("Darwin"); sys.exit(0)
if name == "cargo":
    if "--offline" not in args or "--locked" not in args or os.environ.get("CARGO_NET_OFFLINE") != "true":
        sys.exit(70)
    directory = pathlib.Path(os.environ["CARGO_TARGET_DIR"])
    profile = args[args.index("--profile") + 1]
    output = directory / ("debug" if profile == "dev" else profile)
    output.mkdir(parents=True, exist_ok=True)
    if os.environ.get("RACE_OUTPUT") == "1":
        app = pathlib.Path(os.environ["EVO_APP_OUTPUT_DIR"]) / "Evo.app"
        app.mkdir(parents=True)
        (app / "keep.txt").write_text("concurrently created user artifact")
    for binary in ("evo-desktop", "evo-daemon", "evo-ledger-engine"):
        deps = [["/usr/lib/libSystem.B.dylib", "1.0.0", "1.0.0"]]
        if binary != "evo-ledger-engine":
            deps.append(["@rpath/libonnxruntime.1.dylib", "1.0.0", "1.29.0"])
        if binary == "evo-desktop" and os.environ.get("BAD_APP_DEP"):
            deps.append([os.environ["BAD_APP_DEP"], "1.0.0", "1.0.0"])
        arch = os.environ.get("BAD_APP_ARCH", "arm64") if binary == "evo-desktop" else "arm64"
        (output / binary).write_text(json.dumps({"arch": arch, "type": "EXECUTE",
            "deps": deps, "rpaths": [str(directory / "ort-link")], "minos": "13.0"}))
    sys.exit(0)
if name == "codesign":
    if "--verify" in args and os.environ.get("FAIL_VERIFY") == "1":
        print("fixture strict verification failure", file=sys.stderr); sys.exit(1)
    sys.exit(0)
if name == "xattr":
    sys.exit(0)
if name == "iconutil":
    pathlib.Path(args[args.index("-o") + 1]).write_bytes(b"synthetic icon"); sys.exit(0)
if name == "plutil":
    path = pathlib.Path(args[-1])
    info = plistlib.loads(path.read_bytes())
    info[args[1]] = args[3]
    path.write_bytes(plistlib.dumps(info)); sys.exit(0)
path = pathlib.Path(args[-1])
try:
    info = json.loads(path.read_text())
except (ValueError, OSError):
    print("not synthetic Mach-O", file=sys.stderr); sys.exit(1)
if name == "lipo":
    print(info["arch"])
elif name == "otool":
    print(str(path) + ":")
    if args[0] == "-hv":
        print("Mach header\nmagic cputype cpusubtype caps filetype ncmds sizeofcmds flags")
        print("MH_MAGIC_64 ARM64 ALL 0x00 " + info["type"] + " 1 1 NOUNDEFS")
    elif args[0] == "-D":
        print(info["id"])
    elif args[0] == "-L":
        for dependency, compatibility, current in info["deps"]:
            print(f"\t{dependency} (compatibility version {compatibility}, current version {current})")
    elif args[0] == "-l":
        print("Load command 0\n cmd LC_BUILD_VERSION\n cmdsize 32\n platform 1\n minos " + info["minos"] + "\n sdk 26.2")
        for rpath in info["rpaths"]:
            print("Load command 1\n cmd LC_RPATH\n cmdsize 32\n path " + rpath + " (offset 12)")
    else:
        sys.exit(71)
elif name == "nm":
    if info.get("exports", True):
        print("00000000000bed84 T _OrtGetApiBase")
elif name == "install_name_tool":
    if os.environ.get("BAD_RPATH_EDIT") != "1":
        if args[0] == "-delete_rpath":
            info["rpaths"].remove(args[1])
        elif args[0] == "-add_rpath":
            info["rpaths"].append(args[1])
        else:
            sys.exit(72)
        path.write_text(json.dumps(info))
else:
    sys.exit(73)
'''


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="evo-ort script fixture-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        scripts = self.root / "scripts"
        scripts.mkdir()
        for name in ("build-app.sh", "sign-dev.sh", "evo-doctor.py"):
            shutil.copyfile(SCRIPTS / name, scripts / name)
        (self.root / "Cargo.toml").write_text("[workspace]\nmembers = []\n")
        (self.root / "Cargo.lock").write_text(
            'version = 4\n[[package]]\nname = "ort"\nversion = "2.0.0-rc.13"\n'
            '[[package]]\nname = "ort-sys"\nversion = "2.0.0-rc.13"\n'
        )
        tools = self.root / "fake tools"
        tools.mkdir()
        dispatcher = tools / "dispatch"
        dispatcher.write_text(f"#!{sys.executable}\n" + FAKE_TOOL)
        dispatcher.chmod(0o755)
        for name in ("uname", "cargo", "lipo", "otool", "nm", "codesign", "xattr", "iconutil", "plutil", "install_name_tool"):
            (tools / name).symlink_to(dispatcher)
        inputs = self.root / "supplied notices and runtime"
        inputs.mkdir()
        self.runtime = inputs / "libonnxruntime.1.29.0.dylib"
        self.runtime_info = {
            "arch": "arm64", "type": "DYLIB", "id": DOCTOR.ORT_BUNDLE_ID,
            "deps": [[DOCTOR.ORT_BUNDLE_ID, "1.0.0", "1.29.0"]] + [
                [name, "1.0.0", "1.0.0"] for name in DOCTOR.ORT_SYSTEM_DEPENDENCIES
            ], "rpaths": ["@loader_path"], "minos": "14.0", "exports": True,
        }
        self.write_runtime()
        self.license = inputs / "LICENSE"
        self.notices = inputs / "ThirdPartyNotices.txt"
        self.license.write_text("synthetic supplied license, not a licensing receipt\n")
        self.notices.write_text("synthetic third-party notices, not official upstream notices\n")
        (self.root / "temp").mkdir()
        self.log = self.root / "commands.jsonl"
        self.env = dict(os.environ)
        for name in ("EVO_ORT_PROVISIONING", "ORT_LIB_PATH", "ORT_LIB_LOCATION", "ORT_PREFER_DYNAMIC_LINK", *DOCTOR.ORT_BUNDLE_INPUTS):
            self.env.pop(name, None)
        self.env.update({
            "PATH": str(tools) + os.pathsep + os.environ["PATH"],
            "TMPDIR": str(self.root / "temp"), "FIXTURE_LOG": str(self.log),
            "CARGO_TARGET_DIR": str(self.root / "target"),
            "EVO_APP_OUTPUT_DIR": str(self.root / "output"),
            "EVO_CODESIGN_IDENTITY": "Synthetic Fixture Identity",
            "EVO_ORT_PROVISIONING": "bundled", "EVO_ORT_DYLIB": str(self.runtime),
            "EVO_ORT_LICENSE": str(self.license), "EVO_ORT_THIRD_PARTY_NOTICES": str(self.notices),
        })
        self.app = self.root / "output/Evo.app"

    def write_runtime(self):
        self.runtime.write_text(json.dumps(self.runtime_info))

    def run_script(self, script="build-app.sh", *args, **overrides):
        env = dict(self.env, **overrides)
        return subprocess.run(["bash", str(self.root / "scripts" / script), *args],
                              cwd=self.root, env=env, capture_output=True, text=True, timeout=30)

    def commands(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []

    def assert_no_build_or_sign(self):
        self.assertFalse(any(item["tool"] in {"cargo", "codesign"} for item in self.commands()))

    def assert_no_app_sign(self):
        self.assertFalse(any(item["tool"] == "codesign" and "--sign" in item["args"] and
                             str(self.app) in item["args"][-1] for item in self.commands()))

    def test_bundled_requires_explicit_mode_and_each_exact_input(self):
        for variable in ("EVO_ORT_PROVISIONING", *DOCTOR.ORT_BUNDLE_INPUTS):
            with self.subTest(variable=variable):
                result = self.run_script(**{variable: ""})
                self.assertNotEqual(result.returncode, 0)
                self.assert_no_build_or_sign()

    def test_missing_empty_symlink_and_same_notices_fail_before_sign_or_build(self):
        for value in (str(self.root / "missing"), str(self.root / "empty"), str(self.root / "alias")):
            (self.root / "empty").touch()
            if not (self.root / "alias").exists():
                (self.root / "alias").symlink_to(self.notices)
            with self.subTest(value=value):
                result = self.run_script(EVO_ORT_THIRD_PARTY_NOTICES=value)
                self.assertNotEqual(result.returncode, 0)
                self.assert_no_build_or_sign()
        self.assertNotEqual(self.run_script(EVO_ORT_LICENSE=str(self.notices)).returncode, 0)
        self.assert_no_build_or_sign()

    def test_invalid_runtime_metadata_fails_closed(self):
        cases = [
            ("arch", "x86_64"), ("arch", "arm64 x86_64"), ("type", "EXECUTE"),
            ("id", "/external/libonnxruntime.1.dylib"), ("minos", "15.0"),
            ("exports", False), ("rpaths", ["/external/runtime"]),
            ("deps", self.runtime_info["deps"][:-1]),
            ("deps", self.runtime_info["deps"] + [["/opt/homebrew/lib/libextra.dylib", "1.0.0", "1.0.0"]]),
            ("deps", self.runtime_info["deps"] + [self.runtime_info["deps"][-1]]),
            ("deps", [[DOCTOR.ORT_BUNDLE_ID, "1.0.0", "1.16.0"]] + self.runtime_info["deps"][1:]),
        ]
        for key, value in cases:
            with self.subTest(key=key, value=value):
                original = self.runtime_info[key]
                self.runtime_info[key] = value
                self.write_runtime()
                result = self.run_script()
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assert_no_build_or_sign()
                self.runtime_info[key] = original
        self.write_runtime()

    def test_binding_upgrade_requires_review(self):
        lock = self.root / "Cargo.lock"
        lock.write_text(lock.read_text().replace("2.0.0-rc.13", "2.0.0-rc.14"))
        result = self.run_script()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("re-review ABI", result.stdout)
        self.assert_no_build_or_sign()

    def test_existing_bundle_is_not_overwritten_or_signed(self):
        self.app.mkdir(parents=True)
        marker = self.app / "keep.txt"
        marker.write_text("user artifact")
        result = self.run_script()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(marker.read_text(), "user artifact")
        self.assert_no_build_or_sign()
        marker.unlink()
        self.app.rmdir()
        raced = self.run_script(RACE_OUTPUT="1")
        self.assertNotEqual(raced.returncode, 0)
        self.assertEqual(marker.read_text(), "concurrently created user artifact")
        self.assert_no_app_sign()

    def test_bundled_copies_exact_files_repaths_before_sign_and_strictly_verifies(self):
        source = self.runtime.read_bytes()
        # A competing directory candidate and legacy directory configuration
        # must not replace the explicitly selected runtime file.
        (self.runtime.parent / "libonnxruntime.0.dylib").write_bytes(b"not a runtime")
        result = self.run_script(ORT_LIB_PATH=str(self.root / "missing legacy directory"))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        runtime = self.app / "Contents/Frameworks/libonnxruntime.1.dylib"
        self.assertEqual(runtime.read_bytes(), source)
        self.assertEqual(self.runtime.read_bytes(), source)
        self.assertEqual((self.app / "Contents/Resources/onnxruntime/LICENSE").read_bytes(), self.license.read_bytes())
        self.assertEqual((self.app / "Contents/Resources/onnxruntime/ThirdPartyNotices.txt").read_bytes(), self.notices.read_bytes())
        info = plistlib.loads((self.app / "Contents/Info.plist").read_bytes())
        self.assertEqual(info["LSMinimumSystemVersion"], "14.0")
        for name in ("Evo", "evo-daemon", "evo-ledger-engine"):
            binary = self.app / "Contents/MacOS" / name
            self.assertEqual(json.loads(binary.read_text())["rpaths"], [DOCTOR.ORT_BUNDLE_RPATH])
        commands = self.commands()
        cargo = next(item for item in commands if item["tool"] == "cargo")
        self.assertEqual(cargo["dynamic"], "1")
        self.assertIn("ort-bundled-link.", cargo["ort_path"])
        app_signs = [(index, item) for index, item in enumerate(commands) if item["tool"] == "codesign" and "--sign" in item["args"] and str(self.app) in item["args"][-1]]
        self.assertEqual([item["args"][-1] for _, item in app_signs], [str(runtime)] + [str(self.app / "Contents/MacOS" / name) for name in ("evo-daemon", "evo-ledger-engine", "Evo")] + [str(self.app)])
        self.assertLess(max(index for index, item in enumerate(commands) if item["tool"] == "install_name_tool"), app_signs[0][0])
        self.assertTrue(any(item["tool"] == "codesign" and item["args"] == ["--verify", "--deep", "--strict", str(self.app)] for item in commands))
        self.assertIn("not license approval", result.stdout)

    def test_impossible_app_architecture_refuses_before_app_signing(self):
        result = self.run_script(BAD_APP_ARCH="x86_64")
        self.assertNotEqual(result.returncode, 0)
        self.assert_no_app_sign()

    def test_external_dependency_refuses_before_app_signing(self):
        result = self.run_script(BAD_APP_DEP="/opt/homebrew/lib/extra.dylib")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("unresolved non-system dependency", result.stdout)
        self.assert_no_app_sign()

    def test_failed_rpath_edit_refuses_before_app_signing(self):
        result = self.run_script(BAD_RPATH_EDIT="1")
        self.assertNotEqual(result.returncode, 0)
        self.assert_no_app_sign()

    def test_strict_verification_failure_is_not_reported_as_built(self):
        result = self.run_script(FAIL_VERIFY="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("Built:", result.stdout)

    def test_external_development_path_needs_no_notices_and_remains_unbundled(self):
        result = self.run_script(EVO_ORT_PROVISIONING="external", ORT_LIB_PATH=str(self.runtime.parent),
                                 ORT_PREFER_DYNAMIC_LINK="1", EVO_ORT_LICENSE="", EVO_ORT_THIRD_PARTY_NOTICES="")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse((self.app / "Contents/Frameworks").exists())
        self.assertEqual(plistlib.loads((self.app / "Contents/Info.plist").read_bytes())["LSMinimumSystemVersion"], "13.0")
        self.assertFalse(any(item["tool"] == "install_name_tool" for item in self.commands()))
        self.assertIn("development link; portable release is not verified", result.stdout)

    def test_sign_dev_app_is_explicit_and_signs_nested_code_before_bundle(self):
        built = self.run_script()
        self.assertEqual(built.returncode, 0, built.stdout + built.stderr)
        self.log.unlink()
        rejected = self.run_script("sign-dev.sh", "--app", str(self.app), EVO_ORT_PROVISIONING="external")
        self.assertNotEqual(rejected.returncode, 0)
        self.assert_no_build_or_sign()
        signed = self.run_script("sign-dev.sh", "--app", str(self.app))
        self.assertEqual(signed.returncode, 0, signed.stdout + signed.stderr)
        paths = [item["args"][-1] for item in self.commands() if item["tool"] == "codesign" and "--sign" in item["args"] and str(self.app) in item["args"][-1]]
        self.assertEqual(paths[0], str(self.app / "Contents/Frameworks/libonnxruntime.1.dylib"))
        self.assertEqual(paths[-1], str(self.app))

    def test_sign_dev_default_never_signs_dist_or_packaged_runtime(self):
        built = self.run_script()
        self.assertEqual(built.returncode, 0, built.stdout + built.stderr)
        self.log.unlink()
        signed = self.run_script("sign-dev.sh")
        self.assertEqual(signed.returncode, 0, signed.stdout + signed.stderr)
        self.assert_no_app_sign()

    def test_doctor_closure_check_does_not_sign_repair_or_launch(self):
        built = self.run_script()
        self.assertEqual(built.returncode, 0, built.stdout + built.stderr)
        self.log.unlink()
        command = [sys.executable, str(self.root / "scripts/evo-doctor.py"), "--check-ort-bundle", str(self.app)]
        checked = subprocess.run(command, env=self.env, capture_output=True, text=True, timeout=30)
        self.assertEqual(checked.returncode, 0, checked.stdout + checked.stderr)
        self.assertTrue(all(item["tool"] in {"lipo", "otool", "nm"} for item in self.commands()))
        notices = self.app / "Contents/Resources/onnxruntime/ThirdPartyNotices.txt"
        original = notices.read_bytes()
        notices.unlink()
        self.assertNotEqual(subprocess.run(command, env=self.env, capture_output=True, timeout=30).returncode, 0)
        notices.write_bytes(original)
        plist = self.app / "Contents/Info.plist"
        original = plist.read_bytes()
        info = plistlib.loads(original)
        info["LSMinimumSystemVersion"] = "13.0"
        plist.write_bytes(plistlib.dumps(info))
        self.assertNotEqual(subprocess.run(command, env=self.env, capture_output=True, timeout=30).returncode, 0)
        plist.write_bytes(original)
        for path in (self.app / "Contents/Frameworks/unreviewed.dylib",
                     self.app / "Contents/Resources/unreviewed.py",
                     self.app / "Contents/Resources/onnxruntime/unreviewed.txt"):
            with self.subTest(path=path):
                path.write_bytes(b"synthetic unexpected content")
                self.assertNotEqual(subprocess.run(command, env=self.env, capture_output=True, timeout=30).returncode, 0)
                path.unlink()
        self.assertTrue(all(item["tool"] in {"lipo", "otool", "nm"} for item in self.commands()))


if __name__ == "__main__":
    unittest.main(verbosity=2)
