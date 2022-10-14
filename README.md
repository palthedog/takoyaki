# Takoyaki
Takoyaki is a Splatoon3's Tableturf battle (ナワバトラー) simulator.
The current goals of this project are:
 - AIs stronger than ones in the original game
 - a tool to build strong card decks automagically (the current plan is to use GA)
 - web UIs

## How to build your deck with Takoyaki?
List IDs of cards which you already have in `data/deck/mine` or somewhere else.
Then, run 'train-deck' command:
```
RUST_LOG=info cargo run --release -- train-deck --max-generation=1000 --battles-per-epoch=10 --population-size=30 --elite-count=10 -i data/decks/mine
```
The command can produce decks storonger than the starter deck on random plays (>80% win rate).

## How to simulate battles?
Currently, there is only a random play AI.
You can run a following command to see a battle step by step:
```
cargo run --release -- -s rand -p data/decks/starter -o data/decks/starter
```

# TODOs
- Consider using a faster hasher for HashMap
