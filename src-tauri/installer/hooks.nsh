; Úpravy NSIS instalátoru Ledge nad šablonou Tauri (`bundle.windows.nsis.installerHooks`), náhled A
; (design-mockups/ledge-instalacka/a-klidny-tmavy.html). Texty jsou v Czech.nsh / English.nsh
; (`customLanguageFiles`), obrázky v sidebar.bmp / header.bmp (scripts/installer-art.mjs ze .svg).
;
; KDY SE TO NAČTE: šablona (installer.nsi z @tauri-apps/cli) tenhle soubor `!include`-uje hned za MUI2.nsh,
; tedy PŘED svými `!define PRODUCTNAME/MAINBINARYNAME…` a před deklarací stránek. Proto:
;   - na nejvyšší úrovni jen věci bez těchto definic (stránky je spotřebují, až je šablona deklaruje);
;   - co je potřebuje, je uvnitř maker — rozvinou se až v místě, kde je šablona vloží.
; Tři stránky se upravují přepsáním makra MUI2 (`!macroundef` + stejné tělo jako v MUI2 plus naše
; definice) — bez kopie celé šablony, ta se s Tauri dál aktualizuje sama:
;   MUI_PAGE_INSTFILES  — nadpis průběhu
;   MUI_PAGE_FINISH     — „Spustit Ledge“ (zaškrtávátko Tauri) + „Spouštět s Windows“ místo zástupce na ploše
;   MUI_UNPAGE_CONFIRM  — „Ponechat nastavení“ = zaškrtávátko Tauri „smazat data“ s obráceným významem
;
; SPOUŠTĚT S WINDOWS — musí sedět s appkou (src-tauri/src/autostart.rs): hodnota Run `Ledge` v
; HKCU\…\CurrentVersion\Run = `"<instalace>\ledge.exe" --autostart` (`command_for`). Pravdou je ale config
; (`autostart` v %APPDATA%\Ledge\config.json): appka ho při každém startu promítne do Run. Instalátor proto
; mění obojí — Run hned (platí od příštího přihlášení i bez spuštění Ledge) a config přes autostart.ps1.
; Výchozí stav zaškrtávátka = co dnes říká config (bez configu zapnuto, jako výchozí v appce).
; Tichá instalace (/S — aktualizace z Ledge) ani /P nic z toho nedělá: stránka Dokončení se přeskočí.

!define LEDGE_INSTALLER_SRC "${__FILEDIR__}"
!define LEDGE_RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"

; ---- výběr jazyka (systémový dialog LangDLL; jazyk ještě není zvolený → dvojjazyčně) ----
!define MUI_LANGDLL_WINDOWTITLE "Jazyk instalace / Installer Language"
!define MUI_LANGDLL_INFO "Vyberte prosím jazyk, ve kterém vás má instalace provést.$\r$\nPlease select the installer language."
!define MUI_LANGDLL_ALLLANGUAGES

; ---- uvítání: první stránka šablony (MUI_PAGE_WELCOME) tyhle definice spotřebuje ----
!define MUI_WELCOMEPAGE_TITLE "$(ledgeWelcomeTitle)"
!define MUI_WELCOMEPAGE_TEXT "$(ledgeWelcomeText)"

; "on" = zaškrtnout „Spouštět s Windows“, "off" = config říká vypnuto (čte LedgeReadAutostart)
Var LedgeAutostartWas

; autostart.ps1 do dočasné složky instalátoru (smaže se s ním)
Function LedgePrepHelper
  InitPluginsDir
  File "/oname=$PLUGINSDIR\ledge-autostart.ps1" "${LEDGE_INSTALLER_SRC}\autostart.ps1"
FunctionEnd

!macro LEDGE_HELPER MODE
  nsExec::Exec '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\ledge-autostart.ps1" -Mode ${MODE}'
!macroend

Function LedgeReadAutostart
  Push $0
  StrCpy $LedgeAutostartWas "on"
  Call LedgePrepHelper
  !insertmacro LEDGE_HELPER get
  Pop $0
  ${If} $0 == "1"
    StrCpy $LedgeAutostartWas "off"
  ${EndIf}
  Pop $0
FunctionEnd

Function LedgeNoop
FunctionEnd

; ---- přechod na jiného vydavatele (klíč výrobce) a nový identifikátor ----
; Šablona Tauri skládá klíče z vydavatele: MANUPRODUCTKEY = Software\<vydavatel>\Ledge (tam si instalátor pamatuje
; složku instalace pro RestorePreviousInstallLocation). Starší vydání mělo jiného vydavatele, proto:
;   - nový instalátor tenhle klíč nenajde a nabídl by výchozí složku → druhá kopie, kdo instaloval jinam;
;   - starý klíč zůstane osiřelý.
; Starého vydavatele NEZADÁVÁME jménem: hledá se podklíč `Software\*\Ledge` s výchozí hodnotou = složka, kde leží
; ledge.exe (jiný produkt takový klíč mít nemůže). Co se NEnajde, se nemění — čistý stroj / už uklizeno = beze změny.
Var LedgeIdx
Var LedgeTmp
Var LedgeDir
Var LedgeOldKey
Var LedgeOldDir
Var LedgeFind
Var LedgePagesRan
Var LedgeLen
Var LedgeOk
Var LedgeMid
Var LedgeI
Var LedgeJ
Var LedgeCh
Var LedgeCa
Var LedgeHit

; sada: $LedgeOldKey = jméno podklíče vydavatele a $LedgeOldDir = složka, nebo obojí prázdné
!macro LEDGE_FIND_OLD_MAKER
  StrCpy $LedgeOldKey ""
  StrCpy $LedgeOldDir ""
  StrCpy $LedgeIdx 0
  ${Do}
    EnumRegKey $LedgeTmp SHCTX "Software" $LedgeIdx
    ${If} $LedgeTmp == ""
      ${Break}
    ${EndIf}
    IntOp $LedgeIdx $LedgeIdx + 1
    ${If} $LedgeTmp != "${MANUFACTURER}"
      ReadRegStr $LedgeDir SHCTX "Software\$LedgeTmp\${PRODUCTNAME}" ""
      ${If} $LedgeDir != ""
      ${AndIf} ${FileExists} "$LedgeDir\${MAINBINARYNAME}.exe"
        StrCpy $LedgeOldKey $LedgeTmp
        StrCpy $LedgeOldDir $LedgeDir
        ${Break}
      ${EndIf}
    ${EndIf}
  ${Loop}
!macroend

; osiřelý klíč starého vydavatele pryč (rodič jen když je prázdný)
!macro LEDGE_DROP_OLD_MAKER
  !insertmacro LEDGE_FIND_OLD_MAKER
  ${If} $LedgeOldKey != ""
    DeleteRegKey SHCTX "Software\$LedgeOldKey\${PRODUCTNAME}"
    DeleteRegKey /ifempty SHCTX "Software\$LedgeOldKey"
  ${EndIf}
!macroend

; složka dat WebView2 starého identifikátoru: maska cz.*.ledge, jen když obsahuje EBWebView a není to nový identifikátor.
; Prostřední segment musí splnit stejné pravidlo jako `is_legacy_name` ve webdata.rs: neprázdný, jen a-z 0-9 - .
; (znak se hledá v povolené sadě; porovnání NSIS nerozlišuje velikost písmen). Maska FindFirst je volnější (mezera aj.).
!macro LEDGE_RM_OLD_PROFILES
  FindFirst $LedgeFind $LedgeTmp "$LOCALAPPDATA\cz.*.ledge"
  ${DoWhile} $LedgeTmp != ""
    StrCpy $LedgeOk 0
    StrLen $LedgeLen $LedgeTmp
    ${If} $LedgeLen > 9
      StrCpy $LedgeOk 1
      StrCpy $LedgeMid $LedgeTmp -6 3
      StrLen $LedgeLen $LedgeMid
      StrCpy $LedgeI 0
      ${While} $LedgeI < $LedgeLen
        StrCpy $LedgeCh $LedgeMid 1 $LedgeI
        StrCpy $LedgeHit 0
        StrCpy $LedgeJ 0
        ${While} $LedgeJ < 38
          StrCpy $LedgeCa "abcdefghijklmnopqrstuvwxyz0123456789-." 1 $LedgeJ
          ${If} $LedgeCh == $LedgeCa
            StrCpy $LedgeHit 1
            ${Break}
          ${EndIf}
          IntOp $LedgeJ $LedgeJ + 1
        ${EndWhile}
        ${If} $LedgeHit = 0
          StrCpy $LedgeOk 0
          ${Break}
        ${EndIf}
        IntOp $LedgeI $LedgeI + 1
      ${EndWhile}
    ${EndIf}
    ${If} $LedgeOk = 1
    ${AndIf} $LedgeTmp != "${BUNDLEID}"
    ${AndIf} ${FileExists} "$LOCALAPPDATA\$LedgeTmp\EBWebView\*.*"
      RmDir /r "$LOCALAPPDATA\$LedgeTmp"
    ${EndIf}
    FindNext $LedgeFind $LedgeTmp
  ${Loop}
  FindClose $LedgeFind
!macroend

; $LedgeDir = složka dřívější instalace, kde opravdu leží ledge.exe, jinak "": nejdřív Uninstall\Ledge\InstallLocation
; (v uvozovkách; zapisuje ho šablona, sdílí ho staré i nové vydání), pak starý klíč vydavatele.
!macro LEDGE_RESOLVE_OLD_DIR
  ReadRegStr $LedgeDir SHCTX "${UNINSTKEY}" "InstallLocation"
  StrLen $LedgeLen $LedgeDir
  StrCpy $LedgeTmp $LedgeDir 1
  ${If} $LedgeTmp == '"'
  ${AndIf} $LedgeLen > 2
    IntOp $LedgeLen $LedgeLen - 2
    StrCpy $LedgeDir $LedgeDir $LedgeLen 1
  ${EndIf}
  ${If} $LedgeDir == ""
  ${OrIfNot} ${FileExists} "$LedgeDir\${MAINBINARYNAME}.exe"
    !insertmacro LEDGE_FIND_OLD_MAKER
    StrCpy $LedgeDir $LedgeOldDir
  ${EndIf}
!macroend

; ÚVODNÍ STRÁNKA (první stránka průvodce; vlastní PRE funkce, dělá i to, co SkipIfPassive ze šablony): nový klíč
; výrobce chybí, ale dřívější vydání je nainstalované → zapsat ho s dřívější složkou a nabídnout ji jako výchozí.
; Proč: krok „odinstalovat před instalací“ (PageLeaveReinstall v šabloně) bere složku pro starý odinstalátor (`_?=`)
; PRÁVĚ z ${MANUPRODUCTKEY}; bez něj dostane prázdnou cestu a ledge.exe nesmaže. Běží dřív než stránka Složka, takže
; cokoli tam uživatel zvolí, už se nepřepisuje (PREINSTALL níž je jen pro tichou instalaci bez stránek).
!macroundef MUI_PAGE_WELCOME
!macro MUI_PAGE_WELCOME
  !verbose push
  !verbose ${MUI_VERBOSE}
  !insertmacro MUI_PAGE_INIT
  !undef MUI_PAGE_CUSTOMFUNCTION_PRE
  !define MUI_PAGE_CUSTOMFUNCTION_PRE LedgeWelcomePre
  !insertmacro MUI_PAGEDECLARATION_WELCOME
  !verbose pop

  ; funkce uvnitř makra: ${MANUPRODUCTKEY} a spol. se definují až v šabloně, makro se rozvine za nimi
  Function LedgeWelcomePre
    StrCpy $LedgePagesRan 1
    ReadRegStr $LedgeDir SHCTX "${MANUPRODUCTKEY}" ""
    ${If} $LedgeDir == ""
      !insertmacro LEDGE_RESOLVE_OLD_DIR
      ${If} $LedgeDir != ""
        WriteRegStr SHCTX "${MANUPRODUCTKEY}" "" $LedgeDir
        StrCpy $INSTDIR $LedgeDir
      ${EndIf}
    ${EndIf}
    ${If} $PassiveMode = 1
      Abort
    ${EndIf}
  FunctionEnd
!macroend

; PREINSTALL běží hned po `SetOutPath $INSTDIR` a PŘED CheckIfAppIsRunning / kopírováním (installer.nsi, Section Install).
; Jen když neproběhla úvodní stránka (tichá instalace /S, /UPDATE): složka se nezměnila (INSTDIR = výchozí) a nový klíč
; chybí → dřívější složka. V průvodci to řeší LedgeWelcomePre dřív, takže výběr na stránce Složka stojí.
!macro NSIS_HOOK_PREINSTALL
  ${If} $LedgePagesRan != 1
  ${AndIf} $INSTDIR == "$LOCALAPPDATA\${PRODUCTNAME}"
    ReadRegStr $LedgeDir SHCTX "${MANUPRODUCTKEY}" ""
    ${If} $LedgeDir == ""
      !insertmacro LEDGE_RESOLVE_OLD_DIR
      ${If} $LedgeDir != ""
      ${AndIf} $LedgeDir != $INSTDIR
        StrCpy $INSTDIR $LedgeDir
        SetOutPath $INSTDIR
      ${EndIf}
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; osiřelý klíč starého vydavatele (nový klíč už šablona zapsala)
  !insertmacro LEDGE_DROP_OLD_MAKER
  ; stav pro stránku Dokončení — jen v průvodci (tichá aktualizace nic nečte ani nemění)
  ${IfNot} ${Silent}
  ${AndIf} $PassiveMode <> 1
    Call LedgeReadAutostart
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; „Ponechat nastavení“ odškrtnuto: šablona už smazala %APPDATA% a CELOU %LOCALAPPDATA%\<nový identifikátor> (profil
  ; WebView2: localStorage, cookies oken „web“, cache; uvnitř ní zakládá migrace `webdata.rs` i rozdělanou kopii
  ; EBWebView.migrating, značky .migrated-from / .migrate-deferred a odsunutý čistý profil EBWebView.clean-*, takže je
  ; netřeba mazat zvlášť); my navíc profil starého identifikátoru (maska, ponechává ho
  ; migrace kvůli návratu na starší verzi). Klíč starého vydavatele tu nehledáme: hledání se opírá o ledge.exe, který
  ; šablona už smazala; osiřelý klíč uklízí POSTINSTALL. Ledge má svoje data
  ; pod jménem produktu. Poznámky (%USERPROFILE%\Ledge) se nemažou nikdy.
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    ${If} $APPDATA != ""
      RmDir /r "$APPDATA\${PRODUCTNAME}"
    ${EndIf}
    ${If} $LOCALAPPDATA != ""
      RmDir /r "$LOCALAPPDATA\${PRODUCTNAME}"
      !insertmacro LEDGE_RM_OLD_PROFILES
    ${EndIf}
  ${EndIf}
!macroend

; ---- průběh instalace ----
!macroundef MUI_PAGE_INSTFILES
!macro MUI_PAGE_INSTFILES
  !verbose push
  !verbose ${MUI_VERBOSE}
  !insertmacro MUI_PAGE_INIT
  !define MUI_PAGE_HEADER_TEXT "$(ledgeInstallingTitle)"
  !define MUI_PAGE_HEADER_SUBTEXT "$(ledgeInstallingSubtitle)"
  !insertmacro MUI_PAGEDECLARATION_INSTFILES
  !verbose pop
!macroend

; ---- dokončení ----
!macroundef MUI_PAGE_FINISH
!macro MUI_PAGE_FINISH
  !verbose push
  !verbose ${MUI_VERBOSE}
  !insertmacro MUI_PAGE_INIT
  !define MUI_FINISHPAGE_TITLE "$(ledgeFinishTitle)"
  !define MUI_FINISHPAGE_TEXT "$(ledgeFinishText)"
  !define MUI_FINISHPAGE_RUN_TEXT "$(ledgeRun)"
  ; druhé zaškrtávátko šablony (zástupce na ploše) = „Spouštět s Windows“; náhled A plochu nemá
  !undef MUI_FINISHPAGE_SHOWREADME_TEXT
  !define MUI_FINISHPAGE_SHOWREADME_TEXT "$(ledgeAutostart)"
  !undef MUI_FINISHPAGE_SHOWREADME_FUNCTION
  !define MUI_FINISHPAGE_SHOWREADME_FUNCTION LedgeNoop
  !define MUI_PAGE_CUSTOMFUNCTION_SHOW LedgeFinishShow
  !define MUI_PAGE_CUSTOMFUNCTION_LEAVE LedgeFinishLeave
  !insertmacro MUI_PAGEDECLARATION_FINISH
  !verbose pop

  Function LedgeFinishShow
    ${If} $LedgeAutostartWas == "off"
      SendMessage $mui.FinishPage.ShowReadme ${BM_SETCHECK} ${BST_UNCHECKED} 0
    ${EndIf}
  FunctionEnd

  ; LEAVE běží PŘED „Spustit Ledge“ — appka se tak spustí už s výsledným configem
  Function LedgeFinishLeave
    Push $0
    Call LedgePrepHelper
    ${NSD_GetState} $mui.FinishPage.ShowReadme $0
    ${If} $0 == ${BST_CHECKED}
      WriteRegStr HKCU "${LEDGE_RUN_KEY}" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --autostart'
      !insertmacro LEDGE_HELPER on
    ${Else}
      DeleteRegValue HKCU "${LEDGE_RUN_KEY}" "${PRODUCTNAME}"
      !insertmacro LEDGE_HELPER off
    ${EndIf}
    Pop $0 ; výsledek autostart.ps1 (při chybě config zůstane, jak byl)
    Pop $0
  FunctionEnd
!macroend

; ---- potvrzení odinstalace ----
!macroundef MUI_UNPAGE_CONFIRM
!macro MUI_UNPAGE_CONFIRM
  !verbose push
  !verbose ${MUI_VERBOSE}
  !insertmacro MUI_UNPAGE_INIT
  !define MUI_PAGE_HEADER_TEXT "$(ledgeUninstallTitle)"
  !define MUI_PAGE_HEADER_SUBTEXT "$(ledgeUninstallSubtitle)"
  !define MUI_UNCONFIRMPAGE_TEXT_TOP "$(ledgeUninstallText)"
  !define MUI_UNCONFIRMPAGE_TEXT_LOCATION "$(ledgeUninstallFolder)"
  ; šablona sem dala un.ConfirmShow/Leave (zaškrtávátko „smazat data“) — obalíme je, ne nahradíme
  !undef MUI_PAGE_CUSTOMFUNCTION_SHOW
  !define MUI_PAGE_CUSTOMFUNCTION_SHOW un.LedgeConfirmShow
  !undef MUI_PAGE_CUSTOMFUNCTION_LEAVE
  !define MUI_PAGE_CUSTOMFUNCTION_LEAVE un.LedgeConfirmLeave
  !insertmacro MUI_PAGEDECLARATION_CONFIRM
  !verbose pop

  Function un.LedgeConfirmShow
    Call un.ConfirmShow
    ; text zaškrtávátka je $(deleteAppData) = „Ponechat nastavení“, výchozí zaškrtnuto
    SendMessage $DeleteAppDataCheckbox ${BM_SETCHECK} ${BST_CHECKED} 0
    ; vysvětlivka pod ním (souřadnice jako u zaškrtávátka v šabloně: px při 96 DPI, škálované DPI okna)
    FindWindow $1 "#32770" "" $HWNDPARENT
    System::Call "user32::GetDpiForWindow(p r1) i .r2"
    IntOp $4 18 * $2
    IntOp $5 126 * $2
    IntOp $6 380 * $2
    IntOp $7 34 * $2
    IntOp $4 $4 / 96
    IntOp $5 $5 / 96
    IntOp $6 $6 / 96
    IntOp $7 $7 / 96
    System::Call 'user32::CreateWindowEx(i ${__NSD_Label_EXSTYLE}, w "${__NSD_Label_CLASS}", w "$(ledgeKeepSettingsHint)", i ${__NSD_Label_STYLE}, i r4, i r5, i r6, i r7, p r1, i0, i0, i0) i .s'
    Pop $3
    SendMessage $HWNDPARENT ${WM_GETFONT} 0 0 $0
    SendMessage $3 ${WM_SETFONT} $0 1
  FunctionEnd

  ; šablona maže data při stavu 1 — u „Ponechat“ to znamená NEzaškrtnuto. Bez stránky (/S, /P, odinstalace
  ; před přeinstalací) zůstane stav prázdný = nic se nemaže.
  Function un.LedgeConfirmLeave
    Call un.ConfirmLeave
    ${If} $DeleteAppDataCheckboxState = ${BST_CHECKED}
      StrCpy $DeleteAppDataCheckboxState 0
    ${Else}
      StrCpy $DeleteAppDataCheckboxState 1
    ${EndIf}
  FunctionEnd
!macroend
