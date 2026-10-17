//! Markdown poznámek bez raw HTML: náhled (`to_html`), čistý titulek karty (`plain`) a úkoly
//! `- [ ]`/`- [x]` (`tasks`, `toggle_line`). Čisté funkce bez I/O; čísla řádků se počítají v těle
//! poznámky tak, jak ho vrací `edit::body_view` (stejně je čte náhled i přepínání úkolu).
//!
//! Bezpečnost: veškerý text se escapuje, HTML ze vstupu nikdy neprojde. Odkazy jen http(s) a jen jako
//! `data-href` (otevírá je UI přes `open_url`), obrázky jen relativní (`data-src`, data URL doplní UI).

/// Escapuje text do HTML (i do atributů).
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            c => o.push(c),
        }
    }
    o
}

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

/// Položka seznamu: (odsazení, číslovaná?, zbytek za značkou).
fn list_item(line: &str) -> Option<(usize, bool, &str)> {
    let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
    let t = &line[indent..];
    if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("+ ")) {
        return Some((indent, false, rest));
    }
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    if (1..=9).contains(&digits) {
        let rest = &t[digits..];
        if let Some(r) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return Some((indent, true, r));
        }
    }
    None
}

/// `[ ] text` / `[x] text` → (hotovo?, text).
fn task_box(rest: &str) -> Option<(bool, &str)> {
    let checked = match rest.get(..3)? {
        "[ ]" => false,
        "[x]" | "[X]" => true,
        _ => return None,
    };
    let after = &rest[3..];
    (after.is_empty() || after.starts_with(' ')).then(|| (checked, after.trim_start()))
}

/// Úkoly v těle: (řádek od 0, hotovo?). Bloky kódu se přeskočí.
pub fn tasks(body: &str) -> Vec<(usize, bool)> {
    let mut out = Vec::new();
    let mut code = false;
    for (i, line) in body.split('\n').map(|l| l.trim_end_matches('\r')).enumerate() {
        if is_fence(line) {
            code = !code;
            continue;
        }
        if !code
            && let Some((checked, _)) = list_item(line).and_then(|(_, _, r)| task_box(r))
        {
            out.push((i, checked));
        }
    }
    out
}

/// Nastaví úkol na řádku `line` (od 0) na `checked`. Řádek, který úkolem není, = chyba.
pub fn toggle_line(body: &str, line: usize, checked: bool) -> Result<String, String> {
    if !tasks(body).iter().any(|&(i, _)| i == line) {
        return Err(crate::i18n::tr("Na tomhle řádku už úkol není — poznámka se mezitím změnila.", "There's no task on this line any more — the note changed meanwhile.").into());
    }
    let mut out = String::with_capacity(body.len());
    for (i, l) in body.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if i == line {
            let at = l.find('[').unwrap_or(0);
            out.push_str(&l[..at + 1]);
            out.push(if checked { 'x' } else { ' ' });
            out.push_str(&l[at + 2..]);
        } else {
            out.push_str(l);
        }
    }
    Ok(out)
}

// ---------- inline ----------

fn safe_url(u: &str) -> bool {
    let l = u.trim().to_ascii_lowercase();
    (l.starts_with("http://") || l.starts_with("https://")) && !u.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Relativní obrázek vedle poznámky (`./a.png`, `a.png`) — žádné schéma, absolutní cesta ani `..`.
pub fn relative_image(src: &str) -> bool {
    let s = src.trim();
    !s.is_empty()
        && !s.contains(':')
        && !s.starts_with(['/', '\\'])
        && !s.split(['/', '\\']).any(|p| p == "..")
        && !s.chars().any(char::is_control)
}

/// `[text](url)` od pozice `[`: (text, url, délka v bajtech).
fn bracket_link(s: &str) -> Option<(&str, &str, usize)> {
    let close = s.find("](")?;
    let text = &s[1..close];
    if text.contains('[') {
        return None;
    }
    let rest = &s[close + 2..];
    // URL smí mít vyvážené závorky: `(javascript:alert(1))` je celá jedna URL
    let mut depth = 0usize;
    let end = rest.char_indices().find_map(|(k, ch)| match ch {
        '(' => {
            depth += 1;
            None
        }
        ')' if depth == 0 => Some(k),
        ')' => {
            depth -= 1;
            None
        }
        _ => None,
    })?;
    Some((text, &rest[..end], close + 2 + end + 1))
}

/// Relativní obrázky `![](src)` v těle — bez duplicit, v pořadí výskytu.
pub fn image_refs(body: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    let mut rest = body;
    while let Some(i) = rest.find("![") {
        let s = &rest[i + 1..];
        match bracket_link(s) {
            Some((_, src, len)) => {
                let src = src.trim();
                if relative_image(src) && !out.contains(&src) {
                    out.push(src);
                }
                rest = &s[len..];
            }
            None => rest = &rest[i + 2..],
        }
    }
    out
}

/// Inline Markdown → HTML (vše escapované). `plain` = jen text bez značek (titulek karty).
fn inline(s: &str, out: &mut String, plain: bool) {
    let b = s.as_bytes();
    let mut i = 0;
    let mut text_start = 0;
    let flush = |out: &mut String, from: usize, to: usize| {
        if to > from {
            if plain {
                out.push_str(&s[from..to]);
            } else {
                out.push_str(&esc(&s[from..to]));
            }
        }
    };
    while i < b.len() {
        let c = b[i];
        // \* apod. = doslovný znak
        if c == b'\\' && i + 1 < b.len() && b[i + 1].is_ascii_punctuation() {
            flush(out, text_start, i);
            text_start = i + 1;
            i += 2;
            continue;
        }
        if c == b'`' && let Some(end) = s[i + 1..].find('`') {
            flush(out, text_start, i);
            let code = &s[i + 1..i + 1 + end];
            if plain {
                out.push_str(code);
            } else {
                out.push_str(&format!("<code>{}</code>", esc(code)));
            }
            i += end + 2;
            text_start = i;
            continue;
        }
        if c == b'!' && b.get(i + 1) == Some(&b'[') && let Some((alt, src, len)) = bracket_link(&s[i + 1..]) {
            flush(out, text_start, i);
            if plain {
                out.push_str(alt);
            } else if relative_image(src) {
                out.push_str(&format!("<img class=\"md-img\" data-src=\"{}\" alt=\"{}\">", esc(src.trim()), esc(alt)));
            } else {
                out.push_str(&format!("<span class=\"md-alt\">[{}{}{}]</span>", crate::i18n::tr("obrázek", "image"), if alt.is_empty() { "" } else { ": " }, esc(alt)));
            }
            i += 1 + len;
            text_start = i;
            continue;
        }
        if c == b'[' && let Some((text, url, len)) = bracket_link(&s[i..]) {
            flush(out, text_start, i);
            if !plain && safe_url(url) {
                out.push_str(&format!("<a class=\"md-link\" data-href=\"{}\" title=\"{}\">", esc(url.trim()), esc(url.trim())));
                inline(text, out, false);
                out.push_str("</a>");
            } else {
                inline(text, out, plain);
            }
            i += len;
            text_start = i;
            continue;
        }
        if !plain && (s[i..].starts_with("https://") || s[i..].starts_with("http://")) && (i == 0 || !b[i - 1].is_ascii_alphanumeric()) {
            let end = s[i..].find(|ch: char| ch.is_whitespace() || ch == '<' || ch == '>' || ch == '"').map_or(s.len(), |e| i + e);
            let url = s[i..end].trim_end_matches(['.', ',', ')', ';', ':', '!', '?']);
            flush(out, text_start, i);
            out.push_str(&format!("<a class=\"md-link\" data-href=\"{0}\" title=\"{0}\">{0}</a>", esc(url)));
            i += url.len();
            text_start = i;
            continue;
        }
        if c == b'*' || c == b'_' {
            let strong = b.get(i + 1) == Some(&c);
            let d = if strong { &s[i..i + 2] } else { &s[i..i + 1] };
            let inner_from = i + d.len();
            // `_` jen na hranici slova (snake_case zůstane), obsah nesmí začínat mezerou
            let left_ok = c == b'*' || i == 0 || !b[i - 1].is_ascii_alphanumeric();
            if left_ok && inner_from < b.len() && b[inner_from] != b' ' && let Some(rel) = s[inner_from..].find(d) {
                let close = inner_from + rel;
                let right_ok = c == b'*' || b.get(close + d.len()).is_none_or(|n| !n.is_ascii_alphanumeric());
                if rel > 0 && b[close - 1] != b' ' && right_ok {
                    flush(out, text_start, i);
                    let tag = if strong { "strong" } else { "em" };
                    if !plain {
                        out.push_str(&format!("<{tag}>"));
                    }
                    inline(&s[inner_from..close], out, plain);
                    if !plain {
                        out.push_str(&format!("</{tag}>"));
                    }
                    i = close + d.len();
                    text_start = i;
                    continue;
                }
            }
        }
        i += s[i..].chars().next().map_or(1, char::len_utf8);
    }
    flush(out, text_start, b.len());
}

/// Řádek bez Markdown značek (nadpis, odrážka, úkol, citace, tučné, odkazy…).
pub fn plain(line: &str) -> String {
    let mut t = line.trim();
    t = t.trim_start_matches('#').trim_start();
    if let Some((_, _, rest)) = list_item(t) {
        t = rest;
        if let Some((_, r)) = task_box(t) {
            t = r;
        }
    }
    t = t.strip_prefix('>').map_or(t, str::trim_start);
    let mut out = String::new();
    inline(t, &mut out, true);
    out.trim().to_string()
}

/// Titulek karty: první řádek s textem (mimo samotné obrázky a značky bloků kódu), bez značek.
pub fn title(body: &str) -> String {
    body.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("![") && !is_fence(l))
        .map(plain)
        .find(|p| !p.is_empty())
        .unwrap_or_default()
}

// ---------- bloky ----------

fn heading(line: &str) -> Option<(usize, &str)> {
    let t = line.trim_start();
    let n = t.bytes().take_while(|&c| c == b'#').count();
    if !(1..=6).contains(&n) {
        return None;
    }
    let rest = &t[n..];
    (rest.is_empty() || rest.starts_with(' ')).then(|| (n, rest.trim()))
}

fn is_hr(line: &str) -> bool {
    let t: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    t.len() >= 3 && (t.chars().all(|c| c == '-') || t.chars().all(|c| c == '*') || t.chars().all(|c| c == '_'))
}

/// Tělo poznámky → bezpečné HTML pro náhled. Úkoly nesou `data-line` (řádek těla od 0).
pub fn to_html(body: &str) -> String {
    let mut out = String::new();
    let mut para: Vec<&str> = Vec::new();
    // otevřený seznam: číslovaný?
    let mut list: Option<bool> = None;
    let mut code: Option<String> = None;
    let close_para = |out: &mut String, para: &mut Vec<&str>| {
        if para.is_empty() {
            return;
        }
        out.push_str("<p>");
        for (k, l) in para.iter().enumerate() {
            if k > 0 {
                out.push_str("<br>");
            }
            inline(l.trim(), out, false);
        }
        out.push_str("</p>");
        para.clear();
    };
    let close_list = |out: &mut String, list: &mut Option<bool>| {
        if let Some(ord) = list.take() {
            out.push_str(if ord { "</ol>" } else { "</ul>" });
        }
    };
    for (i, raw) in body.split('\n').enumerate() {
        let line = raw.trim_end_matches('\r');
        if let Some(buf) = code.as_mut() {
            if is_fence(line) {
                out.push_str(&format!("<pre><code>{}</code></pre>", esc(buf.trim_end_matches('\n'))));
                code = None;
            } else {
                buf.push_str(line);
                buf.push('\n');
            }
            continue;
        }
        if is_fence(line) {
            close_para(&mut out, &mut para);
            close_list(&mut out, &mut list);
            code = Some(String::new());
            continue;
        }
        if line.trim().is_empty() {
            close_para(&mut out, &mut para);
            close_list(&mut out, &mut list);
            continue;
        }
        if let Some((indent, ordered, rest)) = list_item(line) {
            close_para(&mut out, &mut para);
            if list != Some(ordered) {
                close_list(&mut out, &mut list);
                out.push_str(if ordered { "<ol>" } else { "<ul>" });
                list = Some(ordered);
            }
            let depth = (indent / 2).min(3);
            let cls = if depth > 0 { format!(" class=\"d{depth}\"") } else { String::new() };
            match task_box(rest) {
                Some((checked, text)) => {
                    out.push_str(&format!(
                        "<li{cls} data-task=\"1\"><span class=\"md-check{}\" role=\"checkbox\" aria-checked=\"{checked}\" data-line=\"{i}\"></span><span>",
                        if checked { " on" } else { "" }
                    ));
                    inline(text, &mut out, false);
                    out.push_str("</span></li>");
                }
                None => {
                    out.push_str(&format!("<li{cls}>"));
                    inline(rest, &mut out, false);
                    out.push_str("</li>");
                }
            }
            continue;
        }
        close_list(&mut out, &mut list);
        if let Some((n, text)) = heading(line) {
            close_para(&mut out, &mut para);
            out.push_str(&format!("<h{n}>"));
            inline(text, &mut out, false);
            out.push_str(&format!("</h{n}>"));
        } else if is_hr(line) {
            close_para(&mut out, &mut para);
            out.push_str("<hr>");
        } else if let Some(q) = line.trim_start().strip_prefix('>') {
            close_para(&mut out, &mut para);
            out.push_str("<blockquote>");
            inline(q.trim(), &mut out, false);
            out.push_str("</blockquote>");
        } else {
            para.push(line);
        }
    }
    if let Some(buf) = code {
        out.push_str(&format!("<pre><code>{}</code></pre>", esc(buf.trim_end_matches('\n'))));
    }
    close_para(&mut out, &mut para);
    close_list(&mut out, &mut list);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_table() {
        let cases: &[(&str, &str)] = &[
            ("# Nadpis", "<h1>Nadpis</h1>"),
            ("### **Tučný** nadpis", "<h3><strong>Tučný</strong> nadpis</h3>"),
            ("a\nb\n\nc", "<p>a<br>b</p><p>c</p>"),
            ("**tučně** a *kurzíva* a _taky_", "<p><strong>tučně</strong> a <em>kurzíva</em> a <em>taky</em></p>"),
            ("snake_case_name zůstane", "<p>snake_case_name zůstane</p>"),
            ("2 * 3 * 4", "<p>2 * 3 * 4</p>"),
            ("`a<b>` kód", "<p><code>a&lt;b&gt;</code> kód</p>"),
            ("```\n<script>x</script>\n# ne\n```", "<pre><code>&lt;script&gt;x&lt;/script&gt;\n# ne</code></pre>"),
            ("- a\n- b", "<ul><li>a</li><li>b</li></ul>"),
            ("1. a\n2. b", "<ol><li>a</li><li>b</li></ol>"),
            ("- a\n  - b", "<ul><li>a</li><li class=\"d1\">b</li></ul>"),
            ("---", "<hr>"),
            ("> citát", "<blockquote>citát</blockquote>"),
            ("\\*doslova\\*", "<p>*doslova*</p>"),
        ];
        for (src, want) in cases {
            assert_eq!(to_html(src), *want, "{src}");
        }
    }

    #[test]
    fn tasks_carry_body_line() {
        let html = to_html("Nákup\n\n- [ ] mléko\n- [x] chleba");
        assert!(html.contains("<span class=\"md-check\" role=\"checkbox\" aria-checked=\"false\" data-line=\"2\"></span><span>mléko</span>"), "{html}");
        assert!(html.contains("md-check on\" role=\"checkbox\" aria-checked=\"true\" data-line=\"3\""), "{html}");
    }

    #[test]
    fn xss_is_escaped() {
        let cases: &[(&str, &str)] = &[
            ("<script>alert(1)</script>", "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>"),
            ("<img src=x onerror=alert(1)>", "<p>&lt;img src=x onerror=alert(1)&gt;</p>"),
            ("[klik](javascript:alert(1))", "<p>klik</p>"),
            ("[klik](JaVaScRiPt:alert(1))", "<p>klik</p>"),
            ("[x](data:text/html,<b>)", "<p>x</p>"),
            ("![a](javascript:alert(1))", "<p><span class=\"md-alt\">[obrázek: a]</span></p>"),
            ("![a](C:\\Windows\\x.png)", "<p><span class=\"md-alt\">[obrázek: a]</span></p>"),
            ("![a](../../tajne.png)", "<p><span class=\"md-alt\">[obrázek: a]</span></p>"),
            ("# <b>nadpis</b>", "<h1>&lt;b&gt;nadpis&lt;/b&gt;</h1>"),
        ];
        for (src, want) in cases {
            assert_eq!(to_html(src), *want, "{src}");
        }
        // povolený odkaz: jen data-href, URL escapovaná
        assert_eq!(
            to_html("[web](https://a.cz/?q=1&b=\"2\")"),
            "<p><a class=\"md-link\" data-href=\"https://a.cz/?q=1&amp;b=&quot;2&quot;\" title=\"https://a.cz/?q=1&amp;b=&quot;2&quot;\">web</a></p>"
        );
        assert_eq!(to_html("viz https://a.cz."), "<p>viz <a class=\"md-link\" data-href=\"https://a.cz\" title=\"https://a.cz\">https://a.cz</a>.</p>");
        assert_eq!(to_html("![obr](./a-1.png)"), "<p><img class=\"md-img\" data-src=\"./a-1.png\" alt=\"obr\"></p>");
        assert!(!to_html("<a href=\"javascript:x\">").contains("<a "));
        // uvozovky v URL ani v textu odkazu nevylezou z atributu
        let h = to_html("[\"><b>](https://x.cz/\"onmouseover=\"a)");
        assert!(h.contains("data-href=\"https://x.cz/&quot;onmouseover=&quot;a\"") && h.contains("&quot;&gt;&lt;b&gt;</a>"), "{h}");
    }

    #[test]
    fn plain_titles() {
        let cases: &[(&str, &str)] = &[
            ("# **Zavolat** Řehořovi", "Zavolat Řehořovi"),
            ("- [ ] koupit `mléko`", "koupit mléko"),
            ("1. [web](https://x.cz) první", "web první"),
            ("> citát", "citát"),
            ("snake_case", "snake_case"),
        ];
        for (src, want) in cases {
            assert_eq!(plain(src), *want, "{src}");
        }
        assert_eq!(title("\n![](./a.png)\n```\n# Úkoly\n- [x] a"), "Úkoly");
        assert_eq!(title("![](./a.png)"), "");
    }

    #[test]
    fn task_detection_and_toggle() {
        let body = "a\n- [ ] jedna\n```\n- [ ] v kódu\n```\n  * [x] dvě\n1. [ ] tři\n- [] ne\n-[ ] ne";
        assert_eq!(tasks(body), vec![(1, false), (5, true), (6, false)]);
        assert_eq!(toggle_line(body, 1, true).unwrap().split('\n').nth(1), Some("- [x] jedna"));
        assert_eq!(toggle_line(body, 5, false).unwrap().split('\n').nth(5), Some("  * [ ] dvě"));
        // jen ten jeden řádek, zbytek bajtově stejný
        let t = toggle_line(body, 6, true).unwrap();
        assert_eq!(t.replace("1. [x] tři", "1. [ ] tři"), body);
        for bad in [0, 3, 7, 99] {
            assert!(toggle_line(body, bad, true).is_err(), "{bad}");
        }
        // CRLF: řádky se počítají po \n, \r zůstane
        assert_eq!(toggle_line("x\r\n- [ ] a\r\n", 1, true).unwrap(), "x\r\n- [x] a\r\n");
    }
}
