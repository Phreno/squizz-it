# Squizz-it
Application CLI d'apprentissage avec flashcards CSV et progression de type Simon.

## Format des decks
Chaque deck est un fichier `.csv` dans `decks/`.

Headers supportés:
- `key` ou `question` pour la question,
- `value` ou `réponse` pour la réponse.

Exemple:
```csv
key,value
capital de la france,Paris
2 + 2,4
```

## Règle de jeu
Le deck est mélangé au démarrage.  
Tu dois reproduire une séquence de plus en plus longue:
1. la première carte,
2. puis les deux premières,
3. puis les trois premières, etc.

Si une réponse est fausse, la même carte est rejouée jusqu'à réussite.
Lorsqu'une carte est rejouée dans la séquence déjà validée, la question est cachée (seul l'index d'étape est affiché).
Quand un deck est entièrement validé, il est automatiquement remélangé et une nouvelle manche démarre.

## Configuration
Le fichier `squizz-it.toml` est chargé automatiquement s'il existe.

Options:
- `decks_dir`: dossier des decks,
- `csv.delimiter`: séparateur CSV,
- `game.answer_mode`: `exact` ou `case_insensitive`,
- `game.normalize_whitespace`: normalisation des espaces,
- `game.shuffle_seed`: seed optionnelle pour un mélange déterministe.

## Exécution
```bash
cargo run -- --deck example
```

Choix interactif:
```bash
cargo run
```

Recherche:
```bash
cargo run -- --search ex
```

Quitter pendant une session:
```bash
q
```
