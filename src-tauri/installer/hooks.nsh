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

!macro NSIS_HOOK_POSTINSTALL
  ; stav pro stránku Dokončení — jen v průvodci (tichá aktualizace nic nečte ani nemění)
  ${IfNot} ${Silent}
  ${AndIf} $PassiveMode <> 1
    Call LedgeReadAutostart
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; „Ponechat nastavení“ odškrtnuto: šablona už smazala %APPDATA% a %LOCALAPPDATA%\cz.hyan.ledge (jen cache
  ; WebView2); Ledge má svoje data pod jménem produktu. Poznámky (%USERPROFILE%\Ledge) se nemažou nikdy.
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    SetShellVarContext current
    ${If} $APPDATA != ""
      RmDir /r "$APPDATA\${PRODUCTNAME}"
    ${EndIf}
    ${If} $LOCALAPPDATA != ""
      RmDir /r "$LOCALAPPDATA\${PRODUCTNAME}"
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
