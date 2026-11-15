#!/usr/bin/env python3
"""Build one unsigned preview ABI; enforce Cargo locks and inspect the actual APK."""

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
import time
import urllib.request
import zipfile
import struct

from prepare_android_assets import build_index, write_index
from inspect_android import ABI_TARGETS, inspect_apk

LOCKFILES = ("Cargo.lock", "apps/desktop/src-tauri/Cargo.lock")
TARGETS = {"aarch64": "aarch64-linux-android", "x86_64": "x86_64-linux-android"}
GRADLE_SHA256 = "61ad310d3c7d3e5da131b76bbf22b5a4c0786e9d892dae8c1658d4b484de3caa"
WRAPPER_SHA256 = "7d3a4ac4de1c32b59bc6a4eb8ecb8e612ccd0cf1ae1e99f66902da64df296172"
WRAPPER_URL = "https://raw.githubusercontent.com/gradle/gradle/v8.14.0/gradle/wrapper/gradle-wrapper.jar"
PAGE_FLAGS = ("-C link-arg=-landroid -C link-arg=-llog -C link-arg=-lOpenSLES "
              "-C link-arg=-Wl,-z,max-page-size=16384 -C link-arg=-Wl,-z,common-page-size=16384")
RESOLVING = {"build", "check", "rustc", "test", "bench", "clippy", "metadata", "fetch", "tree", "doc"}
COMPILING = {"build", "check", "rustc", "test", "bench", "clippy", "doc"}
QUERIES = {"locate-project", "read-manifest", "help", "version"}


def command_position(arguments):
    index = 0
    while index < len(arguments):
        argument = arguments[index]
        if argument.startswith("+") or argument.startswith("-"):
            index += 2 if argument in {"--config", "--color", "-Z"} else 1
        else:
            return index
    return None


def locked_argv(arguments, target):
    arguments = list(arguments)
    position = command_position(arguments)
    if position is None:
        if any(flag in arguments for flag in ("--version", "-V", "-vV", "--help", "-h")):
            return arguments
        raise ValueError("Cargo invocation has no permitted command")
    command = arguments[position]
    if command == "tauri" or command in QUERIES:
        return arguments
    if command not in RESOLVING:
        raise ValueError(f"Cargo command is outside the locked preview recipe: {command}")
    separator = arguments.index("--") if "--" in arguments else len(arguments)
    cargo_options = arguments[position + 1:separator]
    if command in COMPILING:
        targets = []
        for number, option in enumerate(cargo_options):
            if option == "--target" and number + 1 < len(cargo_options):
                targets.append(cargo_options[number + 1])
            elif option.startswith("--target="):
                targets.append(option.split("=", 1)[1])
        if targets != [target]:
            raise ValueError("Every Cargo compilation must explicitly select the requested single Android target")
    if "--locked" not in arguments[:separator] and "--frozen" not in arguments[:separator]:
        arguments.insert(separator, "--locked")
    return arguments


def lock_hashes(project):
    hashes = {}
    for name in LOCKFILES:
        path = project / name
        if path.is_symlink() or not path.is_file():
            raise ValueError("Expected regular Cargo lockfile is unavailable: " + name)
        hashes[name] = hashlib.sha256(path.read_bytes()).hexdigest()
    return hashes


def log_event(path, event):
    descriptor = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o600)
    try:
        data = (json.dumps(event, sort_keys=True) + "\n").encode()
        if os.write(descriptor, data) != len(data):
            raise OSError("Incomplete Cargo evidence log write")
    finally:
        os.close(descriptor)


def cargo_wrapper(arguments):
    configuration = json.loads(Path(os.environ["INOUT_LOCK_WRAPPER_CONFIG"]).read_text())
    project = Path(configuration["project"])
    event_path = Path(configuration["events"])
    try:
        if lock_hashes(project) != configuration["lock_hashes"]:
            raise ValueError("Cargo lockfiles changed before child invocation")
        effective = locked_argv(arguments, configuration["target"])
        position = command_position(effective)
        command = effective[position] if position is not None else None
        log_event(event_path, {"event": "invoke", "original_argv": arguments, "effective_argv": effective,
                              "cargo": configuration["cargo"], "cwd": os.getcwd(), "subcommand": command,
                              "target_compilation": command in COMPILING,
                              "locked": "--locked" in effective[:effective.index("--") if "--" in effective else len(effective)]
                              or "--frozen" in effective})
        result = subprocess.run([configuration["cargo"], *effective], check=False)
        unchanged = lock_hashes(project) == configuration["lock_hashes"]
        log_event(event_path, {"event": "exit", "exit_code": result.returncode, "lockfiles_unchanged": unchanged})
        if not unchanged:
            raise ValueError("Cargo lockfiles changed during child invocation")
        return result.returncode
    except (OSError, ValueError) as error:
        log_event(event_path, {"event": "rejected", "original_argv": arguments, "error": str(error)})
        print("Android Cargo wrapper rejected invocation: " + str(error), file=sys.stderr)
        return 96


def run_locked(command, project, report_dir, target, cargo, env):
    project, report_dir = Path(project).resolve(), Path(report_dir).resolve()
    cargo = Path(cargo).absolute()  # Preserve rustup proxy argv[0]; do not resolve its symlink to rustup.
    report_dir.mkdir(parents=True, exist_ok=False)
    baseline = lock_hashes(project)
    configuration_path = report_dir / "cargo-wrapper.json"
    events_path = report_dir / "cargo-events.jsonl"
    configuration_path.write_text(json.dumps({"project": str(project), "target": target, "cargo": str(cargo),
                                             "events": str(events_path), "lock_hashes": baseline}))
    configuration_path.chmod(0o600)
    wrappers = report_dir / "bin"
    wrappers.mkdir()
    wrapper = wrappers / "cargo"
    wrapper.write_text("#!" + sys.executable + "\nimport os,sys\n"
                       "os.execv(sys.executable, [sys.executable, " + repr(str(Path(__file__).resolve()))
                       + ", '_cargo', *sys.argv[1:]])\n")
    wrapper.chmod(0o700)
    child_env = dict(env, PATH=str(wrappers) + os.pathsep + env.get("PATH", os.defpath),
                     CARGO=str(wrapper), INOUT_LOCK_WRAPPER_CONFIG=str(configuration_path))
    stdout_path, stderr_path = report_dir / "build.stdout", report_dir / "build.stderr"
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        result = subprocess.run([str(part) for part in command], cwd=project, env=child_env,
                                stdout=stdout, stderr=stderr, check=False)
    try:
        after = lock_hashes(project)
        changed = [name for name in LOCKFILES if after[name] != baseline[name]]
    except (OSError, ValueError):
        changed = list(LOCKFILES)
        after = {}
    events = [json.loads(row) for row in events_path.read_text().splitlines()] if events_path.exists() else []
    locked = sum(event.get("event") == "invoke" and event.get("target_compilation") and event.get("locked")
                 for event in events)
    rejected = any(event.get("event") == "rejected" for event in events)
    exit_code = result.returncode or (96 if changed or rejected or not locked else 0)
    report = {"exit_code": exit_code, "child_exit_code": result.returncode, "command": list(map(str, command)),
              "cargo_events": str(events_path), "locked_target_commands": locked, "changed_lockfiles": changed,
              "lock_hashes_before": baseline, "lock_hashes_after": after,
              "stdout": str(stdout_path), "stderr": str(stderr_path)}
    (report_dir / "locked-build.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


def ensure_wrapper(project, source=None, fetch=False):
    destination = project / "apps/desktop/src-tauri/gen/android/gradle/wrapper/gradle-wrapper.jar"
    if destination.exists():
        data = destination.read_bytes()
    elif source:
        data = Path(source).read_bytes()
    elif fetch:
        with urllib.request.urlopen(WRAPPER_URL, timeout=30) as response:
            data = response.read(1024 * 1024 + 1)
    else:
        raise ValueError("Ignored Gradle wrapper JAR is missing; pass --wrapper-jar or --fetch-wrapper explicitly")
    if len(data) > 1024 * 1024 or hashlib.sha256(data).hexdigest() != WRAPPER_SHA256:
        raise ValueError("Gradle8.14 wrapper JAR checksum does not match the official pinned checksum")
    if not destination.exists():
        destination.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(dir=destination.parent, delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(data)
        try:
            os.replace(temporary, destination)
        finally:
            temporary.unlink(missing_ok=True)
    return {"path": str(destination), "sha256": WRAPPER_SHA256, "official_source": WRAPPER_URL,
            "checksum_source": "https://services.gradle.org/distributions/gradle-8.14-wrapper.jar.sha256"}


def prepare_icons(project, npm, output):
    source = project / "apps/desktop/src-tauri/icons/icon.png"
    if source.is_symlink() or not source.is_file():
        raise ValueError("Tracked source icon is unavailable for a clean Android build")
    if output.exists():
        raise ValueError("Android icon output must be a new ignored directory")
    command = [str(npm), "--prefix", str(project / "apps/desktop"), "run", "tauri", "--", "icon",
               str(source), "--output", str(output)]
    result = subprocess.run(command, capture_output=True, text=True, timeout=90)
    if result.returncode:
        raise ValueError("Tauri could not generate Android icons from the tracked source: " + result.stderr)
    resources = []
    for density in ("mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"):
        for name in ("ic_launcher", "ic_launcher_round", "ic_launcher_foreground"):
            relative = Path("mipmap-" + density) / (name + ".png")
            generated = output / "android" / relative
            if generated.is_symlink() or not generated.is_file() or not generated.stat().st_size:
                raise ValueError("Tauri did not generate the expected Android icon: " + str(relative))
            destination = project / "apps/desktop/src-tauri/gen/android/app/src/main/res" / relative
            if destination.is_symlink() or destination.parent.is_symlink():
                raise ValueError("Android icon destination is a symbolic link")
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(generated, destination)
            resources.append({"path": str(destination), "sha256": hashlib.sha256(destination.read_bytes()).hexdigest()})
    return {"command": command, "stdout": result.stdout, "stderr": result.stderr, "resources": resources,
            "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest()}


def validate_preview_project(project):
    native = project / "apps/desktop/src-tauri"
    product = json.loads((project / "product.json").read_text())
    base = json.loads((native / "tauri.conf.json").read_text())
    overlay = json.loads((native / "tauri.android.conf.json").read_text())
    android = overlay["bundle"]["android"]
    resources = {**base["bundle"].get("resources", {}), **overlay["bundle"].get("resources", {})}
    if (product.get("application_id") != "io.github.zocs.inoutnext" or product.get("version") != "0.1.0"
            or base.get("identifier") != product["application_id"] or base.get("version") != "0.1.0"
            or android.get("versionCode") != 100001 or android.get("minSdkVersion") != 24
            or resources.get("../../web/out/") != "web/"
            or resources.get("../../../.local/android-assets/web-index.tsv") != "inout-web-index.tsv"):
        raise ValueError("Preview identity/version or raw Android resource mappings cannot be established")
    gradle = (native / "gen/android/app/build.gradle.kts").read_text()
    required = [r'compileSdk\s*=\s*36\b', r'targetSdk\s*=\s*36\b', r'minSdk\s*=\s*24\b',
                r'buildToolsVersion\s*=\s*"36\.0\.0"', r'ndkVersion\s*=\s*"28\.2\.13676358"',
                r'applicationId\s*=\s*"io\.github\.zocs\.inoutnext"',
                r'environmentVariable\("INOUT_ANDROID_PREVIEW"\).*==\s*"1"',
                r'getByName\("release"\).*?if\s*\(previewBuild\)\s*\{\s*applicationIdSuffix\s*=\s*"\.preview"']
    if any(not re.search(pattern, gradle, re.S) for pattern in required) or re.search(r'\bsigningConfig\w*\b', gradle):
        raise ValueError("Generated Gradle app cannot establish an unsigned optimized preview scope")
    root_gradle = (native / "gen/android/build.gradle.kts").read_text()
    if ('com.android.tools.build:gradle:8.13.1' not in root_gradle
            or 'org.jetbrains.kotlin:kotlin-gradle-plugin:2.2.20' not in root_gradle):
        raise ValueError("Generated Gradle/AGP/Kotlin versions differ from the pinned builder")
    wrapper = (native / "gen/android/gradle/wrapper/gradle-wrapper.properties").read_text()
    if 'gradle-8.14-bin.zip' not in wrapper or 'distributionSha256Sum=' + GRADLE_SHA256 not in wrapper:
        raise ValueError("Gradle distribution version/checksum is not pinned")
    return product


def executable(value):
    path = shutil.which(str(value))
    if not path:
        raise ValueError("Required executable is unavailable: " + str(value))
    return Path(path).absolute()


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "_cargo":
        return cargo_wrapper(sys.argv[2:])
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--abi", choices=TARGETS, default="aarch64")
    parser.add_argument("--export", choices=("reuse", "index", "build"), default="reuse",
                        help="reuse verifies existing export/index; index regenerates only index; build explicitly rebuilds web")
    parser.add_argument("--sdk", type=Path, default=Path(os.environ.get("ANDROID_HOME", "/opt/android-sdk")))
    parser.add_argument("--ndk", type=Path)
    parser.add_argument("--java-home", type=Path, default=Path(os.environ.get("JAVA_HOME", "/usr/lib/jvm/java-17-openjdk")))
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--npm", default="npm")
    parser.add_argument("--wrapper-jar", type=Path)
    parser.add_argument("--fetch-wrapper", action="store_true")
    parser.add_argument("--plan", action="store_true", help="write validated command/configuration without building or fetching")
    parser.add_argument("--report", type=Path, required=True)
    arguments = parser.parse_args()
    project = arguments.project.resolve()
    run_dir = project / ".local/android-build" / (time.strftime("%Y%m%d-%H%M%S") + "-" + str(os.getpid()))
    report = {"passed": False, "abi": ABI_TARGETS[arguments.abi], "target": TARGETS[arguments.abi],
              "preview": True, "unsigned": True, "target_device_verified": False, "run_directory": str(run_dir)}
    try:
        if not sys.platform.startswith("linux"):
            raise ValueError("This Android build recipe currently supports Linux builders")
        validate_preview_project(project)
        cargo, npm = executable(arguments.cargo), executable(arguments.npm)
        sdk = arguments.sdk.resolve()
        ndk = (arguments.ndk or sdk / "ndk/28.2.13676358").resolve()
        executable(arguments.java_home / "bin/java")
        release = (arguments.java_home / "release").read_text()
        if not re.search(r'JAVA_VERSION="17(?:\.|\")', release):
            raise ValueError("Android builder must explicitly use JDK17")
        properties = (ndk / "source.properties").read_text()
        if not re.search(r'Pkg.Revision\s*=\s*28\.2\.13676358\b', properties):
            raise ValueError("Selected Android NDK does not match the pinned revision")
        if not (sdk / "platforms/android-36/android.jar").is_file():
            raise ValueError("Pinned Android platform36 is unavailable")
        compiler = executable(ndk / "toolchains/llvm/prebuilt/linux-x86_64/bin" / (TARGETS[arguments.abi] + "24-clang"))
        aapt2 = executable(sdk / "build-tools/36.0.0/aapt2")
        zipalign = executable(sdk / "build-tools/36.0.0/zipalign")
        command = [str(npm), "--prefix", "apps/desktop", "run", "tauri", "--", "android", "build", "--ci",
                   "--apk", "--target", arguments.abi, "--config", '{"build":{"beforeBuildCommand":""}}']
        report.update({"command": command, "environment": {"INOUT_ANDROID_PREVIEW": "1", "RUSTFLAGS": PAGE_FLAGS,
                       "JAVA_HOME": str(arguments.java_home), "ANDROID_HOME": str(sdk), "NDK_HOME": str(ndk)},
                       "cargo": str(cargo), "compiler": str(compiler), "export_mode": arguments.export,
                       "lock_hashes": lock_hashes(project), "wrapper_source": WRAPPER_URL,
                       "wrapper_sha256": WRAPPER_SHA256})
        if arguments.plan:
            report.update({"passed": True, "plan_only": True, "build_performed": False})
        else:
            report["wrapper"] = ensure_wrapper(project, arguments.wrapper_jar, arguments.fetch_wrapper)
            report["icons"] = prepare_icons(project, npm, run_dir.with_name(run_dir.name + "-icons"))
            web, index = project / "apps/web/out", project / ".local/android-assets/web-index.tsv"
            if arguments.export == "build":
                subprocess.run([str(npm), "--prefix", "apps/web", "run", "build"], cwd=project, check=True)
            actual_index = build_index(web)
            if arguments.export in ("index", "build"):
                write_index(index, actual_index)
            elif index.read_bytes() != actual_index:
                raise ValueError("Existing web export/index differ; choose --export index or build explicitly")
            if lock_hashes(project) != report["lock_hashes"]:
                raise ValueError("Cargo lockfiles changed while preparing Android resources")
            env = dict(os.environ, **report["environment"], ANDROID_SDK_ROOT=str(sdk),
                       PATH=str(cargo.parent) + os.pathsep + os.environ.get("PATH", os.defpath))
            env.pop("CARGO_ENCODED_RUSTFLAGS", None)
            env.pop("CARGO_BUILD_RUSTFLAGS", None)
            started = time.time_ns()
            result = run_locked(command, project, run_dir, TARGETS[arguments.abi], cargo, env)
            report["build"] = result
            # Pinned CLI first builds Rust to discover plugins; Gradle's ABI task
            # then invokes android-studio-script and builds Rust again.
            if result["exit_code"] or result["locked_target_commands"] < 2:
                raise ValueError("Android build failed or lacks unchanged locks/locked-target evidence; see preserved logs")
            candidates = list((project / "apps/desktop/src-tauri/gen/android/app/build/outputs/apk/universal/release").glob("*-unsigned.apk"))
            candidates = [candidate for candidate in candidates if candidate.stat().st_mtime_ns >= started]
            if len(candidates) != 1:
                raise ValueError("Build did not produce exactly one fresh universal release unsigned APK")
            report["inspection"] = inspect_apk(candidates[0], ABI_TARGETS[arguments.abi], web, index,
                                               project / "product.json", aapt2, zipalign)
            if lock_hashes(project) != report["lock_hashes"]:
                raise ValueError("Cargo lockfiles changed before final APK acceptance")
            report.update({"passed": True, "build_performed": True})
    except (OSError, ValueError, KeyError, UnicodeError, struct.error, zipfile.BadZipFile, subprocess.SubprocessError) as error:
        report["error"] = str(error)
    arguments.report.parent.mkdir(parents=True, exist_ok=True)
    arguments.report.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({"passed": report["passed"], "report": str(arguments.report), "error": report.get("error")}, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
