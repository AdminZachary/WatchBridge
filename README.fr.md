<div align="center">

<img src="public/watchbridge.svg" width="104" alt="WatchBridge" />

# WatchBridge

**Préparer le mode développeur de l'Apple Watch et installer les apps avec Watch Companion sur la montre, depuis Windows.**

[English](README.en.md) · [简体中文](README.md) · [한국어](README.ko.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Português](README.pt.md)

[![Build Windows](https://img.shields.io/github/actions/workflow/status/AdminZachary/iloader-for-windows/windows.yml?branch=main&label=windows&logo=github)](https://github.com/AdminZachary/iloader-for-windows/actions/workflows/windows.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4)
![Status](https://img.shields.io/badge/hardware%20validated-2026--11--12-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0--alpha.5-lightgrey)

</div>

> [!IMPORTANT]
> WatchBridge est une dérivation communautaire indépendante du [fork iLoader d'AndreySemjonov](https://github.com/AndreySemjonov/iloader) ; le projet d'origine est [nab138/iLoader](https://github.com/nab138/iloader).
> Ce n'est **pas une version officielle d'Apple ni d'un projet amont, et cela n'implique aucun cautionnement**. Signalez les problèmes sur ce dépôt, pas en amont.

> [!NOTE]
> Le parcours complet a été validé sur des appareils réels :
> préparation du mode développeur → signature → installation sur le téléphone → installation sur la montre → lancement de l'app.

## Présentation

Installer une app iOS auto-signée (comme le fait SideStore) demande trois choses : une signature Apple ID, l'enregistrement de l'appareil cible comme appareil de développement, et l'activation du mode développeur sur l'appareil. La chaîne d'outils officielle est Xcode, disponible uniquement sur macOS.

iLoader a reconstruit la moitié (côté iPhone) sous Windows en Rust. **WatchBridge apporte l'autre moitié** : il récupère le `Watch/*.app` contenu dans l'app installée sur l'iPhone et l'installe sur l'Apple Watch appairée ; et avant cela, il fait apparaître l'interrupteur de mode développeur de la montre — sans Python, sans ligne de commande.

Le chemin vers l'appareil est indirect : Windows ne voit pas la montre, donc le programme utilise l'iPhone comme routeur, transfère un port vers la montre, puis effectue l'appairage, la lecture d'état et l'installation dans cette session transférée.

## Fonctionnalités

| | Fonction | Détail |
|---|---|---|
| 🛠 | **Préparation du mode développeur** | Appairer la montre → demander l'interrupteur → relire l'état. Seul `action 0` est envoyé ; **l'activation, le redémarrage et la confirmation se font toujours par vous sur la montre** |
| 📦 | **Installation du Companion** | Vérifie que l'IPA contient l'hôte iPhone et `Watch/*.app`, le re-signe, l'installe sur le téléphone, puis installe l'app sur la montre |
| 📶 | **Wi-Fi / RSD** | Configurez le Wi-Fi une fois en USB, puis installez et renouvelez sans câble ; la préparation du mode développeur exige toujours l'USB |
| ♻️ | **Renouvellement automatique** | Re-signe avant l'expiration des 7 jours, avec contrôles dans la zone de notification et démarrage facultatif |
| 🔐 | **Compte et certificats** | Connexion Apple ID (identité d'appareil via anisette), consultation/révocation des certificats, gestion des App ID, prise en charge du keyring |
| 🔗 | **Fichier d'appairage** | Importer/exporter un pairing file, réparer l'appairage Wi-Fi |
| 🌍 | **Interface multilingue** | 28 langues, dont le chinois simplifié et traditionnel |

## Utilisation

```text
① Branchez l'iPhone appairé en USB   ② Appairez la montre, confirmez la confiance dessus
   Sélectionnez l'appareil              Demandez l'interrupteur « Mode développeur »
   ↓                                    ↓
   Activez-la sur la montre → redémarrez → confirmez → cliquez sur « Revérifier »
   ↓
③ Connectez-vous avec l'Apple ID → choisissez l'IPA d'origine → signez et installez
   ↓
   Ouvrez l'app vous-même sur la montre et vérifiez qu'elle démarre
```

> [!NOTE]
> Le programme **n'active pas** le mode développeur, ne redémarre pas la montre et ne confirme pas la confiance à votre place. Les enregistrements de confiance ne vivent qu'en mémoire : après un redémarrage du programme, il faut rétablir une session.

## Configuration requise

- Windows 10 22H2 / Windows 11 (x86_64)
- **Apple Mobile Device Service** : fourni avec [iTunes](https://www.apple.com/itunes/) ou l'app Apple Devices du Microsoft Store. Sans lui, ni l'USB ni le Wi-Fi ne fonctionnent
- Runtime WebView2 (intégré à Windows 11)
- Un iPhone déjà appairé à la montre cible ; la préparation nécessite un câble

## Installation et exécution

Téléchargez `watchbridge-windows.zip` depuis [Releases](https://github.com/AdminZachary/iloader-for-windows/releases/latest), décompressez-le et lancez `WatchBridge.exe`.

L'archive contient :

| Fichier | Contenu |
|---|---|
| `WatchBridge.exe` | L'application, **non signée** ; SmartScreen avertit au premier lancement |
| `BUILD.json` | Commit source, état de l'arbre de travail, empreintes des lockfiles, versions des outils, SHA-256 de l'exe |
| `SHA256SUMS.txt` | Sommes de contrôle des fichiers ci-dessus |
| `LICENSES.zip` | Textes des licences tierces et inventaire des dépendances (`inventory.json`) |

## Compiler depuis les sources

Les dépendances et la procédure complète sont dans [BUILDING.md](BUILDING.md). GitHub Actions compile, exécute les tests unitaires Rust d'`iloader` et `isideload`, puis produit l'artefact ci-dessus.

```powershell
# Frontend seulement
bun install --frozen-lockfile --ignore-scripts
bun run build

# Compilation Windows complète (tests + exe + sommes de contrôle)
.\scripts\Build-Windows.ps1 -BuildExecutable
```

Ajoutez `-SkipTests` pour une compilation locale rapide sans tests. La compilation doit être native Windows + MSVC ; la compilation croisée n'est pas prise en charge.

## Compatibilité et limites connues

| Limite | Détail |
|---|---|
| **Une seule montre** | Si un iPhone a plusieurs montres appairées, la préparation refuse au lieu de deviner. Le protocole de transfert n'a pas de paramètre « cibler cette montre », et agir sur la mauvaise est pire qu'un échec |
| **Pas de paquet Watch seul** | L'IPA d'origine doit contenir l'hôte iPhone et `Watch/*.app` |
| **App Group** | Les apps nécessitant un Watch App Group peuvent encore subir le comportement amont qui ignore cette association |
| **Versions du système** | Le flux côté montre vient de l'implémentation expérimentale d'xtool ; chaque combinaison iOS/watchOS doit être validée sur matériel |
| **Outil non officiel** | Pas de jailbreak, pas de contournement de la signature Apple, pas de certificat falsifié |

## FAQ

<details>
<summary><b><code>fail to connect to usbmuxd device socket: io error</code></b></summary>

Sous Windows, usbmuxd est fourni par Apple Mobile Device Service et écoute sur `127.0.0.1:27015`. Cette erreur signifie que rien ne répond sur ce port : le service est généralement absent ou arrêté.

```powershell
Get-Service "Apple Mobile Device Service"      # doit exister et être Running
Test-NetConnection 127.0.0.1 -Port 27015       # doit être True
$env:USBMUXD_SOCKET_ADDRESS                    # doit être vide
```

Si le service n'existe pas, installez iTunes ou l'app Apple Devices. Si l'adresse a été modifiée, supprimez cette variable d'environnement. Le programme **n'embarque pas** usbmuxd ; il dépend entièrement du pilote et du service d'Apple.
</details>

<details>
<summary><b>« Impossible de lire les informations de la montre »</b></summary>

Les anciennes versions interrogeaient le registre d'appairage de l'iPhone pour le nom de la montre avant l'appairage ; sur les versions récentes d'iOS, cette requête ne renvoie rien. La version actuelle établit d'abord la confiance avec la montre, puis lit son identité depuis la session de la montre. Si cela échoue encore, ouvrez « Réglages et journaux → Voir les journaux » et regardez la valeur `kind=` de `watch step failed` : `timed out` signifie que la montre n'a pas répondu ; `NotFound` / `UnexpectedResponse` indiquent une différence de protocole.
</details>

<details>
<summary><b>« Plusieurs montres appairées détectées »</b></summary>

Voir la limite d'une seule montre. Confirmez la montre cible dans l'app Watch de l'iPhone ; éteindre l'autre montre ou l'éloigner ne la **retire pas** du registre d'appairage.
</details>

## Contribuer

- Règles de contribution : [CONTRIBUTING.md](CONTRIBUTING.md)
- L'amont interdit explicitement les contributions générées par IA ; **ce dépôt n'envoie jamais de PR en amont**. Gardez les modifications dans ce fork
- Ne joignez jamais d'Apple ID, mot de passe, clé privée de certificat, enregistrement d'appairage ou journal d'appareil non expurgé

## Licence et crédits

Le code du projet est publié sous MIT ([LICENSE](LICENSE)) ; les conditions de marque sont dans [LICENSE-BRANDING](LICENSE-BRANDING).

> La bibliothèque de signature de la chaîne de dépendances (`apple-codesign-quick` d'`apple-crates`) ne déclare aucune licence ; ses termes restent à clarifier. Voir [docs/LICENSE_REVIEW.md](docs/LICENSE_REVIEW.md).

Sources et attribution : [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md), [vendor/isideload/PROVENANCE.md](vendor/isideload/PROVENANCE.md).

| Source | Contribution |
|---|---|
| [nab138/iLoader](https://github.com/nab138/iloader) | Application d'origine, chaîne de signature et d'installation |
| [AndreySemjonov/iloader](https://github.com/AndreySemjonov/iloader) | Point de départ de ce dépôt : Wi-Fi/RSD et renouvellement automatique |
| [oacevedo/xtool](https://github.com/oacevedo/xtool/tree/feat/experimental-watchos) | Flux de préparation du mode développeur (réécrit nativement en Rust ici, sans Python) |
| Data_C0re_ (Rzbck) | Signature, enregistrement et installation du Watch Companion |
| idevice / isideload et leurs auteurs amont | Communication avec les appareils et implémentation de la signature |

## Avertissement

Ce projet n'est affilié ni à Apple Inc. ni à aucun de ses produits. Apple, iPhone et Apple Watch sont des marques d'Apple Inc.

Uniquement pour l'apprentissage, la recherche et le développement personnel ; chaque utilisateur doit vérifier la législation de sa juridiction et les conditions de licence des apps installées. Les auteurs déclinent toute responsabilité en cas de dommage matériel, perte de données, restriction de compte ou révocation de certificat.
