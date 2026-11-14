# ✨ Poke Together Companion

A free companion app for **Pokémon Platinum Together**: an always-on-top shiny hunting counter, plus a set of apps you pick yourself (Pokédex, map, rare grass, team planning, breeding, battles, shiny replays and three mini games).

## Download

Grab **Shiny Tracker.zip** from the [latest release](https://github.com/ZifuM/Shinytracker/releases/latest), unzip it and run **Shiny Tracker.exe**.
Windows 10/11, nothing else to install. It updates itself, and can opt in to pre-releases (Advanced > Updates).

## Apps

- **Shiny Hunter**: counts every encounter from the game's own log, shiny alerts, odds, stats, Shiny Dex, trophies and share images, with an overlay on top of the game
- **Pokédex**: every Pokémon in Platinum Together, where to catch it, moves, evolutions and its trading cards
- **Map**: all of Sinnoh with every area, and today's rare grass spots
- **Game Info**: the Platinum Together wiki's guides, items, moves and places
- **Team Planner** and **Damage Calculator**: Showdown-style teams, coverage, and Gen 4 damage maths
- **Battle Sim** *(beta)*: battle wild Pokémon and trainers, and test catch odds
- **Breeding** *(beta)*: plan eggs, keep your breeders, find egg moves
- **Card Collector** *(beta)*: hunting earns booster packs of the original Base Set cards; open them, collect them all and trade with friends
- **Pokégotchi** *(beta)*: adopt a shiny you caught as a pocket pet; hunting feeds it, and it keeps you company on the overlay
- **Shiny Bingo**: a new card every day that ticks itself off as you play, with a daily reward
- **Rare Grass** *(beta)*: every Pokémon on rare grass today, and every one that's been out before
- **Shiny Replay** *(beta)*: saves the 15 seconds before your shiny alert and 5 after as an MP4

The **Dashboard** is yours to build from 70+ widgets on a grid you arrange and size yourself.

<p>
  <img src="docs/overlay.png" alt="The shiny overlay" width="49%">
</p>

## Build it yourself

Install [Rust](https://rustup.rs), then run `build.bat`. It builds the app and makes `share\Shiny Tracker.zip`.

---

Fan-made and not affiliated with Nintendo, Game Freak or The Pokémon Company. It only reads the game's log and map files (and Shiny Replay records the game window's picture); it never changes them or the game's memory.

## Credits

The Team Planner's battle data (base stats, abilities, move details, held items and sample sets) comes from [Pokémon Showdown](https://github.com/smogon/pokemon-showdown)'s open data, and its damage calculator follows the Gen 4 steps of Smogon's [damage-calc](https://github.com/smogon/damage-calc). Both are MIT licensed. The Battle Sim's move effects and gender ratios also come from Showdown's data, and its catch rates from [PokéAPI](https://pokeapi.co). Card list from [pokemon-tcg-data](https://github.com/PokemonTCG/pokemon-tcg-data) (MIT); card pictures are downloaded from [pokemontcg.io](https://pokemontcg.io) only if you choose to.
