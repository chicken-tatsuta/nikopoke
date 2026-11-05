export const BATTLE_STAGE_STATS = ["atk", "def", "spa", "spd", "spe"] as const;

export type BattleStageStat = (typeof BATTLE_STAGE_STATS)[number];
export type BattleStages = Record<BattleStageStat, number>;

export const EMPTY_BATTLE_STAGES: BattleStages = {
  atk: 0,
  def: 0,
  spa: 0,
  spd: 0,
  spe: 0,
};

export const BATTLE_STAGE_LABELS: Record<BattleStageStat, string> = {
  atk: "A",
  def: "B",
  spa: "C",
  spd: "D",
  spe: "S",
};

export const clampBattleStage = (stage: number) => Math.max(-6, Math.min(6, stage));

export const getStageMultiplier = (stage: number) => {
  const normalizedStage = clampBattleStage(stage);
  return normalizedStage >= 0
    ? (2 + normalizedStage) / 2
    : 2 / (2 - normalizedStage);
};

export const applyStageMultiplier = (value: number, stage: number) =>
  Math.floor(value * getStageMultiplier(stage));

export const getEffectiveSpeed = (
  baseSpeed: number,
  itemId: string | undefined,
  speedStage: number,
  paralyzed = false
) => {
  let effectiveSpeed = applyStageMultiplier(baseSpeed, speedStage);

  if (itemId === "choiceScarf") {
    effectiveSpeed = Math.floor(effectiveSpeed * 1.5);
  }

  if (paralyzed) {
    effectiveSpeed = Math.floor(effectiveSpeed * 0.5);
  }

  return effectiveSpeed;
};

export const applyBattleStageChanges = (
  stages: BattleStages,
  changes?: Partial<Record<BattleStageStat, number>>
) => {
  const nextStages: BattleStages = { ...stages };
  const appliedChanges: Partial<Record<BattleStageStat, number>> = {};

  if (!changes) {
    return { nextStages, appliedChanges };
  }

  for (const stat of BATTLE_STAGE_STATS) {
    const delta = changes[stat];
    if (!delta) {
      continue;
    }

    const nextStage = clampBattleStage(nextStages[stat] + delta);
    const appliedDelta = nextStage - nextStages[stat];

    if (appliedDelta === 0) {
      continue;
    }

    nextStages[stat] = nextStage;
    appliedChanges[stat] = appliedDelta;
  }

  return { nextStages, appliedChanges };
};

export const formatBattleStageChange = (
  stat: BattleStageStat,
  delta: number
) => `${BATTLE_STAGE_LABELS[stat]}${delta > 0 ? `+${delta}` : `${delta}`}`;
