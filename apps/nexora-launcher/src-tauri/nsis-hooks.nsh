!macro NSIS_HOOK_PREINSTALL
  ; The single-instance plugin keeps the previous build alive. Close it before
  ; replacing files so the old tray icon cannot remain in the notification area.
  ExecWait 'taskkill /F /IM nexora.exe'
!macroend
