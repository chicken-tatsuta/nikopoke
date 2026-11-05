import { chooseBestMove, clearAiCaches, getSingleHitDamageRolls } from "./ai";
import {
  applyBattleStageChanges,
  EMPTY_BATTLE_STAGES,
  formatBattleStageChange,
  getEffectiveSpeed,
  type BattleStageStat,
  type BattleStages,
} from "./battleStages";
import type { PokemonType } from "./damage";
import {
  calcAllStats,
  type BaseStats,
  type EVs,
  type IVs,
  type Nature,
  type StatKey,
} from "./stats";

export type MoveCategory = "physical" | "special" | "status";
export type TargetStatus = "burn" | "paralysis" | "poison" | "toxic";

export type SimPokemon = {
  id: string;
  name: string;
  types: PokemonType[];
  baseStats: BaseStats;
};

export type SimMove = {
  id: string;
  name: string;
  type: PokemonType;
  category: MoveCategory;
  power: number;
  accuracy?: number | null;
  isContact?: boolean;
  burnChance?: number;
  flinchChance?: number;
  poisonChance?: number;
  toxicChance?: number;
  hitCount?: number;
  minHits?: number;
  maxHits?: number;
  accuracyPerHit?: boolean;
  powerStep?: number;
  priority?: number;
  critRank?: number;
  drainRatio?: number;
  recoilRatio?: number;
  recoverRatio?: number;
  wish?: boolean;
  rest?: boolean;
  leechSeed?: boolean;
  aquaRing?: boolean;
  ingrain?: boolean;
  selfStatChanges?: Partial<Record<BattleStageStat, number>>;
  targetStatChanges?: Partial<Record<BattleStageStat, number>>;
  targetStatChangeChance?: number;
  targetStatus?: TargetStatus;
  protect?: boolean;
  substitute?: boolean;
  targets: string[];
};

export type SimulatorInput = {
  pokemon: SimPokemon;
  evs: EVs;
  moveIds: string[];
  itemId?: string;
  nature?: Nature;
};

export type SimBattlePokemon = {
  side: Exclude<SimulationSide, "draw">;
  level: number;
  pokemon: SimPokemon;
  itemId?: string;
  evs: EVs;
  ivs: IVs;
  nature?: Nature;
  stats: Record<StatKey, number>;
  currentHp: number;
  moves: SimMove[];
  effectiveSpeed: number;
  stages: BattleStages;
  focusSashUsed: boolean;
  choiceLockedMoveId?: string;
  burned: boolean;
  paralyzed: boolean;
  poisoned: boolean;
  toxic: boolean;
  toxicTurns: number;
  sleepTurnsRemaining: number;
  substituteHp: number;
  wishTurnsRemaining: number;
  wishHealAmount: number;
  leechSeededBy?: Exclude<SimulationSide, "draw">;
  aquaRingActive: boolean;
  ingrainActive: boolean;
  flinched: boolean;
  protectActive: boolean;
  protectUsedLastTurn: boolean;
};

export type FirstMoveReason = "priority" | "faster" | "speedTie";

export type SimulationSide = "left" | "right" | "draw";

export type BattleTurnMoveHistory = Record<Exclude<SimulationSide, "draw">, string[]>;

export type SimulationBattleResult = {
  winner: SimulationSide;
  turns: number;
  endingHp: Record<Exclude<SimulationSide, "draw">, number>;
  endingHpPercent: Record<Exclude<SimulationSide, "draw">, number>;
  firstMoveCounts: Record<Exclude<SimulationSide, "draw">, number>;
  firstMoveByReason: Record<Exclude<SimulationSide, "draw">, Record<FirstMoveReason, number>>;
  moveUsage: Record<Exclude<SimulationSide, "draw">, Record<string, number>>;
  koHitCount: Partial<Record<Exclude<SimulationSide, "draw">, number>>;
  moveKoCounts: Record<Exclude<SimulationSide, "draw">, Record<string, number>>;
  openingOrderReason: FirstMoveReason;
  openingFirstSide: Exclude<SimulationSide, "draw">;
  turnMoveHistory: BattleTurnMoveHistory;
};

export type SimulationResult = {
  battles: SimulationBattleResult[];
};

export type ItemCandidateSimulationResult = {
  itemId: string;
  battles: SimulationBattleResult[];
};

export type ItemCandidateSimulationGroup = {
  battles: SimulationBattleResult[];
  byRightItem: ItemCandidateSimulationResult[];
};

export type ProjectedBattleAction = {
  side: Exclude<SimulationSide, "draw">;
  moveId: string;
  moveName: string;
  hit: boolean;
  hitCount: number;
  hitDamages: number[];
  statusTexts: string[];
  damage: number;
  targetSide: Exclude<SimulationSide, "draw">;
  targetRemainingHp: number;
};

export type ProjectedBattleTurn = {
  turn: number;
  firstSide: Exclude<SimulationSide, "draw">;
  reason: FirstMoveReason;
  actions: ProjectedBattleAction[];
  endingHp: Record<Exclude<SimulationSide, "draw">, number>;
  endingStatuses: Record<Exclude<SimulationSide, "draw">, string[]>;
};

export type ProjectedBattlePlan = {
  turns: ProjectedBattleTurn[];
  winner: SimulationSide | null;
  probability: number;
};

export type ProjectedWinningPlans = {
  left: ProjectedBattlePlan | null;
  right: ProjectedBattlePlan | null;
};

type SimulateBattleArgs = {
  left: SimulatorInput;
  right: SimulatorInput;
  moveMap: Map<string, SimMove>;
  battleCount: number;
  level?: number;
};

type SimulationBattleStats = Omit<
  SimulationBattleResult,
  "winner" | "turns"
>;

type TurnMoveSelection = {
  leftMove: SimMove | null;
  rightMove: SimMove | null;
};

export type ForcedTurnMoves = Partial<TurnMoveSelection>;

type TurnOrderResult = {
  first: SimBattlePokemon;
  second: SimBattlePokemon;
  firstMove: SimMove;
  secondMove: SimMove;
  reason: FirstMoveReason;
};

type ActionResult = "none" | "defenderFainted" | "attackerFainted" | "doubleFainted";
type TurnOrderOption = TurnOrderResult & { probability: number };
type ProjectedBranchState = {
  leftPokemon: SimBattlePokemon;
  rightPokemon: SimBattlePokemon;
  turns: ProjectedBattleTurn[];
  probability: number;
  winner: SimulationSide | null;
};
type ProjectedStatusOutcome = {
  leftPokemon: SimBattlePokemon;
  rightPokemon: SimBattlePokemon;
  action: ProjectedBattleAction;
  probability: number;
};
const DEFAULT_LEVEL = 50;
const MAX_TURNS = 100;
const MAX_PROJECTED_BRANCHES = 16;
const FAST_SIMULATION_AI_OPTIONS = { enableRollout: false } as const;

const DEFAULT_IVS: IVs = {
  hp: 31,
  atk: 31,
  def: 31,
  spa: 31,
  spd: 31,
  spe: 31,
};

const DEFAULT_EVS: EVs = {
  hp: 0,
  atk: 0,
  def: 0,
  spa: 0,
  spd: 0,
  spe: 0,
};

const cloneEvs = (evs?: Partial<EVs>): EVs => ({
  hp: evs?.hp ?? DEFAULT_EVS.hp,
  atk: evs?.atk ?? DEFAULT_EVS.atk,
  def: evs?.def ?? DEFAULT_EVS.def,
  spa: evs?.spa ?? DEFAULT_EVS.spa,
  spd: evs?.spd ?? DEFAULT_EVS.spd,
  spe: evs?.spe ?? DEFAULT_EVS.spe,
});

export const buildBattlePokemon = (
  side: Exclude<SimulationSide, "draw">,
  input: SimulatorInput,
  moveMap: Map<string, SimMove>,
  level: number
): SimBattlePokemon => {
  const evs = cloneEvs(input.evs);
  const stats = calcAllStats(input.pokemon.baseStats, evs, DEFAULT_IVS, level, input.nature);
  const moves = input.moveIds
    .map((moveId) => moveMap.get(moveId))
    .filter((move): move is SimMove => Boolean(move));

  return {
    side,
    level,
    pokemon: input.pokemon,
    itemId: input.itemId,
    evs,
    ivs: DEFAULT_IVS,
    nature: input.nature,
    stats,
    currentHp: stats.hp,
    moves,
    effectiveSpeed: getEffectiveSpeed(stats.spe, input.itemId, EMPTY_BATTLE_STAGES.spe, false),
    stages: { ...EMPTY_BATTLE_STAGES },
    focusSashUsed: false,
    choiceLockedMoveId: undefined,
    burned: false,
    paralyzed: false,
    poisoned: false,
    toxic: false,
    toxicTurns: 0,
    sleepTurnsRemaining: 0,
    substituteHp: 0,
    wishTurnsRemaining: 0,
    wishHealAmount: 0,
    leechSeededBy: undefined,
    aquaRingActive: false,
    ingrainActive: false,
    flinched: false,
    protectActive: false,
    protectUsedLastTurn: false,
  };
};

const getRandomRoll = <T,>(values: T[]): T => {
  const index = Math.floor(Math.random() * values.length);
  return values[index];
};

export const cloneBattlePokemon = (pokemon: SimBattlePokemon): SimBattlePokemon => ({
  ...pokemon,
  evs: { ...pokemon.evs },
  ivs: { ...pokemon.ivs },
  stats: { ...pokemon.stats },
  moves: [...pokemon.moves],
  stages: { ...pokemon.stages },
  poisoned: pokemon.poisoned,
  toxic: pokemon.toxic,
  toxicTurns: pokemon.toxicTurns,
  sleepTurnsRemaining: pokemon.sleepTurnsRemaining,
  substituteHp: pokemon.substituteHp,
  wishTurnsRemaining: pokemon.wishTurnsRemaining,
  wishHealAmount: pokemon.wishHealAmount,
  leechSeededBy: pokemon.leechSeededBy,
  aquaRingActive: pokemon.aquaRingActive,
  ingrainActive: pokemon.ingrainActive,
  paralyzed: pokemon.paralyzed,
  protectActive: pokemon.protectActive,
  protectUsedLastTurn: pokemon.protectUsedLastTurn,
});

const getMovePriority = (move: SimMove) => {
  return move.priority ?? 0;
};

const isChoiceItem = (itemId?: string) => {
  return itemId === "choiceBand" || itemId === "choiceSpecs" || itemId === "choiceScarf";
};

const applyChoiceLock = (pokemon: SimBattlePokemon, move: SimMove | null) => {
  if (!move || !isChoiceItem(pokemon.itemId) || pokemon.choiceLockedMoveId) {
    return;
  }

  pokemon.choiceLockedMoveId = move.id;
};

const createBattleStats = (): SimulationBattleStats => ({
  endingHp: {
    left: 0,
    right: 0,
  },
  endingHpPercent: {
    left: 0,
    right: 0,
  },
  firstMoveCounts: {
    left: 0,
    right: 0,
  },
  firstMoveByReason: {
    left: { priority: 0, faster: 0, speedTie: 0 },
    right: { priority: 0, faster: 0, speedTie: 0 },
  },
  moveUsage: {
    left: {},
    right: {},
  },
  koHitCount: {},
  moveKoCounts: {
    left: {},
    right: {},
  },
  openingOrderReason: "faster",
  openingFirstSide: "left",
  turnMoveHistory: {
    left: [],
    right: [],
  },
});

const getMoveAccuracy = (
  attacker: SimBattlePokemon,
  move: SimMove
) => {
  if (move.targetStatus === "toxic" && attacker.pokemon.types.includes("poison")) {
    return 100;
  }

  return move.accuracy ?? 100;
};

const calcAttackDamage = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  powerOverride = move.power
): number => {
  return getRandomRoll(getSingleHitDamageRolls(attacker, defender, move, powerOverride));
};

const doesMoveHit = (
  attacker: SimBattlePokemon,
  move: SimMove
) => {
  const accuracy = getMoveAccuracy(attacker, move);
  if (accuracy >= 100) {
    return true;
  }

  return Math.random() * 100 < accuracy;
};

const getHitPowerAtIndex = (move: SimMove, hitIndex: number) => {
  return move.power + (move.powerStep ?? 0) * (hitIndex - 1);
};

const getRandomHitCount = (move: SimMove) => {
  if (!move.minHits || !move.maxHits) {
    return 1;
  }

  if (move.minHits === 2 && move.maxHits === 5) {
    const roll = Math.random();
    if (roll < 0.35) return 2;
    if (roll < 0.7) return 3;
    if (roll < 0.85) return 4;
    return 5;
  }

  return Math.floor(Math.random() * (move.maxHits - move.minHits + 1)) + move.minHits;
};

const getSampledHitPowers = (
  attacker: SimBattlePokemon,
  move: SimMove
) => {
  if (move.hitCount && move.hitCount > 1 && move.accuracyPerHit) {
    const hitPowers = [getHitPowerAtIndex(move, 1)];

    for (let hitIndex = 2; hitIndex <= move.hitCount; hitIndex += 1) {
      if (!doesMoveHit(attacker, move)) {
        break;
      }

      hitPowers.push(getHitPowerAtIndex(move, hitIndex));
    }

    return hitPowers;
  }

  if (move.hitCount && move.hitCount > 1) {
    return Array.from({ length: move.hitCount }, (_, index) =>
      getHitPowerAtIndex(move, index + 1)
    );
  }

  if (move.minHits && move.maxHits) {
    const hitCount = getRandomHitCount(move);
    return Array.from({ length: hitCount }, () => move.power);
  }

  return [move.power];
};

const getRepresentativeHitPowers = (move: SimMove) => {
  if (move.hitCount && move.hitCount > 1 && move.accuracyPerHit) {
    return Array.from({ length: move.hitCount }, (_, index) =>
      getHitPowerAtIndex(move, index + 1)
    );
  }

  if (move.hitCount && move.hitCount > 1) {
    return Array.from({ length: move.hitCount }, (_, index) =>
      getHitPowerAtIndex(move, index + 1)
    );
  }

  if (move.minHits === 2 && move.maxHits === 5) {
    return Array.from({ length: 3 }, () => move.power);
  }

  if (move.minHits && move.maxHits) {
    const representativeHits = Math.floor((move.minHits + move.maxHits) / 2);
    return Array.from({ length: representativeHits }, () => move.power);
  }

  return [move.power];
};

const applyBigRootRecoveryBoost = (
  pokemon: SimBattlePokemon,
  recovery: number
) => {
  if (pokemon.itemId !== "bigRoot") {
    return recovery;
  }

  return Math.floor(recovery * 1.3);
};

const recoverPokemon = (
  pokemon: SimBattlePokemon,
  recovery: number
) => {
  if (pokemon.currentHp <= 0 || pokemon.currentHp >= pokemon.stats.hp || recovery <= 0) {
    return 0;
  }

  const nextHp = Math.min(pokemon.stats.hp, pokemon.currentHp + recovery);
  const recovered = nextHp - pokemon.currentHp;
  pokemon.currentHp = nextHp;
  return recovered;
};

const applyRecoveryEffects = (
  attacker: SimBattlePokemon,
  damageDealt: number,
  move: SimMove
) => {
  if (!move.drainRatio || damageDealt <= 0 || attacker.currentHp <= 0) {
    return;
  }

  const recovery = applyBigRootRecoveryBoost(
    attacker,
    Math.floor(damageDealt * move.drainRatio)
  );
  recoverPokemon(attacker, recovery);
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

  return recoverPokemon(pokemon, recovery);
};

const applyRecoilEffects = (
  attacker: SimBattlePokemon,
  damageDealt: number,
  move: SimMove
) => {
  if (!move.recoilRatio || damageDealt <= 0 || attacker.currentHp <= 0) {
    return;
  }

  const recoil = Math.max(1, Math.floor(damageDealt * move.recoilRatio));
  attacker.currentHp = Math.max(0, attacker.currentHp - recoil);
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

const hasPrimaryStatus = (pokemon: SimBattlePokemon) => {
  return (
    pokemon.burned ||
    pokemon.paralyzed ||
    pokemon.poisoned ||
    pokemon.toxic ||
    pokemon.sleepTurnsRemaining > 0
  );
};

const getPrimaryStatusTexts = (pokemon: SimBattlePokemon) => {
  if (pokemon.currentHp <= 0) {
    return [];
  }

  const statuses: string[] = [];

  if (pokemon.burned) {
    statuses.push("やけど");
  }

  if (pokemon.paralyzed) {
    statuses.push("まひ");
  }

  if (pokemon.toxic) {
    statuses.push("もうどく");
  } else if (pokemon.poisoned) {
    statuses.push("どく");
  }

  if (pokemon.sleepTurnsRemaining > 0) {
    statuses.push("ねむり");
  }

  if (hasActiveSubstitute(pokemon)) {
    statuses.push("みがわり");
  }

  if (pokemon.wishTurnsRemaining > 0) {
    statuses.push("ねがいごと");
  }

  if (pokemon.leechSeededBy) {
    statuses.push("やどりぎ");
  }

  if (pokemon.aquaRingActive) {
    statuses.push("アクアリング");
  }

  if (pokemon.ingrainActive) {
    statuses.push("ねをはる");
  }

  return statuses;
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
  return recoverPokemon(pokemon, healing);
};

const getFieldRecovery = (pokemon: SimBattlePokemon) => {
  return Math.max(1, Math.floor(pokemon.stats.hp / 16));
};

const applyAquaRingRecovery = (pokemon: SimBattlePokemon) => {
  if (!pokemon.aquaRingActive) {
    return 0;
  }

  return recoverPokemon(
    pokemon,
    applyBigRootRecoveryBoost(pokemon, getFieldRecovery(pokemon))
  );
};

const applyIngrainRecovery = (pokemon: SimBattlePokemon) => {
  if (!pokemon.ingrainActive) {
    return 0;
  }

  return recoverPokemon(
    pokemon,
    applyBigRootRecoveryBoost(pokemon, getFieldRecovery(pokemon))
  );
};

const getLeechSeedDamage = (pokemon: SimBattlePokemon) => {
  return Math.max(1, Math.floor(pokemon.stats.hp / 8));
};

const applyLeechSeedEndOfTurn = (
  seeded: SimBattlePokemon,
  healer: SimBattlePokemon
) => {
  if (
    seeded.currentHp <= 0 ||
    healer.currentHp <= 0 ||
    seeded.leechSeededBy !== healer.side
  ) {
    return 0;
  }

  const drainedHp = Math.min(seeded.currentHp, getLeechSeedDamage(seeded));
  seeded.currentHp = Math.max(0, seeded.currentHp - drainedHp);
  recoverPokemon(
    healer,
    applyBigRootRecoveryBoost(healer, drainedHp)
  );

  return drainedHp;
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

const applyRest = (pokemon: SimBattlePokemon) => {
  if (!canUseRest(pokemon)) {
    return 0;
  }

  const recovered = Math.max(0, pokemon.stats.hp - pokemon.currentHp);
  pokemon.currentHp = pokemon.stats.hp;
  pokemon.burned = false;
  pokemon.paralyzed = false;
  pokemon.poisoned = false;
  pokemon.toxic = false;
  pokemon.toxicTurns = 0;
  pokemon.sleepTurnsRemaining = 2;
  refreshEffectiveSpeed(pokemon);
  return recovered;
};

const moveTargetsOpponent = (move: SimMove) => {
  return Boolean(move.targetStatus || move.targetStatChanges || move.leechSeed);
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

const canApplyLeechSeed = (defender: SimBattlePokemon) => {
  return (
    defender.currentHp > 0 &&
    !defender.pokemon.types.includes("grass") &&
    !defender.leechSeededBy
  );
};

const refreshEffectiveSpeed = (pokemon: SimBattlePokemon) => {
  pokemon.effectiveSpeed = getEffectiveSpeed(
    pokemon.stats.spe,
    pokemon.itemId,
    pokemon.stages.spe,
    pokemon.paralyzed
  );
};

const consumeSleepTurn = (pokemon: SimBattlePokemon) => {
  if (pokemon.sleepTurnsRemaining <= 0) {
    return false;
  }

  pokemon.sleepTurnsRemaining = Math.max(0, pokemon.sleepTurnsRemaining - 1);
  return true;
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

const applyDamageToTarget = (
  defender: SimBattlePokemon,
  damage: number
) => {
  if (hasActiveSubstitute(defender)) {
    const damageToSubstitute = Math.min(defender.substituteHp, damage);
    defender.substituteHp = Math.max(0, defender.substituteHp - damage);

    return {
      damageToHp: 0,
      totalDamageDealt: damageToSubstitute,
      hitSubstitute: true,
    };
  }

  const defenderHpBeforeHit = defender.currentHp;
  const wouldBeKo = damage >= defender.currentHp;
  const focusSashActivates =
    defender.itemId === "focusSash" &&
    !defender.focusSashUsed &&
    defender.currentHp === defender.stats.hp &&
    wouldBeKo;

  if (focusSashActivates) {
    defender.focusSashUsed = true;
    defender.currentHp = 1;
  } else {
    defender.currentHp = Math.max(0, defender.currentHp - damage);
  }

  const damageToHp = Math.max(defenderHpBeforeHit - defender.currentHp, 0);

  return {
    damageToHp,
    totalDamageDealt: damageToHp,
    hitSubstitute: false,
  };
};

const maybeApplyBurn = (
  defender: SimBattlePokemon,
  move: SimMove,
  damageDealt: number
) => {
  if (!move.burnChance || damageDealt <= 0 || !canApplyBurn(defender)) {
    return;
  }

  if (Math.random() < move.burnChance) {
    defender.burned = true;
  }
};

const maybeApplyFlinch = (
  defender: SimBattlePokemon,
  move: SimMove,
  damageDealt: number,
  canFlinchTarget: boolean
) => {
  if (!canFlinchTarget || !move.flinchChance || damageDealt <= 0 || defender.currentHp <= 0) {
    return;
  }

  if (Math.random() < move.flinchChance) {
    defender.flinched = true;
  }
};

const maybeApplyPoison = (
  defender: SimBattlePokemon,
  move: SimMove,
  damageDealt: number
) => {
  if (damageDealt <= 0 || defender.currentHp <= 0) {
    return;
  }

  if (move.poisonChance && canApplyPoison(defender, move) && Math.random() < move.poisonChance) {
    defender.poisoned = true;
    defender.toxic = false;
    defender.toxicTurns = 0;
    return;
  }

  if (move.toxicChance && canApplyPoison(defender, move) && Math.random() < move.toxicChance) {
    defender.toxic = true;
    defender.poisoned = false;
    defender.toxicTurns = 1;
  }
};

const maybeFullyParalyzed = (pokemon: SimBattlePokemon) => {
  return pokemon.paralyzed && Math.random() < 0.25;
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
  if (chance <= 0) {
    return [];
  }

  if (chance < 1 && Math.random() >= chance) {
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
    return "やけど";
  }

  if (move.targetStatus === "paralysis" && canApplyParalysis(defender, move)) {
    defender.paralyzed = true;
    refreshEffectiveSpeed(defender);
    return "まひ";
  }

  if (move.targetStatus === "poison" && canApplyPoison(defender, move)) {
    defender.poisoned = true;
    defender.toxic = false;
    defender.toxicTurns = 0;
    return "どく";
  }

  if (move.targetStatus === "toxic" && canApplyPoison(defender, move)) {
    defender.toxic = true;
    defender.poisoned = false;
    defender.toxicTurns = 1;
    return "もうどく";
  }

  return null;
};

const applyHealingFieldEffects = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove,
  includeTargetEffects = true
) => {
  const statusTexts: string[] = [];

  if (includeTargetEffects && move.leechSeed && canApplyLeechSeed(defender)) {
    defender.leechSeededBy = attacker.side;
    statusTexts.push("やどりぎ");
  }

  if (move.aquaRing && !attacker.aquaRingActive) {
    attacker.aquaRingActive = true;
    statusTexts.push("アクアリング");
  }

  if (move.ingrain && !attacker.ingrainActive) {
    attacker.ingrainActive = true;
    statusTexts.push("ねをはる");
  }

  return statusTexts;
};

const canUseProtect = (pokemon: SimBattlePokemon) => {
  return !pokemon.protectUsedLastTurn;
};

const finishTurnState = (leftPokemon: SimBattlePokemon, rightPokemon: SimBattlePokemon) => {
  leftPokemon.protectUsedLastTurn = leftPokemon.protectActive;
  rightPokemon.protectUsedLastTurn = rightPokemon.protectActive;
  leftPokemon.protectActive = false;
  rightPokemon.protectActive = false;
};

const applyEndOfTurnEffects = (
  leftPokemon: SimBattlePokemon,
  rightPokemon: SimBattlePokemon
): SimulationSide | null => {
  if (leftPokemon.currentHp <= 0 || rightPokemon.currentHp <= 0) {
    return null;
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
    applyLeechSeedEndOfTurn(leftPokemon, rightPokemon);
    applyLeechSeedEndOfTurn(rightPokemon, leftPokemon);
  }

  if (leftPokemon.currentHp > 0 && rightPokemon.currentHp > 0) {
    applyWishEndOfTurn(leftPokemon);
    applyWishEndOfTurn(rightPokemon);
    applyAquaRingRecovery(leftPokemon);
    applyAquaRingRecovery(rightPokemon);
    applyIngrainRecovery(leftPokemon);
    applyIngrainRecovery(rightPokemon);
    applyLeftoversRecovery(leftPokemon);
    applyLeftoversRecovery(rightPokemon);
  }

  if (leftPokemon.currentHp <= 0 && rightPokemon.currentHp <= 0) {
    return "draw";
  }

  if (leftPokemon.currentHp <= 0) {
    return "right";
  }

  if (rightPokemon.currentHp <= 0) {
    return "left";
  }

  return null;
};

export const applyRepresentativeEndOfTurnEffects = (
  leftPokemon: SimBattlePokemon,
  rightPokemon: SimBattlePokemon
) => {
  if (leftPokemon.currentHp <= 0 || rightPokemon.currentHp <= 0) {
    return null;
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
    applyLeechSeedEndOfTurn(leftPokemon, rightPokemon);
    applyLeechSeedEndOfTurn(rightPokemon, leftPokemon);
  }

  if (leftPokemon.currentHp > 0 && rightPokemon.currentHp > 0) {
    applyWishEndOfTurn(leftPokemon);
    applyWishEndOfTurn(rightPokemon);
    applyAquaRingRecovery(leftPokemon);
    applyAquaRingRecovery(rightPokemon);
    applyIngrainRecovery(leftPokemon);
    applyIngrainRecovery(rightPokemon);
    applyLeftoversRecovery(leftPokemon);
    applyLeftoversRecovery(rightPokemon);
  }

  if (leftPokemon.currentHp <= 0 && rightPokemon.currentHp <= 0) {
    return "draw";
  }

  if (leftPokemon.currentHp <= 0) {
    return "right";
  }

  if (rightPokemon.currentHp <= 0) {
    return "left";
  }

  finishTurnState(leftPokemon, rightPokemon);

  return null;
};

const resolveMoveAction = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  selectedMove: SimMove,
  battleResult: Pick<SimulationBattleResult, "moveUsage" | "koHitCount" | "moveKoCounts">,
  canFlinchTarget = false
): ActionResult => {
  battleResult.moveUsage[attacker.side][selectedMove.id] =
    (battleResult.moveUsage[attacker.side][selectedMove.id] ?? 0) + 1;

  if (consumeSleepTurn(attacker)) {
    return "none";
  }

  if (maybeFullyParalyzed(attacker)) {
    return "none";
  }

  if (selectedMove.protect) {
    if (canUseProtect(attacker)) {
      attacker.protectActive = true;
    }
    return "none";
  }

  if (selectedMove.substitute) {
    applySubstitute(attacker);
    return "none";
  }

  if (!doesMoveHit(attacker, selectedMove)) {
    return "none";
  }

  if (selectedMove.category === "status") {
    const blockedBySubstitute = hasActiveSubstitute(defender) && moveTargetsOpponent(selectedMove);

    if (selectedMove.rest) {
      applyRest(attacker);
    }

    if (selectedMove.wish) {
      scheduleWish(attacker);
    }

    if (!defender.protectActive && !blockedBySubstitute) {
      applyTargetStatus(defender, selectedMove);
      applyTargetStatChanges(defender, selectedMove);
      applyHealingFieldEffects(attacker, defender, selectedMove);
    }
    applySelfStatChanges(attacker, selectedMove);
    if (defender.protectActive || blockedBySubstitute || !moveTargetsOpponent(selectedMove)) {
      applyHealingFieldEffects(attacker, defender, selectedMove, false);
    }
    if (!selectedMove.wish && !selectedMove.rest) {
      applySelfRecovery(attacker, selectedMove);
    }
    return "none";
  }

  if (defender.protectActive) {
    applySelfStatChanges(attacker, selectedMove);
    return "none";
  }

  let totalHpDamageDealt = 0;

  for (const power of getSampledHitPowers(attacker, selectedMove)) {
    if (attacker.currentHp <= 0 || defender.currentHp <= 0) {
      break;
    }

    const damage = calcAttackDamage(attacker, defender, selectedMove, power);
    const hitResult = applyDamageToTarget(defender, damage);
    totalHpDamageDealt += hitResult.damageToHp;
    applyRockyHelmetDamage(attacker, defender, selectedMove, hitResult.totalDamageDealt);
    applyRecoveryEffects(attacker, hitResult.totalDamageDealt, selectedMove);
    applyRecoilEffects(attacker, hitResult.totalDamageDealt, selectedMove);

    if (attacker.currentHp <= 0) {
      if (defender.currentHp <= 0) {
        battleResult.koHitCount[attacker.side] =
          (battleResult.koHitCount[attacker.side] ?? 0) + 1;
        battleResult.moveKoCounts[attacker.side][selectedMove.id] =
          (battleResult.moveKoCounts[attacker.side][selectedMove.id] ?? 0) + 1;
        return "doubleFainted";
      }
      return "attackerFainted";
    }

    if (defender.currentHp <= 0) {
      battleResult.koHitCount[attacker.side] =
        (battleResult.koHitCount[attacker.side] ?? 0) + 1;
      battleResult.moveKoCounts[attacker.side][selectedMove.id] =
        (battleResult.moveKoCounts[attacker.side][selectedMove.id] ?? 0) + 1;
      return "defenderFainted";
    }
  }

  applySelfStatChanges(attacker, selectedMove);
  if (attacker.currentHp > 0 && defender.currentHp > 0 && !hasActiveSubstitute(defender)) {
    maybeApplyBurn(defender, selectedMove, totalHpDamageDealt);
    maybeApplyPoison(defender, selectedMove, totalHpDamageDealt);
    applyTargetStatChanges(defender, selectedMove);
    maybeApplyFlinch(defender, selectedMove, totalHpDamageDealt, canFlinchTarget);
  }

  return "none";
};

const getRepresentativeDamage = (
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

const doesRepresentativeMoveHit = (
  attacker: SimBattlePokemon,
  move: SimMove
) => {
  return getMoveAccuracy(attacker, move) >= 50;
};

const applyRepresentativeAttack = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  selectedMove: SimMove,
  forcedHit?: boolean
): {
  result: ActionResult;
  action: ProjectedBattleAction;
} => {
  if (consumeSleepTurn(attacker)) {
    return {
      result: "none",
      action: {
        side: attacker.side,
        moveId: selectedMove.id,
        moveName: selectedMove.name,
        hit: false,
        hitCount: 0,
        hitDamages: [],
        statusTexts: ["ねむっている"],
        damage: 0,
        targetSide: defender.side,
        targetRemainingHp: defender.currentHp,
      },
    };
  }

  const willHit = forcedHit ?? doesRepresentativeMoveHit(attacker, selectedMove);

  if (selectedMove.protect) {
    const protectSuccess = canUseProtect(attacker);
    if (protectSuccess) {
      attacker.protectActive = true;
    }

    return {
      result: "none",
      action: {
        side: attacker.side,
        moveId: selectedMove.id,
        moveName: selectedMove.name,
        hit: protectSuccess,
        hitCount: 0,
        hitDamages: [],
        statusTexts: [protectSuccess ? "まもる" : "まもる失敗"],
        damage: 0,
        targetSide: defender.side,
        targetRemainingHp: defender.currentHp,
      },
    };
  }

  if (selectedMove.substitute) {
    const substituteSuccess = applySubstitute(attacker);

    return {
      result: "none",
      action: {
        side: attacker.side,
        moveId: selectedMove.id,
        moveName: selectedMove.name,
        hit: substituteSuccess,
        hitCount: 0,
        hitDamages: [],
        statusTexts: [substituteSuccess ? "みがわり" : "みがわり失敗"],
        damage: 0,
        targetSide: defender.side,
        targetRemainingHp: defender.currentHp,
      },
    };
  }

  if (!willHit) {
    return {
      result: "none",
      action: {
        side: attacker.side,
        moveId: selectedMove.id,
        moveName: selectedMove.name,
        hit: false,
        hitCount: 0,
        hitDamages: [],
        statusTexts: [],
        damage: 0,
        targetSide: defender.side,
        targetRemainingHp: defender.currentHp,
      },
    };
  }

  if (selectedMove.category === "status") {
    const statusTexts = applySelfStatChanges(attacker, selectedMove);
    if (selectedMove.rest) {
      const recoveredHp = applyRest(attacker);
      statusTexts.push(recoveredHp > 0 ? "HP全回復" : "ねむる失敗");
    }
    if (selectedMove.wish) {
      scheduleWish(attacker);
      statusTexts.push("ねがいごと");
    }
    const recoveredHp = selectedMove.wish || selectedMove.rest ? 0 : applySelfRecovery(attacker, selectedMove);
    if (recoveredHp > 0) {
      statusTexts.push("HP回復");
    }
    const blockedBySubstitute = hasActiveSubstitute(defender) && moveTargetsOpponent(selectedMove);
    if (!defender.protectActive && !blockedBySubstitute) {
      statusTexts.push(...applyTargetStatChanges(defender, selectedMove, 1));
      const targetStatusText = applyTargetStatus(defender, selectedMove);
      if (targetStatusText) {
        statusTexts.push(targetStatusText);
      }
      statusTexts.push(...applyHealingFieldEffects(attacker, defender, selectedMove));
    } else if (moveTargetsOpponent(selectedMove)) {
      statusTexts.push(...applyHealingFieldEffects(attacker, defender, selectedMove, false));
      statusTexts.push(defender.protectActive ? "まもられた" : "みがわり");
    } else {
      statusTexts.push(...applyHealingFieldEffects(attacker, defender, selectedMove, false));
    }

    return {
      result: "none",
      action: {
        side: attacker.side,
        moveId: selectedMove.id,
        moveName: selectedMove.name,
        hit: true,
        hitCount: 0,
        hitDamages: [],
        statusTexts,
        damage: 0,
        targetSide: defender.side,
        targetRemainingHp: defender.currentHp,
      },
    };
  }

  if (defender.protectActive) {
    return {
      result: "none",
      action: {
        side: attacker.side,
        moveId: selectedMove.id,
        moveName: selectedMove.name,
        hit: true,
        hitCount: 0,
        hitDamages: [],
        statusTexts: [...applySelfStatChanges(attacker, selectedMove), "まもられた"],
        damage: 0,
        targetSide: defender.side,
        targetRemainingHp: defender.currentHp,
      },
    };
  }

  const hitDamages: number[] = [];
  let totalHpDamageDealt = 0;
  let hitSubstitute = false;

  for (const power of getRepresentativeHitPowers(selectedMove)) {
    if (attacker.currentHp <= 0 || defender.currentHp <= 0) {
      break;
    }

    const damage = getRepresentativeDamage(attacker, defender, selectedMove, power);
    const hitResult = applyDamageToTarget(defender, damage);
    totalHpDamageDealt += hitResult.damageToHp;
    hitSubstitute ||= hitResult.hitSubstitute;
    hitDamages.push(hitResult.totalDamageDealt);
    applyRockyHelmetDamage(
      attacker,
      defender,
      selectedMove,
      hitResult.totalDamageDealt
    );
  }

  const damageDealt = hitDamages.reduce((sum, value) => sum + value, 0);
  applyRecoveryEffects(attacker, damageDealt, selectedMove);
  applyRecoilEffects(attacker, damageDealt, selectedMove);
  const selfStageTexts = applySelfStatChanges(attacker, selectedMove);
  const targetStageTexts =
    totalHpDamageDealt > 0 &&
    defender.currentHp > 0 &&
    !hasActiveSubstitute(defender) &&
    (selectedMove.targetStatChangeChance ?? 1) >= 1
      ? applyTargetStatChanges(defender, selectedMove, 1)
      : [];

  const action: ProjectedBattleAction = {
    side: attacker.side,
    moveId: selectedMove.id,
    moveName: selectedMove.name,
    hit: true,
    hitCount: hitDamages.length,
    hitDamages,
    statusTexts: [...selfStageTexts, ...targetStageTexts, ...(hitSubstitute ? ["みがわり"] : [])],
    damage: hitDamages.reduce((sum, value) => sum + value, 0),
    targetSide: defender.side,
    targetRemainingHp: defender.currentHp,
  };

  if (defender.currentHp <= 0) {
    if (attacker.currentHp <= 0) {
      return { result: "doubleFainted", action };
    }

    return { result: "defenderFainted", action };
  }

  if (attacker.currentHp <= 0) {
    return { result: "attackerFainted", action };
  }

  return { result: "none", action };
};

const getProjectedBattleParticipants = (
  leftPokemon: SimBattlePokemon,
  rightPokemon: SimBattlePokemon,
  actingSide: Exclude<SimulationSide, "draw">
) => {
  return actingSide === "left"
    ? {
        attacker: leftPokemon,
        defender: rightPokemon,
      }
    : {
        attacker: rightPokemon,
        defender: leftPokemon,
      };
};

const branchProjectedStatusOutcomes = (
  branches: ProjectedStatusOutcome[],
  chance: number,
  actingSide: Exclude<SimulationSide, "draw">,
  canApply: (attacker: SimBattlePokemon, defender: SimBattlePokemon) => boolean,
  apply: (
    attacker: SimBattlePokemon,
    defender: SimBattlePokemon
  ) => string[]
) => {
  const normalizedChance = Math.max(0, Math.min(1, chance));
  if (normalizedChance <= 0) {
    return branches;
  }

  return branches.flatMap((branch) => {
    const currentLeft = cloneBattlePokemon(branch.leftPokemon);
    const currentRight = cloneBattlePokemon(branch.rightPokemon);
    const { attacker, defender } = getProjectedBattleParticipants(
      currentLeft,
      currentRight,
      actingSide
    );

    if (!canApply(attacker, defender)) {
      return [branch];
    }

    const outcomes: ProjectedStatusOutcome[] = [];

    if (normalizedChance < 1) {
      outcomes.push({
        leftPokemon: branch.leftPokemon,
        rightPokemon: branch.rightPokemon,
        action: {
          ...branch.action,
          statusTexts: [...branch.action.statusTexts],
        },
        probability: branch.probability * (1 - normalizedChance),
      });
    }

    const statusTexts = apply(attacker, defender);
    outcomes.push({
      leftPokemon: currentLeft,
      rightPokemon: currentRight,
      action: {
        ...branch.action,
        statusTexts: [...branch.action.statusTexts, ...statusTexts],
      },
      probability: branch.probability * normalizedChance,
    });

    return outcomes.filter((outcome) => outcome.probability > 0);
  });
};

const getProjectedStatusOutcomeOptions = (
  leftPokemon: SimBattlePokemon,
  rightPokemon: SimBattlePokemon,
  actingSide: Exclude<SimulationSide, "draw">,
  move: SimMove,
  action: ProjectedBattleAction,
  canFlinchTarget: boolean
): ProjectedStatusOutcome[] => {
  const initialBranches: ProjectedStatusOutcome[] = [
    {
      leftPokemon,
      rightPokemon,
      action: {
        ...action,
        statusTexts: [...action.statusTexts],
      },
      probability: 1,
    },
  ];

  if (action.damage <= 0) {
    return initialBranches;
  }

  const { attacker, defender } = getProjectedBattleParticipants(
    leftPokemon,
    rightPokemon,
    actingSide
  );

  if (attacker.currentHp <= 0 || defender.currentHp <= 0) {
    return initialBranches;
  }

  let branches = initialBranches;

  if (move.burnChance) {
    branches = branchProjectedStatusOutcomes(
      branches,
      move.burnChance,
      actingSide,
      (_, projectedDefender) =>
        !hasActiveSubstitute(projectedDefender) && canApplyBurn(projectedDefender),
      (_, projectedDefender) => {
        projectedDefender.burned = true;
        return ["やけど"];
      }
    );
  }

  if (move.poisonChance) {
    branches = branchProjectedStatusOutcomes(
      branches,
      move.poisonChance,
      actingSide,
      (_, projectedDefender) =>
        !hasActiveSubstitute(projectedDefender) && canApplyPoison(projectedDefender, move),
      (_, projectedDefender) => {
        projectedDefender.poisoned = true;
        projectedDefender.toxic = false;
        projectedDefender.toxicTurns = 0;
        return ["どく"];
      }
    );
  }

  if (move.toxicChance) {
    branches = branchProjectedStatusOutcomes(
      branches,
      move.toxicChance,
      actingSide,
      (_, projectedDefender) =>
        !hasActiveSubstitute(projectedDefender) && canApplyPoison(projectedDefender, move),
      (_, projectedDefender) => {
        projectedDefender.toxic = true;
        projectedDefender.poisoned = false;
        projectedDefender.toxicTurns = 1;
        return ["もうどく"];
      }
    );
  }

  if (move.targetStatChanges) {
    branches = branchProjectedStatusOutcomes(
      branches,
      move.targetStatChangeChance ?? 1,
      actingSide,
      (_, projectedDefender) =>
        projectedDefender.currentHp > 0 && !hasActiveSubstitute(projectedDefender),
      (_, projectedDefender) => applyTargetStatChanges(projectedDefender, move, 1)
    );
  }

  if (canFlinchTarget && move.flinchChance) {
    branches = branchProjectedStatusOutcomes(
      branches,
      move.flinchChance,
      actingSide,
      (_, projectedDefender) =>
        projectedDefender.currentHp > 0 && !hasActiveSubstitute(projectedDefender),
      (_, projectedDefender) => {
        projectedDefender.flinched = true;
        return ["ひるみ"];
      }
    );
  }

  return branches;
};

const getTurnOrder = (
  left: SimBattlePokemon,
  right: SimBattlePokemon,
  leftMove: SimMove,
  rightMove: SimMove
): TurnOrderResult => {
  const options = getTurnOrderOptions(left, right, leftMove, rightMove);

  if (options.length === 1) {
    return options[0];
  }

  return Math.random() < 0.5 ? options[0] : options[1];
};

const getTurnOrderOptions = (
  left: SimBattlePokemon,
  right: SimBattlePokemon,
  leftMove: SimMove,
  rightMove: SimMove
): TurnOrderOption[] => {
  const leftPriority = getMovePriority(leftMove);
  const rightPriority = getMovePriority(rightMove);

  if (leftPriority > rightPriority) {
    return [
      {
        first: left,
        second: right,
        firstMove: leftMove,
        secondMove: rightMove,
        reason: "priority",
        probability: 1,
      },
    ];
  }

  if (rightPriority > leftPriority) {
    return [
      {
        first: right,
        second: left,
        firstMove: rightMove,
        secondMove: leftMove,
        reason: "priority",
        probability: 1,
      },
    ];
  }

  if (left.effectiveSpeed > right.effectiveSpeed) {
    return [
      {
        first: left,
        second: right,
        firstMove: leftMove,
        secondMove: rightMove,
        reason: "faster",
        probability: 1,
      },
    ];
  }

  if (right.effectiveSpeed > left.effectiveSpeed) {
    return [
      {
        first: right,
        second: left,
        firstMove: rightMove,
        secondMove: leftMove,
        reason: "faster",
        probability: 1,
      },
    ];
  }

  return [
    {
      first: left,
      second: right,
      firstMove: leftMove,
      secondMove: rightMove,
      reason: "speedTie",
      probability: 0.5,
    },
    {
      first: right,
      second: left,
      firstMove: rightMove,
      secondMove: leftMove,
      reason: "speedTie",
      probability: 0.5,
    },
  ];
};

const selectTurnMoves = (
  left: SimBattlePokemon,
  right: SimBattlePokemon,
  forcedMoves?: ForcedTurnMoves
): TurnMoveSelection => {
  const leftMove =
    forcedMoves && "leftMove" in forcedMoves
      ? forcedMoves.leftMove ?? null
      : chooseBestMove(left, right, FAST_SIMULATION_AI_OPTIONS);
  const rightMove =
    forcedMoves && "rightMove" in forcedMoves
      ? forcedMoves.rightMove ?? null
      : chooseBestMove(right, left, FAST_SIMULATION_AI_OPTIONS);

  applyChoiceLock(left, leftMove);
  applyChoiceLock(right, rightMove);

  return {
    leftMove,
    rightMove,
  };
};

const getForcedWinner = (
  turnMoves: TurnMoveSelection
): SimulationSide | null => {
  if (!turnMoves.leftMove && !turnMoves.rightMove) {
    return "draw";
  }
  if (!turnMoves.leftMove) {
    return "right";
  }
  if (!turnMoves.rightMove) {
    return "left";
  }
  return null;
};

const finalizeBattle = (
  leftPokemon: SimBattlePokemon,
  rightPokemon: SimBattlePokemon,
  battleStats: SimulationBattleStats,
  winner: SimulationSide,
  turns: number
): SimulationBattleResult => {
  battleStats.endingHp.left = leftPokemon.currentHp;
  battleStats.endingHp.right = rightPokemon.currentHp;
  battleStats.endingHpPercent.left = (leftPokemon.currentHp / leftPokemon.stats.hp) * 100;
  battleStats.endingHpPercent.right = (rightPokemon.currentHp / rightPokemon.stats.hp) * 100;

  return {
    winner,
    turns,
    ...battleStats,
  };
};

const resolveTurn = (
  turn: number,
  leftPokemon: SimBattlePokemon,
  rightPokemon: SimBattlePokemon,
  attackCounts: Record<"left" | "right", number>,
  battleStats: SimulationBattleStats,
  forcedMoves?: ForcedTurnMoves
): SimulationSide | null => {
  leftPokemon.flinched = false;
  rightPokemon.flinched = false;
  leftPokemon.protectActive = false;
  rightPokemon.protectActive = false;

  const turnMoves = selectTurnMoves(leftPokemon, rightPokemon, forcedMoves);
  const forcedWinner = getForcedWinner(turnMoves);

  if (forcedWinner) {
    return forcedWinner;
  }

  if (!turnMoves.leftMove || !turnMoves.rightMove) {
    return "draw";
  }

  const order = getTurnOrder(leftPokemon, rightPokemon, turnMoves.leftMove, turnMoves.rightMove);
  battleStats.firstMoveCounts[order.first.side] += 1;
  battleStats.firstMoveByReason[order.first.side][order.reason] += 1;

  if (turn === 1) {
    battleStats.openingOrderReason = order.reason;
    battleStats.openingFirstSide = order.first.side;
  }

  battleStats.turnMoveHistory.left.push(turnMoves.leftMove.id);
  battleStats.turnMoveHistory.right.push(turnMoves.rightMove.id);

  attackCounts[order.first.side] += 1;
  const firstAction = resolveMoveAction(order.first, order.second, order.firstMove, battleStats, true);
  if (firstAction === "doubleFainted") {
    battleStats.koHitCount[order.first.side] = attackCounts[order.first.side];
    return "draw";
  }
  if (firstAction === "defenderFainted") {
    battleStats.koHitCount[order.first.side] = attackCounts[order.first.side];
    return order.first.side;
  }
  if (firstAction === "attackerFainted") {
    return order.second.side;
  }

  if (!order.second.flinched) {
    attackCounts[order.second.side] += 1;
    const secondAction = resolveMoveAction(order.second, order.first, order.secondMove, battleStats);
    if (secondAction === "doubleFainted") {
      battleStats.koHitCount[order.second.side] = attackCounts[order.second.side];
      return "draw";
    }
    if (secondAction === "defenderFainted") {
      battleStats.koHitCount[order.second.side] = attackCounts[order.second.side];
      return order.second.side;
    }
    if (secondAction === "attackerFainted") {
      return order.first.side;
    }
  }

  const burnWinner = applyEndOfTurnEffects(leftPokemon, rightPokemon);
  finishTurnState(leftPokemon, rightPokemon);
  if (burnWinner) {
    return burnWinner;
  }

  return null;
};

export const simulateSingleBattle = (
  left: SimulatorInput,
  right: SimulatorInput,
  moveMap: Map<string, SimMove>,
  level = DEFAULT_LEVEL
): SimulationBattleResult => {
  const leftPokemon = buildBattlePokemon("left", left, moveMap, level);
  const rightPokemon = buildBattlePokemon("right", right, moveMap, level);
  const attackCounts = {
    left: 0,
    right: 0,
  };
  const battleStats = createBattleStats();

  for (let turn = 1; turn <= MAX_TURNS; turn += 1) {
    const winner = resolveTurn(
      turn,
      leftPokemon,
      rightPokemon,
      attackCounts,
      battleStats
    );

    if (winner) {
      return finalizeBattle(leftPokemon, rightPokemon, battleStats, winner, turn);
    }
  }

  if (leftPokemon.currentHp === rightPokemon.currentHp) {
    return finalizeBattle(leftPokemon, rightPokemon, battleStats, "draw", MAX_TURNS);
  }

  return leftPokemon.currentHp > rightPokemon.currentHp
    ? finalizeBattle(leftPokemon, rightPokemon, battleStats, "left", MAX_TURNS)
    : finalizeBattle(leftPokemon, rightPokemon, battleStats, "right", MAX_TURNS);
};

export const simulateBattleFromState = ({
  leftPokemon,
  rightPokemon,
  openingMoves,
}: {
  leftPokemon: SimBattlePokemon;
  rightPokemon: SimBattlePokemon;
  openingMoves?: ForcedTurnMoves;
}): SimulationBattleResult => {
  clearAiCaches();
  const nextLeftPokemon = cloneBattlePokemon(leftPokemon);
  const nextRightPokemon = cloneBattlePokemon(rightPokemon);
  const attackCounts = {
    left: 0,
    right: 0,
  };
  const battleStats = createBattleStats();

  for (let turn = 1; turn <= MAX_TURNS; turn += 1) {
    const winner = resolveTurn(
      turn,
      nextLeftPokemon,
      nextRightPokemon,
      attackCounts,
      battleStats,
      turn === 1 ? openingMoves : undefined
    );

    if (winner) {
      return finalizeBattle(nextLeftPokemon, nextRightPokemon, battleStats, winner, turn);
    }
  }

  if (nextLeftPokemon.currentHp === nextRightPokemon.currentHp) {
    return finalizeBattle(nextLeftPokemon, nextRightPokemon, battleStats, "draw", MAX_TURNS);
  }

  return nextLeftPokemon.currentHp > nextRightPokemon.currentHp
    ? finalizeBattle(nextLeftPokemon, nextRightPokemon, battleStats, "left", MAX_TURNS)
    : finalizeBattle(nextLeftPokemon, nextRightPokemon, battleStats, "right", MAX_TURNS);
};

export const simulateMatchup = ({
  left,
  right,
  moveMap,
  battleCount,
  level = DEFAULT_LEVEL,
}: SimulateBattleArgs): SimulationResult => {
  clearAiCaches();
  const battles: SimulationBattleResult[] = [];

  for (let i = 0; i < battleCount; i += 1) {
    battles.push(simulateSingleBattle(left, right, moveMap, level));
  }

  return { battles };
};

export const simulateMatchupByRightItemCandidates = ({
  left,
  right,
  rightItemIds,
  moveMap,
  battleCount,
  level = DEFAULT_LEVEL,
}: Omit<SimulateBattleArgs, "right"> & {
  right: SimulatorInput;
  rightItemIds: string[];
}): ItemCandidateSimulationGroup => {
  const normalizedItemIds = Array.from(
    new Set(rightItemIds.map((itemId) => itemId.trim()).filter((itemId) => itemId.length > 0))
  );
  const byRightItem: ItemCandidateSimulationResult[] = [];

  for (const itemId of normalizedItemIds) {
    clearAiCaches();

    const battles: SimulationBattleResult[] = [];
    const rightWithCandidate = {
      ...right,
      itemId,
    };

    for (let i = 0; i < battleCount; i += 1) {
      battles.push(simulateSingleBattle(left, rightWithCandidate, moveMap, level));
    }

    byRightItem.push({
      itemId,
      battles,
    });
  }

  return {
    battles: byRightItem.flatMap((entry) => entry.battles),
    byRightItem,
  };
};

const getRepresentativeMoveOutcomeOptions = (
  attacker: SimBattlePokemon,
  move: SimMove
) => {
  const accuracy = Math.max(0, Math.min(100, getMoveAccuracy(attacker, move))) / 100;

  if (accuracy <= 0) {
    return [{ hit: false, probability: 1 }];
  }

  if (accuracy >= 1) {
    return [{ hit: true, probability: 1 }];
  }

  return [
    { hit: true, probability: accuracy },
    { hit: false, probability: 1 - accuracy },
  ].sort((a, b) => b.probability - a.probability);
};

const shouldUseExpandedProjectedMoveSearch = (
  leftPokemon: SimBattlePokemon,
  rightPokemon: SimBattlePokemon
) => {
  const hasLongGameMove = (pokemon: SimBattlePokemon) =>
    pokemon.moves.some(
      (move) =>
        move.protect ||
        move.substitute ||
        move.targetStatus === "poison" ||
        move.targetStatus === "toxic"
    );

  return hasLongGameMove(leftPokemon) || hasLongGameMove(rightPokemon);
};

const buildProjectedTurn = (
  turn: number,
  order: TurnOrderOption,
  actions: ProjectedBattleAction[],
  leftPokemon: SimBattlePokemon,
  rightPokemon: SimBattlePokemon
): ProjectedBattleTurn => ({
  turn,
  firstSide: order.first.side,
  reason: order.reason,
  actions,
  endingHp: {
    left: leftPokemon.currentHp,
    right: rightPokemon.currentHp,
  },
  endingStatuses: {
    left: getPrimaryStatusTexts(leftPokemon),
    right: getPrimaryStatusTexts(rightPokemon),
  },
});

const expandProjectedBranch = (
  branch: ProjectedBranchState,
  turn: number
): ProjectedBranchState[] => {
  const leftMove = chooseBestMove(branch.leftPokemon, branch.rightPokemon);
  const rightMove = chooseBestMove(branch.rightPokemon, branch.leftPokemon);
  const forcedWinner = getForcedWinner({ leftMove, rightMove });

  if (forcedWinner) {
    return [{ ...branch, winner: forcedWinner }];
  }

  if (!leftMove || !rightMove) {
    return [{ ...branch, winner: "draw" }];
  }

  const expandedBranches: ProjectedBranchState[] = [];

  for (const order of getTurnOrderOptions(
    branch.leftPokemon,
    branch.rightPokemon,
    leftMove,
    rightMove
  )) {
        const baseLeft = cloneBattlePokemon(branch.leftPokemon);
        const baseRight = cloneBattlePokemon(branch.rightPokemon);
        baseLeft.flinched = false;
        baseRight.flinched = false;

        applyChoiceLock(baseLeft, leftMove);
        applyChoiceLock(baseRight, rightMove);

        for (const firstOutcome of getRepresentativeMoveOutcomeOptions(order.first, order.firstMove)) {
          const leftAfterFirst = cloneBattlePokemon(baseLeft);
          const rightAfterFirst = cloneBattlePokemon(baseRight);
          const firstActor = order.first.side === "left" ? leftAfterFirst : rightAfterFirst;
          const firstDefender = order.first.side === "left" ? rightAfterFirst : leftAfterFirst;
          const firstAction = applyRepresentativeAttack(
            firstActor,
            firstDefender,
            order.firstMove,
            firstOutcome.hit
          );
          const firstProbability =
            branch.probability *
            order.probability *
            firstOutcome.probability;

          if (firstProbability <= 0) {
            continue;
          }

          const firstStatusOutcomes =
            firstAction.result === "none"
              ? getProjectedStatusOutcomeOptions(
                  leftAfterFirst,
                  rightAfterFirst,
                  order.first.side,
                  order.firstMove,
                  firstAction.action,
                  true
                )
              : [
                  {
                    leftPokemon: leftAfterFirst,
                    rightPokemon: rightAfterFirst,
                    action: firstAction.action,
                    probability: 1,
                  },
                ];

          for (const firstStatusOutcome of firstStatusOutcomes) {
            const firstResolvedProbability = firstProbability * firstStatusOutcome.probability;
            if (firstResolvedProbability <= 0) {
              continue;
            }

            const leftAfterStatuses = cloneBattlePokemon(firstStatusOutcome.leftPokemon);
            const rightAfterStatuses = cloneBattlePokemon(firstStatusOutcome.rightPokemon);
            const firstActions = [firstStatusOutcome.action];

            if (firstAction.result === "doubleFainted") {
              expandedBranches.push({
                leftPokemon: leftAfterStatuses,
                rightPokemon: rightAfterStatuses,
                turns: [
                  ...branch.turns,
                  buildProjectedTurn(turn, order, firstActions, leftAfterStatuses, rightAfterStatuses),
                ],
                probability: firstResolvedProbability,
                winner: "draw",
              });
              continue;
            }

            if (firstAction.result === "defenderFainted") {
              expandedBranches.push({
                leftPokemon: leftAfterStatuses,
                rightPokemon: rightAfterStatuses,
                turns: [
                  ...branch.turns,
                  buildProjectedTurn(turn, order, firstActions, leftAfterStatuses, rightAfterStatuses),
                ],
                probability: firstResolvedProbability,
                winner: order.first.side,
              });
              continue;
            }

            if (firstAction.result === "attackerFainted") {
              expandedBranches.push({
                leftPokemon: leftAfterStatuses,
                rightPokemon: rightAfterStatuses,
                turns: [
                  ...branch.turns,
                  buildProjectedTurn(turn, order, firstActions, leftAfterStatuses, rightAfterStatuses),
                ],
                probability: firstResolvedProbability,
                winner: order.second.side,
              });
              continue;
            }

            if (order.second.side === "left" ? leftAfterStatuses.flinched : rightAfterStatuses.flinched) {
              const burnWinner = applyRepresentativeEndOfTurnEffects(leftAfterStatuses, rightAfterStatuses);
              expandedBranches.push({
                leftPokemon: leftAfterStatuses,
                rightPokemon: rightAfterStatuses,
                turns: [
                  ...branch.turns,
                  buildProjectedTurn(turn, order, firstActions, leftAfterStatuses, rightAfterStatuses),
                ],
                probability: firstResolvedProbability,
                winner: burnWinner,
              });
              continue;
            }

            for (const secondOutcome of getRepresentativeMoveOutcomeOptions(order.second, order.secondMove)) {
              const leftAfterSecond = cloneBattlePokemon(leftAfterStatuses);
              const rightAfterSecond = cloneBattlePokemon(rightAfterStatuses);
              const secondActor = order.second.side === "left" ? leftAfterSecond : rightAfterSecond;
              const secondDefender = order.second.side === "left" ? rightAfterSecond : leftAfterSecond;
              const secondAction = applyRepresentativeAttack(
                secondActor,
                secondDefender,
                order.secondMove,
                secondOutcome.hit
              );
              const secondProbability = firstResolvedProbability * secondOutcome.probability;

              if (secondProbability <= 0) {
                continue;
              }

              const secondStatusOutcomes =
                secondAction.result === "none"
                  ? getProjectedStatusOutcomeOptions(
                      leftAfterSecond,
                      rightAfterSecond,
                      order.second.side,
                      order.secondMove,
                      secondAction.action,
                      false
                    )
                  : [
                      {
                        leftPokemon: leftAfterSecond,
                        rightPokemon: rightAfterSecond,
                        action: secondAction.action,
                        probability: 1,
                      },
                    ];

              for (const secondStatusOutcome of secondStatusOutcomes) {
                const secondResolvedProbability =
                  secondProbability * secondStatusOutcome.probability;

                if (secondResolvedProbability <= 0) {
                  continue;
                }

                const leftAfterSecondStatuses = cloneBattlePokemon(secondStatusOutcome.leftPokemon);
                const rightAfterSecondStatuses = cloneBattlePokemon(secondStatusOutcome.rightPokemon);
                const actions = [...firstActions, secondStatusOutcome.action];

                let winner: SimulationSide | null = null;
                if (secondAction.result === "doubleFainted") {
                  winner = "draw";
                } else if (secondAction.result === "defenderFainted") {
                  winner = order.second.side;
                } else if (secondAction.result === "attackerFainted") {
                  winner = order.first.side;
                } else {
                  winner = applyRepresentativeEndOfTurnEffects(
                    leftAfterSecondStatuses,
                    rightAfterSecondStatuses
                  );
                }

                expandedBranches.push({
                  leftPokemon: leftAfterSecondStatuses,
                  rightPokemon: rightAfterSecondStatuses,
                  turns: [
                    ...branch.turns,
                    buildProjectedTurn(
                      turn,
                      order,
                      actions,
                      leftAfterSecondStatuses,
                      rightAfterSecondStatuses
                    ),
                  ],
                  probability: secondResolvedProbability,
                  winner,
                });
              }
            }
          }
        }
  }

  return expandedBranches;
};

export const buildStrongestWinningPlans = (
  left: SimulatorInput,
  right: SimulatorInput,
  moveMap: Map<string, SimMove>,
  level = DEFAULT_LEVEL,
  maxTurns?: number
): ProjectedWinningPlans => {
  clearAiCaches();

  const initialLeftPokemon = buildBattlePokemon("left", left, moveMap, level);
  const initialRightPokemon = buildBattlePokemon("right", right, moveMap, level);
  const effectiveMaxTurns =
    maxTurns ??
    (shouldUseExpandedProjectedMoveSearch(initialLeftPokemon, initialRightPokemon) ? 8 : 5);

  let branches: ProjectedBranchState[] = [
    {
      leftPokemon: initialLeftPokemon,
      rightPokemon: initialRightPokemon,
      turns: [],
      probability: 1,
      winner: null,
    },
  ];

  for (let turn = 1; turn <= effectiveMaxTurns; turn += 1) {
    branches = branches
      .flatMap((branch) => (branch.winner ? [branch] : expandProjectedBranch(branch, turn)))
      .sort((a, b) => b.probability - a.probability)
      .slice(0, MAX_PROJECTED_BRANCHES);

    if (branches.every((branch) => branch.winner)) {
      break;
    }
  }

  const toProjectedPlan = (branch: ProjectedBranchState): ProjectedBattlePlan => ({
    turns: branch.turns,
    winner: branch.winner,
    probability: branch.probability,
  });

  const strongestLeft = branches
    .filter((branch) => branch.winner === "left")
    .sort((a, b) => b.probability - a.probability)[0];
  const strongestRight = branches
    .filter((branch) => branch.winner === "right")
    .sort((a, b) => b.probability - a.probability)[0];

  return {
    left: strongestLeft ? toProjectedPlan(strongestLeft) : null,
    right: strongestRight ? toProjectedPlan(strongestRight) : null,
  };
};

export const buildLikelyBattlePlan = (
  left: SimulatorInput,
  right: SimulatorInput,
  moveMap: Map<string, SimMove>,
  level = DEFAULT_LEVEL,
  maxTurns = 5
): ProjectedBattlePlan => {
  const strongestPlans = buildStrongestWinningPlans(left, right, moveMap, level, maxTurns);

  if (strongestPlans.left && strongestPlans.right) {
    return strongestPlans.left.probability >= strongestPlans.right.probability
      ? strongestPlans.left
      : strongestPlans.right;
  }

  return (
    strongestPlans.left ??
    strongestPlans.right ?? {
      turns: [],
      winner: "draw",
      probability: 0,
    }
  );
};
