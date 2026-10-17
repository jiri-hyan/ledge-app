/* @refresh reload */
// Vstup: základní styly a tokeny (styles.css) PRVNÍ — styly komponent je pak přepisují, ne naopak.
import "./styles.css";
import "./fit";
import { render } from "solid-js/web";
import App from "./App";

render(() => <App />, document.getElementById("root") as HTMLElement);
