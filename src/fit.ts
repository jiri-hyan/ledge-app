// Strop výšky šuplíku: Rust posílá v události `view` `maxH` (80 % pracovní plochy monitoru bez hlavního
// panelu, `drawerfit.rs` `HEIGHT_FRACTION`) → CSS proměnná `--drawer-max-h`. Stránky nad limit se posouvají uvnitř
// (`fit.css`), hlavička zůstává. Okno samo Rust ořízne na stejnou hodnotu. Do první události platí odhad
// z pracovní plochy (`fallbackMaxH`) — bez stropu by se výška obsahu vázala na výšku okna (smyčka 0.11.0).
import { on, type ViewMsg } from "./api";
import { fallbackMaxH } from "./theme/size";
import "./fit.css";

document.documentElement.style.setProperty("--drawer-max-h", `${fallbackMaxH(screen.availHeight)}px`);

on<ViewMsg>("view", (m) => {
  if (m.maxH > 0) document.documentElement.style.setProperty("--drawer-max-h", `${Math.floor(m.maxH)}px`);
});
