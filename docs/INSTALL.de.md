# Smart Photo Indexer 1.0.6 installieren

1. Starten Sie SmartPhoto-Setup-1.0.6-x64.exe unter Windows 10/11 x64. Das Paket enthält den Installer für Microsoft WebView2. Für die normale Nutzung sind Node.js, Rust und Quellcode nicht erforderlich.
2. Schließen Sie Smart Photo vor einem Update regulär. Das Update ersetzt die Programmdateien. Bewahren Sie separate Sicherungen wichtiger Fotos und Anwendungsdaten auf.
3. Öffnen Sie einen Ordner. Betrachten benötigt keinen KI-Schlüssel. Lokale Erkennung erfordert ein ausdrücklich vorbereitetes Modell; Cloud-Vorgänge benötigen Ihren eigenen Dienstschlüssel und können Kosten verursachen.
4. Wählen Sie Russisch, Englisch, Deutsch oder Französisch in den Einstellungen. ? öffnet in jedem Fenster eine eigene Einführung und das Nachschlagewerk.

Unter Windows können Sie für ein Bild „Öffnen mit → Smart Photo Indexer“ wählen. Die Anwendung wird als verfügbarer Bildbetrachter registriert, ohne alle Dateizuordnungen automatisch zu ersetzen. Die Standardanwendung legen Sie in den Windows-Einstellungen fest.

Prüfen Sie den Installer mit Get-FileHash -Algorithm SHA256 gegen SHA256SUMS.txt. Der Installer ist noch nicht digital signiert; Windows kann eine Warnung zum Herausgeber oder zur Reputation anzeigen. Die Deinstallation erfolgt über die installierten Windows-Anwendungen. Modelle, Schlüssel und persönliche Fotos sind nicht enthalten.

Lizenz: Apache License 2.0. LICENSE und NOTICE werden mitinstalliert. Hinweise zu Drittanbietern stehen in der Hilfe. Downloads: https://github.com/LittleSunAI/smart-photo-indexer/releases/latest

Bei einem neuen Profil wird im lokalen Modus Schnell · Qwen3-VL 2B verwendet; der Download umfasst etwa 1,93 GB. Klicken Sie auf Installieren und bestätigen Sie die Einrichtung, um Ollama und das Modell herunterzuladen. Eine bereits gespeicherte Modellauswahl bleibt erhalten. Die ausführlichen Qwen3.5-Modi benötigen mehr Zeit; der schnelle Modus erstellt eine kurze Beschreibung und Tags.

Ab Version 1.0.2 erstellen neue Aufgaben mit Qwen3.5 2B und 4B ausführliche Beschreibungen und Tags ohne gesonderte Denkphase. Eine manuelle Umschaltung ist nicht nötig. Die Geschwindigkeit hängt vom Computer und der Bildgröße ab. Prüfen Sie das Ergebnis auf Ungenauigkeiten.
