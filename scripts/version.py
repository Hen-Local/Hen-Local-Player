"""Keep the native package, UI preview and Rust workspace on one release version."""
import argparse
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / "app/tauri.conf.json"
UI = ROOT / "app/ui"
WORKSPACE = ROOT / "Cargo.toml"
WORKSPACE_VERSION = re.compile(r'(\[workspace\.package\]\s*version\s*=\s*")([^"]+)(")')
STABLE = re.compile(r'(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)')


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def check():
    version = read_json(NATIVE)["version"]
    lock = read_json(UI / "package-lock.json")
    versions = {
        "Cargo.toml": WORKSPACE_VERSION.search(WORKSPACE.read_text()).group(2),
        "package.json": read_json(UI / "package.json")["version"],
        "package-lock.json": lock["version"],
        "package-lock.json (root package)": lock["packages"][""]["version"],
    }
    for file, actual in versions.items():
        if actual != version:
            raise SystemExit(f"{file} has {actual}, expected {version}")
    print(version)


def set_version(version):
    if not version or not STABLE.fullmatch(version):
        raise SystemExit("Use a stable version such as 0.2.0")
    config = read_json(NATIVE)
    config["version"] = version
    NATIVE.write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    manifest, replacements = WORKSPACE_VERSION.subn(lambda m: m[1] + version + m[3], WORKSPACE.read_text())
    if replacements != 1:
        raise SystemExit("Could not locate workspace package version")
    WORKSPACE.write_text(manifest, encoding="utf-8")
    for name in ("package.json", "package-lock.json"):
        path = UI / name
        data = read_json(path)
        data["version"] = version
        if name == "package-lock.json":
            data["packages"][""]["version"] = version
        path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    subprocess.run(["cargo", "update", "--workspace", "--offline"], cwd=ROOT, check=True)
    check()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    subcommands.add_parser("check")
    subcommands.add_parser("set").add_argument("version")
    args = parser.parse_args()
    if args.command == "set":
        set_version(args.version)
    else:
        check()
