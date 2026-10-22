!include "${__FILEDIR__}\locale-strings.nsh"
; Register this per-user viewer without replacing .ext defaults or UserChoice.
; The normal Tauri fileAssociations macro overwrites defaults and leaves the
; executable path unquoted. Use documented installer hooks for our own keys.
!define SMART_PHOTO_PROGID "SmartPhotoIndexer.Image"
!define SMART_PHOTO_CAPABILITIES "Software\SmartPhotoIndexer\Capabilities"

!macro SMART_PHOTO_REGISTER_EXTENSION EXT
  WriteRegStr HKCU "Software\Classes\.${EXT}\OpenWithProgids" "${SMART_PHOTO_PROGID}" ""
  WriteRegStr HKCU "${SMART_PHOTO_CAPABILITIES}\FileAssociations" ".${EXT}" "${SMART_PHOTO_PROGID}"
  WriteRegStr HKCU "Software\Classes\Applications\${MAINBINARYNAME}.exe\SupportedTypes" ".${EXT}" ""
!macroend

!macro SMART_PHOTO_UNREGISTER_EXTENSION EXT
  DeleteRegValue HKCU "Software\Classes\.${EXT}\OpenWithProgids" "${SMART_PHOTO_PROGID}"
  DeleteRegKey /ifempty HKCU "Software\Classes\.${EXT}\OpenWithProgids"
  DeleteRegKey /ifempty HKCU "Software\Classes\.${EXT}"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "Software\Classes\${SMART_PHOTO_PROGID}" "" "$(SmartPhotoImageType)"
  WriteRegStr HKCU "Software\Classes\${SMART_PHOTO_PROGID}\DefaultIcon" "" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0'
  WriteRegStr HKCU "Software\Classes\${SMART_PHOTO_PROGID}\shell\open\command" "" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\"'
  WriteRegStr HKCU "Software\Classes\Applications\${MAINBINARYNAME}.exe" "FriendlyAppName" "${PRODUCTNAME}"
  WriteRegStr HKCU "Software\Classes\Applications\${MAINBINARYNAME}.exe\shell\open\command" "" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\"'
  WriteRegStr HKCU "${SMART_PHOTO_CAPABILITIES}" "ApplicationName" "${PRODUCTNAME}"
  WriteRegStr HKCU "${SMART_PHOTO_CAPABILITIES}" "ApplicationDescription" "$(SmartPhotoApplicationDescription)"
  WriteRegStr HKCU "${SMART_PHOTO_CAPABILITIES}" "ApplicationIcon" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0'
  WriteRegStr HKCU "Software\RegisteredApplications" "SmartPhotoIndexer" "${SMART_PHOTO_CAPABILITIES}"
  !insertmacro SMART_PHOTO_REGISTER_EXTENSION "jpg"
  !insertmacro SMART_PHOTO_REGISTER_EXTENSION "jpeg"
  !insertmacro SMART_PHOTO_REGISTER_EXTENSION "png"
  !insertmacro SMART_PHOTO_REGISTER_EXTENSION "bmp"
  !insertmacro SMART_PHOTO_REGISTER_EXTENSION "webp"
  !insertmacro SMART_PHOTO_REGISTER_EXTENSION "gif"
  !insertmacro UPDATEFILEASSOC
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro SMART_PHOTO_UNREGISTER_EXTENSION "jpg"
  !insertmacro SMART_PHOTO_UNREGISTER_EXTENSION "jpeg"
  !insertmacro SMART_PHOTO_UNREGISTER_EXTENSION "png"
  !insertmacro SMART_PHOTO_UNREGISTER_EXTENSION "bmp"
  !insertmacro SMART_PHOTO_UNREGISTER_EXTENSION "webp"
  !insertmacro SMART_PHOTO_UNREGISTER_EXTENSION "gif"
  DeleteRegValue HKCU "Software\RegisteredApplications" "SmartPhotoIndexer"
  DeleteRegKey HKCU "Software\SmartPhotoIndexer"
  DeleteRegKey HKCU "Software\Classes\${SMART_PHOTO_PROGID}"
  DeleteRegKey HKCU "Software\Classes\Applications\${MAINBINARYNAME}.exe"
  !insertmacro UPDATEFILEASSOC
!macroend
