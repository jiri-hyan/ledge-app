// Mini náhledy vzhledů sekcí lišty notche (kolo 27) pro výběr v panelu (`DesignPicker` `thumb`): statická kresba
// 40×24, žádná data, žádná animace. Klíč `sekce:vzhled`; neznámý = obecný náhled.
import type { JSX } from "solid-js";

const R = (x: number, y: number, w: number, h: number, o = 1) => <rect x={x} y={y} width={w} height={h} rx="1.5" fill="currentColor" opacity={o} />;
const P = (d: string, o = 1, w = 2) => <path d={d} fill="none" stroke="currentColor" stroke-width={w} stroke-linecap="round" opacity={o} />;
const C = (cx: number, cy: number, r: number, o = 1) => <circle cx={cx} cy={cy} r={r} fill="currentColor" opacity={o} />;
const RING = (cx: number, r: number) => <><circle cx={cx} cy="12" r={r} fill="none" stroke="currentColor" stroke-width="2.4" opacity="0.3" /><circle cx={cx} cy="12" r={r} fill="none" stroke="currentColor" stroke-width="2.4" stroke-dasharray={`${r * 4} 99`} /></>;
const WEEK = (y: number, dots = false) => <>{[0, 1, 2, 3, 4, 5, 6].map((i) => <>{R(3 + i * 5, y, 4, 5, i === 2 ? 1 : 0.35)}{dots && i % 2 === 0 ? C(5 + i * 5, y + 8, 0.9) : null}</>)}</>;

const THUMBS: Record<string, () => JSX.Element> = {
  "player:cover": () => <>{R(3, 4, 14, 14)}{R(20, 5, 16, 3)}{R(20, 10, 10, 2, 0.5)}{R(20, 15, 14, 3, 0.7)}</>,
  "player:controls": () => <>{R(6, 3, 28, 3, 0.6)}{P("M9 14 L5 17 L9 20", 1)}{C(20, 17, 4.5)}{P("M31 14 L35 17 L31 20", 1)}</>,
  "player:marquee": () => <>{R(0, 6, 40, 3, 0.3)}{R(4, 6, 26, 3)}{C(14, 17, 2.4, 0.6)}{C(20, 17, 3)}{C(26, 17, 2.4, 0.6)}</>,
  "days:strip": () => <>{R(3, 3, 18, 3)}{R(29, 3, 8, 3, 0.7)}{WEEK(12)}</>,
  "days:sheet": () => <>{R(3, 3, 10, 2.5, 0.8)}{R(3, 8, 12, 11)}{R(19, 5, 3, 5)}{R(24, 6, 12, 3, 0.6)}{R(19, 14, 3, 5, 0.5)}{R(24, 15, 9, 3, 0.4)}</>,
  "days:clock": () => <>{R(5, 3, 30, 12)}{R(9, 18, 22, 2.5, 0.5)}</>,
  "days:dots": () => <>{WEEK(3, true)}{R(3, 16, 34, 6, 0.3)}{R(3, 16, 2, 6)}</>,
  "days:analog": () => <><circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="1.6" />{P("M12 12 V6 M12 12 L16 14", 1, 1.8)}{R(24, 7, 13, 3, 0.6)}{R(24, 13, 9, 3)}</>,
  "system:lines": () => <>{R(3, 3, 6, 3)}{P("M12 5 L18 3 L24 6 L37 2", 1, 1.4)}{R(3, 11, 6, 3, 0.6)}{P("M12 13 L20 12 L37 13", 0.6, 1.4)}{R(3, 19, 34, 2.5, 0.35)}</>,
  "system:area": () => <><path d="M3 20 L3 12 L12 8 L20 14 L28 6 L37 9 L37 20 Z" fill="currentColor" opacity="0.3" />{P("M3 12 L12 8 L20 14 L28 6 L37 9", 1, 1.6)}</>,
  "system:bars": () => <>{[0, 1, 2, 3, 4, 5, 6, 7].map((i) => R(3 + i * 4.4, 20 - ((i * 7) % 13) - 4, 3, ((i * 7) % 13) + 4, 0.4 + (i % 3) * 0.2))}</>,
  "system:rings": () => <>{RING(8, 5.5)}{RING(20, 5.5)}{RING(32, 5.5)}</>,
  "system:big": () => <>{R(3, 4, 9, 11)}{R(15, 4, 9, 11, 0.7)}{R(27, 4, 9, 11, 0.5)}{R(3, 18, 9, 2, 0.4)}{R(15, 18, 9, 2, 0.4)}{R(27, 18, 9, 2, 0.4)}</>,
  "system:htop": () => <>{[4, 11, 18].map((y, i) => <>{R(3, y, 5, 3, 0.6)}{P(`M11 ${y} V${y + 3} M37 ${y} V${y + 3}`, 0.6, 1)}{R(13, y, 18 - i * 5, 3)}</>)}</>,
  "volume:vslider": () => <>{R(18, 2, 4, 20, 0.3)}{R(18, 10, 4, 12)}{C(20, 10, 3)}</>,
  "volume:hslider": () => <>{R(3, 10, 26, 4, 0.3)}{R(3, 10, 16, 4)}{C(19, 12, 3)}{R(32, 9, 5, 6, 0.6)}</>,
  "volume:knob": () => <><circle cx="20" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2" opacity="0.5" />{P("M20 12 L25 6", 1, 2.4)}</>,
  "volume:eq": () => <>{[0, 1, 2, 3, 4, 5, 6].map((i) => R(4 + i * 5, 20 - (5 + i * 2), 3.4, 5 + i * 2, i < 4 ? 1 : 0.3))}</>,
  "volume:icon": () => <><path d="M4 9h4l5-4v14l-5-4H4z" fill="currentColor" />{R(19, 6, 16, 12)}</>,
  "limits:bars": () => <>{[0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map((i) => R(3 + i * 3.5, 6, 2.2, 4, i < 7 ? 1 : 0.3))}{[0, 1, 2, 3, 4, 5, 6, 7, 8, 9].map((i) => R(3 + i * 3.5, 15, 2.2, 4, i < 4 ? 0.8 : 0.3))}</>,
  "limits:ring": () => <>{RING(10, 7)}{R(21, 7, 15, 3)}{R(21, 13, 10, 2.5, 0.5)}</>,
  "weather:now": () => <>{C(10, 10, 5)}{R(19, 5, 16, 9)}{R(4, 19, 26, 2.5, 0.5)}</>,
  "weather:range": () => <>{C(8, 7, 4)}{R(15, 4, 12, 6)}{R(3, 17, 34, 3, 0.3)}{C(24, 18.5, 2.5)}</>,
  "meeting:card": () => <>{R(3, 3, 2, 18)}{R(8, 4, 12, 2.2, 0.6)}{R(8, 9, 20, 4)}{R(8, 16, 14, 2.2, 0.5)}{R(30, 9, 8, 5, 0.8)}</>,
  "meeting:countdown": () => <>{R(6, 3, 28, 11)}{R(10, 17, 20, 3, 0.5)}</>,
};

export const barThumb = (sec: string) => (id: string): JSX.Element => (THUMBS[`${sec}:${id}`] ?? (() => <>{R(3, 4, 34, 3)}{R(3, 10, 34, 3, 0.6)}{R(3, 16, 22, 3, 0.35)}</>))();
