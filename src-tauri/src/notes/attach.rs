//! Vstup snímků při ukládání poznámky (`NewNote.images`, `SaveNote.images`) a skládání těla: snímek
//! s `anchorLine` se vloží odsazeně pod bod na tom řádku (za jeho případné přílohy) s alt textem =
//! popisek, ostatní se připojí na konec jako dosud. Čisté funkce bez I/O (soubory píše volající).

use super::brief::image_only;
use super::md::list_item;
use super::md_list::indent_width;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Snímek k uložení: dosavadní holý řetězec (PNG v base64, i data URL) nebo objekt s popiskem a
/// kotvou `{ "png": "…", "caption": "…", "anchorLine": 4 }` (`anchorLine` = řádek těla od 0).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ImageIn {
    Plain(String),
    #[serde(rename_all = "camelCase")]
    Full {
        png: String,
        #[serde(default)]
        caption: Option<String>,
        #[serde(default)]
        anchor_line: Option<usize>,
    },
}

impl From<String> for ImageIn {
    fn from(s: String) -> Self {
        Self::Plain(s)
    }
}

impl ImageIn {
    pub fn png(&self) -> &str {
        match self {
            Self::Plain(s) => s,
            Self::Full { png, .. } => png,
        }
    }

    fn caption(&self) -> Option<&str> {
        match self {
            Self::Full { caption, .. } => caption.as_deref(),
            Self::Plain(_) => None,
        }
    }

    fn anchor(&self) -> Option<usize> {
        match self {
            Self::Full { anchor_line, .. } => *anchor_line,
            Self::Plain(_) => None,
        }
    }
}

/// Alt text: bez hranatých závorek a zalomení.
fn alt(caption: Option<&str>) -> String {
    caption.unwrap_or("").replace(['[', ']'], "").split(['\n', '\r']).map(str::trim).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ")
}

/// Tělo k zápisu: `\n` + text + vložené snímky. `names[i]` = jméno souboru `imgs[i]`. `trimmed` =
/// kolik prázdných řádků volající z počátku textu odstranil (kotva se o ně posune).
pub fn compose(text: &str, imgs: &[ImageIn], names: &[String], trimmed: usize) -> String {
    let cr = text.contains("\r\n");
    let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut tail = Vec::new();
    for (k, im) in imgs.iter().enumerate() {
        let target = im.anchor().and_then(|a| a.checked_sub(trimmed)).filter(|&a| lines.get(a).is_some_and(|l| list_item(l.trim_end_matches('\r')).is_some()));
        match target {
            Some(a) => groups.entry(a).or_default().push(k),
            None => tail.push(k),
        }
    }
    // od nejnižšího řádku nahoru, ať se vkládáním neposouvají kotvy, které teprve přijdou
    for (a, group) in groups.iter().rev() {
        let item = lines[*a].trim_end_matches('\r').to_string();
        let (w, col) = (indent_width(&item), list_item(&item).map_or(2, |(_, _, rest)| item.len() - rest.len()));
        let mut at = a + 1;
        while at < lines.len() && indent_width(&lines[at]) > w && image_only(lines[at].trim_end_matches('\r')).is_some() {
            at += 1;
        }
        for (n, &k) in group.iter().enumerate() {
            let line = format!("{}![{}](./{}){}", " ".repeat(col), alt(imgs[k].caption()), names[k], if cr { "\r" } else { "" });
            lines.insert(at + n, line);
        }
    }
    let mut body = format!("\n{}\n", lines.join("\n"));
    for k in tail {
        body.push_str(&format!("\n![{}](./{})\n", alt(imgs[k].caption()), names[k]));
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn img(caption: Option<&str>, anchor: Option<usize>) -> ImageIn {
        ImageIn::Full { png: "x".into(), caption: caption.map(Into::into), anchor_line: anchor }
    }

    fn names(n: usize) -> Vec<String> {
        (1..=n).map(|i| format!("n-{i}.png")).collect()
    }

    #[test]
    fn deserializes_old_and_new_shape() {
        let v: Vec<ImageIn> = serde_json::from_str(r#"["aaa", {"png":"bbb"}, {"png":"ccc","caption":"c","anchorLine":3}]"#).unwrap();
        assert_eq!(v[0], ImageIn::Plain("aaa".into()));
        assert_eq!(v[1], img(None, None).with_png("bbb"));
        assert_eq!((v[2].png(), v[2].caption(), v[2].anchor()), ("ccc", Some("c"), Some(3)));
    }

    impl ImageIn {
        fn with_png(self, p: &str) -> Self {
            match self {
                Self::Full { caption, anchor_line, .. } => Self::Full { png: p.into(), caption, anchor_line },
                o => o,
            }
        }
    }

    #[test]
    fn compose_table() {
        let text = "# Cíl\n- [ ] a\n  ![starý](./o.png)\n- [ ] b\n1. [ ] c\nvolný řádek";
        let at = |imgs: &[ImageIn], trimmed| compose(text, imgs, &names(imgs.len()), trimmed);
        // bez kotvy = dnešní připojení na konec (i s popiskem jako alt)
        assert_eq!(at(&[ImageIn::Plain("p".into())], 0), format!("\n{text}\n\n![](./n-1.png)\n"));
        assert_eq!(at(&[img(Some("Rozbitý [header]"), None)], 0), format!("\n{text}\n\n![Rozbitý header](./n-1.png)\n"));
        // pod bod, za jeho stávající přílohu
        assert_eq!(at(&[img(Some("Header"), Some(1))], 0), "\n# Cíl\n- [ ] a\n  ![starý](./o.png)\n  ![Header](./n-1.png)\n- [ ] b\n1. [ ] c\nvolný řádek\n");
        // bez příloh; číslovaný bod odsadí o šířku značky; víc snímků u jednoho bodu v pořadí
        assert_eq!(
            at(&[img(Some("1"), Some(3)), img(Some("2"), Some(3)), img(Some("c"), Some(4))], 0),
            "\n# Cíl\n- [ ] a\n  ![starý](./o.png)\n- [ ] b\n  ![1](./n-1.png)\n  ![2](./n-2.png)\n1. [ ] c\n   ![c](./n-3.png)\nvolný řádek\n"
        );
        // kotva na dvou bodech naráz (dolní se vkládá první)
        assert_eq!(
            at(&[img(Some("a"), Some(1)), img(Some("b"), Some(3))], 0),
            "\n# Cíl\n- [ ] a\n  ![starý](./o.png)\n  ![a](./n-1.png)\n- [ ] b\n  ![b](./n-2.png)\n1. [ ] c\nvolný řádek\n"
        );
        // kotva mimo bod / mimo text = na konec; posun o odstraněné úvodní řádky
        assert_eq!(at(&[img(Some("x"), Some(0))], 0), format!("\n{text}\n\n![x](./n-1.png)\n"));
        assert_eq!(at(&[img(Some("x"), Some(99))], 0), format!("\n{text}\n\n![x](./n-1.png)\n"));
        assert!(at(&[img(Some("x"), Some(3))], 2).contains("- [ ] a\n  ![starý](./o.png)\n  ![x](./n-1.png)\n"));
        // CRLF
        assert_eq!(compose("- [ ] a\r\nb", &[img(Some("x"), Some(0))], &names(1), 0), "\n- [ ] a\r\n  ![x](./n-1.png)\r\nb\n");
    }
}
