# Balení médií pro showcase (kolo 24): ffmpeg z Playwrightu nemá ani GIF, ani pngquant, takže PNG / WebP / GIF
# komprimuje Pillow. Volá `shoot.mjs`, ručně: python -I pack.py png|webp|gif ...
import sys, glob, os
from PIL import Image

def load(p, width):
    im = Image.open(p).convert("RGB")
    if width and im.width != width:
        im = im.resize((width, round(im.height * width / im.width)), Image.LANCZOS)
    return im

def main(a):
    cmd = a[0]
    if cmd == "png":  # in out width colors
        im = load(a[1], int(a[3]))
        im.quantize(colors=int(a[4]), method=Image.Quantize.MEDIANCUT, dither=Image.Dither.FLOYDSTEINBERG).save(a[2], optimize=True)
    elif cmd == "webp":  # in out width quality
        load(a[1], int(a[3])).save(a[2], "WEBP", quality=int(a[4]), method=6)
    elif cmd == "gif":  # frames_dir out fps width colors  (Playwright nahrává 25 fps → vybere se každý n-tý snímek)
        files = sorted(glob.glob(os.path.join(a[1], "f-*.png")))
        fps = float(a[3])
        pick = [files[min(len(files) - 1, int(i * 25 / fps))] for i in range(int(len(files) * fps / 25))]
        frames = [load(f, int(a[4])) for f in pick]
        # společná paleta ze vzorku snímků (první bývá prázdný / před otevřením)
        sample = frames[:: max(1, len(frames) // 8)]
        mont = Image.new("RGB", (sample[0].width, sum(f.height for f in sample)))
        y = 0
        for f in sample:
            mont.paste(f, (0, y)); y += f.height
        pal = mont.quantize(colors=int(a[5]), method=Image.Quantize.MEDIANCUT)
        dith = Image.Dither.FLOYDSTEINBERG if (len(a) > 6 and a[6] == "fs") else Image.Dither.NONE
        out = [f.quantize(palette=pal, dither=dith) for f in frames]
        out[0].save(a[2], save_all=True, append_images=out[1:], duration=round(1000 / fps), loop=0, optimize=True, disposal=1)
    else:
        sys.exit("neznámý příkaz " + cmd)

main(sys.argv[1:])
