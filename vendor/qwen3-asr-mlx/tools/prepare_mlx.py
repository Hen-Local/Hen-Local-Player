#!/usr/bin/env python3
"""Prepare the pinned MLX native libraries for the player's macOS 14 deployment target.

Print the completed MLX_PREBUILT_PATH to stdout. Build diagnostics go to stderr.
Requires CMake 3.25+, Apple Silicon, an installed macOS SDK and Python 3.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import zipfile

# All build inputs are pinned independently of the user's model installation.
ASSETS = {
    "mlx": ("https://codeload.github.com/ml-explore/mlx/tar.gz/c215b6f88cf0fee0b0895623e4046cda797ef397",
            3238821, "7ff170579ad8a12e938908c25aefcdd3aa903534865532502817083fb6bc5c8a"),
    "fmt": ("https://codeload.github.com/fmtlib/fmt/tar.gz/e69e5f977d458f2650bb346dadf2ad30c5320281",
            855053, "1723f27eed50e751037f49dcdf73e33b17658f1178ea1c1f829a30bb02335745"),
    "gguflib": ("https://codeload.github.com/antirez/gguf-tools/tar.gz/8fa6eb65236618e28fd7710a0fba565f7faa1848",
                34394, "9e30bc1eb82cc2231150d39ce37dcdd6f844d6994fba18da83fc537a487ba86f"),
    "json": ("https://github.com/nlohmann/json/releases/download/v3.11.3/json.tar.xz",
             110988, "d6c65aca6b1ed68e7a182f4757257b107ae403032760ed6ef121c9d55e81757d"),
    "metal_cpp": ("https://developer.apple.com/metal/cpp/files/metal-cpp_26.zip",
                  260405, "4df3c078b9aadcb516212e9cb03004cbc5ce9a3e9c068fa3144d021db585a3a4"),
    "kernels": ("https://files.pythonhosted.org/packages/09/3f/0be35ddad7e13d8ecd33a9185895f9739bb00b96ef0cce36cf0405d4aec0/mlx_metal-0.30.1-py3-none-macosx_14_0_arm64.whl",
                36864966, "e7e92c6bdbd7ac8083f528a4c6640552ae106a57bb3d99856ac10a32e93a4b5e"),
}
FILES = ("libmlx.a", "libmlxc.a", "libgguflib.a", "mlx.metallib")


def run(*args):
    subprocess.run(args, stdout=sys.stderr, check=True)


def fetch(name, directory):
    url, size, sha = ASSETS[name]
    asset = directory / name
    if not asset.is_file() or asset.stat().st_size != size:
        part = asset.with_suffix(".part")
        run("curl", "-fsSL", "--retry", "3", "-o", part, url)
        part.replace(asset)
    if asset.stat().st_size != size or hashlib.sha256(asset.read_bytes()).hexdigest() != sha:
        asset.unlink()
        raise ValueError(f"Checksum or size mismatch for {name}")
    return asset


def prepare(target):
    destination = target / "mlx-0.30.1-macos14-v1"
    if all((destination / name).is_file() for name in FILES):
        return destination

    # Cargo.lock pins the C API at the same OminiX commit as the Rust bindings.
    root = Path(__file__).resolve().parents[3]
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--locked", "--manifest-path", root / "Cargo.toml",
        "--format-version", "1", "--filter-platform", "aarch64-apple-darwin",
    ]))
    wrapper = next(Path(p["manifest_path"]).parent / "src/mlx-c" for p in metadata["packages"]
                   if p["name"] == "mlx-sys" and p["source"].endswith("#6aac996db8b71fb7dae7a2409c46b4f2ade93092"))
    downloads = target / "hlp-native-downloads"
    downloads.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=target, prefix="mlx-src-") as tmp:
        source = Path(tmp)
        cmake_args = []
        for name in ("mlx", "fmt", "gguflib", "json", "metal_cpp"):
            unpacked = source / name
            unpacked.mkdir()
            run("tar", "-xf", fetch(name, downloads), "-C", unpacked, "--strip-components", "1")
            cmake_args.append(f"-DFETCHCONTENT_SOURCE_DIR_{name.upper()}={unpacked}")
        # The Metal kernels come from the official wheel of the same release.
        # Building the C/C++ pieces this way works with standalone Command Line
        # Tools, which do not contain the offline Metal shader compiler.
        metal = source / "mlx/mlx/backend/metal"
        kernels = metal / "kernels/CMakeLists.txt"
        kernels.write_text(kernels.read_text().replace("add_dependencies(mlx mlx-metallib)", ""))
        header_tool = source / "metal_headers.py"
        shutil.copyfile(Path(__file__).with_name("metal_headers.py"), header_tool)
        (metal / "make_compiled_preamble.sh").write_text(f'exec python3 "{header_tool}" "$@"\n')

        build = source / "build"
        run("cmake", "-S", wrapper, "-B", build, "-DCMAKE_BUILD_TYPE=Release",
            "-DCMAKE_OSX_DEPLOYMENT_TARGET=14.0", "-DMLX_BUILD_METAL=ON",
            "-DBUILD_SHARED_LIBS=OFF", "-DMLX_C_BUILD_EXAMPLES=OFF",
            "-DFETCHCONTENT_FULLY_DISCONNECTED=ON", *cmake_args)
        run("cmake", "--build", build, "--parallel", str(min(8, os.cpu_count() or 1)), "--target", "mlxc")
        ready = source / "prebuilt"
        ready.mkdir()
        for lib in FILES[:3]:
            matches = list(build.rglob(lib))
            if len(matches) != 1:
                raise RuntimeError(f"Expected one {lib}, found {len(matches)}")
            shutil.copyfile(matches[0], ready / lib)
        with zipfile.ZipFile(fetch("kernels", downloads)) as wheel:
            with wheel.open("mlx/lib/mlx.metallib") as library, (ready / "mlx.metallib").open("wb") as out:
                shutil.copyfileobj(library, out)
        ready.rename(destination)
    return destination


if __name__ == "__main__":
    target = Path(os.environ.get("CARGO_TARGET_DIR") or (Path(tempfile.gettempdir()) / "hen-local-player-target")).resolve()
    if " " in str(target):
        sys.exit("Choose a Cargo target directory without spaces for MLX.")
    target.mkdir(parents=True, exist_ok=True)
    os.environ["CARGO_TARGET_DIR"] = str(target)
    with (target / "hlp-native.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        print(prepare(target))
