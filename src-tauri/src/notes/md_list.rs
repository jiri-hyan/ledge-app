//! Zásobník otevřených seznamů pro `md::to_html`: skutečně vnořené `<ul>`/`<ol>` v `<li>`, `start`
//! u číslovaných, pokračovací řádky patří k poslední položce. Čisté, jen skládá HTML (obsah řádků
//! escapuje volající přes `md::inline`).

/// Nejhlubší vnoření (víc jen přidá položku na stejné úrovni) — brání absurdně hlubokému DOM.
const MAX_DEPTH: usize = 8;

struct Level {
    /// Šířka odsazení značky položky (mezera 1, tabulátor 4).
    indent: usize,
    ordered: bool,
    /// Co zavře otevřenou položku (`</li>` / `</span></li>` u úkolu); prázdné = žádná není otevřená.
    suffix: &'static str,
}

#[derive(Default)]
pub(super) struct Lists(Vec<Level>);

/// Šířka úvodního odsazení řádku (mezera 1, tabulátor 4).
pub(super) fn indent_width(line: &str) -> usize {
    line.chars().take_while(|c| matches!(c, ' ' | '\t')).map(|c| if c == '\t' { 4 } else { 1 }).sum()
}

impl Lists {
    pub(super) fn is_open(&self) -> bool {
        !self.0.is_empty()
    }

    fn push(&mut self, out: &mut String, indent: usize, num: Option<u32>) {
        match num {
            Some(n) if n != 1 => out.push_str(&format!("<ol start=\"{n}\">")),
            Some(_) => out.push_str("<ol>"),
            None => out.push_str("<ul>"),
        }
        self.0.push(Level { indent, ordered: num.is_some(), suffix: "" });
    }

    fn pop(&mut self, out: &mut String) {
        if let Some(l) = self.0.pop() {
            out.push_str(l.suffix);
            out.push_str(if l.ordered { "</ol>" } else { "</ul>" });
        }
    }

    pub(super) fn close_all(&mut self, out: &mut String) {
        while self.is_open() {
            self.pop(out);
        }
    }

    /// Připraví místo pro novou položku s odsazením `w` (`num` = číslo u číslované). Po volání
    /// volající vypíše `<li…>…` a zavolá `set_item` s příponou, která položku zavře.
    pub(super) fn open_item(&mut self, out: &mut String, w: usize, num: Option<u32>) {
        while self.0.len() > 1 && self.0.last().is_some_and(|l| l.indent > w) {
            self.pop(out);
        }
        let Some((indent, ordered)) = self.0.last().map(|l| (l.indent, l.ordered)) else {
            return self.push(out, w, num);
        };
        if w >= indent + 2 && self.0.len() < MAX_DEPTH {
            // vnořený seznam uvnitř ještě otevřené položky
            return self.push(out, w, num);
        }
        if ordered != num.is_some() {
            self.pop(out);
            return self.push(out, w, num);
        }
        if let Some(top) = self.0.last_mut() {
            out.push_str(top.suffix);
            top.suffix = "";
        }
    }

    pub(super) fn set_item(&mut self, suffix: &'static str) {
        if let Some(l) = self.0.last_mut() {
            l.suffix = suffix;
        }
    }

    /// Pokračovací řádek poslední položky (`<br>` + obsah doplní volající).
    pub(super) fn continue_item(&self, out: &mut String) -> bool {
        let open = self.0.last().is_some_and(|l| !l.suffix.is_empty());
        if open {
            out.push_str("<br>");
        }
        open
    }
}

#[cfg(test)]
mod tests {
    use crate::notes::md::to_html;

    #[test]
    fn list_table() {
        let cases: &[(&str, &str)] = &[
            // čísla: 1,1,2 → jedno <ol>, prohlížeč čísluje sám
            ("1. a
1. b
2. c", "<ol><li>a</li><li>b</li><li>c</li></ol>"),
            // prázdný řádek mezi položkami seznam nezavře
            ("1. a

2. b", "<ol><li>a</li><li>b</li></ol>"),
            ("3. x", "<ol start=\"3\"><li>x</li></ol>"),
            ("0. x", "<ol start=\"0\"><li>x</li></ol>"),
            ("2) a
3) b", "<ol start=\"2\"><li>a</li><li>b</li></ol>"),
            // vnořený stejný typ, pak sourozenec
            ("1. a
   1. b
2. c", "<ol><li>a<ol><li>b</li></ol></li><li>c</li></ol>"),
            // vnořený jiný typ
            ("- a
  1. b", "<ul><li>a<ol><li>b</li></ol></li></ul>"),
            ("1. a
   - b
   - c
2. d", "<ol><li>a<ul><li>b</li><li>c</li></ul></li><li>d</li></ol>"),
            // vnořený číslovaný nezačínající jedničkou
            ("- a
  5. b", "<ul><li>a<ol start=\"5\"><li>b</li></ol></li></ul>"),
            // tabulátor = vnoření
            ("- a
	- b", "<ul><li>a<ul><li>b</li></ul></li></ul>"),
            // vícenásobný návrat o víc úrovní
            ("- a
  - b
    - c
- d", "<ul><li>a<ul><li>b<ul><li>c</li></ul></li></ul></li><li>d</li></ul>"),
            // pokračovací řádek (odsazený i bez odsazení) patří k položce
            ("1. a
   dál
2. b", "<ol><li>a<br>dál</li><li>b</li></ol>"),
            ("- a
lazy", "<ul><li>a<br>lazy</li></ul>"),
            ("1. a

   odstavec
2. b", "<ol><li>a<br>odstavec</li><li>b</li></ol>"),
            // prázdný řádek + neodsazený text seznam ukončí
            ("1. a

text", "<ol><li>a</li></ol><p>text</p>"),
            // změna typu na stejné úrovni = nový seznam
            ("- a
1. b", "<ul><li>a</li></ul><ol><li>b</li></ol>"),
            ("1. a
- b", "<ol><li>a</li></ol><ul><li>b</li></ul>"),
            // nadpis / citát / hr seznam ukončí
            ("- a
# H", "<ul><li>a</li></ul><h1>H</h1>"),
            ("- a
> q", "<ul><li>a</li></ul><blockquote>q</blockquote>"),
            ("- a

---", "<ul><li>a</li></ul><hr>"),
            // odstavec před seznamem
            ("text
- a", "<p>text</p><ul><li>a</li></ul>"),
            // blok kódu: odsazený zůstane v položce, neodsazený seznam ukončí
            ("1. a
   ```
   x
   ```
2. b", "<ol><li>a<pre><code>   x</code></pre></li><li>b</li></ol>"),
            ("- a
```
x
```", "<ul><li>a</li></ul><pre><code>x</code></pre>"),
            // úkoly (i vnořené a smíšené s čísly) nesou data-line řádku těla
            (
                "- [ ] a
  1. [x] b",
                "<ul><li data-task=\"1\"><span class=\"md-check\" role=\"checkbox\" aria-checked=\"false\" data-line=\"0\"></span><span>a<ol><li data-task=\"1\"><span class=\"md-check on\" role=\"checkbox\" aria-checked=\"true\" data-line=\"1\"></span><span>b</span></li></ol></span></li></ul>",
            ),
        ];
        for (src, want) in cases {
            assert_eq!(to_html(src), *want, "{src:?}");
        }
    }

    #[test]
    fn list_html_is_well_nested_and_escaped() {
        // bez ohledu na vstup se značky otevírají a zavírají vyváženě a obsah je escapovaný
        let src = "- a <b>
  1. b

     c
- [ ] d
    - e
 1. f
	x
2. g

1. h";
        let h = to_html(src);
        for tag in ["ul", "ol", "li"] {
            assert_eq!(h.matches(&format!("<{tag}")).count(), h.matches(&format!("</{tag}>")).count(), "{tag}: {h}");
        }
        assert!(h.contains("a &lt;b&gt;") && !h.contains("<b>"), "{h}");
        // hloubka je omezená
        let deep: String = (0..40).map(|k| format!("{}- x
", " ".repeat(k * 2))).collect();
        assert!(to_html(&deep).matches("<ul>").count() <= 8);
    }
}
