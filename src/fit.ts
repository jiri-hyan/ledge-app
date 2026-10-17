// Strop výšky šuplíku: Rust posílá v události `view` `maxH` (pracovní plocha monitoru bez hlavního
// panelu, `drawerfit.rs`) → CSS proměnná `--drawer-max-h`. Stránky nad limit se posouvají uvnitř
// (`fit.css`), hlavička zůstává. Okno samo Rust ořízne na stejnou hodnotu.
import { on, type ViewMsg } from "./api";
import "./fit.css";

on<ViewMsg>("view", (m) => {
  if (m.maxH > 0) document.documentElement.style.setProperty("--drawer-max-h", `${Math.floor(m.maxH)}px`);
});
