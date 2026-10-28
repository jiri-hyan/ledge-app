/* @refresh reload */
// Vstup okna Ledge Studio (`settings.html`, Rust `studio.rs`): základní styly a tokeny (styles.css) první, pak
// Studio. Anglické slovníky se stáhnou líně až při `setLang("en")` (stejný chunk jako šuplík). Mimo Tauri
// s `?demo=1` (jen snímky v prohlížeči) se zapne ukázkový hostitel `demo.ts` — v Tauri nikdy.
import "../styles.css";
import { setDictLoader } from "../i18n";
import { inTauri } from "../api";
import { render } from "solid-js/web";
import StudioWindow from "./StudioWindow";

setDictLoader(() => import("../i18n/dicts"));

async function start() {
  if (!inTauri && new URLSearchParams(location.search).get("demo") === "1") (await import("./demo")).installDemo();
  render(() => <StudioWindow />, document.getElementById("root") as HTMLElement);
}
void start();
