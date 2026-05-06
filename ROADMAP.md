# Roadmap Squizz-it

Légende : ✅ fait · 🟡 partiel · ⬜ à faire

## 1. Persistance et répétition espacée (impact majeur)

Le manque le plus critique : rien ne survit entre deux sessions. Chaque lancement repart de zéro.

✅ Sauvegarder l'état sur disque (JSON dans ~/.local/share/squizz-it/) : scores par carte, nombre d'erreurs, date du dernier passage.
✅ Algorithme de répétition espacée (SM-2) : planifier automatiquement quand revoir chaque carte. Les cartes maîtrisées apparaissent moins souvent ; celles qui posent problème reviennent vite.
⬜ File d'attente quotidienne : cargo run -- --review pour lancer une session contenant uniquement les cartes échues aujourd'hui.

## 2. Statistiques et feedback de progression

Sans mesure, pas de motivation durable.

🟡 Résumé de fin de session : précision (%) et nombre de réponses correctes/incorrectes sont affichés. Manquent : temps moyen par carte, nombre de cartes nouvelles vs revues, streak en cours.
⬜ Barre de progression visuelle dans le header (un Gauge ratatui suffit).
⬜ Historique par deck : courbe d'acquisition au fil des jours (exportable en CSV ou affichable en TUI avec sparklines).

## 3. Modes d'apprentissage complémentaires

Le mode Simon est efficace pour le rappel séquentiel, mais il ne couvre pas tous les angles.

⬜ Mode classique : une carte, une réponse, sans contrainte de séquence. Utile pour la première découverte d'un deck.
⬜ Mode inversé : répondre la question à partir de la réponse (value → key). Renforce la liaison bidirectionnelle.
⬜ Mode chrono : temps limité par carte ; entraîne la fluency, pas seulement l'exactitude.
⬜ Sélection via --mode simon|classic|reverse|timed.

## 4. Système d'indices progressifs

Afficher la réponse en bloc après une erreur fonctionne, mais un indice graduel serait plus pédagogique.

⬜ Première erreur : nombre de caractères attendu (_ _ _ _ _).
⬜ Deuxième erreur : première lettre révélée (P _ _ _ _).
⬜ Troisième erreur : réponse complète (comportement actuel).
⬜ Configurable via game.hint_mode = "progressive" | "immediate" | "none".

## 5. Support multi-réponses et tolérance

⬜ Réponses alternatives : une colonne aliases dans le CSV (Paris;paris;Paname) pour accepter les synonymes.
⬜ Distance de Levenshtein : signaler les quasi-bonnes réponses ("Tu as écrit Prais, tu voulais dire Paris ?") au lieu d'un rejet sec.
⬜ Réponses partielles : pour les cartes longues, accepter un score de correspondance configurable.

## 6. Gestion des decks enrichie

⬜ Sous-decks et tags : un champ tags dans le CSV pour filtrer par catégorie (--tag grammaire).
⬜ Import Anki/Markdown : convertir les formats courants vers le CSV squizz-it.
⬜ Éditeur intégré : ajouter/modifier des cartes depuis la TUI (e pour éditer la carte courante).
⬜ Fusion de decks : --deck "geo+histoire" pour combiner plusieurs decks en une session.

## 7. Engagement et gamification légère

🟡 Streak par carte : le suivi par carte existe (streak courant et meilleur streak dans CardStats). Manque : streak quotidien (jours consécutifs avec au moins une session).
⬜ Niveaux de maîtrise par carte : afficher un indicateur (🟥🟧🟨🟩) selon le taux de réussite.
⬜ Objectif configurable : game.daily_goal = 20 cartes par jour, avec une notification de complétion.

## Priorité suggérée

Si je devais choisir un ordre d'implémentation pour maximiser l'efficacité d'apprentissage :

1. ~~Persistance + répétition espacée~~ ✅
2. Statistiques de fin de session (feedback immédiat) 🟡
3. Mode classique + inversé (couvre les cas d'usage courants)
4. Indices progressifs (réduit la frustration)
5. Tolérance Levenshtein (qualité de vie)
6. Le reste selon tes priorités
