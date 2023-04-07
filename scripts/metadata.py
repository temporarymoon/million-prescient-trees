from math import comb, factorial

creature_count = 11
edict_count = 5
battlefield_count = 6

battlefield_player_effect_count = 2
creature_player_effect_count = 4

def infoset_count(turn): 
  score_range = 1 + turn * 2

  graveyard_range = comb(creature_count, turn)
  hand_range = comb(creature_count - turn, 5 - turn)

  battlefield_effect_range = battlefield_player_effect_count * 2 + 1
  creature_effect_range = creature_player_effect_count * 2 + 1

  effect_range = creature_effect_range * battlefield_effect_range
  battlefields_range = factorial(battlefield_count) * factorial(1  if turn - 3 else 3 - turn)

  edict_range = 0

  # Steward effect returns edicts to hand
  for i in range(0, turn + 1):
     edict_range += comb(edict_count, 5 - turn) * comb(edict_count, 5 - i)

  print(f"- score: {score_range}")
  print(f"- graveryard: {graveyard_range}")
  print(f"- hand: {hand_range}")
  print(f"- effect: {effect_range}")
  print(f"- edict: {edict_range}")
  print(f"- battlefields_range: {battlefields_range}")

  return score_range * hand_range * edict_range * graveyard_range * effect_range * battlefields_range

def infoset_fixed_start_count(turn): 
  score_range = 1 + turn * 2

  graveyard_range = comb(creature_count, turn)

  battlefield_effect_range = battlefield_player_effect_count * 2 + 1
  creature_effect_range = creature_player_effect_count * 2 + 1

  effect_range = creature_effect_range * battlefield_effect_range

  edict_range = 0

  # Steward effect returns edicts to hand
  for i in range(0, turn + 1):
     edict_range += comb(edict_count, 5 - turn) * comb(edict_count, 5 - i)

  print(f"- score: {score_range}")
  print(f"- graveryard: {graveyard_range}")
  print(f"- effect: {effect_range}")
  print(f"- edict: {edict_range}")

  return score_range * edict_range * graveyard_range * effect_range

total_fixed = 0
total_unfixed = 0

for i in range(0, 4): 
  # count_unfixed = infoset_count(i)
  count_fixed = infoset_fixed_start_count(i)

  # total_unfixed += count_unfixed
  total_fixed += count_fixed

  # print(f"Turn {i} has {count_unfixed} infosets")
  print(f"Turn {i} has {count_fixed} fixed start infosets")

print(f"Unfixed: {total_unfixed}. Fixed: {total_fixed}")

infoset_byte_size = 56
print(f"{infoset_byte_size * total_fixed / 1024 / 1024 / 1024} gb required to store all nodes")
print(f"{infoset_byte_size * total_fixed / 1024 / 1024} mb required to store all nodes")

tree_size = 25**2 * 16**2 * 9**2 * 4**2 * 11 *2
print(tree_size)
print(f"{tree_size * 4 / 1024 / 1024 / 1024} gb required to store all nodes")
