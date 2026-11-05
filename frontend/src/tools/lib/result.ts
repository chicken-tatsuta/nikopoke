import type { SimulationBattleResult, SimulationSide } from "./simulator";

type TurnMoveRate = {
  turn: number;
  totalSelections: number;
  moves: Array<{
    moveId: string;
    count: number;
    rate: number;
  }>;
};

type SideSummary = {
  winRate: number;
  averageRemainingHp: number;
  averageRemainingHpPercent: number;
  averageRemainingHpPercentWhenWin: number;
  firstMoveRate: number;
  speedTieLeadRate: number;
  speedTieWinRate: number;
  averageKoHits: number;
  winWhenMovingFirst: number;
  winWhenMovingSecond: number;
  moveUsageRates: Array<{
    moveId: string;
    count: number;
    rate: number;
  }>;
  moveKoRates: Array<{
    moveId: string;
    kos: number;
    koRate: number;
  }>;
  turnMoveRates: TurnMoveRate[];
};

export type SimulationSummary = {
  totalBattles: number;
  leftWins: number;
  rightWins: number;
  draws: number;
  leftWinRate: number;
  rightWinRate: number;
  averageTurns: number;
  left: SideSummary;
  right: SideSummary;
};

const countWins = (
  battles: SimulationBattleResult[],
  winner: SimulationSide
): number => {
  return battles.filter((battle) => battle.winner === winner).length;
};

const summarizeTurnMoveRates = (
  battles: SimulationBattleResult[],
  side: "left" | "right"
): TurnMoveRate[] => {
  const countsByTurn = battles.reduce<
    Map<number, { totalSelections: number; moveCounts: Record<string, number> }>
  >((acc, battle) => {
    battle.turnMoveHistory[side].forEach((moveId, index) => {
      if (!moveId) {
        return;
      }

      const turn = index + 1;
      const turnEntry =
        acc.get(turn) ??
        {
          totalSelections: 0,
          moveCounts: {},
        };

      turnEntry.totalSelections += 1;
      turnEntry.moveCounts[moveId] = (turnEntry.moveCounts[moveId] ?? 0) + 1;
      acc.set(turn, turnEntry);
    });

    return acc;
  }, new Map());

  return Array.from(countsByTurn.entries())
    .sort(([leftTurn], [rightTurn]) => leftTurn - rightTurn)
    .map(([turn, { totalSelections, moveCounts }]) => ({
      turn,
      totalSelections,
      moves: Object.entries(moveCounts)
        .map(([moveId, count]) => ({
          moveId,
          count,
          rate: totalSelections === 0 ? 0 : (count / totalSelections) * 100,
        }))
        .sort((a, b) => b.count - a.count),
    }));
};

export const summarizeSimulationResults = (
  battles: SimulationBattleResult[]
): SimulationSummary => {
  const totalBattles = battles.length;

  if (totalBattles === 0) {
    return {
      totalBattles: 0,
      leftWins: 0,
      rightWins: 0,
      draws: 0,
      leftWinRate: 0,
      rightWinRate: 0,
      averageTurns: 0,
      left: {
        winRate: 0,
        averageRemainingHp: 0,
        averageRemainingHpPercent: 0,
        averageRemainingHpPercentWhenWin: 0,
        firstMoveRate: 0,
        speedTieLeadRate: 0,
        speedTieWinRate: 0,
        averageKoHits: 0,
        winWhenMovingFirst: 0,
        winWhenMovingSecond: 0,
        moveUsageRates: [],
        moveKoRates: [],
        turnMoveRates: [],
      },
      right: {
        winRate: 0,
        averageRemainingHp: 0,
        averageRemainingHpPercent: 0,
        averageRemainingHpPercentWhenWin: 0,
        firstMoveRate: 0,
        speedTieLeadRate: 0,
        speedTieWinRate: 0,
        averageKoHits: 0,
        winWhenMovingFirst: 0,
        winWhenMovingSecond: 0,
        moveUsageRates: [],
        moveKoRates: [],
        turnMoveRates: [],
      },
    };
  }

  const leftWins = countWins(battles, "left");
  const rightWins = countWins(battles, "right");
  const draws = countWins(battles, "draw");
  const averageTurns =
    battles.reduce((sum, battle) => sum + battle.turns, 0) / totalBattles;
  const summarizeSide = (side: "left" | "right", wins: number): SideSummary => {
    const openingFirstCount = battles.filter(
      (battle) => battle.openingFirstSide === side
    ).length;
    const totalRemainingHp = battles.reduce((sum, battle) => sum + battle.endingHp[side], 0);
    const totalRemainingHpPercent = battles.reduce((sum, battle) => sum + battle.endingHpPercent[side], 0);
    const winBattles = battles.filter((battle) => battle.winner === side);
    const averageRemainingHpPercentWhenWin =
      winBattles.length === 0
        ? 0
        : winBattles.reduce((sum, battle) => sum + battle.endingHpPercent[side], 0) / winBattles.length;
    const speedTieBattles = battles.filter((battle) => battle.openingOrderReason === "speedTie");
    const speedTieLeads = speedTieBattles.filter((battle) => battle.openingFirstSide === side).length;
    const speedTieWins = speedTieBattles.filter((battle) => battle.winner === side).length;
    const koHits = battles
      .map((battle) => battle.koHitCount[side])
      .filter((value): value is number => typeof value === "number");
    const totalKoHits = koHits.reduce((sum, value) => sum + value, 0);
    const moveCounts = battles.reduce<Record<string, number>>((acc, battle) => {
      const entries = Object.entries(battle.moveUsage[side]);
      for (const [moveId, count] of entries) {
        acc[moveId] = (acc[moveId] ?? 0) + count;
      }
      return acc;
    }, {});
    const totalMoveUses = Object.values(moveCounts).reduce((sum, value) => sum + value, 0);
    const moveKoCounts = battles.reduce<Record<string, number>>((acc, battle) => {
      const entries = Object.entries(battle.moveKoCounts[side]);
      for (const [moveId, count] of entries) {
        acc[moveId] = (acc[moveId] ?? 0) + count;
      }
      return acc;
    }, {});
    const totalKos = Object.values(moveKoCounts).reduce((sum, value) => sum + value, 0);
    const moveUsageRates = Object.entries(moveCounts)
      .map(([moveId, count]) => ({
        moveId,
        count,
        rate: totalMoveUses === 0 ? 0 : (count / totalMoveUses) * 100,
      }))
      .sort((a, b) => b.count - a.count);
    const moveKoRates = Object.entries(moveKoCounts)
      .map(([moveId, kos]) => ({
        moveId,
        kos,
        koRate: totalKos === 0 ? 0 : (kos / totalKos) * 100,
      }))
      .sort((a, b) => b.kos - a.kos);
    const winWhenMovingFirst = battles.filter(
      (battle) => battle.winner === side && battle.openingFirstSide === side
    ).length;
    const winWhenMovingSecond = battles.filter(
      (battle) => battle.winner === side && battle.openingFirstSide !== side
    ).length;

    return {
      winRate: (wins / totalBattles) * 100,
      averageRemainingHp: totalRemainingHp / totalBattles,
      averageRemainingHpPercent: totalRemainingHpPercent / totalBattles,
      averageRemainingHpPercentWhenWin,
      firstMoveRate: totalBattles === 0 ? 0 : (openingFirstCount / totalBattles) * 100,
      speedTieLeadRate: speedTieBattles.length === 0 ? 0 : (speedTieLeads / speedTieBattles.length) * 100,
      speedTieWinRate: speedTieBattles.length === 0 ? 0 : (speedTieWins / speedTieBattles.length) * 100,
      averageKoHits: koHits.length === 0 ? 0 : totalKoHits / koHits.length,
      winWhenMovingFirst,
      winWhenMovingSecond,
      moveUsageRates,
      moveKoRates,
      turnMoveRates: summarizeTurnMoveRates(battles, side),
    };
  };

  return {
    totalBattles,
    leftWins,
    rightWins,
    draws,
    leftWinRate: (leftWins / totalBattles) * 100,
    rightWinRate: (rightWins / totalBattles) * 100,
    averageTurns,
    left: summarizeSide("left", leftWins),
    right: summarizeSide("right", rightWins),
  };
};
