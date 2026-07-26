; Custom NSIS installer hooks for PoE Wishlist Overlay.
;
; On uninstall, after the program files are removed, offer to also delete the
; user's local settings + cached webview data for a completely clean uninstall.

!macro NSIS_HOOK_POSTUNINSTALL
  MessageBox MB_YESNO|MB_ICONQUESTION "Also remove your PoE Wishlist Overlay settings and cached data?$\r$\n$\r$\nChoose Yes for a completely clean uninstall (nothing left behind)." IDNO nsis_keep_userdata
    RMDir /r "$APPDATA\com.xddgaming.wishlist-overlay"
    RMDir /r "$LOCALAPPDATA\com.xddgaming.wishlist-overlay"
  nsis_keep_userdata:
!macroend
