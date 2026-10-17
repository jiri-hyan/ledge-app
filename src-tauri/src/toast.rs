//! Oznámení Windows (toast) přes WinRT z crate `windows` (bez nové závislosti, bez PowerShellu).
//! Volá ho jen `attention::on_change` (config `claude.toast`, výchozí vypnuto). Toast nebere fokus;
//! odesílá se z krátkého jednorázového vlákna. Identita appky = `identifier` z `tauri.conf.json`
//! (NSIS instalátor Tauri ho zapisuje jako AppUserModelID zástupce ve Startu) — bez nainstalované
//! kopie Windows toast tiše zahodí, chyba jde jen do logu.

use crate::state::log;

/// Přijímá Windows právě oznámení? (`SHQueryUserNotificationState` = ACCEPTS_NOTIFICATIONS; hra,
/// prezentace, „nerušit“ i celoobrazovková appka = ne.)
pub(crate) fn accepts_notifications() -> bool {
    use windows::Win32::UI::Shell::{QUNS_ACCEPTS_NOTIFICATIONS, SHQueryUserNotificationState};
    unsafe { SHQueryUserNotificationState() }.is_ok_and(|s| s == QUNS_ACCEPTS_NOTIFICATIONS)
}

/// Text do XML (toast šablona).
pub(crate) fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if (c as u32) < 0x20 && c != '\t' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// XML toastu: titulek + volitelné tělo, šablona ToastGeneric.
pub(crate) fn toast_xml(title: &str, body: &str) -> String {
    let body = if body.is_empty() { String::new() } else { format!("<text>{}</text>", xml_escape(body)) };
    format!(r#"<toast><visual><binding template="ToastGeneric"><text>{}</text>{body}</binding></visual></toast>"#, xml_escape(title))
}

fn show_now(app_id: &str, xml: &str) -> windows::core::Result<()> {
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};
    use windows::core::HSTRING;
    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml))?;
    let toast = ToastNotification::CreateToastNotification(&doc)?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?.Show(&toast)
}

/// Pošle toast na pozadí (krátké vlákno, nečeká se na něj).
pub(crate) fn show(app_id: &str, title: String, body: String) {
    let app_id = app_id.to_string();
    let xml = toast_xml(&title, &body);
    let _ = std::thread::Builder::new().name("toast".into()).spawn(move || {
        if let Err(e) = show_now(&app_id, &xml) {
            log(&format!("toast: {e}"));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_is_escaped() {
        let cases = [
            ("a", "", r#"<toast><visual><binding template="ToastGeneric"><text>a</text></binding></visual></toast>"#),
            ("Claude <&>", "x \"y\" 'z'", r#"<toast><visual><binding template="ToastGeneric"><text>Claude &lt;&amp;&gt;</text><text>x &quot;y&quot; &apos;z&apos;</text></binding></visual></toast>"#),
            ("řádek\nnový", "", r#"<toast><visual><binding template="ToastGeneric"><text>řádek nový</text></binding></visual></toast>"#),
        ];
        for (t, b, want) in cases {
            assert_eq!(toast_xml(t, b), want);
        }
    }
}
