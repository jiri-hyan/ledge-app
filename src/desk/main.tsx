/* @refresh reload */
// Vstup okna widgetu na ploše (`desk.html`, Rust `desk/win.rs`): základní styly a tokeny (styles.css) první,
// pak jen kód widgetu a karet, které znovupoužívá. Anglické slovníky se stáhnou líně až při `setLang("en")`
// (stejný chunk jako šuplík, `i18n/dicts.ts`); při češtině se nenačítá nic.
import "../styles.css";
import "./desk.css";
import { setDictLoader } from "../i18n";
import { render } from "solid-js/web";
import DeskApp from "./DeskApp";

setDictLoader(() => import("../i18n/dicts"));

render(() => <DeskApp />, document.getElementById("root") as HTMLElement);
