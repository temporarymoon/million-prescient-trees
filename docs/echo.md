# Echo

This document provides a detailed description of my understanding of all the rules of echo. The rules will be eplained in a way that might not be human friendly, but is useful for computer implementations.

The ✅ emoji means somethings is present in my implementation. The 🧪 emoji means it is also included in the automatic tests.

## Setup

The 11 creature cards are shuffled. One random card is placed away. This card will be referred to as the `overseer`. Half the remaining cards are distributed to each player (at random).

Four battlefields are chosen. There are different variations of the rules for choosing battlefields, so my program can handle any four field combination.

Each player is given one of each edict, leading to a total of five edicts per player. For the rest of this document, it will be assumed that both players have perfect knowedge of the edicts the opponent has in their possesion (this information is easily trackable).

## Turns

The game takes place during four turns, one for each field. The battlefield corresponding to the turn talked about will be referred to as the "current battlefield".

## Battlefields

Each battlefield offers a fixed reward for winning a battle. This reward is equal to `3` for all battlefields but `last stranding`, where it is equal to `5`.

Each battlefield has a list of creatures it offers bonuses to. Each such creature gains `+2` strength.

### Glade

- ✅🧪 The winner of the current battle gets the `glade` status effect for the following turn.

### Mountain

- ✅🧪 The winner of the current battle gets the `mountain` status effect for the following turn.

### Urban

- ✅🧪 Increses the edict multiplier by `1`.

### Night

- ✅🧪 Gives both players the night status effect.

### Last strand

(no effects)

## Creatures

The game contains 11 different creatures. Each creature has a strength value. A standard creature ordering exists (it is the order the creatures are presented in the table below).

| Creature  | Strength |
| --------- | -------- |
| Wall      | 0        |
| Seer      | 0        |
| Rogue     | 1        |
| Bard      | 2        |
| Diplomat  | 2        |
| Ranger    | 2        |
| Steward   | 2        |
| Barbarian | 3        |
| Witch     | 3        |
| Mercenary | 4        |
| Monarch   | 6        |

## Creature effects

Each creature has a set of unique effects.

### Wall

1. ✅ The battle ends in a tie.

### Seer

1. ✅ The player is given the `seer` status effect for the next turn.

### Rogue

1. ✅ Negates the `seer` character.
2. ✅ Wins agains the `monarch` and the `wall`.

### Bard

1. ✅ The player is given the `bard` status effect for the next turn.

### Diplomat

1. ✅ Wins when both players have played the same edict.

> Note: the card text originally says

### Ranger

1. ✅ Gains `+2` strength if the played receives a battlefield bonus and the opponent does not.

### Steward

1. ✅🧪 Increase the edict multiplier by `1` for the current player, this turn only.
2. ✅🧪 At the end of the turn, return all edicts back to the hand.

### Barbarian

1. ✅ Gains `+2` strength if the `barbarian` status effect is active.

### Witch

1. ✅ Negates the effect of the opposing creature.
2. ✅ Cannot gain strength from edicts.
3. ✅ Wins against the `wall`.

### Mercenary

1. ✅ Gives the current player the `mercenary` status effect.

### Monarch

1. ✅ Not winning the current battle yields the opponent `2` victory points.

## Edicts

The effects of the edicts is multiplied by the "edict multiplier". Edict multiplier bonuses are additive 🧪.

### Rile the public

1. ✅ The current battlefield is worth `+1` victory points.
2. ✅ Negates `divert attention`.

### Divert attention

1. ✅ The current battlefield is worth `-1` victory points.

### Gambit

1. ✅ Gain `+1` strength.
2. ✅ You lose on ties.

### Ambush

1. ✅ If your creature has a battlefield bonus, gain an additional `+1` strength.

## Player status effects

### Seer

The player can play two creatures during the main phase. During the seer phase, the player can choose one creature to return to their hand before revealing the other.

### Bard

1. ✅ The player gains `+1` strength.
2. ✅ The player gains `+1` if when winning a battle.

### Barbarian

- ✅ When losing a battle, a player gets this effect for the following turn if the `barbarian` has not yet been played.

### Mercenary

- ✅ Gives the current creature `-1` strength.

### Glade

- ✅🧪 The player gains `2` additional victory points by winning this battle.

### Mountain

- ✅🧪 The current creature gains `+1` strength.

### Night

- ✅ The player gains an additional victory point by winning this battle.

## Phases

Each turn consists of three phases. Some phases might not require any actions during some turns.

### Main phase

Each player chooses the creature(s) and edict to play.

### Main -> Sabotage transition

The edicts are revealed

### Sabotage phase

Each player who's played a `sabotage` edict in the main phase writes down a creature name.

### Sabotage -> Seer transition

All players who are not affected by the `seer` status effect reveal their creatures

### Seer phase

All payers who are affected by the `seer` status effect pick a creature to return to their hand.

### Seer -> Main phase transition

The remaining played creature cards are revealed. The gamestate is evaluated (all played cards go to the graveyard).

## Game state evaluation

A quick sketch of the implementation:

1. Resolve isntant win effects.
2. Resole the `wall` instant tie effect.
3. Resolve modified strength values.
4. Compute battle result based on strength values.
5. If a tie occurs at any point, resolve the second `gambit` effect.

   - if neither or both players have played a `gambit`, the battle ends in a tie
   - otherwise, the player who has played a `gambit` loses

6. Compute number of victory points earned by the victor
7. Prepare status effects for the following turn
