//! Export poznámky (`note_export`): čisté funkce — Markdown bez frontmatteru s titulkem a samostatné
//! HTML (vložené CSS, obrázky jako data URL, odkazy klikatelné). PDF = totéž HTML vytištěné skrytým
//! WebView2 (`pdf.rs`). Dialog a zápis řeší příkaz.
//! Bezpečnost jako `md::to_html`: veškerý obsah je escapovaný, jediné nové značky jsou naše.

use super::edit::NoteImage;
use super::md::{self, esc, heading, is_hr};
use super::store::slug;

/// Formát exportu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Md,
    Html,
    /// HTML z `export_html` vytištěné do PDF (`notes::pdf`)
    Pdf,
}

impl Format {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "md" => Ok(Self::Md),
            "html" => Ok(Self::Html),
            "pdf" => Ok(Self::Pdf),
            _ => Err(trf!("Neznámý formát exportu „{s}“.", "Unknown export format \"{s}\".")),
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Self::Md => "md",
            Self::Html => "html",
            Self::Pdf => "pdf",
        }
    }

    /// Jméno filtru v dialogu „Uložit jako“.
    pub fn filter(self) -> &'static str {
        match self {
            Self::Md => "Markdown",
            Self::Html => "HTML",
            Self::Pdf => "PDF",
        }
    }
}

/// Výchozí jméno souboru: slug poznámky + přípona formátu.
pub fn default_name(body: &str, f: Format) -> String {
    format!("{}.{}", slug(body), f.ext())
}

/// Cesta z dialogu s příponou formátu (dialog ji nemusí doplnit).
pub fn with_ext(path: &str, f: Format) -> String {
    let has = std::path::Path::new(path).extension().is_some_and(|e| e.eq_ignore_ascii_case(f.ext()));
    if has { path.to_string() } else { format!("{path}.{}", f.ext()) }
}

/// Obsah souboru podle formátu (`images` se použijí jen v HTML). U PDF je to HTML, které se vytiskne.
pub fn render(f: Format, title: &str, body: &str, images: &[NoteImage]) -> String {
    match f {
        Format::Md => export_md(title, body),
        Format::Html | Format::Pdf => export_html(title, body, images),
    }
}

/// Markdown bez frontmatteru. Titulek (`# …`) se přidá, jen když tělo nezačíná nadpisem: obyčejný
/// první řádek, který titulek tvoří, se na nadpis povýší, jinak se titulek přidá nad tělo.
pub fn export_md(title: &str, body: &str) -> String {
    let body = body.trim_matches(['\n', '\r']);
    let lines: Vec<&str> = body.lines().collect();
    let Some(at) = lines.iter().position(|l| !l.trim().is_empty()) else {
        return if title.is_empty() { String::new() } else { format!("# {title}\n") };
    };
    let line = lines[at];
    if title.is_empty() || heading(line).is_some() {
        return format!("{body}\n");
    }
    if is_plain_paragraph(line) && md::plain(line) == title {
        let promoted = format!("# {}", line.trim());
        let mut out = lines.clone();
        out[at] = &promoted;
        return format!("{}\n", out.join("\n"));
    }
    format!("# {title}\n\n{body}\n")
}

/// Řádek, který není značka bloku (seznam, citát, kód, obrázek, čára).
fn is_plain_paragraph(line: &str) -> bool {
    let t = line.trim_start();
    !(t.starts_with(['-', '*', '+', '>', '`', '~', '#', '|']) || t.starts_with("![") || t.starts_with(|c: char| c.is_ascii_digit()) || is_hr(line))
}

const CSS: &str = "\
body{margin:0;background:#fff;color:#1f2328;font:16px/1.6 -apple-system,\"Segoe UI\",Roboto,sans-serif}\
main{max-width:46rem;margin:0 auto;padding:2.5rem 1.25rem 4rem}\
h1,h2,h3,h4,h5,h6{line-height:1.25;margin:1.6em 0 .5em}h1{font-size:1.8rem}h2{font-size:1.45rem}h3{font-size:1.2rem}\
p{margin:.8em 0}a{color:#0969da}hr{border:0;border-top:1px solid #d0d7de;margin:1.6em 0}\
blockquote{margin:.8em 0;padding:.1em 1em;border-left:4px solid #d0d7de;color:#57606a}\
code{font:.9em ui-monospace,Consolas,monospace;background:#f2f4f6;padding:.15em .35em;border-radius:4px}\
pre{background:#f2f4f6;padding:.9em 1em;border-radius:6px;overflow:auto}pre code{background:none;padding:0}\
img{max-width:100%;height:auto}ul,ol{padding-left:1.6em}li{margin:.2em 0}\
li[data-task]{list-style:none;margin-left:-1.4em}\
.md-check{display:inline-block;width:.95em;height:.95em;margin-right:.5em;border:1.5px solid #6e7781;border-radius:3px;vertical-align:-.1em;position:relative}\
.md-check.on{background:#1a7f37;border-color:#1a7f37}\
.md-check.on::after{content:\"\";position:absolute;left:.28em;top:.05em;width:.25em;height:.5em;border:solid #fff;border-width:0 2px 2px 0;transform:rotate(45deg)}\
.md-alt{color:#57606a;font-style:italic}\
@media print{main{padding:0}}";

/// Samostatný HTML dokument. `images` = obrázky poznámky jako data URL (`edit::NoteImage`); obrázek,
/// který v nich není (mimo složku poznámky, chybí), zůstane bez zdroje a ukáže se jen jeho popisek.
pub fn export_html(title: &str, body: &str, images: &[NoteImage]) -> String {
    let mut html = md::to_html(body);
    for im in images {
        html = html.replace(&format!("data-src=\"{}\"", esc(&im.src)), &format!("src=\"{}\"", im.data));
    }
    // odkazy: v náhledu je otevírá UI přes `data-href`, v samostatném souboru musí mít `href`
    // (URL je už escapovaná a jen http/https, viz `md::safe_url`)
    let html = html.replace("data-href=\"", "rel=\"noopener noreferrer\" href=\"");
    format!(
        "<!doctype html>\n<html lang=\"cs\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>{}</title>\n<style>{CSS}</style>\n</head>\n<body>\n<main>\n{html}\n</main>\n</body>\n</html>\n",
        esc(title)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_gets_a_title_only_when_missing() {
        let cases: &[(&str, &str, &str)] = &[
            // už má nadpis → beze změny (jen koncový \n)
            ("Nadpis", "# Nadpis\n\ntext\n", "# Nadpis\n\ntext\n"),
            ("Nadpis", "\n# Nadpis\n\ntext", "# Nadpis\n\ntext\n"),
            // titulek je první řádek → povýšit, ne duplikovat
            ("Zavolat Řehořovi", "Zavolat Řehořovi\n\nzítra", "# Zavolat Řehořovi\n\nzítra\n"),
            // titulek z jiného místa → přidat nahoru
            ("Koupit mléko", "- [ ] koupit mléko\n- [ ] chleba", "# Koupit mléko\n\n- [ ] koupit mléko\n- [ ] chleba\n"),
            ("Úkoly", "![](./a.png)\n\n# Úkoly", "# Úkoly\n\n![](./a.png)\n\n# Úkoly\n"),
            ("", "text", "text\n"),
            ("Jen titulek", "", "# Jen titulek\n"),
            ("T", "\r\nahoj\r\n", "# T\n\nahoj\n"),
        ];
        for (title, body, want) in cases {
            assert_eq!(export_md(title, body), *want, "{title:?} {body:?}");
        }
        // `---` v těle zůstane
        assert!(export_md("a", "a\n\n---\nb").contains("\n---\n"));
    }

    #[test]
    fn html_is_standalone_and_escaped() {
        let img = NoteImage { src: "./a-1.png".into(), data: "data:image/png;base64,AAAA".into() };
        let body = "# Čau <b>\n\nodkaz [web](https://a.cz/?q=1&b=2) a ![](./a-1.png) a ![](./b.png)\n\n- [x] hotovo\n1. dvě\n";
        let h = export_html("Čau <b> & \"x\"", body, &[img]);
        assert!(h.starts_with("<!doctype html>"), "{h}");
        assert!(h.contains("<meta charset=\"utf-8\">") && h.contains("<title>Čau &lt;b&gt; &amp; &quot;x&quot;</title>"), "{h}");
        // obrázek poznámky je vložený, cizí zůstane bez zdroje
        assert!(h.contains("<img class=\"md-img\" src=\"data:image/png;base64,AAAA\" alt=\"\">"), "{h}");
        assert!(h.contains("<img class=\"md-img\" data-src=\"./b.png\""), "{h}");
        // odkaz je klikatelný, bez data-href
        assert!(h.contains("rel=\"noopener noreferrer\" href=\"https://a.cz/?q=1&amp;b=2\"") && !h.contains("data-href"), "{h}");
        // úkol a číslovaný seznam
        assert!(h.contains("md-check on") && h.contains("<ol><li>dvě</li></ol>"), "{h}");
        // nic z těla neprojde jako značka
        assert!(!h.contains("<b>") && h.contains("<h1>Čau &lt;b&gt;</h1>"), "{h}");
        assert!(!h.contains("<script"), "{h}");
        // žádné externí zdroje (CDN, @import, url())
        assert!(!h.contains("@import") && !h.contains("url("), "{h}");
    }

    #[test]
    fn names_and_extensions() {
        assert_eq!(Format::parse("md"), Ok(Format::Md));
        assert_eq!(Format::parse("html").unwrap().ext(), "html");
        assert_eq!(Format::parse("pdf"), Ok(Format::Pdf));
        assert!(Format::parse("docx").is_err() && Format::parse("PDF").is_err() && Format::parse("").is_err());
        assert_eq!([Format::Md, Format::Html, Format::Pdf].map(Format::filter), ["Markdown", "HTML", "PDF"]);
        assert_eq!(default_name("# Zápis ze schůzky", Format::Pdf), "zapis-ze-schuzky.pdf");
        assert_eq!(with_ext("C:\\x\\a", Format::Pdf), "C:\\x\\a.pdf");
        assert_eq!(with_ext("a.PDF", Format::Pdf), "a.PDF");
        // PDF tiskne totéž samostatné HTML (obrázky vložené, nic externího)
        assert_eq!(render(Format::Pdf, "T", "# T\n\nx", &[]), render(Format::Html, "T", "# T\n\nx", &[]));
        assert!(render(Format::Pdf, "T", "x", &[]).starts_with("<!doctype html>"));
        assert_eq!(default_name("# Zavolat Řehořovi!\n", Format::Html), "zavolat-rehorovi.html");
        assert_eq!(default_name("", Format::Md), "poznamka.md");
        assert_eq!(with_ext("C:\\x\\a", Format::Md), "C:\\x\\a.md");
        assert_eq!(with_ext("C:\\x\\a.MD", Format::Md), "C:\\x\\a.MD");
        assert_eq!(with_ext("a.md", Format::Html), "a.md.html");
        assert!(render(Format::Md, "T", "x", &[]).starts_with("# T"));
    }
}
