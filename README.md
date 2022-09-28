# Takoyaki
Takoyaki is a Splatoon3's Tableturf battle (ナワバトラー) simulator.
This project's goals are to build:
 - AIs stronger than ones in the original game
 - a tool to build card decks automagically (the current plan is to use GA)
 - web UIs

## How to build your deck with Takoyaki?
List your cards in `data/deck/mine` or somewhere else.
Then, run something like:
```
RUST_LOG=info cargo run --release -- train-deck --max-generation=1000 --battles-per-epoch=10 --population-size=30 --elite-count=10 -i data/decks/mine
```
You may need to tune hyperparameters (the current implementation isn't great, though).
