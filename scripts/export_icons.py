#!/usr/bin/env python3
"""Regenerate player artwork on macOS with Xcode, rsvg-convert and Pillow."""

from io import BytesIO
from pathlib import Path
import shutil
import subprocess
from tempfile import TemporaryDirectory
from xml.etree import ElementTree

from PIL import Image, ImageCms


ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "app/icons"
DOCUMENT = ICONS / "AppIcon.icon"
SIZES = (16, 24, 32, 48, 64, 128, 256, 512, 1024)


def compact_document(document, size):
    # At 16 and 24 px the gaps and pause bars need separate pixel columns.
    # Keep the baseline and right margin aligned with the master artwork.
    assets = document / "Assets"
    playhead = ElementTree.parse(assets / "Playhead.svg").getroot()[0]
    rail = ElementTree.parse(assets / "Remaining.svg").getroot()[0]
    scale = size / 1024
    axis = float(playhead.get("cy")) * scale
    radius = float(rail.get("rx")) * scale
    right = (float(rail.get("x")) + float(rail.get("width"))) * scale
    if size == 16:
        left, tip, first, second, bar_width, half_height, start = 2, 3.8, 4, 6, 1, 1.35, 8
    else:
        left, tip, first, second, bar_width, half_height, start = 3, 5.6, 6, 8, 1.15, 1.55, 10.5

    def write(name, shapes):
        (assets / f"{name}.svg").write_text(
            f'<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" '
            f'viewBox="0 0 {size} {size}">{shapes}</svg>\n'
        )

    ink = ElementTree.parse(assets / "PlayPause.svg").getroot().get("fill")
    triangle = f'<path d="M{left} {axis-half_height}L{tip} {axis}L{left} {axis+half_height}Z"/>'
    bars = "".join(
        f'<rect x="{x}" y="{axis-half_height}" width="{bar_width}" height="{2*half_height}" rx=".18"/>'
        for x in (first, second)
    )
    write("PlayPause", f'<g fill="{ink}">{triangle}{bars}</g>')
    center = (start + right) / 2
    for name, end in (("Remaining", right), ("Elapsed", center)):
        color = ElementTree.parse(assets / f"{name}.svg").getroot()[0].get("fill")
        write(name, f'<rect x="{start}" y="{axis-radius}" width="{end-start}" height="{2*radius}" rx="{radius}" fill="{color}"/>')
    head_radius = float(playhead.get("r")) * scale
    write("Playhead", f'<circle cx="{center}" cy="{axis}" r="{head_radius}" fill="{playhead.get("fill")}"/>')


def export(out):
    developer = Path(subprocess.check_output(["xcode-select", "-p"], text=True).strip())
    compiler = developer / "usr/bin/actool"
    renderer = developer.parent / "Applications/Icon Composer.app/Contents/Executables/ictool"
    subprocess.run([
        compiler, DOCUMENT, "--compile", out, "--output-partial-info-plist", out / "partial.plist",
        "--app-icon", "AppIcon", "--include-all-app-icons", "--enable-on-demand-resources", "NO",
        "--target-device", "mac", "--minimum-deployment-target", "14.0", "--platform", "macosx",
    ], check=True)
    srgb = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB"))
    for size in SIZES:
        document = DOCUMENT
        if size < 32:
            document = out / f"Small-{size}.icon"
            shutil.copytree(DOCUMENT, document)
            compact_document(document, size)
        path = out / f"{size}.png"
        subprocess.run([
            renderer, document, "--export-image", "--output-file", path,
            "--platform", "macOS", "--rendition", "Default", "--width", str(size),
            "--height", str(size), "--scale", "1", "--design-generation", "26",
        ], check=True)
        with Image.open(path) as rendered:
            source = ImageCms.ImageCmsProfile(BytesIO(rendered.info["icc_profile"]))
            image = ImageCms.profileToProfile(rendered, source, srgb, outputMode="RGBA")
        image.info.clear()
        image.save(path, optimize=True, icc_profile=srgb.tobytes())

    iconset = out / "AppIcon.iconset"
    iconset.mkdir()
    for size in (16, 32, 128, 256, 512):
        shutil.copyfile(out / f"{size}.png", iconset / f"icon_{size}x{size}.png")
        shutil.copyfile(out / f"{2*size}.png", iconset / f"icon_{size}x{size}@2x.png")
    subprocess.run(["iconutil", "-c", "icns", "-o", out / "icon.icns", iconset], check=True)
    ico_sizes = SIZES[:7]
    frames = [Image.open(out / f"{n}.png") for n in ico_sizes]
    frames[-1].save(out / "icon.ico", sizes=[(n, n) for n in ico_sizes], append_images=frames[:-1])
    subprocess.run(["rsvg-convert", ICONS / "Welcome.svg", "-o", out / "foreground.png"], check=True)

    for name in ("Assets.car", "icon.icns", "icon.ico"):
        shutil.copyfile(out / name, ICONS / name)
    for size, name in ((32, "32x32.png"), (128, "128x128.png"), (256, "128x128@2x.png"), (1024, "icon.png")):
        shutil.copyfile(out / f"{size}.png", ICONS / name)
    shutil.copyfile(out / "32.png", ROOT / "app/ui/public/hen-local-player.png")
    shutil.copyfile(out / "foreground.png", ROOT / "app/ui/src/assets/hen-player-clear.png")


if __name__ == "__main__":
    with TemporaryDirectory(prefix="hen-player-icons-") as temporary:
        export(Path(temporary))
