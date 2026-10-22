# Smart Photo Indexer

[English](README.md) · [Русский](README.ru.md) · [Deutsch](README.de.md) · [Français](README.fr.md)
<a href="docs/media/cover.png"><img src="docs/media/cover.png" alt="Smart Photo Indexer by LittleSunAI" width="100%"></a>

**Une photothèque que l’on peut explorer avec des mots.** Une application Windows de **LittleSunAI** pour consulter des photos, créer des descriptions et des mots-clés, rechercher dans les métadonnées enregistrées et retoucher des images.

**1.0.6 · Windows 10/11 x64 · RU / EN / DE / FR**

> Logiciel libre de **LittleSunAI**, distribué sous [licence Apache 2.0](LICENSE). Voir [NOTICE](NOTICE) et les [informations de licence](docs/LICENSING_STATUS.md).

OpenRouter est temporairement désactivé en attendant sa vérification. Fal AI et les modèles locaux restent disponibles ; les paramètres et identifiants OpenRouter enregistrés sont conservés.

## Possibilités

- Parcourir les dossiers en grille, en liste ou en tableau ; utiliser les favoris et les collections.
- Créer des descriptions, des mots-clés ou les deux avec un modèle Ollama local ou un service cloud configuré séparément.
- Rechercher dans les descriptions, mots-clés, noms de fichiers et chemins enregistrés, dans le dossier courant ou toute la bibliothèque indexée. Il s’agit d’une recherche textuelle, sans promesse de recherche vectorielle par similarité.
- Préparer des files de traitement et suivre la progression, les erreurs et les reprises disponibles.
- Ouvrir une visionneuse dédiée avec zoom, déplacement et diaporama, ou passer à l’éditeur pour le recadrage et le redimensionnement locaux ainsi que les retouches cloud prises en charge.
- Utiliser les mots-clés par lot, les fichiers XMP, l’export des données et les projets de l’éditeur. Les résultats de l’IA doivent être vérifiés.
- Parcourir un arbre des variantes, comparer les images et reprendre à partir d’une image choisie. Enregistrer l’historique du projet séparément de la photo et examiner les fichiers intermédiaires avant nettoyage.

## Le principe en images

Quatre illustrations expliquent le parcours. Ce sont des schémas explicatifs, pas des captures d’une interface inventée. Les photos ont été générées ; les descriptions et les mots-clés sont des exemples.

[![Choisir un dossier de photos](docs/media/01-your-library.png)](docs/media/01-your-library.png)

**1. Choisir un dossier.** Les photos restent dans leurs dossiers habituels. Sélectionnez celles sur lesquelles vous souhaitez travailler.

[![Créer des descriptions et des mots-clés](docs/media/02-describe-and-tag.png)](docs/media/02-describe-and-tag.png)

**2. Décrire les images.** Choisissez le résultat voulu et un traitement local ou un service cloud.

[![Rechercher dans les informations enregistrées](docs/media/03-search-by-words.png)](docs/media/03-search-by-words.png)

**3. Retrouver une scène avec des mots.** La recherche utilise les informations déjà enregistrées ; inutile de se souvenir du nom du fichier.

[![Consulter, comparer et retoucher](docs/media/04-view-and-refine.png)](docs/media/04-view-and-refine.png)

**4. Regarder de plus près.** Examinez une photo, comparez des variantes et ouvrez l’éditeur lorsqu’une retouche est nécessaire.

## L’application réelle

Ces captures montrent l’application préparée. Les exemples de descriptions et de mots-clés ont été saisis manuellement : aucune requête IA payante n’était nécessaire pour la documentation. Cliquez sur une image pour ouvrir l’original en pleine résolution.

[![Photothèque principale](docs/media/screenshot-library.png)](docs/media/screenshot-library.png)

[![Éditeur de photos](docs/media/screenshot-editor.png)](docs/media/screenshot-editor.png)

[![Visionneuse dédiée](docs/media/screenshot-viewer.png)](docs/media/screenshot-viewer.png)

## Installer et commencer

Téléchargez l’installateur Windows dans les [Releases](https://github.com/LittleSunAI/smart-photo-indexer/releases/latest) : `SmartPhoto-Setup-1.0.6-x64.exe`. Le code source est disponible ici et dans les archives Source code. Les instructions et SHA256SUMS.txt sont joints séparément.

1. Installez la version Windows x64. Node.js et Rust ne sont pas nécessaires à l’utilisation normale.
2. Ouvrez un dossier. La consultation ne nécessite ni clé IA ni modèle.
3. Pour la reconnaissance, préparez explicitement un modèle local ou configurez votre propre clé cloud. Les opérations cloud peuvent être payantes.
4. Choisissez une description, des mots-clés ou les deux, puis vérifiez les informations produites.
5. Choisissez la langue dans les paramètres. **?** ouvre une visite et un guide de référence propres à chaque fenêtre.

[Instructions d’installation](docs/INSTALL.fr.md) · [Données et services](docs/DATA_AND_SERVICES.md)

## Compilation, vérifications et limites

Les prérequis et la compilation Windows sont décrits dans [BUILD.md](docs/BUILD.md). Les tests portables par défaut ne lisent pas les clés enregistrées, ne téléchargent pas de modèles et n’utilisent pas d’API payantes.

```powershell
npm ci
npm run verify:structure
npm test
npm run build
```

L’installateur n’est pas encore signé numériquement. Les modèles et les clés ne sont pas inclus. La vitesse et la mémoire nécessaires aux modèles locaux dépendent de l’ordinateur ; la disponibilité du cloud, les frais et la taille des images renvoyées dépendent du service. Tous les modèles de retouche n’acceptent pas les masques ou les images supplémentaires. Sauvegardez les originaux importants avant toute commande modifiant les fichiers.

## Auteur et composants tiers

**LittleSunAI** — [AUTHORS.md](AUTHORS.md). Code propre au projet : [licence Apache 2.0](LICENSE), [NOTICE](NOTICE). Les composants tiers conservent leurs conditions : [mentions](THIRD_PARTY_NOTICES.md), textes complets dans `third-party-licenses/`.

[Signaler un problème / contribuer](CONTRIBUTING.md) · [Sécurité](SECURITY.md) · [Notes de version](docs/RELEASE_NOTES.md)

[Audit de préparation](docs/PREPARATION_AUDIT.md)
