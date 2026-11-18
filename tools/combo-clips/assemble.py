"""Assemble the combo clips: one captioned, silent MP4 per fighter, in parts of at most 4 MB.

The clips come from `cargo run -p sf-host --release --example combo_clips -- <folder>`,
which writes one lossless `<slug>.mkv` per fighter and era style plus `results.tsv`.

python -I tools/combo-clips/assemble.py <clips folder> <out folder> [crf]

Each clip is encoded on its own (768x448, 60 fps, H.264, no audio) with a caption
naming the fighter, the style and the count, then the clips of a fighter are joined
without re-encoding into parts that stay under the size limit. Writes the videos and
`INDEX.md` into the out folder.
"""

import shutil
import subprocess
import sys
from pathlib import Path

LIMIT = 4 * 1000 * 1000  # bytes; under 4 MB by either reading of "MB"
ROOM = 40_000  # left for the container when clips are joined
LEAD_CUT = 12  # frames dropped from each clip's 30-frame lead-in
FIGHTERS = ["Ryu", "Ken", "Retsu", "Geki", "Joe", "Mike", "Lee", "Gen", "Birdie", "Eagle", "Adon", "Sagat"]
REVISIONS = [
    "1987", "SF1+", "SF2 WW", "SF2 CE", "SF2 HF", "SF2 Super", "SF2 ST", "SFZ1", "SFZ2", "SFZ2A", "SFZ3",
    "SF3 NG", "SF3 2I", "SF3 3S", "SF4", "SSF4", "AE2012", "USF4", "SF5", "SF5 AE", "SF5 CE", "SF6",
]
FONT = Path("C:/Windows/Fonts/consolab.ttf")


def rows(folder):
    """The last line of results.tsv for each pair, in file order of first appearance."""
    last, order = {}, []
    for line in (folder / "results.tsv").read_text(encoding="utf-8").splitlines():
        cells = line.split("\t")
        if len(cells) < 10:
            continue
        slug = cells[0]
        if slug not in last:
            order.append(slug)
        # A pair once recorded stays recorded.
        if slug in last and last[slug]["status"] == "ok":
            continue
        last[slug] = {
            "slug": slug, "fighter": cells[1], "revision": cells[2], "loadout": cells[3], "kind": cells[4],
            "status": cells[5], "hits": int(cells[6]), "frames": int(cells[7]), "route": cells[9],
            "stance": cells[10] if len(cells) > 11 else "crouching",
            "best": max(int(cells[6]), last.get(slug, {}).get("best", 0)),
        }
    return [last[slug] for slug in order]


def style(row):
    return row["revision"] if row["loadout"] == "Standard" else f'{row["revision"]} {row["loadout"]}'


def probe(path, entries):
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", entries, "-of", "csv=p=0", str(path)],
        capture_output=True, text=True, check=True,
    )
    return [line for line in out.stdout.splitlines() if line.strip()]


def encode(src, dst, caption, crf, work):
    text = work / "caption.txt"
    text.write_text(caption, encoding="utf-8")
    draw = (
        "drawtext=fontfile=font.ttf:textfile=caption.txt:fontsize=16:fontcolor=white:"
        "box=1:boxcolor=black@0.6:boxborderw=5:x=(w-text_w)/2:y=136"
    )
    subprocess.run(
        [
            "ffmpeg", "-y", "-loglevel", "error", "-i", str(src.resolve()), "-an",
            "-vf", f"trim=start_frame={LEAD_CUT},setpts=PTS-STARTPTS,scale=768:448:flags=neighbor,{draw}",
            "-c:v", "libx264", "-preset", "veryslow", "-crf", str(crf), "-pix_fmt", "yuv420p",
            "-r", "60", "-video_track_timescale", "15360", str(dst.resolve()),
        ],
        check=True, cwd=work,
    )


def join(parts, dst, work):
    listing = work / "list.txt"
    listing.write_text("".join(f"file '{p.resolve().as_posix()}'\n" for p in parts), encoding="utf-8")
    subprocess.run(
        ["ffmpeg", "-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", str(listing), "-an",
         "-c", "copy", "-movflags", "+faststart", str(dst)],
        check=True,
    )


def main():
    clips, out = Path(sys.argv[1]), Path(sys.argv[2])
    crf = int(sys.argv[3]) if len(sys.argv) > 3 else 23
    work = out / ".work"
    work.mkdir(parents=True, exist_ok=True)
    shutil.copy(FONT, work / "font.ttf")
    for old in out.glob("*.mp4"):
        old.unlink()

    table = rows(clips)
    table.sort(key=lambda r: (FIGHTERS.index(r["fighter"]), REVISIONS.index(r["revision"])))
    index = [
        "# Combo clips",
        "",
        "Silent clips of a combo of five or more hits, counted by the game's own combo counter, for each",
        "fighter in each era style. Made by `crates/sf-host/examples/combo_clips.rs` (search and lossless",
        "recording) and `tools/combo-clips/assemble.py` (captions, grouping, encoding).",
        "",
        "How they were recorded: a Training fight under Definitive Overhaul with FILL METERS on, P1's pad",
        "scripted, every frame drawn by the game. The opponent (Ken, or the fighter's default pick) holds",
        "Down or stands still as the table says: the recorder switches the Training fight's kind to a",
        "two-pad fight so that the second pad can hold Down, which the Training dummy cannot do. Routes",
        "were found by an automatic search, so they are routes that work, not hand-picked showpieces.",
        "",
        "Route notation: numbers are numpad directions (236 = quarter circle forward), `[4]6` a charge,",
        "`d,d` Down twice, PP/KK/PPP/KKK two or three punches or kicks together.",
        "",
    ]
    videos = []
    for fighter in FIGHTERS:
        mine = [r for r in table if r["fighter"] == fighter and r["status"] == "ok"]
        encoded = []
        for row in mine:
            src = clips / f'{row["slug"]}.mkv'
            dst = work / f'{row["slug"]}.mp4'
            caption = f'{fighter.upper()}  {style(row)}  {row["hits"]} HITS'
            encode(src, dst, caption, crf, work)
            size = dst.stat().st_size
            if size > LIMIT - ROOM:
                raise SystemExit(f"{dst.name} alone is {size} bytes: raise the crf")
            encoded.append((row, dst, size))
        groups, current, total = [], [], 0
        for item in encoded:
            if current and total + item[2] > LIMIT - ROOM:
                groups.append(current)
                current, total = [], 0
            current.append(item)
            total += item[2]
        if current:
            groups.append(current)
        for n, group in enumerate(groups, 1):
            name = f"{fighter.lower()}-{n:02}.mp4" if len(groups) > 1 else f"{fighter.lower()}.mp4"
            join([dst for _, dst, _ in group], out / name, work)
            videos.append((name, group))

    index += ["## Videos", ""]
    for name, group in videos:
        path = out / name
        size = path.stat().st_size
        streams = probe(path, "stream=codec_type")
        frames = sum(int(x) for x in probe(path, "stream=nb_frames"))
        if size > LIMIT or streams != ["video"]:
            raise SystemExit(f"{name}: {size} bytes, streams {streams}")
        index += [
            f"### {name}",
            "",
            f"{size:,} bytes ({size / 1_000_000:.2f} MB), {frames / 60:.1f} s, {len(group)} combos, video stream only.",
            "",
            "| Fighter | Era style | Kind | Hits | Opponent | Route |",
            "|---|---|---|---|---|---|",
        ]
        for row, _, _ in group:
            index.append(
                f'| {row["fighter"]} | {style(row)} | {row["kind"]} | {row["hits"]} | {row["stance"]} | {row["route"]} |'
            )
        index.append("")

    left = [r for r in table if r["status"] != "ok"]
    index += [
        "## Left out",
        "",
        f"{len(left)} of {len(table)} pairs have no clip.",
        "",
        "| Fighter | Era style | Kind | Best count found | Why |",
        "|---|---|---|---|---|",
    ]
    for row in left:
        if row["revision"] == "1987":
            why = "The SF1 1987 style has no true combos or cancels: a hit fighter cannot be hit again, so the counter never passes 1."
        elif row["status"] == "replay-short":
            why = "The search found a route on the bare machine that did not reach five when replayed through the fight screen."
        else:
            why = "The automatic search found no route to five; not proof that none exists."
        index.append(f'| {row["fighter"]} | {style(row)} | {row["kind"]} | {row["best"]} | {why} |')
    index.append("")
    (out / "INDEX.md").write_text("\n".join(index), encoding="utf-8")
    shutil.rmtree(work)
    total = sum((out / name).stat().st_size for name, _ in videos)
    print(f"{len(videos)} videos, {sum(len(g) for _, g in videos)} combos, {total:,} bytes; {len(left)} pairs left out")


if __name__ == "__main__":
    main()
