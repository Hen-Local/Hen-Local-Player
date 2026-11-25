"""Collect notices from the locked Rust and frontend packages used by a desktop build."""

import argparse
import json
import os
import re
import subprocess
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
UI = ROOT / "app/ui"
VENDOR = ROOT / "vendor/qwen3-asr-mlx"
SUPPLEMENTS = ROOT / "docs/licenses/registry"
NOTICE_NAME = re.compile(r"^(?:unlicen[sc]e|licen[sc]es?|copying|notices?|copyright)(?:$|[._-])", re.I)
OMINIX = "https://github.com/OminiX-ai/OminiX-MLX/tree/6aac996db8b71fb7dae7a2409c46b4f2ade93092"


def notice_files(directory, recursive=True):
    for folder, dirs, files in os.walk(directory):
        dirs[:] = [d for d in sorted(dirs) if recursive and d not in {".git", "target", "node_modules"}]
        for filename in sorted(files):
            path = Path(folder) / filename
            if (NOTICE_NAME.match(filename)
                and path.suffix.lower() not in {".rs", ".c", ".cpp", ".cc", ".h", ".json", ".py", ".sh", ".xml", ".toml"}
                and path.is_file()):
                yield path


def rust_packages(target):
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--format-version=1", "--filter-platform", target,
         "--features", "henlocal-player/custom-protocol"], cwd=ROOT))
    packages = {package["id"]: package for package in metadata["packages"]}
    dependencies = {node["id"]: node["deps"] for node in metadata["resolve"]["nodes"]}
    queue = [key for key, pkg in packages.items() if pkg["name"] in {"henlocal-player", "hlp-asr-worker"}]
    seen = set()
    while queue:
        key = queue.pop()
        if key in seen:
            continue
        seen.add(key)
        for dep in dependencies[key]:
            if any(edge["kind"] != "dev" for edge in dep["dep_kinds"]):
                queue.append(dep["pkg"])
        pkg = packages[key]
        name = pkg["name"]
        path = Path(pkg["manifest_path"]).parent
        if pkg["source"] is None and path != VENDOR:
            continue
        files = list(notice_files(path))
        if pkg["license_file"]:
            files.append(path / pkg["license_file"])
        url = f"https://crates.io/crates/{name}/{pkg['version']}"
        if path == VENDOR or "OminiX-MLX" in (pkg["source"] or ""):
            url = OMINIX
            files += sorted(VENDOR.glob("LICENSE-*"))
        elif name.startswith("objc2") or name in {"dispatch2", "block2"}:
            files += [SUPPLEMENTS / "objc2.txt", VENDOR / "LICENSE-MIT"]
        elif name.startswith("webview2-com"):
            files.append(SUPPLEMENTS / "webview2.txt")
        elif name == "selectors":
            files.append(SUPPLEMENTS / "MPL-2.0.txt")
        elif (SUPPLEMENTS / f"{name}.txt").is_file():
            files.append(SUPPLEMENTS / f"{name}.txt")
        yield name, pkg["version"], pkg["license"], url, pkg["authors"], path, files


def frontend_packages():
    lock = json.loads((UI / "package-lock.json").read_text())
    for location, pkg in lock["packages"].items():
        if not location or pkg.get("dev"):
            continue
        path = UI / location
        manifest = json.loads((path / "package.json").read_text())
        name = manifest["name"]
        files = list(notice_files(path, recursive=False))
        if name in {"is-reference", "locate-character"}:
            files.append(VENDOR / "LICENSE-MIT")
        if name.startswith("@tauri-apps/"):
            files.append(SUPPLEMENTS / "tauri-plugin.txt")
        author = manifest.get("author", "")
        if isinstance(author, dict):
            author = author.get("name", "")
        yield name, pkg["version"], pkg.get("license"), f"https://www.npmjs.com/package/{name}/v/{pkg['version']}", [author], path, files


def render(target):
    index = []
    texts = defaultdict(set)
    packages = sorted([*rust_packages(target), *frontend_packages()], key=lambda p: (p[0], p[1], p[3]))
    for name, version, license_id, url, authors, directory, files in packages:
        if not files:
            raise ValueError(f"No license text was found for {name} {version}")
        ident = f"{name} {version}"
        index.append(f"{ident} | {license_id or 'see notice'} | {url}")
        if any(authors):
            index.append("    " + ", ".join(authors))
        for path in set(files):
            content = path.read_text(encoding="utf-8-sig", errors="replace").strip()
            label = path.relative_to(directory) if path.is_relative_to(directory) else path.name
            texts[content].add(f"{ident} ({label})")
    output = ["Rust and JavaScript dependency notices", "", "Source packages for the shipped versions are available at the links below.", *index]
    for content, owners in sorted(texts.items(), key=lambda item: sorted(item[1])):
        output.extend(["", "=" * 72, *sorted(owners), "", content])
    return "\n".join(output) + "\n", len(packages)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", help="Rust target triple for the installer")
    parser.add_argument("--output", type=Path, default=ROOT / "app/binaries/Dependency-Notices.txt")
    args = parser.parse_args()
    notices, count = render(args.target)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(notices, encoding="utf-8")
    print(f"Staged license notices for {count} packages ({args.target}).")
