"""Check timing, text, formats, media decisions and cache reuse against generated speech."""

import argparse
import datetime
import json
import os
import platform
import subprocess
import sys
import tempfile
import time
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MEDIA = {
    "ja": ("mp4", "direct"),
    "fr": ("mkv", "remux"),
    "es": ("avi", "transcode"),
    "de": ("webm", "direct"),
    "zh": ("mov", "direct"),
    "ko_51": ("mkv", None),
    "en": ("m4a", None),
    "music": ("mp4", None),
    "long_en": ("mp4", None),
}


def normalized(s):
    return "".join(
        c for c in unicodedata.normalize("NFKC", s).casefold() if c.isalnum()
    )


def edit_distance(reference, actual):
    # Myers' bit-vector algorithm with arbitrary-width Python integers.
    if not reference:
        return len(actual)
    masks = {}
    for pos, char in enumerate(reference):
        masks[char] = masks.get(char, 0) | (1 << pos)
    plus, minus, distance = ~0, 0, len(reference)
    last = 1 << (len(reference) - 1)
    for char in actual:
        match = masks.get(char, 0)
        vertical = match | minus
        horizontal = (((match & plus) + plus) ^ plus) | match
        positive = minus | ~(horizontal | plus)
        negative = plus & horizontal
        distance += bool(positive & last) - bool(negative & last)
        positive = (positive << 1) | 1
        plus = (negative << 1) | ~(vertical | positive)
        minus = positive & vertical
    return distance


def run(args, env):
    p = subprocess.run(
        [str(a) for a in args],
        env=env,
        text=True,
        encoding="utf-8",
        capture_output=True,
    )
    if p.returncode:
        raise RuntimeError(f"{args[0]} exited {p.returncode}: {p.stderr[-1200:]}")
    return p.stdout


def validate(cli, ffmpeg, name, out, env):
    ext, plan = MEDIA[name]
    source = ROOT / "tests/fixtures" / f"fixture_{name}.{ext}"
    truth = json.loads(source.with_suffix(".truth.json").read_text())
    webview = "webview2" if platform.system() == "Windows" else "wkwebview"
    # HEVC support is runtime-dependent in WebView2. The CLI baseline assumes it is missing.
    if webview == "webview2" and name == "zh":
        plan = "transcode"
    probe = json.loads(run([cli, "probe", source, "--webview", webview], env))
    out.mkdir(exist_ok=True)
    args = [
        cli,
        "subtitle",
        source,
        "--to",
        "zh",
        "--formats",
        "srt,vtt,ass",
        "--json-progress",
        "--out-dir",
        out,
    ]
    started = time.monotonic()
    first = [json.loads(s) for s in run(args, env).splitlines()]
    wall = time.monotonic() - started
    before = time.monotonic()
    cached = [json.loads(s) for s in run(args, env).splitlines()]
    cached_seconds = time.monotonic() - before
    summary = first[-1]
    cues = {}
    for event in first:
        if event["event"] == "media://opened":
            cues = {c["id"]: c for c in event["payload"]["cues"]}
        if event["event"] == "pipeline://cues":
            for key in event["payload"]["removedIds"]:
                cues.pop(key, None)
            for c in event["payload"]["upserts"]:
                cues[c["id"]] = c
    cues = sorted(cues.values(), key=lambda c: c["startMs"])
    text = normalized("".join(row["text"] for row in truth["sentences"]))
    actual = normalized("".join(c["original"] for c in cues))
    cer = edit_distance(text, actual) / max(1, len(text))
    onset_error = max(
        (
            min((abs(c["startMs"] - row["onsetMs"]) for c in cues), default=10**9)
            for row in truth["sentences"]
        ),
        default=0,
    )
    assert summary["event"] == "summary" and cached[-1]["event"] == "summary"
    checks = {
        "language": summary["detected"] == truth["language"],
        "characterErrorRate": cer <= 0.15,
        "onsets": onset_error <= 600,
        "positiveNonoverlapping": all(
            c["startMs"] < c["endMs"]
            and (not i or cues[i - 1]["endMs"] <= c["startMs"])
            for i, c in enumerate(cues)
        ),
        "translation": all(
            (
                (not c["translation"])
                if truth["language"] == "zh"
                else bool(c["translation"]) and c["translation"] != c["original"]
            )
            for c in cues
        ),
        "music": name != "music" or not cues,
        "cache": cached_seconds < 2
        and cached[-1]["asrRequests"] == 0
        and cached[-1]["translationRequests"] == 0,
        "playbackPlan": not plan or probe["plan"] == plan,
    }
    files = list(out.glob("*"))
    checks["exports"] = {p.suffix for p in files} == {".srt", ".vtt", ".ass"}
    if cues:
        # FFmpeg's null muxer does not select subtitle streams. Transcoding to
        # WebVTT explicitly decodes each format, including all dialogue packets.
        for path in files:
            decoded = run(
                [
                    ffmpeg,
                    "-nostdin",
                    "-v",
                    "error",
                    "-i",
                    path,
                    "-map",
                    "0:s:0",
                    "-c:s",
                    "webvtt",
                    "-f",
                    "webvtt",
                    "-",
                ],
                env,
            )
            checks["exports"] &= "-->" in decoded
    # The output directory belongs to this temporary test run. Keep portable file names in evidence.
    summary["outputs"] = [Path(p).name for p in summary["outputs"]]
    return dict(
        checks=checks,
        wallSeconds=round(wall, 3),
        cachedSeconds=round(cached_seconds, 3),
        cer=round(cer, 4),
        maxOnsetErrorMs=onset_error,
        cues=len(cues),
        summary=summary,
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--ffmpeg", required=True, type=Path)
    parser.add_argument("fixtures", nargs="*")
    args = parser.parse_args()
    date = datetime.date.today().isoformat()
    report = f"{date}-{platform.system().lower()}"
    if args.fixtures:
        unknown = set(args.fixtures) - MEDIA.keys()
        if unknown:
            parser.error("Unknown fixtures: " + ", ".join(sorted(unknown)))
        report += "-" + "-".join(args.fixtures)
    result_file = ROOT / "tests/results" / f"{report}.json"
    result_file.parent.mkdir(parents=True, exist_ok=True)
    results = {}
    with tempfile.TemporaryDirectory(prefix="hlp-e2e-") as td:
        base = Path(td)
        env = {**os.environ, "HLP_DATA_DIR": str(base / "data")}
        for fixture in args.fixtures or MEDIA:
            try:
                results[fixture] = validate(
                    args.cli, args.ffmpeg, fixture, base / fixture, env
                )
            except (
                OSError,
                KeyError,
                ValueError,
                RuntimeError,
                AssertionError,
            ) as error:
                results[fixture] = {"error": str(error)}
            print(
                f"{fixture}: {json.dumps(results[fixture],ensure_ascii=False)}",
                flush=True,
            )
            result_file.write_text(
                json.dumps(results, indent=2, ensure_ascii=False) + "\n"
            )
    return (
        0
        if all("checks" in r and all(r["checks"].values()) for r in results.values())
        else 1
    )


if __name__ == "__main__":
    sys.exit(main())
