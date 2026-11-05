import { calcDamage, getTypeEffectiveness } from "./damage";
import {
  applyBattleStageChanges,
  applyStageMultiplier,
  formatBattleStageChange,
  getEffectiveSpeed,
  type BattleStageStat,
} from "./battleStages";
import { getLearnedMoveBias, getLearnedStateValueBias } from "./learnedModel";
import type { SimBattlePokemon, SimMove } from "./simulator";

export type MoveEvaluation = {
  move: SimMove;
  expectedDamage: number;
  koChance: number;
  priority: number;
  accuracy: number;
  score: number;
  twoTurnKoChance: number;
  twoTurnSetupScore: number;
};

export type AiEvaluationOptions = {
  enableRollout?: boolean;
};

type FinishingPlan = {
  move: SimMove | null;
  finishChance: number;
  expectedDamage: number;
};

type StageImpactSummary = {
  attackSwing: number;
  defenseBreak: number;
  speedSwing: number;
};

type WeightedMoveOption = {
  move: SimMove;
  probability: number;
  score: number;
};

type ProjectedTurnOutcome = {
  attacker: SimBattlePokemon;
  defender: SimBattlePokemon;
  probability: number;
  opponentMove: SimMove;
};

type DamageDistribution = Map<number, number>;
type DefenderState = {
  hp: number;
  focusSashUsed: boolean;
  substituteHp: number;
};

const CRIT_BUCKETS = 24;
const MAX_BASE_DAMAGE_CACHE_SIZE = 5000;
const MAX_EVALUATION_CACHE_SIZE = 3000;
const MAX_MOVE_DISTRIBUTION_CACHE_SIZE = 5000;
const MAX_WEIGHTED_POLICY_CACHE_SIZE = 3000;
const MAX_BASE_EVALUATION_CACHE_SIZE = 3000;
const MAX_ROLLOUT_CACHE_SIZE = 8000;
const MAX_ROLLOUT_STATE_CACHE_SIZE = 4000;
const ROLLOUT_DEPTH = 3;
const ROLLOUT_CANDIDATE_COUNT = 3;
const ROLLOUT_OPPONENT_OPTION_COUNT = 3;
const baseDamageRollCache = new Map<string, number[]>();
const evaluationCache = new Map<string, MoveEvaluation[]>();
const moveDamageDistributionCache = new Map<string, Array<[number, number]>>();
const weightedMovePolicyCache = new Map<string, WeightedMoveOption[]>();
const baseEvaluationCache = new Map<string, MoveEvaluation[]>();
const rolloutMoveCache = new Map<string, number>();
const rolloutStateCache = new Map<string, number>();

const capCacheSize = (cache: Map<string, unknown>, maxSize: number) => {
  if (cache.size >= maxSize) {
    cache.clear();
  }
};

const getStaticDamageKey = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  powerOverride = move.power
) => {
  const relevantAttackStat = getRelevantStats(attacker, move).attack;
  const relevantDefenseStat = getDefenseStatWithItem(defender, move.category);

  return [
    attacker.level,
    attacker.pokemon.id,
    attacker.itemId ?? "",
    relevantAttackStat,
    attacker.pokemon.types.join(","),
    defender.pokemon.id,
    defender.itemId ?? "",
    relevantDefenseStat,
    defender.pokemon.types.join(","),
    move.id,
    powerOverride,
    move.critRank ?? 0,
  ].join("|");
};

const getEvaluationKey = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon
) =>
  [
    attacker.pokemon.id,
    attacker.itemId ?? "",
    attacker.choiceLockedMoveId ?? "",
    attacker.currentHp,
    attacker.effectiveSpeed,
    attacker.focusSashUsed ? 1 : 0,
    attacker.burned ? 1 : 0,
    attacker.paralyzed ? 1 : 0,
    attacker.poisoned ? 1 : 0,
    attacker.toxic ? 1 : 0,
    attacker.toxicTurns,
    attacker.sleepTurnsRemaining,
    attacker.substituteHp,
    attacker.wishTurnsRemaining,
    attacker.wishHealAmount,
    attacker.protectActive ? 1 : 0,
    attacker.protectUsedLastTurn ? 1 : 0,
    attacker.stages.atk,
    attacker.stages.def,
    attacker.stages.spa,
    attacker.stages.spd,
    attacker.stages.spe,
    attacker.stats.hp,
    attacker.stats.atk,
    attacker.stats.def,
    attacker.stats.spa,
    attacker.stats.spd,
    attacker.stats.spe,
    attacker.moves.map((move) => move.id).join(","),
    defender.pokemon.id,
    defender.itemId ?? "",
    defender.choiceLockedMoveId ?? "",
    defender.currentHp,
    defender.effectiveSpeed,
    defender.focusSashUsed ? 1 : 0,
    defender.burned ? 1 : 0,
    defender.paralyzed ? 1 : 0,
    defender.poisoned ? 1 : 0,
    defender.toxic ? 1 : 0,
    defender.toxicTurns,
    defender.sleepTurnsRemaining,
    defender.substituteHp,
    defender.wishTurnsRemaining,
    defender.wishHealAmount,
    defender.protectActive ? 1 : 0,
    defender.protectUsedLastTurn ? 1 : 0,
    defender.stages.atk,
    defender.stages.def,
    defender.stages.spa,
    defender.stages.spd,
    defender.stages.spe,
    defender.stats.hp,
    defender.stats.atk,
    defender.stats.def,
    defender.stats.spa,
    defender.stats.spd,
    defender.stats.spe,
    defender.moves.map((move) => move.id).join(","),
  ].join("|");

const getMoveDistributionKey = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => [getEvaluationKey(attacker, defender), move.id].join("|");

export const clearAiCaches = () => {
  baseDamageRollCache.clear();
  evaluationCache.clear();
  moveDamageDistributionCache.clear();
  weightedMovePolicyCache.clear();
  baseEvaluationCache.clear();
  rolloutMoveCache.clear();
  rolloutStateCache.clear();
};

const getAttackStatItemMultiplier = (
  itemId: string | undefined,
  category: SimMove["category"]
) => {
  if (itemId === "choiceBand" && category === "physical") {
    return 1.5;
  }

  if (itemId === "choiceSpecs" && category === "special") {
    return 1.5;
  }

  if (itemId === "lifeOrb") {
    return 1.3;
  }

  return 1;
};

const getAttackAbilityMultiplier = (
  pokemon: SimBattlePokemon,
  category: SimMove["category"]
) => {
  if (category === "physical" && pokemon.pokemon.id === "reosan") {
    return 2;
  }

  return 1;
};

const getFinalDamageItemMultiplier = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => {
  let multiplier = 1;

  if (attacker.itemId === "lifeOrb") {
    multiplier *= 1.3;
  }

  if (
    attacker.itemId === "expertBelt" &&
    getTypeEffectiveness(move.type, defender.pokemon.types) > 1
  ) {
    multiplier *= 1.2;
  }

  return multiplier;
};

const getDefenseStatWithItem = (
  pokemon: SimBattlePokemon,
  category: SimMove["category"]
) => {
  if (category === "special") {
    const baseDefense = pokemon.stats.spd;
    const stagedDefense = applyStageMultiplier(baseDefense, pokemon.stages.spd);
    return pokemon.itemId === "assaultVest" ? Math.floor(stagedDefense * 1.5) : stagedDefense;
  }

  const stagedDefense = applyStageMultiplier(pokemon.stats.def, pokemon.stages.def);
  if (pokemon.pokemon.id === "bucchi") {
    return stagedDefense * 2;
  }

  return stagedDefense;
};

const getRelevantStats = (pokemon: SimBattlePokemon, move: SimMove) => {
  if (move.category === "physical") {
    return {
      attack: Math.floor(
        applyStageMultiplier(pokemon.stats.atk, pokemon.stages.atk) *
          getAttackStatItemMultiplier(pokemon.itemId, move.category) *
          getAttackAbilityMultiplier(pokemon, move.category) *
          (pokemon.burned ? 0.5 : 1)
      ),
    };
  }

  return {
    attack: Math.floor(
      applyStageMultiplier(pokemon.stats.spa, pokemon.stages.spa) *
        getAttackStatItemMultiplier(pokemon.itemId, move.category) *
        getAttackAbilityMultiplier(pokemon, move.category)
    ),
  };
};

const getRockyHelmetDamage = (pokemon: SimBattlePokemon) => {
  return Math.max(1, Math.floor(pokemon.stats.hp / 6));
};

const getBurnDamage = (pokemon: SimBattlePokemon) => {
  return Math.max(1, Math.floor(pokemon.stats.hp / 16));
};

const getPoisonDamage = (pokemon: SimBattlePokemon) => {
  return Math.max(1, Math.floor(pokemon.stats.hp / 8));
};

const getToxicDamage = (pokemon: SimBattlePokemon) => {
  return Math.max(1, Math.floor((pokemon.stats.hp * Math.max(1, pokemon.toxicTurns)) / 16));
};

const getLeftoversHealing = (pokemon: SimBattlePokemon) => {
  return Math.max(1, Math.floor(pokemon.stats.hp / 16));
};

const getSubstituteCost = (pokemon: SimBattlePokemon) => {
  return Math.max(1, Math.floor(pokemon.stats.hp / 4));
};

const hasActiveSubstitute = (pokemon: SimBattlePokemon) => {
  return pokemon.currentHp > 0 && pokemon.substituteHp > 0;
};

const canUseSubstitute = (pokemon: SimBattlePokemon) => {
  const cost = getSubstituteCost(pokemon);
  return !hasActiveSubstitute(pokemon) && pokemon.currentHp > cost;
};

const applySubstitute = (pokemon: SimBattlePokemon) => {
  if (!canUseSubstitute(pokemon)) {
    return false;
  }

  const cost = getSubstituteCost(pokemon);
  pokemon.currentHp -= cost;
  pokemon.substituteHp = cost;
  return true;
};

const applyLeftoversRecovery = (pokemon: SimBattlePokemon) => {
  if (pokemon.currentHp <= 0 || pokemon.itemId !== "leftovers" || pokemon.currentHp >= pokemon.stats.hp) {
    return 0;
  }

  const healing = getLeftoversHealing(pokemon);
  const nextHp = Math.min(pokemon.stats.hp, pokemon.currentHp + healing);
  const recovered = nextHp - pokemon.currentHp;
  pokemon.currentHp = nextHp;
  return recovered;
};

const getWishHealAmount = (pokemon: SimBattlePokemon) => {
  return Math.max(1, Math.floor(pokemon.stats.hp / 2));
};

const scheduleWish = (pokemon: SimBattlePokemon) => {
  pokemon.wishTurnsRemaining = 2;
  pokemon.wishHealAmount = getWishHealAmount(pokemon);
};

const applyWishEndOfTurn = (pokemon: SimBattlePokemon) => {
  if (pokemon.wishTurnsRemaining <= 0) {
    return 0;
  }

  pokemon.wishTurnsRemaining = Math.max(0, pokemon.wishTurnsRemaining - 1);
  if (pokemon.wishTurnsRemaining > 0 || pokemon.currentHp <= 0) {
    return 0;
  }

  const healAmount = Math.min(
    pokemon.wishHealAmount,
    Math.max(0, pokemon.stats.hp - pokemon.currentHp)
  );
  pokemon.currentHp += healAmount;
  pokemon.wishHealAmount = 0;
  return healAmount;
};

const canUseRest = (pokemon: SimBattlePokemon) => {
  if (pokemon.currentHp <= 0 || pokemon.sleepTurnsRemaining > 0) {
    return false;
  }

  return pokemon.currentHp < pokemon.stats.hp || hasPrimaryStatus(pokemon);
};

const getRestRecoveryAmount = (pokemon: SimBattlePokemon) => {
  if (!canUseRest(pokemon)) {
    return 0;
  }

  return Math.max(0, pokemon.stats.hp - pokemon.currentHp);
};

const applyRest = (pokemon: SimBattlePokemon) => {
  if (!canUseRest(pokemon)) {
    return 0;
  }

  const recovery = Math.max(0, pokemon.stats.hp - pokemon.currentHp);
  pokemon.currentHp = pokemon.stats.hp;
  pokemon.burned = false;
  pokemon.paralyzed = false;
  pokemon.poisoned = false;
  pokemon.toxic = false;
  pokemon.toxicTurns = 0;
  pokemon.sleepTurnsRemaining = 2;
  refreshEffectiveSpeed(pokemon);
  return recovery;
};

const getSelfRecoveryAmount = (
  pokemon: SimBattlePokemon,
  move: SimMove
) => {
  if (!move.recoverRatio || pokemon.currentHp <= 0 || pokemon.currentHp >= pokemon.stats.hp) {
    return 0;
  }

  const missingHp = pokemon.stats.hp - pokemon.currentHp;
  const recovery = Math.max(1, Math.floor(pokemon.stats.hp * move.recoverRatio));
  return Math.min(missingHp, recovery);
};

const applySelfRecovery = (
  pokemon: SimBattlePokemon,
  move: SimMove
) => {
  const recovery = getSelfRecoveryAmount(pokemon, move);
  if (recovery <= 0) {
    return 0;
  }

  pokemon.currentHp = Math.min(pokemon.stats.hp, pokemon.currentHp + recovery);
  return recovery;
};

const moveTargetsOpponent = (move: SimMove) => {
  return Boolean(move.targetStatus || move.targetStatChanges);
};

const hasPrimaryStatus = (pokemon: SimBattlePokemon) => {
  return (
    pokemon.burned ||
    pokemon.paralyzed ||
    pokemon.poisoned ||
    pokemon.toxic ||
    pokemon.sleepTurnsRemaining > 0
  );
};

const consumeSleepTurn = (pokemon: SimBattlePokemon) => {
  if (pokemon.sleepTurnsRemaining <= 0) {
    return false;
  }

  pokemon.sleepTurnsRemaining = Math.max(0, pokemon.sleepTurnsRemaining - 1);
  return true;
};

const canApplyBurn = (defender: SimBattlePokemon) => {
  return !hasPrimaryStatus(defender) && !defender.pokemon.types.includes("fire");
};

const canApplyParalysis = (defender: SimBattlePokemon, move?: SimMove) => {
  if (hasPrimaryStatus(defender)) {
    return false;
  }

  if (move?.id === "でんじは") {
    return !defender.pokemon.types.includes("ground") && !defender.pokemon.types.includes("electric");
  }

  return true;
};

const canApplyPoison = (defender: SimBattlePokemon, move?: SimMove) => {
  if (hasPrimaryStatus(defender)) {
    return false;
  }

  if (
    defender.pokemon.types.includes("poison") ||
    defender.pokemon.types.includes("steel")
  ) {
    return false;
  }

  if (move?.id === "どくのこな" && defender.pokemon.types.includes("grass")) {
    return false;
  }

  return true;
};

const hasResidualStatus = (pokemon: SimBattlePokemon) => {
  return pokemon.burned || pokemon.poisoned || pokemon.toxic;
};

const getResidualDamagePressure = (pokemon: SimBattlePokemon) => {
  return (
    (pokemon.burned ? getBurnDamage(pokemon) : 0) +
    (pokemon.poisoned ? getPoisonDamage(pokemon) : 0) +
    (pokemon.toxic ? getToxicDamage(pokemon) : 0)
  );
};

const getStallToolkitStrength = (pokemon: SimBattlePokemon) => {
  let strength = 0;

  if (pokemon.itemId === "leftovers") {
    strength += 1;
  }

  if (getAvailableMoves(pokemon).some((move) => move.protect)) {
    strength += 1;
  }

  if (getAvailableMoves(pokemon).some((move) => move.substitute)) {
    strength += 1;
  }

  return strength;
};

const getPoisonSetupMove = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon
) => {
  return getAvailableMoves(attacker).find((candidateMove) =>
    (candidateMove.targetStatus === "toxic" || candidateMove.targetStatus === "poison") &&
    canApplyPoison(defender, candidateMove)
  );
};

const getStallSetupPenalty = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  immediateThreat: { expectedDamage: number; koChance: number }
) => {
  if ((!move.protect && !move.substitute) || hasResidualStatus(defender)) {
    return 0;
  }

  const poisonSetupMove = getPoisonSetupMove(attacker, defender);
  if (!poisonSetupMove) {
    return 0;
  }

  const toolkitStrength = getStallToolkitStrength(attacker);
  if (toolkitStrength <= 0) {
    return 0;
  }

  const directThreatRelief =
    immediateThreat.expectedDamage * (move.protect ? 1 : 0.45) +
    immediateThreat.koChance * (move.protect ? 9000 : 4500);
  const setupTax = toolkitStrength * (move.protect ? 11_000 : 13_000);

  return Math.max(0, setupTax - directThreatRelief);
};

const getPoisonPlanBonus = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => {
  const isPoisonMove = move.targetStatus === "poison" || move.targetStatus === "toxic";
  if (!isPoisonMove || hasResidualStatus(defender) || hasActiveSubstitute(defender)) {
    return 0;
  }

  const toolkitStrength = getStallToolkitStrength(attacker);
  const leftoversBonus = attacker.itemId === "leftovers" ? getLeftoversHealing(attacker) * 110 : 0;
  const toxicMultiplier = move.targetStatus === "toxic" ? 1.45 : 1;

  return (
    toolkitStrength * 18_000 * toxicMultiplier +
    leftoversBonus +
    getPoisonDamage(defender) * 140 * toxicMultiplier
  );
};

const getWishPlanValue = (
  attacker: SimBattlePokemon,
  move: SimMove,
  projectedAttacker: SimBattlePokemon,
  projectedDefender: SimBattlePokemon,
  immediateThreat: { expectedDamage: number; koChance: number }
) => {
  if (!move.wish || projectedAttacker.wishTurnsRemaining <= 0 || projectedAttacker.currentHp <= 0) {
    return 0;
  }

  const nextThreat =
    projectedDefender.currentHp > 0
      ? getBestAttackMoveMetrics(projectedDefender, projectedAttacker)
      : { move: null, expectedDamage: 0, koChance: 0, score: 0 };
  const survivalChance = Math.max(0, 1 - nextThreat.koChance);
  const protectCoverage = getAvailableMoves(projectedAttacker).some((candidateMove) => candidateMove.protect)
    ? 1.2
    : 1;
  const healAmount = projectedAttacker.wishHealAmount || getWishHealAmount(projectedAttacker);
  const urgency = 1 + immediateThreat.koChance * 2 + Math.min(1.5, immediateThreat.expectedDamage / Math.max(attacker.currentHp, 1));

  return healAmount * 95 * survivalChance * protectCoverage * urgency;
};

const getRestPlanValue = (
  attacker: SimBattlePokemon,
  projectedAttacker: SimBattlePokemon,
  projectedDefender: SimBattlePokemon,
  immediateThreat: { expectedDamage: number; koChance: number }
) => {
  if (projectedAttacker.sleepTurnsRemaining <= 0 || projectedAttacker.currentHp <= 0) {
    return 0;
  }

  const healing = getRestRecoveryAmount(attacker);
  const statusCureBonus =
    (attacker.burned ? getBurnDamage(attacker) * 18 : 0) +
    (attacker.paralyzed ? 9_000 : 0) +
    (attacker.poisoned ? getPoisonDamage(attacker) * 28 : 0) +
    (attacker.toxic ? getToxicDamage(attacker) * 34 : 0);
  const nextThreat =
    projectedDefender.currentHp > 0
      ? getBestAttackMoveMetrics(projectedDefender, projectedAttacker)
      : { move: null, expectedDamage: 0, koChance: 0, score: 0 };
  const sleepPenalty =
    nextThreat.expectedDamage * 24 +
    nextThreat.koChance * 14_000 +
    projectedAttacker.sleepTurnsRemaining * 2_500;
  const urgency =
    1 +
    immediateThreat.koChance * 2.5 +
    Math.min(1.5, immediateThreat.expectedDamage / Math.max(attacker.currentHp, 1));
  const crisisBonus =
    attacker.currentHp <= Math.floor(attacker.stats.hp / 3)
      ? 14_000 + immediateThreat.koChance * 8_000
      : 0;

  return healing * 180 * urgency + statusCureBonus + crisisBonus - sleepPenalty;
};

const getStallAttackPenalty = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  immediateWinChance: number,
  expectedDamageRatio: number
) => {
  const poisonSetupMove = getPoisonSetupMove(attacker, defender);
  if (
    !poisonSetupMove ||
    hasResidualStatus(defender) ||
    move.id === poisonSetupMove.id ||
    move.protect ||
    move.substitute
  ) {
    return 0;
  }

  if (immediateWinChance >= 0.7 || expectedDamageRatio >= 0.8) {
    return 0;
  }

  const toolkitStrength = getStallToolkitStrength(attacker);
  if (toolkitStrength <= 0) {
    return 0;
  }

  return toolkitStrength * 20_000 * (1 - Math.min(expectedDamageRatio, 0.7));
};

const refreshEffectiveSpeed = (pokemon: SimBattlePokemon) => {
  pokemon.effectiveSpeed = getEffectiveSpeed(
    pokemon.stats.spe,
    pokemon.itemId,
    pokemon.stages.spe,
    pokemon.paralyzed
  );
};

const applyRockyHelmetDamage = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  damageDealt: number
) => {
  if (!move.isContact || damageDealt <= 0 || defender.itemId !== "rockyHelmet") {
    return;
  }

  attacker.currentHp = Math.max(0, attacker.currentHp - getRockyHelmetDamage(attacker));
};

const getMoveCritChance = (move: SimMove) => {
  const rank = move.critRank ?? 0;

  if (rank >= 3) {
    return 1;
  }

  if (rank === 2) {
    return 0.5;
  }

  if (rank === 1) {
    return 1 / 8;
  }

  return 1 / 24;
};

const getMoveAccuracy = (
  move: SimMove,
  attacker?: SimBattlePokemon
) => {
  if (attacker && move.targetStatus === "toxic" && attacker.pokemon.types.includes("poison")) {
    return 100;
  }

  return move.accuracy ?? 100;
};

const getAccuracyMultiplier = (move: SimMove, attacker?: SimBattlePokemon) =>
  getMoveAccuracy(move, attacker) / 100;

const getProjectedMoveOutcomeOptions = (
  attacker: SimBattlePokemon,
  move: SimMove
) => {
  if (move.protect) {
    return [{ hit: true, probability: 1 }];
  }

  const accuracyRate = getAccuracyMultiplier(move, attacker);
  if (accuracyRate <= 0) {
    return [{ hit: false, probability: 1 }];
  }

  if (accuracyRate >= 1) {
    return [{ hit: true, probability: 1 }];
  }

  return [
    { hit: true, probability: accuracyRate },
    { hit: false, probability: 1 - accuracyRate },
  ];
};

const getHitPowerAtIndex = (move: SimMove, hitIndex: number) => {
  return move.power + (move.powerStep ?? 0) * (hitIndex - 1);
};

const getConditionalHitSequences = (move: SimMove) => {
  if (move.hitCount && move.hitCount > 1 && move.accuracyPerHit) {
    const accuracyRate = getAccuracyMultiplier(move);
    const sequences: Array<{ powers: number[]; probability: number }> = [];

    for (let landedHits = 1; landedHits <= move.hitCount; landedHits += 1) {
      const probability =
        landedHits === move.hitCount
          ? Math.pow(accuracyRate, move.hitCount - 1)
          : Math.pow(accuracyRate, landedHits - 1) * (1 - accuracyRate);

      sequences.push({
        powers: Array.from({ length: landedHits }, (_, index) =>
          getHitPowerAtIndex(move, index + 1)
        ),
        probability,
      });
    }

    return sequences;
  }

  if (move.hitCount && move.hitCount > 1) {
    return [
      {
        powers: Array.from({ length: move.hitCount }, (_, index) =>
          getHitPowerAtIndex(move, index + 1)
        ),
        probability: 1,
      },
    ];
  }

  if (move.minHits && move.maxHits) {
    const minHits = move.minHits;
    const maxHits = move.maxHits;
    const hitCounts = Array.from(
      { length: maxHits - minHits + 1 },
      (_, index) => minHits + index
    );
    const probabilities =
      minHits === 2 && maxHits === 5
        ? [0.35, 0.35, 0.15, 0.15]
        : hitCounts.map(() => 1 / hitCounts.length);

    return hitCounts.map((count, index) => ({
      powers: Array.from({ length: count }, () => move.power),
      probability: probabilities[index] ?? 0,
    }));
  }

  return [
    {
      powers: [move.power],
      probability: 1,
    },
  ];
};

const getRepresentativeHitPowers = (move: SimMove) => {
  const sequences = getConditionalHitSequences(move).sort((left, right) => {
    if (right.probability !== left.probability) {
      return right.probability - left.probability;
    }

    return right.powers.length - left.powers.length;
  });

  return sequences[0]?.powers ?? [move.power];
};

const getExpectedHitCount = (move: SimMove) => {
  return getConditionalHitSequences(move).reduce(
    (sum, sequence) => sum + sequence.powers.length * sequence.probability,
    0
  );
};

const getScaledDamageRolls = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  isCrit: boolean,
  powerOverride = move.power
) => {
  const { attack } = getRelevantStats(attacker, move);
  const damage = calcDamage({
    level: attacker.level,
    power: powerOverride,
    attack,
    defense: getDefenseStatWithItem(defender, move.category),
    moveType: move.type,
    attackerTypes: attacker.pokemon.types,
    defenderTypes: defender.pokemon.types,
    defenderHp: defender.currentHp,
    isCrit,
  });

  const finalDamageMultiplier = getFinalDamageItemMultiplier(attacker, defender, move);
  return damage.rolls.map((roll) => Math.floor(roll * finalDamageMultiplier));
};

export const getSingleHitDamageRolls = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  powerOverride = move.power
): number[] => {
  const staticDamageKey = getStaticDamageKey(attacker, defender, move, powerOverride);
  let scaledRolls = baseDamageRollCache.get(staticDamageKey);

  if (!scaledRolls) {
    const critChance = getMoveCritChance(move);
    const normalRolls = getScaledDamageRolls(attacker, defender, move, false, powerOverride);
    const critRolls = getScaledDamageRolls(attacker, defender, move, true, powerOverride);
    const critBucketCount = Math.round(critChance * CRIT_BUCKETS);
    const normalBucketCount = Math.max(0, CRIT_BUCKETS - critBucketCount);
    const builtRolls: number[] = [];

    for (let i = 0; i < normalBucketCount; i += 1) {
      builtRolls.push(...normalRolls);
    }

    for (let i = 0; i < critBucketCount; i += 1) {
      builtRolls.push(...critRolls);
    }

    if (builtRolls.length === 0) {
      builtRolls.push(...normalRolls);
    }

    scaledRolls = builtRolls.sort((left, right) => left - right);
    capCacheSize(baseDamageRollCache, MAX_BASE_DAMAGE_CACHE_SIZE);
    baseDamageRollCache.set(staticDamageKey, scaledRolls);
  }

  return scaledRolls;
};

const getRollProbabilityMap = (rolls: number[]) => {
  const distribution: DamageDistribution = new Map();

  if (rolls.length === 0) {
    distribution.set(0, 1);
    return distribution;
  }

  for (const roll of rolls) {
    distribution.set(roll, (distribution.get(roll) ?? 0) + 1 / rolls.length);
  }

  return distribution;
};

const applyDamageToDefenderState = (
  state: DefenderState,
  defender: SimBattlePokemon,
  damage: number
): DefenderState => {
  if (state.hp <= 0) {
    return state;
  }

  if (state.substituteHp > 0) {
    return {
      hp: state.hp,
      focusSashUsed: state.focusSashUsed,
      substituteHp: Math.max(0, state.substituteHp - damage),
    };
  }

  const focusSashActivates =
    defender.itemId === "focusSash" &&
    !state.focusSashUsed &&
    state.hp === defender.stats.hp &&
    damage >= state.hp;

  if (focusSashActivates) {
    return {
      hp: 1,
      focusSashUsed: true,
      substituteHp: 0,
    };
  }

  return {
    hp: Math.max(0, state.hp - damage),
    focusSashUsed: state.focusSashUsed,
    substituteHp: 0,
  };
};

const getDamageDistribution = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => {
  const distributionKey = getMoveDistributionKey(attacker, defender, move);
  const cached = moveDamageDistributionCache.get(distributionKey);

  if (cached) {
    return new Map(cached);
  }

  const startHp = defender.currentHp;
  const sequenceDistributions = getConditionalHitSequences(move);
  const totalDistribution: DamageDistribution = new Map();

  for (const sequence of sequenceDistributions) {
    let stateDistribution = new Map<string, number>([
      [`${defender.currentHp}|${defender.focusSashUsed ? 1 : 0}|${defender.substituteHp}`, 1],
    ]);

    for (const power of sequence.powers) {
      const hitRolls = getRollProbabilityMap(
        getSingleHitDamageRolls(attacker, defender, move, power)
      );
      const nextStateDistribution = new Map<string, number>();

      for (const [stateKey, stateProbability] of stateDistribution.entries()) {
        const [hpText, sashText, substituteText] = stateKey.split("|");
        const state: DefenderState = {
          hp: Number(hpText),
          focusSashUsed: sashText == "1",
          substituteHp: Number(substituteText),
        };

        if (state.hp <= 0) {
          nextStateDistribution.set(
            stateKey,
            (nextStateDistribution.get(stateKey) ?? 0) + stateProbability
          );
          continue;
        }

        for (const [damage, damageProbability] of hitRolls.entries()) {
          const nextState = applyDamageToDefenderState(state, defender, damage);
          const nextStateKey = `${nextState.hp}|${nextState.focusSashUsed ? 1 : 0}|${nextState.substituteHp}`;
          nextStateDistribution.set(
            nextStateKey,
            (nextStateDistribution.get(nextStateKey) ?? 0) + stateProbability * damageProbability
          );
        }
      }

      stateDistribution = nextStateDistribution;
    }

    for (const [stateKey, stateProbability] of stateDistribution.entries()) {
      const [hpText] = stateKey.split("|");
      const remainingHp = Number(hpText);
      const totalDamage = Math.max(0, startHp - remainingHp);
      totalDistribution.set(
        totalDamage,
        (totalDistribution.get(totalDamage) ?? 0) + stateProbability * sequence.probability
      );
    }
  }

  capCacheSize(moveDamageDistributionCache, MAX_MOVE_DISTRIBUTION_CACHE_SIZE);
  moveDamageDistributionCache.set(distributionKey, Array.from(totalDistribution.entries()));

  return totalDistribution;
};

const getDefenderStateDistribution = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => {
  const sequenceDistributions = getConditionalHitSequences(move);
  const totalDistribution = new Map<string, number>();

  for (const sequence of sequenceDistributions) {
    let stateDistribution = new Map<string, number>([
      [`${defender.currentHp}|${defender.focusSashUsed ? 1 : 0}|${defender.substituteHp}`, 1],
    ]);

    for (const power of sequence.powers) {
      const hitRolls = getRollProbabilityMap(
        getSingleHitDamageRolls(attacker, defender, move, power)
      );
      const nextStateDistribution = new Map<string, number>();

      for (const [stateKey, stateProbability] of stateDistribution.entries()) {
        const [hpText, sashText, substituteText] = stateKey.split("|");
        const state: DefenderState = {
          hp: Number(hpText),
          focusSashUsed: sashText == "1",
          substituteHp: Number(substituteText),
        };

        if (state.hp <= 0) {
          nextStateDistribution.set(
            stateKey,
            (nextStateDistribution.get(stateKey) ?? 0) + stateProbability
          );
          continue;
        }

        for (const [damage, damageProbability] of hitRolls.entries()) {
          const nextState = applyDamageToDefenderState(state, defender, damage);
          const nextStateKey = `${nextState.hp}|${nextState.focusSashUsed ? 1 : 0}|${nextState.substituteHp}`;
          nextStateDistribution.set(
            nextStateKey,
            (nextStateDistribution.get(nextStateKey) ?? 0) + stateProbability * damageProbability
          );
        }
      }

      stateDistribution = nextStateDistribution;
    }

    for (const [stateKey, stateProbability] of stateDistribution.entries()) {
      totalDistribution.set(
        stateKey,
        (totalDistribution.get(stateKey) ?? 0) + stateProbability * sequence.probability
      );
    }
  }

  return totalDistribution;
};

const getExpectedSubstituteDamage = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => {
  if (!hasActiveSubstitute(defender)) {
    return 0;
  }

  const startSubstituteHp = defender.substituteHp;
  const stateDistribution = getDefenderStateDistribution(attacker, defender, move);
  let expectedDamage = 0;

  for (const [stateKey, probability] of stateDistribution.entries()) {
    const [, , substituteText] = stateKey.split("|");
    const remainingSubstituteHp = Number(substituteText);
    expectedDamage += (startSubstituteHp - remainingSubstituteHp) * probability;
  }

  return expectedDamage;
};

const getSubstituteBreakChance = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => {
  if (!hasActiveSubstitute(defender)) {
    return 0;
  }

  const stateDistribution = getDefenderStateDistribution(attacker, defender, move);
  let breakChance = 0;

  for (const [stateKey, probability] of stateDistribution.entries()) {
    const [, , substituteText] = stateKey.split("|");
    if (Number(substituteText) <= 0) {
      breakChance += probability;
    }
  }

  return breakChance;
};

export const getDamageRolls = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
): number[] => {
  const distribution = getDamageDistribution(attacker, defender, move);
  const rolls: number[] = [];

  for (const [damage, probability] of [...distribution.entries()].sort((left, right) => left[0] - right[0])) {
    const bucketCount = Math.max(1, Math.round(probability * 100));
    for (let index = 0; index < bucketCount; index += 1) {
      rolls.push(damage);
    }
  }

  return rolls;
};

export const getRepresentativeDamage = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => {
  const distribution = [...getDamageDistribution(attacker, defender, move).entries()].sort(
    (left, right) => left[0] - right[0]
  );
  let cumulativeProbability = 0;

  for (const [damage, probability] of distribution) {
    cumulativeProbability += probability;
    if (cumulativeProbability >= 0.5) {
      return damage;
    }
  }

  return distribution[distribution.length - 1]?.[0] ?? 0;
};

export const getExpectedDamage = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
): number => {
  const distribution = getDamageDistribution(attacker, defender, move);
  let expectedDamage = 0;

  for (const [damage, probability] of distribution.entries()) {
    expectedDamage += damage * probability;
  }

  return expectedDamage;
};

export const getKoChance = (
  distribution: DamageDistribution,
  currentHp: number
): number => {
  if (distribution.size === 0 || currentHp <= 0) {
    return 0;
  }

  let koChance = 0;
  for (const [damage, probability] of distribution.entries()) {
    if (damage >= currentHp) {
      koChance += probability;
    }
  }

  return koChance;
};

const getMovePriority = (move: SimMove) => {
  return move.priority ?? 0;
};

const getEffectiveKoChance = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => {
  const distribution = getDamageDistribution(attacker, defender, move);
  return getKoChance(distribution, defender.currentHp) * getAccuracyMultiplier(move, attacker);
};

const cloneBattlePokemon = (pokemon: SimBattlePokemon): SimBattlePokemon => ({
  ...pokemon,
  evs: { ...pokemon.evs },
  ivs: { ...pokemon.ivs },
  stats: { ...pokemon.stats },
  moves: [...pokemon.moves],
  stages: { ...pokemon.stages },
  sleepTurnsRemaining: pokemon.sleepTurnsRemaining,
  wishTurnsRemaining: pokemon.wishTurnsRemaining,
  wishHealAmount: pokemon.wishHealAmount,
});

const isChoiceItem = (itemId?: string) => {
  return itemId === "choiceBand" || itemId === "choiceSpecs" || itemId === "choiceScarf";
};

const getAvailableMoves = (pokemon: SimBattlePokemon) => {
  if (!pokemon.choiceLockedMoveId) {
    return pokemon.moves;
  }

  const lockedMove = pokemon.moves.find((move) => move.id === pokemon.choiceLockedMoveId);
  return lockedMove ? [lockedMove] : pokemon.moves;
};

const getAvailableAttackMoves = (pokemon: SimBattlePokemon) =>
  getAvailableMoves(pokemon).filter((move) => move.category !== "status" && !move.protect);

const hasRolloutSensitiveMove = (pokemon: SimBattlePokemon) =>
  getAvailableMoves(pokemon).some(
    (move) =>
      move.protect ||
      move.substitute ||
      Boolean(move.recoverRatio) ||
      Boolean(move.rest) ||
      Boolean(move.wish) ||
      move.targetStatus === "poison" ||
      move.targetStatus === "toxic"
  );

const getWeightedMovePolicyKey = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  opposingMove?: SimMove | null
) => [getEvaluationKey(attacker, defender), opposingMove?.id ?? ""].join("|");

const applyChoiceLock = (pokemon: SimBattlePokemon, move: SimMove | null) => {
  if (!move || !isChoiceItem(pokemon.itemId) || pokemon.choiceLockedMoveId) {
    return;
  }

  pokemon.choiceLockedMoveId = move.id;
};

const getRepresentativeSingleHitDamage = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  powerOverride = move.power
) => {
  const rolls = getSingleHitDamageRolls(attacker, defender, move, powerOverride);
  if (rolls.length === 0) {
    return 0;
  }

  return rolls[Math.floor(rolls.length / 2)];
};

const applySelfStatChanges = (
  pokemon: SimBattlePokemon,
  move: SimMove
) => {
  if (!move.selfStatChanges || pokemon.currentHp <= 0) {
    return [];
  }

  const { nextStages, appliedChanges } = applyBattleStageChanges(
    pokemon.stages,
    move.selfStatChanges
  );
  pokemon.stages = nextStages;
  refreshEffectiveSpeed(pokemon);

  return (Object.entries(appliedChanges) as Array<[BattleStageStat, number]>).map(
    ([stat, delta]) => formatBattleStageChange(stat, delta)
  );
};

const applyTargetStatChanges = (
  defender: SimBattlePokemon,
  move: SimMove,
  forcedChance?: number
) => {
  if (!move.targetStatChanges || defender.currentHp <= 0) {
    return [];
  }

  const chance = forcedChance ?? move.targetStatChangeChance ?? 1;
  if (chance < 1 && chance <= 0.5) {
    return [];
  }

  const { nextStages, appliedChanges } = applyBattleStageChanges(
    defender.stages,
    move.targetStatChanges
  );
  defender.stages = nextStages;
  refreshEffectiveSpeed(defender);

  return (Object.entries(appliedChanges) as Array<[BattleStageStat, number]>).map(
    ([stat, delta]) => formatBattleStageChange(stat, delta)
  );
};

const applyTargetStatus = (
  defender: SimBattlePokemon,
  move: SimMove
) => {
  if (move.targetStatus === "burn" && canApplyBurn(defender)) {
    defender.burned = true;
    return true;
  }

  if (move.targetStatus === "paralysis" && canApplyParalysis(defender, move)) {
    defender.paralyzed = true;
    refreshEffectiveSpeed(defender);
    return true;
  }

  if (move.targetStatus === "poison" && canApplyPoison(defender, move)) {
    defender.poisoned = true;
    defender.toxic = false;
    defender.toxicTurns = 0;
    return true;
  }

  if (move.targetStatus === "toxic" && canApplyPoison(defender, move)) {
    defender.toxic = true;
    defender.poisoned = false;
    defender.toxicTurns = 1;
    return true;
  }

  return false;
};

const applyProjectedMove = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  willHit = true
) => {
  if (consumeSleepTurn(attacker)) {
    return;
  }

  if (move.protect) {
    if (!attacker.protectUsedLastTurn) {
      attacker.protectActive = true;
    }
    return;
  }

  if (move.substitute) {
    applySubstitute(attacker);
    return;
  }

  if (!willHit) {
    return;
  }

  if (move.category === "status") {
    if (move.rest) {
      applyRest(attacker);
    }
    if (move.wish) {
      scheduleWish(attacker);
    }
    const blockedBySubstitute = hasActiveSubstitute(defender) && moveTargetsOpponent(move);
    if (!defender.protectActive && !blockedBySubstitute) {
      applyTargetStatChanges(defender, move, 1);
      applyTargetStatus(defender, move);
    }
    applySelfStatChanges(attacker, move);
    if (!move.wish && !move.rest) {
      applySelfRecovery(attacker, move);
    }
    return;
  }

  if (defender.protectActive) {
    applySelfStatChanges(attacker, move);
    return;
  }

  let totalDamageDealt = 0;
  let damageToHpDealt = 0;
  for (const power of getRepresentativeHitPowers(move)) {
    if (attacker.currentHp <= 0 || defender.currentHp <= 0) {
      break;
    }

    const damage = getRepresentativeSingleHitDamage(attacker, defender, move, power);
    const defenderHpBeforeSingleHit = defender.currentHp;
    const defenderSubstituteBeforeSingleHit = defender.substituteHp;
    const nextState = applyDamageToDefenderState(
      {
        hp: defender.currentHp,
        focusSashUsed: defender.focusSashUsed,
        substituteHp: defender.substituteHp,
      },
      defender,
      damage
    );

    defender.currentHp = nextState.hp;
    defender.focusSashUsed = nextState.focusSashUsed;
    defender.substituteHp = nextState.substituteHp;
    const damageToHpThisHit = Math.max(defenderHpBeforeSingleHit - defender.currentHp, 0);
    const damageToSubstituteThisHit = Math.max(
      defenderSubstituteBeforeSingleHit - defender.substituteHp,
      0
    );
    const totalDamageThisHit = damageToHpThisHit + damageToSubstituteThisHit;
    totalDamageDealt += totalDamageThisHit;
    damageToHpDealt += damageToHpThisHit;
    applyRockyHelmetDamage(
      attacker,
      defender,
      move,
      totalDamageThisHit
    );
  }

  if (
    damageToHpDealt > 0 &&
    defender.currentHp > 0 &&
    !hasActiveSubstitute(defender) &&
    move.burnChance &&
    move.burnChance >= 0.5 &&
    canApplyBurn(defender)
  ) {
    defender.burned = true;
  }

  if (
    damageToHpDealt > 0 &&
    defender.currentHp > 0 &&
    !hasActiveSubstitute(defender) &&
    move.poisonChance &&
    move.poisonChance >= 0.5 &&
    canApplyPoison(defender, move)
  ) {
    defender.poisoned = true;
    defender.toxic = false;
    defender.toxicTurns = 0;
  }

  if (
    damageToHpDealt > 0 &&
    defender.currentHp > 0 &&
    !hasActiveSubstitute(defender) &&
    move.toxicChance &&
    move.toxicChance >= 0.5 &&
    canApplyPoison(defender, move)
  ) {
    defender.toxic = true;
    defender.poisoned = false;
    defender.toxicTurns = 1;
  }

  if (move.drainRatio && totalDamageDealt > 0 && attacker.currentHp > 0) {
    const recovery = Math.floor(totalDamageDealt * move.drainRatio);
    const boostedRecovery =
      attacker.itemId === "bigRoot" ? Math.floor(recovery * 1.3) : recovery;
    attacker.currentHp = Math.min(
      attacker.stats.hp,
      attacker.currentHp + boostedRecovery
    );
  }

  if (move.recoilRatio && totalDamageDealt > 0 && attacker.currentHp > 0) {
    attacker.currentHp = Math.max(
      0,
      attacker.currentHp - Math.max(1, totalDamageDealt * move.recoilRatio)
    );
  }

  applySelfStatChanges(attacker, move);
  if (damageToHpDealt > 0 && defender.currentHp > 0 && !hasActiveSubstitute(defender)) {
    applyTargetStatChanges(defender, move, 1);
  }
};

type AttackMoveMetrics = {
  move: SimMove | null;
  expectedDamage: number;
  koChance: number;
  score: number;
};

const getBestAttackMoveMetrics = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon
): AttackMoveMetrics => {
  let bestMove: SimMove | null = null;
  let bestExpectedDamage = 0;
  let bestKoChance = 0;
  let bestScore = -Infinity;

  for (const move of getAvailableAttackMoves(attacker)) {
    const expectedDamage =
      getExpectedDamage(attacker, defender, move) * getAccuracyMultiplier(move, attacker);
    const koChance = getEffectiveKoChance(attacker, defender, move);
    const substituteDamage =
      hasActiveSubstitute(defender)
        ? getExpectedSubstituteDamage(attacker, defender, move) * getAccuracyMultiplier(move, attacker)
        : 0;
    const substituteBreakChance =
      hasActiveSubstitute(defender)
        ? getSubstituteBreakChance(attacker, defender, move) * getAccuracyMultiplier(move, attacker)
        : 0;
    const score =
      koChance * 20_000 +
      expectedDamage +
      substituteDamage * 10 +
      substituteBreakChance * 18_000 +
      Math.max(getMovePriority(move), 0) * 250;

    if (score > bestScore) {
      bestMove = move;
      bestExpectedDamage = expectedDamage + substituteDamage;
      bestKoChance = koChance;
      bestScore = score;
    }
  }

  return {
    move: bestMove,
    expectedDamage: bestExpectedDamage,
    koChance: bestKoChance,
    score: Math.max(bestScore, 0),
  };
};

const getTargetStageImpactSummary = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
): StageImpactSummary => {
  if (!move.targetStatChanges) {
    return {
      attackSwing: 0,
      defenseBreak: 0,
      speedSwing: 0,
    };
  }

  const projectedDefender = cloneBattlePokemon(defender);
  applyTargetStatChanges(projectedDefender, move, 1);

  const currentThreat = getBestAttackMoveMetrics(defender, attacker);
  const nextThreat = getBestAttackMoveMetrics(projectedDefender, attacker);
  const currentOffense = getBestAttackMoveMetrics(attacker, defender);
  const nextOffense = getBestAttackMoveMetrics(attacker, projectedDefender);

  return {
    attackSwing: Math.max(0, currentThreat.expectedDamage - nextThreat.expectedDamage),
    defenseBreak: Math.max(0, nextOffense.expectedDamage - currentOffense.expectedDamage),
    speedSwing:
      attacker.effectiveSpeed <= defender.effectiveSpeed &&
      attacker.effectiveSpeed > projectedDefender.effectiveSpeed
        ? 1
        : 0,
  };
};

const getShallowMoveScore = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  opposingMove?: SimMove | null
) => {
  const accuracy = getMoveAccuracy(move, attacker);
  const priority = getMovePriority(move);
  const accuracyPenalty = (100 - accuracy) * 8;
  const fallbackThreat = getBestAttackMoveMetrics(defender, attacker);
  const immediateThreat = getImmediateThreatSummary(attacker, defender, fallbackThreat, opposingMove);
  const referenceThreatMove = opposingMove && opposingMove.category !== "status" && !opposingMove.protect
    ? opposingMove
    : fallbackThreat.move;
  const residualPressure = getResidualDamagePressure(defender);
  const learnedBias = getLearnedMoveBias(attacker, defender, move);

  if (move.protect) {
    const stallSetupPenalty = getStallSetupPenalty(attacker, defender, move, immediateThreat);
    return (
      (attacker.protectUsedLastTurn ? -60_000 : 0) +
      immediateThreat.expectedDamage * 22 +
      immediateThreat.koChance * 9_000 +
      residualPressure * 170 +
      (attacker.itemId === "leftovers" ? getLeftoversHealing(attacker) * 130 : 0) -
      stallSetupPenalty +
      learnedBias +
      priority * 250
    );
  }

  if (move.category === "status") {
    if (move.substitute && !canUseSubstitute(attacker)) {
      return -80_000;
    }
    if (move.rest && !canUseRest(attacker)) {
      return -80_000;
    }

    const projectedAttacker = cloneBattlePokemon(attacker);
    const projectedDefender = cloneBattlePokemon(defender);
    const currentBestAttack = getBestAttackMoveMetrics(attacker, defender);

    applyProjectedMove(projectedAttacker, projectedDefender, move, true);
    applyProjectedEndOfTurnEffects(projectedAttacker, projectedDefender);

    const nextBestAttack = getBestAttackMoveMetrics(projectedAttacker, projectedDefender);
    const projectedOpponentThreat =
      projectedDefender.currentHp > 0
        ? getBestAttackMoveMetrics(projectedDefender, projectedAttacker)
        : { move: null, expectedDamage: 0, koChance: 0, score: 0 };
    const offenseGain = Math.max(0, nextBestAttack.score - currentBestAttack.score);
    const defenseGain = Math.max(
      0,
      fallbackThreat.expectedDamage - projectedOpponentThreat.expectedDamage
    ) * 18;
    const speedSwingValue =
      projectedAttacker.effectiveSpeed > projectedDefender.effectiveSpeed &&
      attacker.effectiveSpeed <= defender.effectiveSpeed
        ? 8_000
        : 0;
    const burnValue =
      move.targetStatus === "burn" && !hasActiveSubstitute(defender)
        ? (getAvailableAttackMoves(defender).some((candidateMove) => candidateMove.category === "physical")
            ? defender.stats.atk * 10
            : 0) + getBurnDamage(defender) * 90
        : 0;
    const paralysisValue =
      move.targetStatus === "paralysis" && !hasActiveSubstitute(defender)
        ? (attacker.effectiveSpeed < defender.effectiveSpeed &&
            projectedAttacker.effectiveSpeed > projectedDefender.effectiveSpeed
            ? 5_000
            : 0) + fallbackThreat.expectedDamage * 10 + fallbackThreat.koChance * 6_000
        : 0;
    const poisonValue =
      move.targetStatus === "poison" && !hasActiveSubstitute(defender)
        ? getPoisonDamage(defender) * 120 + fallbackThreat.expectedDamage * 6
        : 0;
    const toxicValue =
      move.targetStatus === "toxic" && !hasActiveSubstitute(defender)
        ? getPoisonDamage(defender) * 220 + fallbackThreat.expectedDamage * 10 + fallbackThreat.koChance * 4_000
        : 0;
    const poisonPlanBonus = getPoisonPlanBonus(attacker, defender, move);
    const targetStageImpact = !hasActiveSubstitute(defender)
      ? getTargetStageImpactSummary(attacker, defender, move)
      : { attackSwing: 0, defenseBreak: 0, speedSwing: 0 };
    const stallSetupPenalty = getStallSetupPenalty(attacker, defender, move, immediateThreat);
    const substituteValue =
      move.substitute
        ? (projectedAttacker.substituteHp > 0 ? 8_000 + projectedAttacker.substituteHp * 24 : -10_000) +
          Math.max(0, fallbackThreat.expectedDamage - projectedOpponentThreat.expectedDamage) * 8 +
          residualPressure * 90 +
          (attacker.itemId === "leftovers" ? getLeftoversHealing(attacker) * 70 : 0)
        : 0;
    const wishValue = getWishPlanValue(
      attacker,
      move,
      projectedAttacker,
      projectedDefender,
      immediateThreat
    );
    const restValue =
      move.rest
        ? getRestPlanValue(
            attacker,
            projectedAttacker,
            projectedDefender,
            immediateThreat
          )
        : 0;
    const immediateThreatWeight =
      1 +
      immediateThreat.koChance * 4 +
      Math.min(2, immediateThreat.expectedDamage / Math.max(attacker.currentHp, 1));
    const selfRecoveryValue =
      move.recoverRatio
        ? getSelfRecoveryAmount(attacker, move) * 110 * immediateThreatWeight
        : 0;

    return (
      getAccuracyMultiplier(move, attacker) *
        (offenseGain +
          defenseGain +
          speedSwingValue +
          burnValue +
          paralysisValue +
          poisonValue +
          toxicValue +
          poisonPlanBonus +
          (targetStageImpact.attackSwing * 14 +
            targetStageImpact.defenseBreak * 12 +
            targetStageImpact.speedSwing * 8_000) +
          wishValue +
          restValue +
          selfRecoveryValue +
          substituteValue +
          learnedBias +
          priority * 250 -
          stallSetupPenalty) -
      accuracyPenalty
    );
  }

  const expectedDamage =
    getExpectedDamage(attacker, defender, move) * getAccuracyMultiplier(move, attacker);
  const koChance = getEffectiveKoChance(attacker, defender, move);
  const actsBeforeReference =
    referenceThreatMove ? moveActsBeforeOpponent(attacker, defender, move, referenceThreatMove) : true;
  const defenderHasPhysicalMoves = getAvailableAttackMoves(defender).some(
    (candidateMove) => candidateMove.category === "physical"
  );
  const burnBonus =
    move.burnChance && !hasActiveSubstitute(defender) && canApplyBurn(defender)
      ? move.burnChance *
        ((defenderHasPhysicalMoves ? defender.stats.atk * 8 : 0) + getBurnDamage(defender) * 30)
      : 0;
  const poisonBonus =
    move.poisonChance && !hasActiveSubstitute(defender) && canApplyPoison(defender, move)
      ? move.poisonChance * (getPoisonDamage(defender) * 36 + fallbackThreat.expectedDamage * 5)
      : 0;
  const toxicBonus =
    move.toxicChance && !hasActiveSubstitute(defender) && canApplyPoison(defender, move)
      ? move.toxicChance * (getPoisonDamage(defender) * 80 + fallbackThreat.expectedDamage * 8)
      : 0;
  const targetStageImpact = !hasActiveSubstitute(defender)
    ? getTargetStageImpactSummary(attacker, defender, move)
    : { attackSwing: 0, defenseBreak: 0, speedSwing: 0 };
  const targetStageBonus =
    (move.targetStatChangeChance ?? (move.targetStatChanges ? 1 : 0)) *
    (targetStageImpact.attackSwing * 14 +
      targetStageImpact.defenseBreak * 12 +
      targetStageImpact.speedSwing * 8_000);
  const substituteBonus =
    hasActiveSubstitute(defender)
      ? getExpectedSubstituteDamage(attacker, defender, move) *
          getAccuracyMultiplier(move, attacker) *
          10 +
        getSubstituteBreakChance(attacker, defender, move) *
          getAccuracyMultiplier(move, attacker) *
          18_000
      : 0;
  const flinchBonus =
    move.flinchChance && actsBeforeReference && !hasActiveSubstitute(defender)
      ? move.flinchChance * (fallbackThreat.koChance * 8_000 + expectedDamage * 6)
      : 0;
  const rockyHelmetPenalty =
    defender.itemId === "rockyHelmet" && move.isContact
      ? getRockyHelmetDamage(attacker) * getExpectedHitCount(move) * 10
      : 0;
  const drainRecoveryMultiplier = attacker.itemId === "bigRoot" ? 1.3 : 1;
  const hpSwingBonus =
    expectedDamage * (move.drainRatio ?? 0) * drainRecoveryMultiplier * 12 -
    expectedDamage * (move.recoilRatio ?? 0) * 10 -
    rockyHelmetPenalty;
  const stallAttackPenalty = getStallAttackPenalty(
    attacker,
    defender,
    move,
    koChance,
    defender.currentHp <= 0 ? 0 : expectedDamage / defender.currentHp
  );

  return (
    koChance * 20_000 +
    expectedDamage +
    priority * 250 +
    hpSwingBonus +
    burnBonus +
    poisonBonus +
    toxicBonus +
    targetStageBonus +
    substituteBonus +
    flinchBonus -
    stallAttackPenalty -
    accuracyPenalty +
    learnedBias
  );
};

const getWeightedMoveOptions = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  opposingMove?: SimMove | null
): WeightedMoveOption[] => {
  const cacheKey = getWeightedMovePolicyKey(attacker, defender, opposingMove);
  const cached = weightedMovePolicyCache.get(cacheKey);

  if (cached) {
    return cached;
  }

  const availableMoves = getAvailableMoves(attacker);
  if (availableMoves.length === 0) {
    return [];
  }

  const scoredMoves = availableMoves.map((move) => ({
    move,
    score: getShallowMoveScore(attacker, defender, move, opposingMove),
  }));
  const maxScore = Math.max(...scoredMoves.map((entry) => entry.score));
  const weightedScores = scoredMoves.map((entry) => ({
    ...entry,
    weight: Math.exp(Math.max(-8, Math.min(0, (entry.score - maxScore) / 8_000))),
  }));
  const totalWeight = weightedScores.reduce((sum, entry) => sum + entry.weight, 0);
  const options = weightedScores.map((entry) => ({
    move: entry.move,
    score: entry.score,
    probability: totalWeight <= 0 ? 1 / weightedScores.length : entry.weight / totalWeight,
  }));

  capCacheSize(weightedMovePolicyCache, MAX_WEIGHTED_POLICY_CACHE_SIZE);
  weightedMovePolicyCache.set(cacheKey, options);

  return options;
};

const trimWeightedMoveOptions = (options: WeightedMoveOption[]) => {
  if (options.length <= ROLLOUT_OPPONENT_OPTION_COUNT) {
    return options;
  }

  const sorted = [...options].sort((a, b) => b.probability - a.probability);
  const trimmed: WeightedMoveOption[] = [];
  let cumulativeProbability = 0;

  for (const option of sorted) {
    trimmed.push(option);
    cumulativeProbability += option.probability;
    if (
      trimmed.length >= ROLLOUT_OPPONENT_OPTION_COUNT ||
      cumulativeProbability >= 0.9
    ) {
      break;
    }
  }

  const totalProbability = trimmed.reduce((sum, option) => sum + option.probability, 0);
  if (totalProbability <= 0) {
    return trimmed;
  }

  return trimmed.map((option) => ({
    ...option,
    probability: option.probability / totalProbability,
  }));
};

const getStateUtility = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon
) => {
  if (attacker.currentHp <= 0 && defender.currentHp <= 0) {
    return 0;
  }

  if (defender.currentHp <= 0) {
    return 1_000_000 + attacker.currentHp * 600 + attacker.substituteHp * 8;
  }

  if (attacker.currentHp <= 0) {
    return -1_000_000 - defender.currentHp * 600 - defender.substituteHp * 8;
  }

  const attackerHpRatio = attacker.currentHp / attacker.stats.hp;
  const defenderHpRatio = defender.currentHp / defender.stats.hp;
  const hpSwing = (attackerHpRatio - defenderHpRatio) * 24_000;
  const residualSwing =
    (getResidualDamagePressure(defender) - getResidualDamagePressure(attacker)) * 220;
  const substituteSwing = (attacker.substituteHp - defender.substituteHp) * 36;
  const speedSwing =
    attacker.effectiveSpeed > defender.effectiveSpeed
      ? 2_400
      : attacker.effectiveSpeed < defender.effectiveSpeed
        ? -2_400
        : 0;
  const pressureSwing =
    getBestAttackMoveMetrics(attacker, defender).expectedDamage -
    getBestAttackMoveMetrics(defender, attacker).expectedDamage;
  const learnedStateBias = getLearnedStateValueBias(attacker, defender);

  return hpSwing + residualSwing + substituteSwing + speedSwing + pressureSwing * 22 + learnedStateBias;
};

const getBaseEvaluations = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon
) => {
  const cacheKey = getEvaluationKey(attacker, defender);
  const cached = baseEvaluationCache.get(cacheKey);

  if (cached) {
    return cached;
  }

  const evaluations = attacker.moves
    .filter((move) =>
      attacker.choiceLockedMoveId ? move.id === attacker.choiceLockedMoveId : true
    )
    .map((move) => scoreMove(attacker, defender, move))
    .sort((a, b) => b.score - a.score);

  capCacheSize(baseEvaluationCache, MAX_BASE_EVALUATION_CACHE_SIZE);
  baseEvaluationCache.set(cacheKey, evaluations);

  return evaluations;
};

const getImmediateThreatSummary = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  fallbackThreat: AttackMoveMetrics,
  opposingMove?: SimMove | null
) => {
  if (!opposingMove) {
    return {
      expectedDamage: fallbackThreat.expectedDamage,
      koChance: fallbackThreat.koChance,
    };
  }

  if (opposingMove.category === "status" || opposingMove.protect || opposingMove.substitute) {
    return {
      expectedDamage: 0,
      koChance: 0,
    };
  }

  return {
    expectedDamage:
      getExpectedDamage(defender, attacker, opposingMove) *
      getAccuracyMultiplier(opposingMove, defender),
    koChance: getEffectiveKoChance(defender, attacker, opposingMove),
  };
};

const applyProjectedEndOfTurnEffects = (
  leftPokemon: SimBattlePokemon,
  rightPokemon: SimBattlePokemon
) => {
  if (leftPokemon.currentHp <= 0 || rightPokemon.currentHp <= 0) {
    return;
  }

  if (leftPokemon.burned) {
    leftPokemon.currentHp = Math.max(0, leftPokemon.currentHp - getBurnDamage(leftPokemon));
  }

  if (leftPokemon.poisoned) {
    leftPokemon.currentHp = Math.max(0, leftPokemon.currentHp - getPoisonDamage(leftPokemon));
  }

  if (leftPokemon.toxic) {
    leftPokemon.currentHp = Math.max(0, leftPokemon.currentHp - getToxicDamage(leftPokemon));
    leftPokemon.toxicTurns += 1;
  }

  if (rightPokemon.burned) {
    rightPokemon.currentHp = Math.max(0, rightPokemon.currentHp - getBurnDamage(rightPokemon));
  }

  if (rightPokemon.poisoned) {
    rightPokemon.currentHp = Math.max(0, rightPokemon.currentHp - getPoisonDamage(rightPokemon));
  }

  if (rightPokemon.toxic) {
    rightPokemon.currentHp = Math.max(0, rightPokemon.currentHp - getToxicDamage(rightPokemon));
    rightPokemon.toxicTurns += 1;
  }

  if (leftPokemon.currentHp > 0 && rightPokemon.currentHp > 0) {
    applyWishEndOfTurn(leftPokemon);
    applyWishEndOfTurn(rightPokemon);
    applyLeftoversRecovery(leftPokemon);
    applyLeftoversRecovery(rightPokemon);
  }

  leftPokemon.protectUsedLastTurn = leftPokemon.protectActive;
  rightPokemon.protectUsedLastTurn = rightPokemon.protectActive;
  leftPokemon.protectActive = false;
  rightPokemon.protectActive = false;
};

const getFinishingMovePlan = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon
) : FinishingPlan => {
  let bestMove: SimMove | null = null;
  let bestFinishChance = -1;
  let bestExpectedDamage = -1;

  for (const move of getAvailableAttackMoves(attacker)) {
    const opponentOptions = getWeightedMoveOptions(defender, attacker, move);
    const expectedDamage =
      getExpectedDamage(attacker, defender, move) * getAccuracyMultiplier(move, attacker);
    let finishChance = 0;

    for (const option of opponentOptions) {
      const moveHits = getAccuracyMultiplier(move, attacker);

      if (moveActsBeforeOpponent(attacker, defender, move, option.move)) {
        const defenderKoChance =
          option.move.category === "status" || option.move.protect
            ? 0
            : getEffectiveKoChance(defender, attacker, option.move);
        const myKoChance = getEffectiveKoChance(attacker, defender, move);
        const hitThroughProtectMultiplier = option.move.protect ? 0 : 1;

        finishChance += option.probability * moveHits * myKoChance * hitThroughProtectMultiplier;
        if (myKoChance < 1 && defenderKoChance < 1) {
          finishChance +=
            option.probability *
            moveHits *
            (1 - myKoChance) *
            (1 - defenderKoChance) *
            0;
        }
      } else {
        const survivalChance =
          option.move.category === "status" || option.move.protect
            ? 1
            : 1 - getEffectiveKoChance(defender, attacker, option.move);
        const hitThroughProtectMultiplier = option.move.protect ? 0 : 1;
        finishChance +=
          option.probability *
          Math.max(survivalChance, 0) *
          moveHits *
          getEffectiveKoChance(attacker, defender, move) *
          hitThroughProtectMultiplier;
      }
    }

    if (
      finishChance > bestFinishChance ||
      (finishChance === bestFinishChance && expectedDamage > bestExpectedDamage)
    ) {
      bestMove = move;
      bestFinishChance = finishChance;
      bestExpectedDamage = expectedDamage;
    }
  }

  return {
    move: bestMove,
    finishChance: Math.max(bestFinishChance, 0),
    expectedDamage: Math.max(bestExpectedDamage, 0),
  };
};

const getProjectedTurnOutcomes = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
): ProjectedTurnOutcome[] => {
  const opponentOptions = trimWeightedMoveOptions(
    getWeightedMoveOptions(defender, attacker, move)
  );
  const attackerOutcomeOptions = getProjectedMoveOutcomeOptions(attacker, move);
  const outcomes: ProjectedTurnOutcome[] = [];

  for (const option of opponentOptions) {
    const opponentOutcomeOptions = getProjectedMoveOutcomeOptions(defender, option.move);

    for (const attackerOutcome of attackerOutcomeOptions) {
      for (const opponentOutcome of opponentOutcomeOptions) {
        const probability =
          option.probability * attackerOutcome.probability * opponentOutcome.probability;

        if (probability <= 0) {
          continue;
        }

        const projectedAttacker = cloneBattlePokemon(attacker);
        const projectedDefender = cloneBattlePokemon(defender);

        applyChoiceLock(projectedAttacker, move);
        applyChoiceLock(projectedDefender, option.move);

        if (moveActsBeforeOpponent(projectedAttacker, projectedDefender, move, option.move)) {
          applyProjectedMove(projectedAttacker, projectedDefender, move, attackerOutcome.hit);
          if (projectedAttacker.currentHp > 0 && projectedDefender.currentHp > 0) {
            applyProjectedMove(projectedDefender, projectedAttacker, option.move, opponentOutcome.hit);
          }
        } else {
          applyProjectedMove(projectedDefender, projectedAttacker, option.move, opponentOutcome.hit);
          if (projectedAttacker.currentHp > 0 && projectedDefender.currentHp > 0) {
            applyProjectedMove(projectedAttacker, projectedDefender, move, attackerOutcome.hit);
          }
        }

        applyProjectedEndOfTurnEffects(projectedAttacker, projectedDefender);
        outcomes.push({
          attacker: projectedAttacker,
          defender: projectedDefender,
          probability,
          opponentMove: option.move,
        });
      }
    }
  }

  return outcomes;
};

const moveActsBeforeOpponent = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  myMove: SimMove,
  opponentMove: SimMove
) => {
  const myPriority = getMovePriority(myMove);
  const opponentPriority = getMovePriority(opponentMove);

  if (myPriority !== opponentPriority) {
    return myPriority > opponentPriority;
  }

  if (attacker.effectiveSpeed !== defender.effectiveSpeed) {
    return attacker.effectiveSpeed > defender.effectiveSpeed;
  }

  return false;
};

const getTwoTurnPlan = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => {
  const projectedTurnOutcomes = getProjectedTurnOutcomes(attacker, defender, move);
  let twoTurnKoChance = 0;
  let weightedFinisherPriorityBonus = 0;

  for (const outcome of projectedTurnOutcomes) {
    if (outcome.attacker.currentHp <= 0 || outcome.defender.currentHp <= 0) {
      continue;
    }

    const finishingPlan = getFinishingMovePlan(outcome.attacker, outcome.defender);
    if (!finishingPlan.move || finishingPlan.finishChance <= 0) {
      continue;
    }

    const weightedFinishChance = outcome.probability * finishingPlan.finishChance;
    twoTurnKoChance += weightedFinishChance;
    weightedFinisherPriorityBonus +=
      weightedFinishChance * Math.max(getMovePriority(finishingPlan.move), 0) * 1000;
  }

  return {
    twoTurnKoChance,
    twoTurnSetupScore: twoTurnKoChance * 30_000 + weightedFinisherPriorityBonus,
  };
};

const evaluateRolloutState = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  depth: number
): number => {
  if (depth <= 0 || attacker.currentHp <= 0 || defender.currentHp <= 0) {
    return getStateUtility(attacker, defender);
  }

  const cacheKey = `${getEvaluationKey(attacker, defender)}|state|${depth}`;
  const cached = rolloutStateCache.get(cacheKey);
  if (cached !== undefined) {
    return cached;
  }

  const candidates = getBaseEvaluations(attacker, defender).slice(0, ROLLOUT_CANDIDATE_COUNT);
  if (candidates.length === 0) {
    const value = getStateUtility(attacker, defender);
    capCacheSize(rolloutStateCache, MAX_ROLLOUT_STATE_CACHE_SIZE);
    rolloutStateCache.set(cacheKey, value);
    return value;
  }

  let bestValue = -Infinity;

  for (const candidate of candidates) {
    const value = getMoveRolloutValue(attacker, defender, candidate.move, depth);
    if (value > bestValue) {
      bestValue = value;
    }
  }

  const finalValue = Number.isFinite(bestValue)
    ? bestValue
    : getStateUtility(attacker, defender);
  capCacheSize(rolloutStateCache, MAX_ROLLOUT_STATE_CACHE_SIZE);
  rolloutStateCache.set(cacheKey, finalValue);
  return finalValue;
};

const getMoveRolloutValue = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  depth: number
): number => {
  const cacheKey = `${getEvaluationKey(attacker, defender)}|move|${move.id}|${depth}`;
  const cached = rolloutMoveCache.get(cacheKey);
  if (cached !== undefined) {
    return cached;
  }

  const outcomes = getProjectedTurnOutcomes(attacker, defender, move);
  if (outcomes.length === 0) {
    const value = getStateUtility(attacker, defender);
    capCacheSize(rolloutMoveCache, MAX_ROLLOUT_CACHE_SIZE);
    rolloutMoveCache.set(cacheKey, value);
    return value;
  }

  const totalProbability = outcomes.reduce((sum, outcome) => sum + outcome.probability, 0);
  const value = outcomes.reduce((sum, outcome) => {
    const outcomeValue =
      depth <= 1 ||
      outcome.attacker.currentHp <= 0 ||
      outcome.defender.currentHp <= 0
        ? getStateUtility(outcome.attacker, outcome.defender)
        : evaluateRolloutState(outcome.attacker, outcome.defender, depth - 1);

    return sum + outcome.probability * outcomeValue;
  }, 0) / Math.max(totalProbability, 1e-9);

  capCacheSize(rolloutMoveCache, MAX_ROLLOUT_CACHE_SIZE);
  rolloutMoveCache.set(cacheKey, value);
  return value;
};

export const scoreMove = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
): MoveEvaluation => {
  const learnedBias = getLearnedMoveBias(attacker, defender, move);

  if (move.category === "status" || move.protect) {
    if (move.substitute && !canUseSubstitute(attacker)) {
      return {
        move,
        expectedDamage: 0,
        koChance: 0,
        priority: getMovePriority(move),
        accuracy: getMoveAccuracy(move, attacker),
        score: -80_000,
        twoTurnKoChance: 0,
        twoTurnSetupScore: 0,
      };
    }
    if (move.rest && !canUseRest(attacker)) {
      return {
        move,
        expectedDamage: 0,
        koChance: 0,
        priority: getMovePriority(move),
        accuracy: getMoveAccuracy(move, attacker),
        score: -80_000,
        twoTurnKoChance: 0,
        twoTurnSetupScore: 0,
      };
    }

    const accuracy = getMoveAccuracy(move, attacker);
    const priority = getMovePriority(move);
    const accuracyPenalty = (100 - accuracy) * 8;
    const currentBestAttack = getBestAttackMoveMetrics(attacker, defender);
    const opponentThreat = getBestAttackMoveMetrics(defender, attacker);
    const immediateThreat = getImmediateThreatSummary(attacker, defender, opponentThreat, move);
    const projectedTurnOutcomes = getProjectedTurnOutcomes(attacker, defender, move);
    const chanceToUse = projectedTurnOutcomes.reduce((sum, outcome) => sum + outcome.probability, 0);

    if (chanceToUse <= 0) {
      return {
        move,
        expectedDamage: 0,
        koChance: 0,
        priority,
        accuracy,
        score: -50_000 - accuracyPenalty,
        twoTurnKoChance: 0,
        twoTurnSetupScore: 0,
      };
    }

    const aggregated = projectedTurnOutcomes.reduce(
      (sum, outcome) => {
        if (outcome.attacker.currentHp <= 0) {
          return sum;
        }

        const nextBestAttack = getBestAttackMoveMetrics(outcome.attacker, outcome.defender);
        const followUpPlan =
          outcome.attacker.currentHp > 0 && outcome.defender.currentHp > 0
            ? getFinishingMovePlan(outcome.attacker, outcome.defender)
            : { move: null, finishChance: 0, expectedDamage: 0 };
        const projectedOpponentThreat =
          outcome.defender.currentHp > 0
            ? getBestAttackMoveMetrics(outcome.defender, outcome.attacker)
            : { move: null, expectedDamage: 0, koChance: 0, score: 0 };

        sum.nextBestAttackScore += outcome.probability * nextBestAttack.score;
        sum.projectedOpponentThreat += outcome.probability * projectedOpponentThreat.expectedDamage;
        sum.followUpBonus +=
          outcome.probability *
          (followUpPlan.finishChance * 45_000 +
            followUpPlan.expectedDamage * 18 +
            (followUpPlan.move ? Math.max(getMovePriority(followUpPlan.move), 0) * 1_500 : 0));
        sum.speedSwingValue +=
          outcome.probability *
          (outcome.attacker.effectiveSpeed > outcome.defender.effectiveSpeed &&
          attacker.effectiveSpeed <= defender.effectiveSpeed
            ? 8_000
            : 0);

        return sum;
      },
      {
        nextBestAttackScore: 0,
        projectedOpponentThreat: 0,
        followUpBonus: 0,
        speedSwingValue: 0,
      }
    );
    const offenseGain = Math.max(0, aggregated.nextBestAttackScore - currentBestAttack.score);
    const defenseGain = Math.max(
      0,
      opponentThreat.expectedDamage - aggregated.projectedOpponentThreat
    ) * 18;
    const speedSwingValue = aggregated.speedSwingValue;
    const burnValue =
      move.targetStatus === "burn" && !hasActiveSubstitute(defender)
        ? (getAvailableAttackMoves(defender).some((candidateMove) => candidateMove.category === "physical")
            ? defender.stats.atk * 10
            : 0) + getBurnDamage(defender) * 90
        : 0;
    const paralysisValue =
      move.targetStatus === "paralysis" && !hasActiveSubstitute(defender)
        ? (attacker.effectiveSpeed < defender.effectiveSpeed &&
            speedSwingValue > 0
            ? 5_000
            : 0) + opponentThreat.expectedDamage * 10 + opponentThreat.koChance * 6_000
        : 0;
    const poisonValue =
      move.targetStatus === "poison" && !hasActiveSubstitute(defender)
        ? getPoisonDamage(defender) * 120 + opponentThreat.expectedDamage * 6
        : 0;
    const toxicValue =
      move.targetStatus === "toxic" && !hasActiveSubstitute(defender)
        ? getPoisonDamage(defender) * 220 + opponentThreat.expectedDamage * 10 + opponentThreat.koChance * 4_000
        : 0;
    const poisonPlanBonus = getPoisonPlanBonus(attacker, defender, move);
    const targetStageImpact = !hasActiveSubstitute(defender)
      ? getTargetStageImpactSummary(attacker, defender, move)
      : { attackSwing: 0, defenseBreak: 0, speedSwing: 0 };
    const targetStageValue =
      targetStageImpact.attackSwing * 18 +
      targetStageImpact.defenseBreak * 16 +
      targetStageImpact.speedSwing * 10_000;
    const residualPressure = getResidualDamagePressure(defender);
    const stallSetupPenalty = getStallSetupPenalty(attacker, defender, move, immediateThreat);
    const protectValue =
      move.protect
        ? (attacker.protectUsedLastTurn ? -60_000 : 0) +
          immediateThreat.expectedDamage * 22 +
          immediateThreat.koChance * 9_000 +
          residualPressure * 170 +
          (attacker.itemId === "leftovers" ? getLeftoversHealing(attacker) * 130 : 0)
        : 0;
    const substituteValue =
      move.substitute
        ? projectedTurnOutcomes.reduce((sum, outcome) => {
            if (outcome.attacker.currentHp <= 0) {
              return sum - outcome.probability * 12_000;
            }

            const survivingSubstituteBonus =
              outcome.attacker.substituteHp > 0
                ? 8_000 + outcome.attacker.substituteHp * 24
                : 0;
            return (
              sum +
              outcome.probability *
                (survivingSubstituteBonus +
                  Math.max(
                    0,
                    opponentThreat.expectedDamage - aggregated.projectedOpponentThreat
                  ) *
                    4 +
                  residualPressure * 90 +
                (attacker.itemId === "leftovers" ? getLeftoversHealing(attacker) * 70 : 0))
            );
          }, 0)
        : 0;
    const wishValue =
      move.wish
        ? projectedTurnOutcomes.reduce((sum, outcome) => {
            if (outcome.attacker.currentHp <= 0 || outcome.attacker.wishTurnsRemaining <= 0) {
              return sum;
            }

            const nextThreat =
              outcome.defender.currentHp > 0
                ? getBestAttackMoveMetrics(outcome.defender, outcome.attacker)
                : { move: null, expectedDamage: 0, koChance: 0, score: 0 };
            const survivalChance = Math.max(0, 1 - nextThreat.koChance);
            const protectCoverage = getAvailableMoves(outcome.attacker).some(
              (candidateMove) => candidateMove.protect
            )
              ? 1.2
              : 1;

            return (
              sum +
              outcome.probability *
                outcome.attacker.wishHealAmount *
                95 *
                survivalChance *
                protectCoverage
            );
          }, 0)
        : 0;
    const restValue =
      move.rest
        ? projectedTurnOutcomes.reduce((sum, outcome) => {
            if (outcome.attacker.currentHp <= 0 || outcome.attacker.sleepTurnsRemaining <= 0) {
              return sum;
            }

            return (
              sum +
              outcome.probability *
                getRestPlanValue(
                  attacker,
                  outcome.attacker,
                  outcome.defender,
                  immediateThreat
                )
            );
          }, 0)
        : 0;
    const immediateThreatWeight =
      1 +
      immediateThreat.koChance * 4 +
      Math.min(2, immediateThreat.expectedDamage / Math.max(attacker.currentHp, 1));
    const selfRecoveryValue =
      move.recoverRatio
        ? chanceToUse * getSelfRecoveryAmount(attacker, move) * 110 * immediateThreatWeight +
          projectedTurnOutcomes.reduce((sum, outcome) => {
            if (outcome.attacker.currentHp <= 0) {
              return sum;
            }

            return (
              sum +
              outcome.probability *
                Math.max(
                  0,
                  outcome.attacker.currentHp - attacker.currentHp + immediateThreat.expectedDamage * 0.6
                ) *
                90
            );
          }, 0)
        : 0;
    const score =
      offenseGain +
      aggregated.followUpBonus +
      defenseGain +
      speedSwingValue +
      burnValue +
      paralysisValue +
      poisonValue +
      toxicValue +
      poisonPlanBonus +
      targetStageValue +
      protectValue +
      wishValue +
      restValue +
      selfRecoveryValue +
      substituteValue +
      priority * 250 -
      stallSetupPenalty -
      accuracyPenalty;

    return {
      move,
        expectedDamage: 0,
        koChance: 0,
        priority,
        accuracy,
        score: score + learnedBias,
        twoTurnKoChance: 0,
        twoTurnSetupScore:
          offenseGain +
          aggregated.followUpBonus +
          defenseGain +
        speedSwingValue +
        burnValue +
        paralysisValue +
        poisonValue +
        toxicValue +
          poisonPlanBonus +
          targetStageValue +
          protectValue +
          wishValue +
          restValue +
          selfRecoveryValue +
          substituteValue +
          learnedBias,
      };
  }

  const distribution = getDamageDistribution(attacker, defender, move);
  const expectedDamage = getExpectedDamage(attacker, defender, move);
  const koChance = getKoChance(distribution, defender.currentHp);
  const accuracy = getMoveAccuracy(move, attacker);
  const priority = getMovePriority(move);
  const projectedTurnOutcomes = getProjectedTurnOutcomes(attacker, defender, move);
  const immediateWinChance = projectedTurnOutcomes.reduce(
    (sum, outcome) => sum + (outcome.attacker.currentHp > 0 && outcome.defender.currentHp <= 0 ? outcome.probability : 0),
    0
  );
  const opponentMoveOptions = getWeightedMoveOptions(defender, attacker, move);
  const weightedPreemptiveKoChance = opponentMoveOptions.reduce((sum, option) => {
    if (option.move.category === "status" || option.move.protect) {
      return sum;
    }

    if (!moveActsBeforeOpponent(attacker, defender, move, option.move)) {
      return sum + option.probability * getEffectiveKoChance(defender, attacker, option.move);
    }

    return sum;
  }, 0);
  const actsBeforeOpponentProbability = opponentMoveOptions.reduce((sum, option) => {
    return sum + (moveActsBeforeOpponent(attacker, defender, move, option.move) ? option.probability : 0);
  }, 0);
  const expectedDamageRatio = defender.currentHp <= 0 ? 0 : expectedDamage / defender.currentHp;
  const accuracyPenalty = (100 - accuracy) * 8;
  const priorityBonus = priority * 250;
  const defenderHasPhysicalMoves = getAvailableAttackMoves(defender).some(
    (candidateMove) => candidateMove.category === "physical"
  );
  const burnBonus =
    move.burnChance && !hasActiveSubstitute(defender) && canApplyBurn(defender)
      ? move.burnChance *
        ((defenderHasPhysicalMoves ? defender.stats.atk * 8 : 0) + getBurnDamage(defender) * 30)
      : 0;
  const flinchBonus =
    move.flinchChance && actsBeforeOpponentProbability > 0 && !hasActiveSubstitute(defender)
      ? move.flinchChance * (weightedPreemptiveKoChance * 8000 + expectedDamageRatio * 1200) * actsBeforeOpponentProbability
      : 0;
  const poisonBonus =
    move.poisonChance && !hasActiveSubstitute(defender) && canApplyPoison(defender, move)
      ? move.poisonChance * (getPoisonDamage(defender) * 36 + weightedPreemptiveKoChance * 2_500)
      : 0;
  const toxicBonus =
    move.toxicChance && !hasActiveSubstitute(defender) && canApplyPoison(defender, move)
      ? move.toxicChance * (getPoisonDamage(defender) * 90 + weightedPreemptiveKoChance * 3_500)
      : 0;
  const targetStageImpact = !hasActiveSubstitute(defender)
    ? getTargetStageImpactSummary(attacker, defender, move)
    : { attackSwing: 0, defenseBreak: 0, speedSwing: 0 };
  const targetStageBonus =
    (move.targetStatChangeChance ?? (move.targetStatChanges ? 1 : 0)) *
    (targetStageImpact.attackSwing * 18 +
      targetStageImpact.defenseBreak * 14 +
      targetStageImpact.speedSwing * 10_000);
  const substituteBonus =
    hasActiveSubstitute(defender)
      ? getExpectedSubstituteDamage(attacker, defender, move) *
          getAccuracyMultiplier(move, attacker) *
          10 +
        getSubstituteBreakChance(attacker, defender, move) *
          getAccuracyMultiplier(move, attacker) *
          18_000
      : 0;
  const rockyHelmetPenalty =
    defender.itemId === "rockyHelmet" && move.isContact
      ? getRockyHelmetDamage(attacker) * getExpectedHitCount(move) * 10
      : 0;
  const immediateRockyHelmetTradePenalty =
    defender.itemId === "rockyHelmet" && move.isContact && getRockyHelmetDamage(attacker) >= attacker.currentHp
      ? 1_100_000
      : 0;
  const drainRecoveryMultiplier = attacker.itemId === "bigRoot" ? 1.3 : 1;
  const hpSwingBonus =
    expectedDamage * (move.drainRatio ?? 0) * drainRecoveryMultiplier * 12 -
    expectedDamage * (move.recoilRatio ?? 0) * 10 -
    rockyHelmetPenalty;
  const revengeBonus =
    actsBeforeOpponentProbability < 1 && weightedPreemptiveKoChance > 0
      ? priority * 3000 + expectedDamageRatio * 600
      : 0;
  const survivalPressurePenalty =
    actsBeforeOpponentProbability < 1 && weightedPreemptiveKoChance > 0
      ? weightedPreemptiveKoChance * 2500 - priority * 500
      : 0;
  const { twoTurnKoChance, twoTurnSetupScore } = getTwoTurnPlan(attacker, defender, move);
  const poisonPlanBonus = getPoisonPlanBonus(attacker, defender, move);
  const stallAttackPenalty = getStallAttackPenalty(
    attacker,
    defender,
    move,
    immediateWinChance,
    expectedDamageRatio
  );
  const score =
    immediateWinChance * 1_000_000 +
    expectedDamage +
    priorityBonus +
    revengeBonus -
    survivalPressurePenalty -
    accuracyPenalty +
    hpSwingBonus +
    burnBonus +
    poisonBonus +
    toxicBonus +
    poisonPlanBonus +
    targetStageBonus +
    substituteBonus +
    flinchBonus +
    twoTurnSetupScore -
    stallAttackPenalty -
    immediateRockyHelmetTradePenalty +
    accuracy +
    learnedBias;

  return {
    move,
    expectedDamage,
    koChance,
    priority,
    accuracy,
    score,
    twoTurnKoChance,
    twoTurnSetupScore,
  };
};

export const evaluateMoves = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  options: AiEvaluationOptions = {}
): MoveEvaluation[] => {
  const enableRollout = options.enableRollout ?? true;
  const evaluationKey = `${getEvaluationKey(attacker, defender)}|rollout:${enableRollout ? 1 : 0}`;
  const cached = evaluationCache.get(evaluationKey);

  if (cached) {
    return cached;
  }

  const baseEvaluations = getBaseEvaluations(attacker, defender);
  const shouldUseRollout =
    enableRollout && hasRolloutSensitiveMove(attacker);

  const evaluations = shouldUseRollout
    ? (() => {
        const rolloutMoveIds = new Set(
          baseEvaluations
            .slice(0, ROLLOUT_CANDIDATE_COUNT)
            .map((evaluation) => evaluation.move.id)
        );

        return baseEvaluations
          .map((evaluation) => {
            if (!rolloutMoveIds.has(evaluation.move.id)) {
              return evaluation;
            }

            const rolloutValue = getMoveRolloutValue(
              attacker,
              defender,
              evaluation.move,
              ROLLOUT_DEPTH
            );

            return {
              ...evaluation,
              score: rolloutValue + evaluation.score * 0.2,
              twoTurnSetupScore:
                evaluation.twoTurnSetupScore + rolloutValue * 0.15,
            };
          })
          .sort((a, b) => b.score - a.score);
      })()
    : baseEvaluations;

  capCacheSize(evaluationCache, MAX_EVALUATION_CACHE_SIZE);
  evaluationCache.set(evaluationKey, evaluations);

  return evaluations;
};

export const chooseBestMove = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  options: AiEvaluationOptions = {}
): SimMove | null => {
  if (attacker.moves.length === 0) {
    return null;
  }

  return evaluateMoves(attacker, defender, options)[0]?.move ?? null;
};
