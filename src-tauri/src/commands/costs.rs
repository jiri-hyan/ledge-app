//! Útrata Claude (odhad v cenách API): UI žádá přepočet při rozbalení sekce v kartě Claude
//! a při otevření šuplíku s rozbalenou sekcí. Logika a omezení (1× / 60 s, vlákno s nízkou
//! prioritou) v `costslive.rs`; výsledek přijde ve snapshotu (`costs`).

#[tauri::command]
pub(crate) fn costs_refresh() {
    crate::costslive::request();
}
