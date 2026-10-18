// Ikona zástupce s vlastním vzhledem (`TileLook`): obsah = emoji / systémová ikona (Segoe Fluent Icons) /
// vlastní obrázek z cache / automatický (logo, favicon, písmena), podklad = barva vzhledu nebo barva
// projektu / akce, tvar `data-shape`. Kresba na barevném podkladu je černá nebo bílá podle kontrastu
// (`inkDark`, ≥ 4.5:1); obrázek, který by s podkladem splynul, dostane světlou podložku (`useTone`).
// Bez vzhledu se nekreslí — dlaždice jdou původní cestou (vzhled přesně jako dřív).
import { Show, type JSX } from "solid-js";
import { useTone } from "../ui/iconTone";
import { glyphChar } from "./glyphs";
import { inkDark, isHex, type TileLook } from "./model";
import { lookImageSrc } from "./state";
import "./tilelook.css";

export type AutoIcon = {
  /** automatický obrázek (logo projektu, favicon akce) */
  src?: string;
  /** písmena, když obrázek není */
  mono: string;
  /** barva projektu / akce (`#rrggbb`) */
  color: string;
  /** automatický obrázek leží na světlé podložce (logo projektu); akce ji nemá */
  plate: boolean;
};

export default function TileIcon(p: {
  look: TileLook;
  auto: AutoIcon;
  /** CSS délka strany (`var(--logo)`, `30px`) */
  size: string;
  /** kořen je logo dlaždice projektu (třída `logo` — náklon, prstenec běhu) */
  logo?: boolean;
  title?: string;
  children?: JSX.Element;
}) {
  const color = () => p.look.color ?? (isHex(p.auto.color) ? p.auto.color : "#64748b");
  const shape = () => p.look.shape ?? "rounded";
  const glass = () => shape() === "glass";
  const dark = () => !glass() && inkDark(color());
  const custom = () => (p.look.icon?.kind === "image" ? lookImageSrc(p.look.icon.value) : undefined);
  /** obrázek, který se opravdu kreslí (vlastní, jinak automatický — jen když jiná ikona není zvolená) */
  const img = () => custom() ?? (p.look.icon && p.look.icon.kind !== "image" ? undefined : p.auto.src);
  const tone = useTone(img);
  const plate = () => {
    if (!img()) return false;
    if (!p.look.color && !glass() && p.auto.plate && !custom()) return true;
    // kresba stejného tónu jako podklad by zmizela
    return glass() ? false : dark() ? tone() === "light" : tone() === "dark";
  };
  return (
    <div
      class="tl-ico"
      classList={{ logo: !!p.logo }}
      data-shape={shape()}
      data-ink={glass() ? "theme" : dark() ? "dark" : "light"}
      data-plate={plate() ? "" : undefined}
      style={{ "--tl-c": color(), "--tl-s": p.size }}
      title={p.title}
    >
      <Show when={img()} fallback={
        <Show when={p.look.icon?.kind === "emoji" ? p.look.icon.value : null} fallback={
          <Show when={p.look.icon?.kind === "glyph" ? p.look.icon.value : null} fallback={<span class="tl-mono">{p.auto.mono}</span>}>
            {(cp) => <span class="tl-glyph" aria-hidden="true">{glyphChar(cp())}</span>}
          </Show>
        }>
          {(e) => <span class="tl-emoji" aria-hidden="true">{e()}</span>}
        </Show>
      }>
        {(src) => <img class="tl-img" src={src()} alt="" draggable={false} />}
      </Show>
      {p.children}
    </div>
  );
}
