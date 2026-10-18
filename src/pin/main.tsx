/* @refresh reload */
// Vstup okna lístku na plochu (`pin.html`, Rust `pins/win.rs`): jen kód lístku, ne celý šuplík.
// Anglické slovníky, které lístek potřebuje (registerDict), dřív než cokoli volá t().
import "../i18n/en-main";
import "../i18n/en-notes";
import "./en";
import "./pin.css";
import { render } from "solid-js/web";
import PinApp from "./PinApp";

render(() => <PinApp />, document.getElementById("root") as HTMLElement);
