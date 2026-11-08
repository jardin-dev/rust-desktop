# rust-desktop — widget de bureau Miklfe

Widget de bureau natif écrit en Rust avec [egui/eframe](https://github.com/emilk/egui),
recréant le skin Rainmeter **Miklfe** sous Windows : horloge analogique, visualiseur
audio et bandes d'ombre latérales, sans Rainmeter. Il est dérivé du skin
**Cantarell** d'[ApexXx-SenSei](https://www.deviantart.com/apexxx-sensei/art/Cantarell-770482237).

Le projet est composé de trois fenêtres sans bordure, transparentes et ancrées au
bas de la pile des fenêtres :

| Fenêtre | Rôle |
| --- | --- |
| `Miklfe` | Horloge analogique, date et effet de verre |
| `Miklfe visualizer` | Spectre audio 256 bandes |
| `Miklfe bar` | Dégradés d'ombre gauche et droit en plein écran (`Bar.ini`) |

Une quatrième fenêtre, `Miklfe host`, reste invisible (1 × 1 pixel hors écran) :
elle fait tourner la boucle d'événements et possède les trois viewports visibles.

## Fonctionnalités

- **Horloge** : cadran, aiguilles heure/minute/seconde, bloc de date et texture de
  verre, avec les polices du skin (Arca Majora 2 Heavy + Segoe UI Light).
- **Visualiseur audio** : capture loopback WASAPI de la sortie par défaut, FFT 8192,
  256 bandes réparties logarithmiquement entre 15 Hz et 24 kHz, attaque immédiate et
  décroissance exponentielle.
- **Bandes d'ombre** : dégradés noir 50 % → transparent sur les côtés de l'écran.
- **Intégration bureau** : fenêtres sans décoration, toujours au fond, absentes de la
  barre des tâches et d'Alt+Tab ; la barre et le visualiseur laissent passer les clics.
- **Déplacement** : l'horloge se glisse à la souris ; sa position et celle du
  visualiseur sont mémorisées automatiquement.
- **Win + D** : les widgets restent visibles quand « Afficher le bureau » est activé.
- **Réveil de veille** : les cadres Win32/DWM sont vérifiés et réparés automatiquement
  si Windows les redessine au retour de veille.
- **Mode démo** : `--demo` anime le visualiseur sans accès à l'audio.
- **Mono-instance** : une seconde exécution quitte immédiatement.

## Prérequis

- Windows 10 ou 11.
- Rust stable récent (édition 2024, Rust ≥ 1.85).
- Un pilote OpenGL fonctionnel (le rendu utilise le backend `glow`).
- Pour le visualiseur : une sortie audio par défaut au format 32 bits flottant
  (le mode partagé WASAPI est utilisé tel quel). Sans cela, les bandes restent à zéro.

## Compilation

```powershell
cd app
cargo build --release
```

Le binaire est produit dans `app\target\release\miklfe.exe`.

Pour lancer directement :

```powershell
cargo run --release
cargo run --release -- --demo   # visualiseur sans audio
```

## Installation au démarrage

```powershell
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\Miklfe" | Out-Null
Copy-Item app\target\release\miklfe.exe "$env:LOCALAPPDATA\Miklfe\miklfe.exe"
```

Crée ensuite un raccourci vers `%LOCALAPPDATA%\Miklfe\miklfe.exe` dans le dossier
`shell:startup` pour un lancement automatique à l'ouverture de session.

## Utilisation

- Glisser l'horloge pour la déplacer.
- Clic droit sur l'horloge → **Quitter**.
- Les positions sont enregistrées dans `%APPDATA%\Miklfe\window.json`. Ce fichier
  n'est pas un fichier de configuration : il est réécrit automatiquement.

## Variables d'environnement (diagnostic)

| Variable | Effet |
| --- | --- |
| `MIKLFE_NO_BAR` | Masque les bandes d'ombre |
| `MIKLFE_NO_VIZ` | Masque le visualiseur |
| `MIKLFE_NO_STACK` | Désactive la réorganisation de l'ordre Z |
| `MIKLFE_NO_DESKTOP` | Désactive la gestion de « Afficher le bureau » |
| `MIKLFE_NO_CLOCK_DRAW` | Affiche l'horloge sans son contenu |
| `MIKLFE_RED` | Fond rouge semi-transparent (test de transparence) |

## Structure du projet

```text
app/
├─ src/
│  ├─ main.rs        boucle eframe, viewports, persistance des positions
│  ├─ clock.rs       rendu du cadran, des aiguilles et de la date
│  ├─ visualizer.rs  rendu du spectre audio
│  ├─ bar.rs         dégradés latéraux plein écran
│  ├─ audio.rs       capture loopback WASAPI + analyse FFT
│  ├─ wave.rs        lissage des bandes
│  ├─ win.rs         retouches Win32 / DWM (cadres, ordre Z, veille)
│  ├─ theme.rs       constantes visuelles du skin
│  ├─ state.rs       sauvegarde des positions
│  └─ assets.rs      chargement des textures et des polices
├─ assets/           textures et police fournies avec le skin
├─ examples/         leak_probe, banc d'essai mémoire
└─ vendor/           copie corrigée d'egui_glow
```

### Correctif `vendor/egui_glow`

Le dossier `vendor/egui_glow` contient une copie locale de la dépendance, activée
par `[patch.crates-io]` dans `app/Cargo.toml`. Le rendu d'origine réallouait ses
tampons de sommets/indices à chaque image (`glBufferData`) et fuyait de la mémoire
avec certains pilotes NVIDIA. La copie alloue une fois puis utilise
`glBufferSubData` (voir `vendor/egui_glow/src/painter.rs`).

## Crédits et source

Ce projet réécrit en Rust le skin Rainmeter **Miklfe**, dérivé du skin **Cantarell**
créé par [ApexXx-SenSei](https://www.deviantart.com/apexxx-sensei/art/Cantarell-770482237)
et publié le 30 octobre 2018 sur DeviantArt.

Les textures (`Background.png`, `Glass.png`), la police Arca Majora 2 Heavy et les
proportions du skin proviennent de ces skins.

## Notes

- `app/target/` et `app/build.log` ne sont pas versionnés.
