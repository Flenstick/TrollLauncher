#!/usr/bin/env python3
"""Renders the GitHub banner, release covers and framed screenshots.

    python3 design/github/render.py            # everything
    python3 design/github/render.py 1.1.1      # one release cover

Screenshots come from design/github/shots/*.png (captures of the launcher
window). Release covers are described in releases.json. Output goes to
.github/media/. Needs chromium.
"""

import html
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
SHOTS = HERE / "shots"
MEDIA = ROOT / ".github" / "media"
MARK = (ROOT / "design" / "logo" / "mark.svg").read_text()

BASE_CSS = """
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body { width: 100%; height: 100%; }
body {
	font-family: 'Golos Text', 'Inter', sans-serif;
	color: #f4f4f6;
	-webkit-font-smoothing: antialiased;
	overflow: hidden;
}
.canvas {
	position: relative; width: 100%; height: 100%; overflow: hidden;
	background:
		radial-gradient(900px 600px at 82% 78%, rgba(229, 72, 77, .30), transparent 70%),
		radial-gradient(700px 500px at 0% 0%, rgba(229, 72, 77, .10), transparent 70%),
		linear-gradient(160deg, #141418 0%, #0b0b0e 100%);
}
.canvas::before {
	content: ''; position: absolute; inset: 0;
	background-image: radial-gradient(rgba(255,255,255,.07) 1px, transparent 1.4px);
	background-size: 28px 28px;
	mask-image: linear-gradient(110deg, transparent 35%, black 75%);
}
.brand { display: flex; align-items: center; gap: 14px; font-weight: 700; letter-spacing: -.01em; }
.brand svg { display: block; }
.window {
	position: absolute; border-radius: 18px; overflow: hidden;
	border: 1.5px solid rgba(255,255,255,.10);
	box-shadow: 0 40px 120px rgba(0,0,0,.65), 0 0 0 1px rgba(0,0,0,.6), 0 0 140px rgba(229,72,77,.25);
	background: #0d0d10;
}
.window img { display: block; width: 100%; }
.glow {
	position: absolute; width: 700px; height: 500px; border-radius: 50%;
	background: rgba(229, 72, 77, .45); filter: blur(120px);
}
.character { position: absolute; filter: drop-shadow(0 30px 40px rgba(0,0,0,.6)); }
.ground {
	position: absolute; width: 420px; transform: translateX(-60px);
	background: radial-gradient(50% 10% at 50% 96%, rgba(229,72,77,.55), transparent 100%);
}
.chip {
	display: inline-flex; align-items: center; gap: 8px;
	padding: 9px 16px; border-radius: 999px; font-size: 18px; font-weight: 600;
	background: rgba(255,255,255,.06); border: 1px solid rgba(255,255,255,.10); color: #e6e6ea;
}
.chip.red { background: #e5484d; border-color: #e5484d; color: #fff; }
"""


def mark(size: int) -> str:
    return MARK.replace("<svg ", f'<svg width="{size}" height="{size}" ', 1)


def size(name: str) -> tuple[int, int]:
    out = subprocess.run(
        ["magick", "identify", "-format", "%w %h", str(SHOTS / f"{name}.png")],
        check=True, capture_output=True, text=True,
    ).stdout.split()
    return int(out[0]), int(out[1])


def character(pose: str, height: int, style: str) -> str:
    src = (HERE / "characters" / f"{pose}.png").as_uri()
    return (
        f'<div class="ground" style="{style}; height:{height}px"></div>'
        f'<img class="character" src="{src}" style="{style}; height:{height}px">'
    )


def shot(name: str) -> str:
    return (SHOTS / f"{name}.png").as_uri()


def window(name: str, width: int, style: str, crop: list[int] | None = None) -> str:
    """A screenshot in a window frame; `crop` = [x, y, w, h] in screenshot pixels."""
    full_width, full_height = size(name)
    x, y, w, h = crop or [0, 0, full_width, full_height]
    scale = width / w
    return (
        f'<div class="window" style="width:{width}px; height:{round(h * scale)}px; {style}">'
        f'<img src="{shot(name)}" style="width:{round(full_width * scale)}px; max-width:none;'
        f' margin-left:{-round(x * scale)}px; margin-top:{-round(y * scale)}px"></div>'
    )


def render(page: str, out: Path, width: int, height: int, transparent: bool = False) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", suffix=".html", delete=False) as file:
        file.write(f"<!doctype html><meta charset=utf-8><style>{BASE_CSS}</style>{page}")
        path = file.name
    args = [
        "chromium", "--headless=new", "--no-sandbox", "--hide-scrollbars",
        "--force-device-scale-factor=1", "--allow-file-access-from-files",
        f"--window-size={width},{height}", f"--screenshot={out}",
        "--virtual-time-budget=3000",
    ]
    if transparent:
        args.append("--default-background-color=00000000")
    subprocess.run([*args, f"file://{path}"], check=True, capture_output=True)
    Path(path).unlink()
    print("rendered", out.relative_to(ROOT))


def banner() -> None:
    page = f"""
<div class="canvas">
	<div style="position:absolute; left:72px; top:86px; width:660px">
		<div class="brand" style="font-size:22px; color:#ffb4b8; letter-spacing:.18em; text-transform:uppercase">
			Minecraft лаунчер
		</div>
		<div class="brand" style="margin-top:18px; gap:18px">
			{mark(84)}
			<span style="font-size:78px; letter-spacing:-.04em">TrollLauncher</span>
		</div>
		<p style="margin-top:22px; font-size:28px; line-height:1.35; color:#c9c9d1">
			Сборки и моды, библиотека скинов, вход через Ely.by
			и&nbsp;интерфейс, который настраивается под тебя.
		</p>
		<div style="margin-top:34px; display:flex; gap:12px; flex-wrap:wrap">
			<span class="chip red">Arch Linux</span>
			<span class="chip">Ely.by</span>
			<span class="chip">Автообновление</span>
			<span class="chip">Открытый код</span>
		</div>
	</div>
	<div class="glow" style="left:900px; top:180px"></div>
	{window('instance', 980, "left:720px; top:70px; transform: perspective(1800px) rotateY(-17deg) rotateX(6deg) rotateZ(1deg)", [0, 0, 1889, 1010])}
	{character('wave', 560, 'left:1230px; top:70px')}
</div>"""
    render(page, MEDIA / "banner.png", 1600, 640)


def cover(release: dict) -> None:
    features = "".join(
        f'<li><span class="dot"></span>{html.escape(item)}</li>' for item in release["features"]
    )
    page = f"""
<style>
	ul {{ list-style: none; display: flex; flex-direction: column; gap: 14px; margin-top: 34px; }}
	li {{ display: flex; align-items: center; gap: 14px; font-size: 26px; color: #d9d9df; font-weight: 500; }}
	.dot {{ width: 10px; height: 10px; border-radius: 3px; background: #e5484d; flex: none;
		box-shadow: 0 0 14px rgba(229,72,77,.9); }}
</style>
<div class="canvas">
	<div style="position:absolute; left:72px; top:64px; width:600px">
		<div class="brand" style="font-size:26px">{mark(46)} TrollLauncher</div>
		<div style="margin-top:34px; font-size:150px; font-weight:800; line-height:.9; letter-spacing:-.05em;
			background: linear-gradient(180deg, #ffffff 30%, #ff9ca0); -webkit-background-clip:text; color:transparent">
			{html.escape(release['version'])}
		</div>
		<div style="margin-top:22px; font-size:36px; font-weight:700; line-height:1.15; letter-spacing:-.02em">
			{html.escape(release['title'])}
		</div>
		<ul>{features}</ul>
	</div>
	<div class="glow" style="left:880px; top:300px"></div>
	{window(release['shot'], release.get('width', 1060), f"left:{release.get('left', 680)}px; top:{release.get('top', 120)}px; transform: perspective(2000px) rotateY(-18deg) rotateX(6deg) rotateZ(1.5deg)", release.get('crop'))}
	{character(release.get('pose', 'idle'), 620, 'left:1250px; top:236px')}
</div>"""
    render(page, MEDIA / "releases" / f"{release['version']}.png", 1600, 900)


def framed(name: str) -> None:
    page = f"""
<div style="padding:40px 56px 72px">
	<div class="window" style="position:relative; box-shadow: 0 30px 60px rgba(0,0,0,.45), 0 0 0 1px rgba(0,0,0,.5)">
		<img src="{shot(name)}">
	</div>
</div>"""
    width, height = size(name)
    render(page, MEDIA / "screens" / f"{name}.png", 1920, round(height * 1808 / width) + 112, transparent=True)


def main() -> None:
    releases = json.loads((HERE / "releases.json").read_text())
    only = sys.argv[1:]
    if only:
        for release in releases:
            if release["version"] in only:
                cover(release)
        return
    banner()
    for release in releases:
        cover(release)
    for path in sorted(SHOTS.glob("*.png")):
        framed(path.stem)


if __name__ == "__main__":
    main()
