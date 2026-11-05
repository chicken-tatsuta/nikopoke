import { getTypeEffectiveness } from "./damage";
import type { SimBattlePokemon, SimMove } from "./simulator";

export type SparseFeatureMap = Record<string, number>;

const addFeature = (
  features: SparseFeatureMap,
  key: string,
  value: number
) => {
  if (!Number.isFinite(value) || value === 0) {
    return;
  }

  features[key] = (features[key] ?? 0) + value;
};

const addFlag = (
  features: SparseFeatureMap,
  key: string,
  enabled: boolean
) => {
  if (enabled) {
    addFeature(features, key, 1);
  }
};

const getHpRatio = (pokemon: SimBattlePokemon) =>
  pokemon.stats.hp <= 0 ? 0 : pokemon.currentHp / pokemon.stats.hp;

const getStageRatio = (stage: number) => stage / 6;

export const extractStateFeatures = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon
): SparseFeatureMap => {
  const features: SparseFeatureMap = {};
  const attackerHpRatio = getHpRatio(attacker);
  const defenderHpRatio = getHpRatio(defender);
  const attackerSpeed = Math.max(attacker.effectiveSpeed, 1);
  const defenderSpeed = Math.max(defender.effectiveSpeed, 1);

  addFeature(features, "bias", 1);
  addFeature(features, `attacker:pokemon:${attacker.pokemon.id}`, 1);
  addFeature(features, `defender:pokemon:${defender.pokemon.id}`, 1);

  if (attacker.itemId) {
    addFeature(features, `attacker:item:${attacker.itemId}`, 1);
  }

  if (defender.itemId) {
    addFeature(features, `defender:item:${defender.itemId}`, 1);
  }

  addFeature(features, "hp:attacker_ratio", attackerHpRatio);
  addFeature(features, "hp:defender_ratio", defenderHpRatio);
  addFeature(features, "hp:ratio_diff", attackerHpRatio - defenderHpRatio);
  addFeature(features, "hp:attacker_missing", 1 - attackerHpRatio);
  addFeature(features, "hp:defender_missing", 1 - defenderHpRatio);
  addFeature(features, "speed:ratio", attackerSpeed / defenderSpeed);
  addFeature(features, "speed:delta", (attacker.effectiveSpeed - defender.effectiveSpeed) / 200);

  addFeature(features, "stage:attacker_atk", getStageRatio(attacker.stages.atk));
  addFeature(features, "stage:attacker_def", getStageRatio(attacker.stages.def));
  addFeature(features, "stage:attacker_spa", getStageRatio(attacker.stages.spa));
  addFeature(features, "stage:attacker_spd", getStageRatio(attacker.stages.spd));
  addFeature(features, "stage:attacker_spe", getStageRatio(attacker.stages.spe));
  addFeature(features, "stage:defender_atk", getStageRatio(defender.stages.atk));
  addFeature(features, "stage:defender_def", getStageRatio(defender.stages.def));
  addFeature(features, "stage:defender_spa", getStageRatio(defender.stages.spa));
  addFeature(features, "stage:defender_spd", getStageRatio(defender.stages.spd));
  addFeature(features, "stage:defender_spe", getStageRatio(defender.stages.spe));

  addFlag(features, "status:attacker_burn", attacker.burned);
  addFlag(features, "status:attacker_paralysis", attacker.paralyzed);
  addFlag(features, "status:attacker_poison", attacker.poisoned);
  addFlag(features, "status:attacker_toxic", attacker.toxic);
  addFlag(features, "status:attacker_sleep", attacker.sleepTurnsRemaining > 0);
  addFlag(features, "status:defender_burn", defender.burned);
  addFlag(features, "status:defender_paralysis", defender.paralyzed);
  addFlag(features, "status:defender_poison", defender.poisoned);
  addFlag(features, "status:defender_toxic", defender.toxic);
  addFlag(features, "status:defender_sleep", defender.sleepTurnsRemaining > 0);

  addFeature(features, "status:attacker_toxic_turns", attacker.toxicTurns / 8);
  addFeature(features, "status:defender_toxic_turns", defender.toxicTurns / 8);
  addFeature(features, "status:attacker_sleep_turns", attacker.sleepTurnsRemaining / 3);
  addFeature(features, "status:defender_sleep_turns", defender.sleepTurnsRemaining / 3);
  addFeature(features, "substitute:attacker_ratio", attacker.substituteHp / Math.max(attacker.stats.hp, 1));
  addFeature(features, "substitute:defender_ratio", defender.substituteHp / Math.max(defender.stats.hp, 1));
  addFlag(features, "protect:attacker_active", attacker.protectActive);
  addFlag(features, "protect:attacker_used_last_turn", attacker.protectUsedLastTurn);
  addFlag(features, "protect:defender_active", defender.protectActive);
  addFlag(features, "protect:defender_used_last_turn", defender.protectUsedLastTurn);
  addFlag(features, "flag:attacker_focus_sash_used", attacker.focusSashUsed);
  addFlag(features, "flag:defender_focus_sash_used", defender.focusSashUsed);
  addFlag(features, "flag:attacker_choice_locked", Boolean(attacker.choiceLockedMoveId));
  addFlag(features, "flag:defender_choice_locked", Boolean(defender.choiceLockedMoveId));

  return features;
};

export const extractMoveFeatures = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
): SparseFeatureMap => {
  const features = extractStateFeatures(attacker, defender);
  const accuracy = (move.accuracy ?? 100) / 100;
  const priority = (move.priority ?? 0) / 4;

  addFeature(features, `move:id:${move.id}`, 1);
  addFeature(features, `move:type:${move.type}`, 1);
  addFeature(features, `move:category:${move.category}`, 1);
  addFeature(features, "move:power", Math.min(move.power, 200) / 200);
  addFeature(features, "move:accuracy", accuracy);
  addFeature(features, "move:priority", priority);
  addFeature(features, "move:crit_rank", (move.critRank ?? 0) / 3);
  addFeature(features, "move:drain_ratio", move.drainRatio ?? 0);
  addFeature(features, "move:recoil_ratio", move.recoilRatio ?? 0);
  addFeature(features, "move:recover_ratio", move.recoverRatio ?? 0);
  addFeature(features, "move:burn_chance", move.burnChance ?? 0);
  addFeature(features, "move:flinch_chance", move.flinchChance ?? 0);
  addFeature(features, "move:poison_chance", move.poisonChance ?? 0);
  addFeature(features, "move:toxic_chance", move.toxicChance ?? 0);
  addFeature(features, "move:hit_count", move.hitCount ?? 0);
  addFeature(features, "move:min_hits", move.minHits ?? 0);
  addFeature(features, "move:max_hits", move.maxHits ?? 0);

  addFlag(features, "move:is_contact", Boolean(move.isContact));
  addFlag(features, "move:protect", Boolean(move.protect));
  addFlag(features, "move:substitute", Boolean(move.substitute));
  addFlag(features, "move:rest", Boolean(move.rest));
  addFlag(features, "move:accuracy_per_hit", Boolean(move.accuracyPerHit));

  if (move.targetStatus) {
    addFeature(features, `move:target_status:${move.targetStatus}`, 1);
  }

  if (move.selfStatChanges) {
    for (const [stat, delta] of Object.entries(move.selfStatChanges)) {
      addFeature(features, `move:self_stage:${stat}`, delta / 6);
    }
  }

  if (move.targetStatChanges) {
    for (const [stat, delta] of Object.entries(move.targetStatChanges)) {
      addFeature(features, `move:target_stage:${stat}`, delta / 6);
    }
  }

  if (move.category !== "status" && !move.protect) {
    const typeEffectiveness = getTypeEffectiveness(move.type, defender.pokemon.types);
    addFeature(features, "move:type_effectiveness", Math.log2(Math.max(typeEffectiveness, 0.125)));
    addFlag(features, "move:stab", attacker.pokemon.types.includes(move.type));
  }

  return features;
};
