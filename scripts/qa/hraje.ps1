# QA: zjisti, jestli uzivatel hraje (SHQueryUserNotificationState 3/4) a vypise cislo stavu. ASCII only.
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class QaQuns { [DllImport("shell32.dll")] public static extern int SHQueryUserNotificationState(out int s); }
"@
$s = 0; [void][QaQuns]::SHQueryUserNotificationState([ref]$s)
$s
