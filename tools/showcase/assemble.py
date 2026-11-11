"""Assemble the showcase mp4: a key-art title card, the captioned reel, an end card.

The reel comes from `cargo run -p sf-host --release --example showcase -- <folder>`;
the cards use the README's key-art banner. Needs ffmpeg and ffprobe on PATH.

python -I tools/showcase/assemble.py <showcase folder> <out mp4> [crf]

docs/media/showcase.mp4 was made with crf 20 (60 s, under 4 MB).
"""
import subprocess
import sys
from pathlib import Path

folder, out = Path(sys.argv[1]), sys.argv[2]
crf = sys.argv[3] if len(sys.argv) > 3 else "20"
banner = str(Path(__file__).resolve().parents[2] / "docs" / "media" / "key-art-illustrated-banner.png")
FONT = "C\\:/Windows/Fonts/segoeuib.ttf"
TITLE_S, END_S, FADE = 3.5, 4.5, 0.4


def esc(text):
    return text.replace("\\", "\\\\").replace(":", "\\:").replace("'", "’").replace(",", "\\,")


frames = int(subprocess.run(
    ["ffprobe", "-v", "error", "-count_packets", "-select_streams", "v", "-show_entries", "stream=nb_read_packets",
     "-of", "csv=p=0", str(folder / "reel.mkv")], capture_output=True, text=True, check=True).stdout.strip())
reel_s = frames / 60
scenes = []
for line in (folder / "scenes.txt").read_text(encoding="utf-8").splitlines():
    t, text = line.split("\t", 1)
    scenes.append((float(t), text))

captions = []
for i, (t, text) in enumerate(scenes):
    end = scenes[i + 1][0] if i + 1 < len(scenes) else reel_s
    end = min(end, t + 4.0)
    alpha = f"if(lt(t-{t:.3f},0.3),(t-{t:.3f})/0.3,if(gt(t,{end - 0.3:.3f}),({end:.3f}-t)/0.3,1))"
    captions.append(
        f"drawtext=fontfile='{FONT}':text='{esc(text)}':fontsize=20:fontcolor=white:alpha='{alpha}'"
        f":box=1:boxcolor=black@0.65:boxborderw=8:x=(w-text_w)/2:y=h-44:enable='between(t,{t:.3f},{end:.3f})'"
    )

card = "scale=768:384:flags=lanczos,pad=768:448:0:0:black"
title = (f"[1:v]loop=-1:1,fps=60,trim=duration={TITLE_S},{card},"
         f"drawtext=fontfile='{FONT}':text='{esc('STREET FIGHTER: THE WORLD TOURNAMENT')}':fontsize=26:fontcolor=white:x=(w-text_w)/2:y=398,"
         f"fade=in:0:d={FADE},fade=out:st={TITLE_S - FADE}:d={FADE},setsar=1,format=yuv420p[t]")
end = (f"[2:v]loop=-1:1,fps=60,trim=duration={END_S},{card},"
       f"drawtext=fontfile='{FONT}':text='{esc('A fan port of the 1987 arcade game, rebuilt in Rust')}':fontsize=20:fontcolor=white:x=(w-text_w)/2:y=392,"
       f"drawtext=fontfile='{FONT}':text='{esc('Fan-made, not affiliated with or endorsed by Capcom.')}':fontsize=14:fontcolor=0xbbbbbb:x=(w-text_w)/2:y=422,"
       f"fade=in:0:d={FADE},fade=out:st={END_S - FADE}:d={FADE},setsar=1,format=yuv420p[e]")
reel = (f"[0:v]{','.join(captions)},fade=in:0:d=0.25,fade=out:st={reel_s - 0.3:.3f}:d=0.3,setsar=1,format=yuv420p[r]")
graph = ";".join([title, reel, end, "[t][r][e]concat=n=3:v=1:a=0[v]"])

subprocess.run(
    ["ffmpeg", "-y", "-v", "error", "-i", str(folder / "reel.mkv"), "-i", banner, "-i", banner,
     "-filter_complex", graph, "-map", "[v]", "-an", "-c:v", "libx264", "-preset", "slower", "-crf", crf,
     "-tune", "animation", "-profile:v", "high", "-pix_fmt", "yuv420p", "-movflags", "+faststart", out],
    check=True)
print(f"reel {reel_s:.1f} s, total {TITLE_S + reel_s + END_S:.1f} s, {Path(out).stat().st_size / 1e6:.2f} MB")
