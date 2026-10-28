/* @refresh reload */
// Vstup: základní styly a tokeny (styles.css) PRVNÍ — styly komponent je pak přepisují, ne naopak.
import "./styles.css";
import "./fit";
import { render } from "solid-js/web";
import App from "./App";
import { inTauri } from "./api";

// Ukázkový šuplík pro snímky a GIFy (`index.html?demo=1`, jen mimo Tauri; `src/showcase/`)
async function start() {
  if (!inTauri && new URLSearchParams(location.search).get("demo") === "1") (await import("./showcase/demo")).installShowcase();
  render(() => <App />, document.getElementById("root") as HTMLElement);
}
void start();
