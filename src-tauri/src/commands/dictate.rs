//! Diktát 🎤 v rychlé poznámce: otevře Windows hlasové zadávání (Win+H) do pole, které právě drží fokus.
//! 0 Kč, 0 MB — nic vlastního se neposlouchá. Klávesy se posílají JEN když je popředí okno Ledge (šuplík) a UI
//! potvrdí, že pole má fokus; jinak nic (Ledge nekrade fokus a nesahá na cizí okna).

/// Smí se Win+H poslat? Obě podmínky musí platit: popředí je naše okno a pole v něm má fokus.
pub fn may_send(foreground_is_ours: bool, field_focused: bool) -> bool {
    foreground_is_ours && field_focused
}

/// Pošle Win+H do popředí, pokud to `may_send` dovolí. `Ok(false)` = nic se neposlalo (jiné okno / pole bez fokusu).
#[tauri::command]
pub(crate) fn dictate_start(field_focused: bool) -> Result<bool, String> {
    use tauri::Manager;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_H, VK_LWIN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GA_ROOT, GetAncestor, GetForegroundWindow};
    let ours = {
        let Some(w) = crate::state::core().app.get_webview_window(crate::drawer::LABEL) else { return Ok(false) };
        let Ok(h) = w.hwnd() else { return Ok(false) };
        let fg = unsafe { GetForegroundWindow() };
        !fg.0.is_null() && (fg.0 as isize == h.0 as isize || unsafe { GetAncestor(fg, GA_ROOT) }.0 as isize == h.0 as isize)
    };
    if !may_send(ours, field_focused) {
        return Ok(false);
    }
    let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) }, time: 0, dwExtraInfo: 0 } },
    };
    let seq = [key(VK_LWIN, false), key(VK_H, false), key(VK_H, true), key(VK_LWIN, true)];
    let sent = unsafe { SendInput(&seq, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != seq.len() {
        return Err(crate::i18n::tr("Hlasové zadávání se nepodařilo otevřít.", "Couldn't open voice typing.").into());
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::may_send;

    #[test]
    fn win_h_jen_do_naseho_okna_s_fokusem_v_poli() {
        for (ours, field, want) in [(true, true, true), (true, false, false), (false, true, false), (false, false, false)] {
            assert_eq!(may_send(ours, field), want, "ours={ours} field={field}");
        }
    }
}
