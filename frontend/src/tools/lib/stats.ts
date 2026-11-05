// src/lib/stats.ts

// ステータスのキー
export type StatKey = "hp" | "atk" | "def" | "spa" | "spd" | "spe";

// 種族値・努力値・個体値の型
export type BaseStats = Record<StatKey, number>;
export type EVs = Record<StatKey, number>;
export type IVs = Record<StatKey, number>;

// 性格：上がるステータスと下がるステータスを持つ
export type Nature = {
  name: string;
  increased?: Exclude<StatKey, "hp">;
  decreased?: Exclude<StatKey, "hp">;
};

// 性格補正の倍率を返す（1.1 / 0.9 / 1.0）
export const getNatureMultiplier = (
  stat: StatKey,
  nature?: Nature
): number => {
  if (!nature || stat === "hp") return 1.0;
  if (stat === nature.increased) return 1.1;
  if (stat === nature.decreased) return 0.9;
  return 1.0;
};

// 個別ステータス1つを計算
export const calcStat = (
  base: number,
  iv: number,
  ev: number,
  level: number,
  stat: StatKey,
  nature?: Nature
): number => {
  if (stat === "hp") {
    // HP の計算式
    return (
      Math.floor(((base * 2 + iv) * level) / 100) + Math.min(32, Math.max(0, ev)) +
      level +
      10
    );
  } else {
    // 攻撃・防御など
    const natureMult = getNatureMultiplier(stat, nature);
    const raw =
      Math.floor(((base * 2 + iv) * level) / 100) + 5 + Math.min(32, Math.max(0, ev));
    return Math.floor(raw * natureMult);
  }
};

// 全ステータスまとめて計算
export const calcAllStats = (
  baseStats: BaseStats,
  evs: EVs,
  ivs: IVs,
  level: number,
  nature?: Nature
): Record<StatKey, number> => {
  const result: Partial<Record<StatKey, number>> = {};
  (Object.keys(baseStats) as StatKey[]).forEach((key) => {
    result[key] = calcStat(
      baseStats[key],
      ivs[key],
      evs[key],
      level,
      key,
      nature
    );
  });
  return result as Record<StatKey, number>;
};
