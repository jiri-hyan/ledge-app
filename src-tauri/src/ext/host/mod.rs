//! Sdílený kód hostitele rozšíření s oknem (kolo 33, E2): Win32 okno + smyčka zpráv (`win`) a WebView2
//! bez Tauri (`webview`). Rozšíření jako `wall` ho používají ze svého `Body::run`; sama logika rozšíření
//! (kam okno připnout, co zobrazit) je v `ext/<id>/`.

pub mod webview;
pub mod win;
