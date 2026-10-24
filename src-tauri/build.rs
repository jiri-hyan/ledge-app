// ACL vlastních příkazů (audit N4): seznam příkazů z `generate_handler![…]` v src/lib.rs jde do
// `AppManifest::commands` — od té chvíle smí okno volat jen příkazy, které mu povolí jeho capability
// (`capabilities/*.json`, `allow-<příkaz>`). Pravidla a tabulky: src/ipcacl.rs.
#[allow(dead_code)]
#[path = "src/ipcacl.rs"]
mod ipcacl;

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=src/ipcacl.rs");
    let lib = std::fs::read_to_string("src/lib.rs").expect("build.rs: src/lib.rs nejde přečíst");
    let cmds = ipcacl::handler_commands(&lib);
    assert!(!cmds.is_empty(), "build.rs: v src/lib.rs chybí generate_handler![…]");
    // tauri-build chce &'static — build skript běží jednou, únik nevadí
    let cmds: &'static [&'static str] = Box::leak(cmds.into_iter().map(|c| &*Box::leak(c.into_boxed_str())).collect::<Vec<_>>().into_boxed_slice());
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(cmds))).expect("tauri-build selhal");
}
