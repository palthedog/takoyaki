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
RUST_LOG=info cargo run --bin takoyaki --release -- train-deck --max-generation=1000 --battles-per-epoch=10 --population-size=30 --elite-count=10 -i data/decks/mine
```
The command can produce decks storonger than the starter deck on random plays (>80% win rate).

## How to run battles?
Currently, Takoyaki doesn't support playing games against AIs. Instead, you can watch AI v.s. AI battles.
You can run a following command to see a battle:
```
cargo run --bin takoyaki --release -- --step-execution --player=random --opponent=mcts-1000 play --play-cnt=1 --player-deck-path=data/decks/starter  --opponent-deck-path=data/decks/starter
```
the result would be something like
```
UDON[~/work/takoyaki/](master)$ cargo run --bin takoyaki --release -- --step-execution --player=random --opponent=mcts-1000 play --play-cnt=1 --player-deck-path=data/decks/starter  --opponent-deck-path=data/decks/starter
    Finished release [optimized] target(s) in 0.01s
     Running `target/release/takoyaki --step-execution --player=random --opponent=mcts-1000 play --play-cnt=1 --player-deck-path=data/decks/starter --opponent-deck-path=data/decks/starter`
Player action: Put(batoroika) @ [p: [2,18], r: Right, s: false]
103: batoroika
cnt: 10 cost: 4
  =
  =
=====
  =*
  =

Opponent action: Put(splamaneuver) @ [p: [5,5], r: Left, s: false]
45: splamaneuver
cnt: 8 cost: 3
 ====
 =*
==

Turn: 2
Massugu Street
###########
#.........#
#.........#
#.........#
#....O....#
#....o....#
#....o....#
#....oO...#
#....ooo..#
#......o..#
#.........#
#.........#
#.........#
#.........#
#.........#
#.........#
#.........#
#.........#
#...p.....#
#...p.....#
#.ppppp...#
#..Pp.....#
#...p.....#
#....P....#
#.........#
#.........#
#.........#
###########
Score: 11, 9
Special: 0, 0

Turn 1 has finished. Press enter key to continue

... snipped ...

Turn 11 has finished. Press enter key to continue

Player action: Special!(splamaneuver) @ [p: [4,17], r: Down, s: true]
45: splamaneuver
cnt: 8 cost: 3
 ====
 =*
==

Opponent action: Put(splacharger) @ [p: [1,1], r: Up, s: false]
28: splacharger
cnt: 8 cost: 3
=======
  *

Turn: 13
Massugu Street
###########
#ooooooo..#
#ooOo.oooo#
#oOo.....O#
#.oo.Ooooo#
#o...o...o#
#Oo..ooooo#
#oo..oO...#
#.o..ooooo#
#.oo.o.oOo#
#ooooooo.o#
#.Oo.o.o..#
#..ooOoo..#
#ooOooo...#
#oooo.o...#
#o..ooo...#
#..OooOoo.#
#Oooo.ppp.#
#oo.pPPppp#
#o..ppppP.#
#oppppp.p.#
#opPppppp.#
#pppp.Ppp.#
#PpPpP....#
#ppppppppp#
#p..pP....#
#.........#
###########
Score: 51, 99
Special: 0, 0

Turn 12 has finished. Press enter key to continue

[2022-10-20T17:37:41Z INFO  takoyaki::play] Battle #0. 51 v.s. 99
[2022-10-20T17:37:41Z INFO  takoyaki::play] Player won cnt: 0 (0.000)
[2022-10-20T17:37:41Z INFO  takoyaki::play] Opponent won cnt: 1 (1.000)
[2022-10-20T17:37:41Z INFO  takoyaki::play] Draw cnt: 0
[2022-10-20T17:37:41Z INFO  takoyaki::play]
    * All battles have finished
[2022-10-20T17:37:41Z INFO  takoyaki::play] Used decks: p: Some("data/decks/starter"), o: Some("data/decks/starter")
[2022-10-20T17:37:41Z INFO  takoyaki::play] Board: Massugu Street
[2022-10-20T17:37:41Z INFO  takoyaki::play] Player won cnt: 0 (0.000)
[2022-10-20T17:37:41Z INFO  takoyaki::play] Opponent won cnt: 1 (1.000)
[2022-10-20T17:37:41Z INFO  takoyaki::play] Draw cnt: 0
```

### Available AIs
You can specify types of AI with command line options `--player` and `--oppoenent`.
You can choose one from;
 - `random`
    The AI choose a random action
 - `mcts-10`
   The AI choose an action based on a naive Monte-Carlo Tree Search(MCTS). It runs 10 iterations to find an action.
 - `mcts-100`
   The AI uses MCTS but with 100 iterations.
 - `mcts-1000`
   The AI uses MCTS but with 1000 iterations.

## AI strength
I don't know :) but `mcts-1000` seems to win almost all games against the `random` player.

# TODOs
- Consider using a faster hasher for HashMap
- Make the logic runs on multi threads
