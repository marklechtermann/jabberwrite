; NSIS hooks for the Windows installer (bundle > windows > nsis > installerHooks).

; On update, the updater starts this installer and exits the app right after. The app may
; still be shutting down when the installer checks for it, and the Restart Manager cannot
; kill a process that is already exiting, so the install aborts with "Failed to kill".
; Wait (up to 10 s) until the exe is no longer locked before that check runs.
!macro NSIS_HOOK_PREINSTALL
  ${If} $UpdateMode = 1
  ${AndIf} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
    StrCpy $R9 0
    jabberwrite_wait_for_exit:
      ClearErrors
      FileOpen $R8 "$INSTDIR\${MAINBINARYNAME}.exe" a
      ${IfNot} ${Errors}
        FileClose $R8
      ${ElseIf} $R9 < 40
        IntOp $R9 $R9 + 1
        Sleep 250
        Goto jabberwrite_wait_for_exit
      ${EndIf}
  ${EndIf}
!macroend
