#!/usr/bin/env python3
"""Sign the final platform packages and write a per-release Tauri updater manifest."""
import argparse
import base64
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
RELEASE_REPOSITORY = "Hen-Local/Hen-Local-Player"
PAYLOADS = {"darwin-aarch64": "macos-arm64.app.tar.gz", "windows-x86_64": "windows-x64-setup.exe"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag", help="Stable source tag, e.g. v0.1.0")
    parser.add_argument("platforms", nargs="+", choices=PAYLOADS)
    parser.add_argument("--dist", type=Path, default=ROOT / "dist")
    parser.add_argument("--notes", type=Path, help="Release notes for this version")
    args = parser.parse_args()
    version = json.loads((ROOT / "app/tauri.conf.json").read_text())["version"]
    if args.tag != f"v{version}" or not re.fullmatch(r"(?:0|[1-9]\d*)(?:\.(?:0|[1-9]\d*)){2}", version):
        parser.error(f"expected a stable tag matching packaged version {version}")

    # A wrong CI secret would otherwise generate valid signatures that every installed app rejects.
    with tempfile.TemporaryDirectory(dir=args.dist) as temporary:
        stage = Path(temporary)
        public_key = stage / "updater.pub"
        public_key.write_bytes(base64.b64decode((ROOT / "app/updater.pub").read_bytes().strip(), validate=True))
        decoded_signature = stage / "payload.minisig"
        platforms = {}
        for platform in dict.fromkeys(args.platforms):
            filename = f"Hen-Local-Player-{version}-{PAYLOADS[platform]}"
            artifact = args.dist / filename
            if not artifact.is_file() or not artifact.stat().st_size:
                parser.error(f"missing package: {artifact}")
            # Sign in the staging directory so an incorrect key cannot replace working sidecars.
            os.link(artifact, stage / filename)
            subprocess.run(
                ["node", ROOT / "app/ui/node_modules/@tauri-apps/cli/tauri.js",
                 "signer", "sign", "--app-version", version, stage / filename], check=True,
                stdout=subprocess.DEVNULL,
            )
            signature = (stage / (filename + ".sig")).read_text().strip()
            decoded_signature.write_bytes(base64.b64decode(signature, validate=True))
            subprocess.run(["minisign", "-Vm", artifact, "-p", public_key, "-x", decoded_signature], check=True)
            checksum = hashlib.sha256()
            with artifact.open("rb") as stream:
                for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                    checksum.update(chunk)
            platforms[platform] = {
                "signature": signature,
                "url": f"https://github.com/{RELEASE_REPOSITORY}/releases/download/{args.tag}/{filename}",
                "size": artifact.stat().st_size,
                "sha256": checksum.hexdigest(),
            }

        manifest = {
            "version": version,
            "notes": args.notes.read_text(encoding="utf-8").strip() if args.notes else f"Hen Local Player {version}",
            "pub_date": datetime.now(timezone.utc).isoformat(timespec="seconds"),
            "platforms": platforms,
        }
        manifest_path = stage / "latest.json"
        manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        for sidecar in stage.glob("*.sig"):
            sidecar.replace(args.dist / sidecar.name)
        manifest_path.replace(args.dist / manifest_path.name)


if __name__ == "__main__":
    main()
