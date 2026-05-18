; K-HUB Custom NSIS Installer Hooks
; Kills old K-HUB and hub-service processes before installing

!macro NSIS_HOOK_PREINSTALL
  ; Kill K-HUB main process
  DetailPrint "Checking for running K-HUB processes..."
  nsExec::ExecToStack 'cmd /c taskkill /F /IM "K-HUB.exe" 2>nul'
  Pop $0
  Pop $1
  StrCmp $0 "0" 0 +2
    DetailPrint "Killed K-HUB.exe"

  ; Kill hub-service process
  nsExec::ExecToStack 'cmd /c taskkill /F /IM "hub-service.exe" 2>nul'
  Pop $0
  Pop $1
  StrCmp $0 "0" 0 +2
    DetailPrint "Killed hub-service.exe"

  ; Wait a moment for processes to fully exit
  Sleep 1000
  DetailPrint "Ready to install."
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Nothing needed post-install
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Kill processes before uninstall too
  DetailPrint "Stopping K-HUB processes before uninstall..."
  nsExec::ExecToStack 'cmd /c taskkill /F /IM "K-HUB.exe" 2>nul'
  Pop $0
  Pop $1
  nsExec::ExecToStack 'cmd /c taskkill /F /IM "hub-service.exe" 2>nul'
  Pop $0
  Pop $1
  Sleep 1000
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Nothing needed post-uninstall
!macroend
