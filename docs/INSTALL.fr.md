# Installer Smart Photo Indexer 1.0.6

1. Exécutez SmartPhoto-Setup-1.0.6-x64.exe sous Windows 10/11 x64. Le paquet inclut l’installateur de Microsoft WebView2. Node.js, Rust et le code source ne sont pas nécessaires à l’utilisation normale.
2. Fermez normalement Smart Photo avant une mise à jour. Celle-ci remplace les fichiers de l’application ; conservez des sauvegardes séparées des photos et données importantes.
3. Ouvrez un dossier. La consultation ne nécessite pas de clé IA. La reconnaissance locale demande un modèle préparé explicitement ; les opérations cloud demandent votre propre clé de service et peuvent être payantes.
4. Choisissez le russe, l’anglais, l’allemand ou le français dans les paramètres. ? ouvre une visite et un guide propres à chaque fenêtre.

Dans Windows, utilisez « Ouvrir avec → Smart Photo Indexer » pour une image. L’application s’enregistre comme visionneuse disponible sans remplacer automatiquement toutes les associations. L’application par défaut se choisit dans les paramètres Windows.

Vérifiez l’installateur avec Get-FileHash -Algorithm SHA256 et SHA256SUMS.txt. Il n’est pas encore signé numériquement ; Windows peut afficher un avertissement concernant l’éditeur ou la réputation. Désinstallez-le dans la liste des applications Windows. Les modèles, clés et photos personnelles ne sont pas inclus.

Licence : Apache License 2.0. LICENSE et NOTICE sont installés avec l’application. Les mentions tierces sont accessibles dans l’aide. Téléchargements : https://github.com/LittleSunAI/smart-photo-indexer/releases/latest

Pour un nouveau profil, le mode local utilise Rapide · Qwen3-VL 2B, avec un téléchargement d’environ 1,93 Go. Cliquez sur Installer et confirmez la préparation pour télécharger Ollama et le modèle. Un choix de modèle déjà enregistré est conservé. Les modes détaillés Qwen3.5 prennent davantage de temps ; le mode rapide produit une courte description et des tags.

Dans la version 1.0.2, les nouvelles tâches Qwen3.5 2B et 4B produisent des descriptions détaillées et des tags sans étape de raisonnement distincte. Aucun réglage manuel n’est nécessaire. La vitesse dépend de votre ordinateur et de la taille de l’image ; vérifiez que le résultat ne contient pas d’inexactitudes.
