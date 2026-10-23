//! Příkaz hledání města pro Nastavení → Osobní → Počasí (našeptávač). Logika a síť v `weather.rs`
//! (`search_places`); tady jen tenká obálka. Volá se jen na psaní v poli Město, nikdy v klidu.

use crate::weather::{self, Place};

/// Návrhy měst z Open-Meteo geocodingu (nejvýš 5; dotaz kratší než 2 znaky = prázdný seznam bez dotazu na síť).
#[tauri::command]
pub(crate) async fn geo_search(query: String) -> Result<Vec<Place>, String> {
    weather::search_places(&query).await
}
