/* @refresh reload */
// Vstup okna lístku na plochu (`pin.html`, Rust `pins/win.rs`): jen kód lístku, ne celý šuplík.
// Anglické slovníky, které lístek potřebuje (registerDict), se stáhnou líně až při `setLang("en")` (z `PinApp`);
// při češtině se nenačítá nic.
import { setDictLoader } from "../i18n";
import "./pin.css";
import { render } from "solid-js/web";
import PinApp from "./PinApp";

setDictLoader(() => Promise.all([import("../i18n/en-main"), import("../i18n/en-notes"), import("./en")]));

render(() => <PinApp />, document.getElementById("root") as HTMLElement);
