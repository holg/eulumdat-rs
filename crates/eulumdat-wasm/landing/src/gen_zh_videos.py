#!/usr/bin/env python3
"""Writes landing/zh/index.html: the Chinese video tutorials, played from our own server.

YouTube cannot be reached from mainland China, so the Chinese versions are also served
from /assets/zh/ (uploaded by hand: NN_name.zh.mp4, not in the repository or the build).
This page lists them with posters (zh/NN_name.jpg), subtitles (zh/NN_name.vtt) and
chapters. Data: src/zh_videos.json (name, title, lead, seconds, chapters).

    python3 crates/eulumdat-wasm/landing/src/gen_zh_videos.py
"""
import html, json, os

HERE = os.path.dirname(os.path.abspath(__file__))
videos = json.load(open(os.path.join(HERE, "zh_videos.json"), encoding="utf-8"))
e = html.escape


def secs(mmss):
    m, s = mmss.split(":")
    return int(m) * 60 + int(s)


def card(v):
    n, num = v["name"], v["name"][:2]
    chapters = "".join(f'<li><button type="button" data-t="{secs(t)}">{e(t)}</button> {e(c)}</li>' for t, c in v["chapters"])
    return f"""
    <article class="card" id="v{num}">
      <video controls preload="none" playsinline poster="{n}.jpg">
        <source src="../assets/zh/{n}.zh.mp4" type="video/mp4">
        <track kind="subtitles" srclang="zh" label="中文" src="{n}.vtt" default>
      </video>
      <div class="body">
        <h2><span class="num">{num}</span>{e(v["title"])}</h2>
        <p>{e(v["lead"])}</p>
        <details><summary>章节 · {v["seconds"] // 60} 分 {v["seconds"] % 60:02d} 秒</summary><ol>{chapters}</ol></details>
      </div>
    </article>"""


def article():
    """src/zh_article.md (headings ##, numbered lists, **bold**, paragraphs) as HTML."""
    import re
    out, lst = [], False

    def inline(t):
        t = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", e(t, quote=False))
        return t.replace("GitHub 上提交 issue", '<a href="https://github.com/holg/eulumdat-rs/issues" rel="noopener">GitHub 上提交 issue</a>')
    for line in open(os.path.join(HERE, "zh_article.md"), encoding="utf-8").read().split("\n"):
        line = line.strip()
        m = re.match(r"\d+\. (.*)", line)
        if m:
            if not lst:
                out.append("<ol>")
                lst = True
            out.append(f"<li>{inline(m.group(1))}</li>")
            continue
        if lst:
            out.append("</ol>")
            lst = False
        if line.startswith("## "):
            out.append(f"<h3>{inline(line[3:])}</h3>")
        elif line:
            out.append(f"<p>{inline(line)}</p>")
    if lst:
        out.append("</ol>")
    return "\n      ".join(out)


ld = json.dumps({"@context": "https://schema.org", "@type": "ItemList", "itemListElement": [
    {"@type": "VideoObject", "position": i + 1, "name": v["title"], "description": v["lead"], "inLanguage": "zh-CN",
     "thumbnailUrl": f"https://candelabra.eu/zh/{v['name']}.jpg", "contentUrl": f"https://candelabra.eu/assets/zh/{v['name']}.zh.mp4",
     "duration": f"PT{v['seconds'] // 60}M{v['seconds'] % 60}S", "uploadDate": "2026-11-23"} for i, v in enumerate(videos)]}, ensure_ascii=False)

page = f"""<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Candelabra 视频教程（中文）– 浏览器中的照明设计与光度学工具</title>
<meta name="description" content="Candelabra 中文视频教程，共 {len(videos)} 集：打开 EULUMDAT、IES、TM-33 和 GLDF 配光文件，室内、道路与应急照明设计，传感器，光谱与颜色质量，黑视素照明，植物照明与温室。视频直接在本站播放。">
<link rel="canonical" href="https://candelabra.eu/zh/">
<meta property="og:title" content="Candelabra 视频教程（中文）">
<meta property="og:type" content="website">
<meta property="og:url" content="https://candelabra.eu/zh/">
<meta property="og:image" content="https://candelabra.eu/zh/{videos[0]['name']}.jpg">
<script type="application/ld+json">{ld}</script>
<style>
  :root {{ --bg: #f6f7f9; --surface: #ffffff; --text: #16202b; --muted: #5b6673; --line: #e3e7ec; --accent: #2563eb; --accent-ink: #ffffff; --tint: #eaf0ff; }}
  @media (prefers-color-scheme: dark) {{
    :root:not([data-theme="light"]) {{ --bg: #0e1116; --surface: #161b22; --text: #e6edf3; --muted: #9aa6b2; --line: #262d37; --accent: #5b8def; --accent-ink: #0b1020; --tint: #16213a; }}
  }}
  :root[data-theme="dark"] {{ --bg: #0e1116; --surface: #161b22; --text: #e6edf3; --muted: #9aa6b2; --line: #262d37; --accent: #5b8def; --accent-ink: #0b1020; --tint: #16213a; }}
  * {{ box-sizing: border-box; }}
  body {{ margin: 0; background: var(--bg); color: var(--text); font: 16px/1.65 -apple-system, BlinkMacSystemFont, "PingFang SC", "Microsoft YaHei", "Noto Sans SC", "Segoe UI", sans-serif; }}
  a {{ color: var(--accent); text-decoration: none; }}
  a:hover {{ text-decoration: underline; }}
  .wrap {{ max-width: 1120px; margin: 0 auto; padding: 0 16px; }}
  header {{ display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 10px; padding: 14px 0; }}
  .brand {{ font-weight: 700; font-size: 1.15rem; color: var(--text); }}
  .btn {{ display: inline-block; padding: 10px 18px; border-radius: 10px; font-weight: 600; border: 1px solid var(--line); background: var(--surface); color: var(--text); }}
  .btn.primary {{ background: var(--accent); color: var(--accent-ink); border-color: var(--accent); }}
  h1 {{ font-size: clamp(1.6rem, 4vw, 2.3rem); line-height: 1.25; margin: 26px 0 10px; }}
  .sub {{ color: var(--muted); max-width: 62em; margin: 0 0 8px; }}
  nav.index {{ display: flex; flex-wrap: wrap; gap: 6px; margin: 18px 0 26px; }}
  nav.index a {{ border: 1px solid var(--line); background: var(--surface); border-radius: 8px; padding: 3px 10px; font-size: .9rem; color: var(--text); }}
  .grid {{ display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 460px), 1fr)); gap: 22px; }}
  .card {{ background: var(--surface); border: 1px solid var(--line); border-radius: 14px; overflow: hidden; scroll-margin-top: 12px; }}
  .card video {{ display: block; width: 100%; aspect-ratio: 16 / 9; background: #000; }}
  .card .body {{ padding: 14px 16px 16px; }}
  .card h2 {{ font-size: 1.05rem; line-height: 1.4; margin: 0 0 6px; }}
  .num {{ display: inline-block; min-width: 1.9em; margin-right: 8px; padding: 0 6px; border-radius: 6px; background: var(--tint); color: var(--accent); text-align: center; font-variant-numeric: tabular-nums; }}
  .card p {{ margin: 0 0 8px; color: var(--muted); font-size: .93rem; }}
  details summary {{ cursor: pointer; font-size: .9rem; color: var(--muted); }}
  details ol {{ list-style: none; margin: 8px 0 0; padding: 0; font-size: .9rem; columns: 2; column-gap: 18px; }}
  details li {{ break-inside: avoid; padding: 1px 0; }}
  details button {{ font: inherit; color: var(--accent); background: none; border: 0; padding: 0 4px 0 0; cursor: pointer; font-variant-numeric: tabular-nums; }}
  .article {{ background: var(--surface); border: 1px solid var(--line); border-radius: 14px; padding: 8px 22px 20px; margin-top: 40px; scroll-margin-top: 12px; }}
  .article h2 {{ font-size: 1.45rem; line-height: 1.3; margin: 18px 0 6px; }}
  .article h3 {{ font-size: 1.1rem; margin: 26px 0 6px; }}
  .article p, .article li {{ max-width: 46em; }}
  .article p {{ margin: 8px 0; }}
  .article ol {{ margin: 8px 0; padding-left: 1.6em; }}
  footer {{ color: var(--muted); font-size: .9rem; padding: 34px 0 44px; }}
</style>
</head>
<body>
<div class="wrap">
  <header>
    <a class="brand" href="../">Candelabra</a>
    <span><a class="btn" href="../">首页</a> <a class="btn primary" href="../app/">打开应用</a></span>
  </header>

  <h1>Candelabra 视频教程（中文）</h1>
  <p class="sub">Candelabra 是在浏览器中运行的照明设计与光度学工具：打开 EULUMDAT、IES、TM-33 和 GLDF 文件，直接查看、编辑和计算，文件不会上传。这 {len(videos)} 集短视频逐一演示各个工具，应用界面和旁白均为中文。</p>
  <p class="sub">视频直接从本站播放，无需其他平台。字幕可以在播放器中关闭；点开"章节"可跳到需要的步骤。旁白是用软件合成的开发者本人的声音。</p>

  <p class="sub"><a href="#standards">技术背景：支持的文件格式、标准与计算方法 ↓</a></p>

  <nav class="index" aria-label="目录">{"".join(f'<a href="#v{v["name"][:2]}">{v["name"][:2]}</a>' for v in videos)}</nav>

  <div class="grid">{"".join(card(v) for v in videos)}
  </div>

  <section class="article" id="standards">
    <h2>支持的文件格式、标准与计算方法</h2>
      {article()}
  </section>

  <footer>
    <p>其他语言的版本（English、Deutsch、Français、Español、Italiano、Português、Русский）在 <a href="https://www.youtube.com/@Candelabra-light" rel="noopener">YouTube 频道</a>。</p>
    <p><a href="../">candelabra.eu</a></p>
  </footer>
</div>
<script>
  // a chapter button starts its video at that time; starting one video pauses the others
  document.addEventListener('click', function (ev) {{
    var b = ev.target.closest('button[data-t]');
    if (!b) return;
    var v = b.closest('.card').querySelector('video');
    var t = +b.dataset.t;
    // with preload="none" nothing is loaded yet: seek once the length is known
    if (v.readyState >= 1) {{ v.currentTime = t; }}
    else {{ v.addEventListener('loadedmetadata', function () {{ v.currentTime = t; }}, {{ once: true }}); }}
    v.play();
  }});
  document.addEventListener('play', function (ev) {{
    document.querySelectorAll('video').forEach(function (v) {{ if (v !== ev.target) v.pause(); }});
  }}, true);
</script>
</body>
</html>
"""
out = os.path.join(HERE, "..", "zh", "index.html")
open(out, "w", encoding="utf-8").write(page)
print("wrote", os.path.normpath(out), len(videos), "videos")
