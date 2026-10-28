// Ledge — landing page: jazyk (EN v HTML, CS zde), motiv, odkaz na instalátor z posledního vydání.
// Žádné cookies, žádná analytika. Jediný síťový dotaz: veřejné GitHub API vydání (jméno a velikost instalátoru).
(function () {
  "use strict";
  var REPO = "jiri-hyan/ledge-app";
  var root = document.documentElement;

  var CS = {
    "skip": "Přeskočit na obsah",
    "nav.claude": "Claude Code", "nav.features": "Funkce", "nav.looks": "Vzhled", "nav.install": "Instalace",
    "hero.eyebrow": "Zdarma a open source · Windows 10 a 11",
    "hero.title": "Celý pracovní den <span class=\"grad-text\">jeden pohyb myší</span> od tebe.",
    "hero.lead": "Ledge je šuplík u horního okraje obrazovky Windows: projekty a dev servery, poznámky, rychlé akce — a všechny session Claude Code se schvalováním, otázkami a limity, které vyřídíš bez hledání terminálu.",
    "cta.download": "Stáhnout pro Windows", "cta.sub": "Zdarma · instalátor x64 · bez práv správce",
    "cta.star": "Hvězdička na GitHubu", "cta.issue": "Navrhnout funkci",
    "facts.focus": "Nikdy nebere fokus", "facts.game": "Při hře na celou obrazovku mlčí",
    "facts.telemetry": "Bez telemetrie a bez účtu", "facts.mit": "Licence MIT",
    "hero.note": "Skutečné rozhraní Ledge s ukázkovými daty (vymyšlené projekty). Vysune se, když se myší dotkneš horního okraje obrazovky.",
    "why.kicker": "Proč Ledge", "why.title": "Pořád po ruce. Nikdy v cestě.",
    "why.lead": "Staví ho jeden vývojář pro svou každodenní práci na Windows s Claude Code — a sdílí ho jako open source, takže ho můžeš používat, jak je, doladit si ho, nebo forknout.",
    "why1.t": "Neruší", "why1.p": "Tenký proužek u horního okraje, dokud ho nepotřebuješ. Nikdy ti nevezme klávesnici, při hře na celou obrazovku mlčí a v klidu má rozpočet procesoru pod 0,1 %.",
    "why2.t": "Claude Code konečně vidět i na Windows", "why2.p": "Všechny session na jeden pohled, schválení a odpovědi Claudovi přímo ze šuplíku a limity 5h i týdne — bez přepínání mezi terminály.",
    "why3.t": "Tvoje, lokální a otevřené", "why3.p": "Žádný účet, žádný cloud, žádná telemetrie. Poznámky jsou obyčejné soubory Markdown ve tvé složce, nastavení jeden JSON a zdroják je na GitHubu pod licencí MIT.",
    "claude.kicker": "Claude Code na Windows", "claude.title": "Claude se ptá. Ty odpovíš ze šuplíku.",
    "claude.lead": "Ledge se ke Claude Code připojí přes jeho oficiální hooky a stavový řádek. Když tě Claude potřebuje, vysune se náhled — a editor ti při tom nevezme fokus.",
    "c1": "<b>Schvalování.</b> Žádost o povolení ukáže příkaz nebo diff; Povolit nebo Zamítnout jedním klikem. Když neodpovíš, žádost se vrátí do terminálu — Ledge sám nikdy nic neschválí.",
    "c2": "<b>Otázky a plány.</b> Odpovídej na <code>AskUserQuestion</code> (jedna volba, více voleb nebo vlastní text) a posuzuj plány: schválit, nebo vrátit s připomínkou.",
    "c3": "<b>Všechny session.</b> Aplikace i CLI, včetně subagentů: čeká na tebe, pracuje, nečinná, nedokončená práce. Klik přepne do jejího okna.",
    "c4": "<b>Limity a tempo.</b> Spotřeba 5h a týdenního okna s odpočtem resetu, volitelná ryska tempa a připomínka, když se plné okno obnoví.",
    "c5": "<b>Aktivita a útrata.</b> Co Claude právě dělá a odhad útraty v cenách API spočítaný z tvých lokálních přepisů.",
    "c6": "<b>Nastaví ho Claude.</b> Pošli Claudovi návod; přes MCP přidá tvoje projekty, odkazy a rozložení — až připojení sám zapneš.",
    "claude.note": "<b>Schválit smí jen člověk.</b> Záměrně neexistuje API, nástroj MCP ani volba, přes kterou by za tebe schvaloval kdokoli jiný.",
    "feat.kicker": "Všechno v jednom šuplíku", "feat.title": "Jeden šuplík, karty podle tebe.",
    "feat.lead": "Zapni jen karty, které chceš. Rozlož je na vlastní záložky ve čtyřech velikostech.",
    "f1.t": "Projekty a dev servery", "f1.p": "Dlaždice na projekt: spustí dev server nebo aplikaci, ukáže port a log, Otevřít nebo Zastavit. Loga se najdou v projektu samy; necommitnutá a nepushnutá práce se označí.",
    "f2.t": "Poznámky a úkoly", "f2.p": "Rychlé poznámky v Markdownu s kategoriemi, úkoly, obrázky a připomínkami. Obyčejné soubory <code>.md</code>, volitelná synchronizace přes tvoje vlastní soukromé git repo.",
    "f3.t": "Paleta příkazů", "f3.p": "Weby, složky, soubory, aplikace a příkazy — vše hledatelné přes <kbd>Ctrl</kbd>+<kbd>K</kbd>, k tomu historie schránky, která vynechá hesla a tokeny.",
    "f4.t": "Přihrádka na soubory", "f4.p": "Pusť soubory na proužek, odlož je a pak je vytáhni tam, kde je potřebuješ.",
    "f5.t": "Osobní záložka", "f5.p": "Hodiny, počasí, kalendář (ICS, Outlook, účet Microsoft nebo Google), právě hraje s hlasitostí a ztlumením mikrofonu — odděleně od práce.",
    "f6.t": "Widgety kdekoli", "f6.p": "Připni kteroukoli kartu na plochu nebo druhý monitor, nebo si napiš vlastní widget jako skript ve stylu xbar. Karty GitHub a Systém jsou v ceně.",
    "looks.kicker": "Výřez (notch) pro Windows", "looks.title": "Domov pro tvůj stav ve stylu Dynamic Island.",
    "looks.lead": "NotchNook, Boring Notch a Alcove jsou jen pro macOS. Ledge tu myšlenku přináší na Windows: vyber si, jak zavřený Ledge u horního okraje vypadá — a naroste, jen když se opravdu něco děje.",
    "l.island": "Černá kapsle, která se při dění roztáhne", "l.notch": "Výřez nahoře uprostřed", "l.line": "3 px barvy přes celou obrazovku",
    "l.dots": "Claude, servery, repa, ty", "l.bar": "Stavová lišta přes celou šířku", "l.corner": "Malý odznak v rohu",
    "l.ghost": "Neviditelný, dokud tě Claude nepotřebuje", "l.custom": "Slož si vlastní z dílků",
    "looks.note": "Tvary jsou ilustrace. Dále: Ikony, Hodiny, Průběh, Ticker, Minimum a Trio.",
    "ch1": "13 designových stylů", "ch2": "16 barevných motivů + vlastní", "ch3": "9 animací otevření včetně tekuté kapsle",
    "ch4": "Kompaktní náhled nebo ostrůvek po najetí", "ch5": "Čeština a angličtina",
    "g3": "Ledge Studio: každá změna platí hned a jde vrátit. Ukázková data.",
    "g4": "Záložka Osobní: počasí, Právě hraje, kalendář, odkazy a poznámky. Ukázková data.",
    "g5": "Ctrl+K: jedno okno pro akce, projekty, poznámky i nastavení. Ukázková data.",
    "lr.closed": "Zavřený", "lr.open": "Otevřený",
    "lr.cclosed": "Osm vzhledů zavřeného Ledge u horního okraje. Ukázková data.",
    "lr.copen": "Šest stylů otevřeného šuplíku, stejná data. Ukázková data.",
    "alt.lclosed": "Osm vzhledů zavřeného Ledge", "alt.lopen": "Šest stylů otevřeného šuplíku se stejnými daty",
    "alt.personal": "Záložka Osobní s počasím, Právě hraje, kalendářem a poznámkami", "alt.palette": "Paleta Ctrl+K v šuplíku Ledge",
    "y.kicker": "Uprav si ho", "y.title": "Dolaď ho. Nebo forkni.",
    "y.lead": "Ledge se nesnaží sedět každému hned po instalaci. Má hodně nastavení — a když ti pořád nesedí, kód je tvůj a můžeš ho změnit.",
    "y1.t": "Nejdřív nastavení", "y1.p": "Karty a záložky, vzhled zavřeného stavu, designové styly i vlastní barevné motivy, zkratky, rychlé akce a widgety ze skriptů — v aplikaci, nebo v jednom <code>config.json</code> s JSON schématem.",
    "y2.t": "Forkni ho s Claude Code", "y2.p": "Licence MIT a repo připravené pro Claude Code: <code>CLAUDE.md</code> říká, kde začít u nové karty, config klíče, překladu nebo panelu Nastavení, a mapy kódu vedou z každé oblasti k souborům. Otevři svůj fork a řekni si o změnu.",
    "y3.t": "Nebo ho nech nastavit Clauda", "y3.p": "<code>AGENTS.md</code> popisuje config, formát poznámek, lokální API a MCP — Claude tak nastaví tvoje projekty, odkazy a rozložení bez sahání do kódu.",
    "y.fork": "Forknout na GitHubu",
    "inst.kicker": "Instalace za minutu", "inst.title": "Tři kroky. Bez práv správce.",
    "s1.t": "Stáhni", "s1.p": "Stáhni <code>Ledge_&lt;verze&gt;_x64-setup.exe</code> z posledního vydání na GitHubu. Nainstaluje se jen pro tvého uživatele do <code>%LOCALAPPDATA%\\Ledge</code>.",
    "s2.t": "Projdi SmartScreen", "s2.p": "Instalátor zatím nemá podpis kódu, takže Windows hlásí neznámého vydavatele. Klikni na <b>Další informace → Přesto spustit</b>.",
    "s3.t": "Dotkni se horního okraje", "s3.p": "Najeď myší k hornímu okraji obrazovky. Při prvním spuštění krátký průvodce najde tvoje repa, aplikace a Claude Code.",
    "ss.t": "Proč mě Windows varují?",
    "ss.p1": "SmartScreen varuje u každého instalátoru bez placeného certifikátu Authenticode — to znamená „neznámý vydavatel“, ne že by našel virus. Ledge ten certifikát zatím nemá; je v plánu.",
    "ss.p2": "Co si můžeš ověřit už dnes: každé vydání uvádí SHA256 instalátoru a zdrojový kód je veřejný. Aktualizace uvnitř Ledge jdou dál — před spuštěním se ověří proti podpisovému klíči zabudovanému v aplikaci.",
    "ss.p3": "Výsledek porovnej se souborem <code>SHA256SUMS.txt</code> ve vydání.",
    "ss.mock": "Ilustrace dialogu Windows (anglicky): klikni na ① More info (Další informace), pak se objeví ② Run anyway (Přesto spustit).",
    "copy": "Kopírovat", "copied": "Zkopírováno",
    "req": "Požadavky: Windows 10 nebo 11 (x64) s Microsoft Edge WebView2 (ve Windows 11 už je). Claude Code je volitelný.",
    "priv.kicker": "Soukromí", "priv.title": "Bez telemetrie. Bez účtu. Bez cloudu.",
    "priv.lead": "Ledge jde na síť jen kvůli věcem, které vidíš a můžeš vypnout.",
    "pv1.t": "Co jde do sítě", "pv1.p": "Kontrola aktualizací (vydání tohoto repa na GitHubu, výchozí jednou za 6 hodin — jde vypnout). Počasí z Open-Meteo, jen když zadáš polohu. Jednorázově favicon webového zástupce. Kalendáře, pošta a synchronizace poznámek jen když je nastavíš. Kontrola stavu Anthropicu je ve výchozím stavu vypnutá.",
    "pv2.t": "Co zůstává v počítači", "pv2.p": "Session Claude Code, hooky, limity a odhad útraty se čtou lokálně. Lokální API poslouchá jen na 127.0.0.1 a požadavky z prohlížeče odmítá. Přihlášení k účtům probíhá v tvém prohlížeči a tokeny jsou ve Správci pověření Windows.",
    "faq.title": "Otázky",
    "q1": "Je Ledge zdarma?", "a1": "Ano. Ledge je zdarma a open source pod licencí MIT. Žádná placená verze, žádný účet.",
    "q2": "Potřebuju Claude Code?", "a2": "Ne. Bez Claude Code je Ledge spouštěč projektů, správce dev serverů, poznámky a šuplík rychlých akcí. S Claude Code navíc ukazuje session, schválení, otázky a limity.",
    "q3": "Může Ledge schvalovat za Clauda sám?", "a3": "Ne — záměrně. Žádost schválí jen tvůj klik. Když neodpovíš včas, žádost se vrátí do terminálu; nic se neschválí samo.",
    "q4": "Bude mi překážet při psaní nebo hraní?", "a4": "Ledge si při samovolném otevření nikdy nevezme klávesnici a při hře na celou obrazovku mlčí.",
    "q5": "Je to NotchNook nebo Dynamic Island pro Windows?", "a5": "V duchu ano: šuplík a stavová plocha nahoře na obrazovce, se vzhledy Ostrůvek a Výřez a tekutou animací otevření. Je ale stavěný pro Windows a pro vývojářskou práci, ne jako kopie.",
    "q6": "Jak fungují aktualizace?", "a6": "Ledge kontroluje vydání tohoto repa na GitHubu a ukáže čip „Nová verze“. Instalátor stáhne přes HTTPS, ověří SHA256 a podpis vydavatele a nainstaluje až po tvém kliknutí.",
    "q7": "macOS nebo Linux?", "a7": "Zatím ne. Ledge je jen pro Windows (10 a 11, x64).",
    "q8": "Jak ho odinstaluju?", "a8": "Nastavení Windows → Aplikace → Ledge. Nastavení a poznámky ve tvých složkách zůstanou.",
    "road.kicker": "Co dál", "road.title": "Plán", "road.lead": "Rozpracované — bez slibovaných termínů.",
    "rt.wip": "Rozpracováno", "rt.plan": "V plánu",
    "r1": "<b>Ledge Studio</b> — přestavěné nastavení vzhledu a rozložení.",
    "r2": "<b>winget a Scoop</b> — instalace jedním příkazem.",
    "r3": "<b>Podpis kódu Windows</b> (Authenticode) — aby SmartScreen přestal varovat.",
    "r4": "<b>Aktivita Clauda</b> — jasnější fáze a chyby v živé kartě aktivity.",
    "final.title": "Dej hornímu okraji obrazovky práci.",
    "final.p": "Zdarma, open source a pod 7 MB. Jestli ti Ledge pomáhá, hvězdička na GitHubu pomůže ostatním ho najít — a nápady i hlášení chyb jsou moc vítané.",
    "ft.releases": "Vydání", "ft.issues": "Hlášení", "ft.license": "Licence MIT",
    "ft.note": "Nesouvisí s Anthropicem ani Microsoftem. Žádné cookies, žádná analytika.",
    "alt.drawer": "Šuplík Ledge: dlaždice projektů s větvemi gitu, rychlé akce, repa s necommitnutými změnami a poznámky",
    "alt.question": "Otázka z Claude Code v šuplíku Ledge s možnostmi a tlačítkem Odeslat",
    "alt.settings": "Nastavení Ledge s živým náhledem vzhledu",
    "meta.title": "Ledge — šuplík pro Windows s Claude Code uvnitř",
    "dl.sub": "Zdarma · v{v} · {mb} MB · bez práv správce"
  };
  var EN_SUB = "Free · v{v} · {mb} MB · no admin rights";

  // EN texty si pamatujeme z HTML, aby přepínání fungovalo oběma směry
  var nodes = document.querySelectorAll("[data-i18n]");
  var altNodes = document.querySelectorAll("[data-i18n-alt]");
  var en = {}, enAlt = {}, enTitle = document.title, rel = null;
  nodes.forEach(function (n, i) { en[i] = n.innerHTML; });
  altNodes.forEach(function (n, i) { enAlt[i] = n.getAttribute("alt"); });

  function cs() { return root.lang === "cs"; }
  function applyLang() {
    var c = cs();
    nodes.forEach(function (n, i) { var k = n.getAttribute("data-i18n"); n.innerHTML = c && CS[k] != null ? CS[k] : en[i]; });
    altNodes.forEach(function (n, i) { var k = n.getAttribute("data-i18n-alt"); n.setAttribute("alt", c && CS[k] ? CS[k] : enAlt[i]); });
    document.title = c ? CS["meta.title"] : enTitle;
    document.getElementById("lang").textContent = c ? "EN" : "CS";
    paintRelease();
  }

  function store(k, v) { try { localStorage[k] = v; } catch (e) {} }
  document.getElementById("lang").addEventListener("click", function () {
    root.lang = cs() ? "en" : "cs"; store("ledgeLang", root.lang); applyLang();
  });
  document.getElementById("theme").addEventListener("click", function () {
    root.dataset.theme = root.dataset.theme === "light" ? "dark" : "light"; store("ledgeTheme", root.dataset.theme);
  });

  document.querySelectorAll("[data-copy]").forEach(function (b) {
    b.addEventListener("click", function () {
      var code = b.parentNode.querySelector("code");
      if (!code || !navigator.clipboard) return;
      navigator.clipboard.writeText(code.textContent).then(function () {
        b.textContent = cs() ? CS.copied : "Copied";
        setTimeout(function () { b.textContent = cs() ? CS.copy : "Copy"; }, 1500);
      });
    });
  });

  // Přímý odkaz na instalátor posledního vydání. Bez odpovědi zůstane odkaz na stránku vydání.
  function paintRelease() {
    if (!rel) return;
    document.querySelectorAll("a[data-dl]").forEach(function (a) { a.href = rel.url; });
    var tpl = cs() ? CS["dl.sub"] : EN_SUB;
    document.querySelectorAll("[data-dl-sub]").forEach(function (s) { s.textContent = tpl.replace("{v}", rel.v).replace("{mb}", cs() ? rel.mb.replace(".", ",") : rel.mb); });
  }
  function useRelease(r) {
    var a = (r.assets || []).filter(function (x) { return /-setup\.exe$/i.test(x.name); })[0];
    if (!a || !/^v\d+\.\d+\.\d+$/.test(r.tag_name)) return;
    var mb = (a.size / 1048576).toFixed(1);
    rel = { url: a.browser_download_url, v: r.tag_name.slice(1), mb: mb };
    paintRelease();
  }
  var cached = null;
  try { cached = JSON.parse(sessionStorage.ledgeRel || "null"); } catch (e) {}
  if (cached) useRelease(cached);
  else if (window.fetch) {
    fetch("https://api.github.com/repos/" + REPO + "/releases/latest", { headers: { Accept: "application/vnd.github+json" } })
      .then(function (r) { return r.ok ? r.json() : null; })
      .then(function (r) {
        if (!r) return;
        var slim = { tag_name: r.tag_name, assets: (r.assets || []).map(function (a) { return { name: a.name, size: a.size, browser_download_url: a.browser_download_url }; }) };
        try { sessionStorage.ledgeRel = JSON.stringify(slim); } catch (e) {}
        useRelease(slim);
      })
      .catch(function () {});
  }

  // galerie „Zavřený / Otevřený“ + video v hlavičce se při „omezit pohyb“ nepřehrává
  var seg = document.getElementById("looks-real");
  if (seg) seg.querySelectorAll("button[data-pane]").forEach(function (b) {
    b.addEventListener("click", function () {
      seg.querySelectorAll("button[data-pane]").forEach(function (x) { x.setAttribute("aria-pressed", String(x === b)); });
      seg.querySelectorAll("figure[data-pane]").forEach(function (f) { f.hidden = f.getAttribute("data-pane") !== b.getAttribute("data-pane"); });
    });
  });
  var hv = document.getElementById("hero-video");
  if (hv && window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches) { hv.removeAttribute("autoplay"); hv.pause(); }

  if (cs()) applyLang(); else document.getElementById("lang").textContent = "CS";
})();
