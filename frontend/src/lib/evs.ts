import type { EVStats } from '../types/pokemon';

export const EV_STAT_MAX = 32;
export const EV_TOTAL_MAX = 66;
export const EV_KEYS = ['hp', 'atk', 'def', 'spa', 'spd', 'spe'] as const;

export const EMPTY_EVS: EVStats = {
    hp: 0,
    atk: 0,
    def: 0,
    spa: 0,
    spd: 0,
    spe: 0,
};

export function evTotal(evs: Partial<EVStats> | null | undefined): number {
    return EV_KEYS.reduce((total, key) => total + Math.max(0, Math.floor(evs?.[key] ?? 0)), 0);
}

export function normalizeEvs(evs: Partial<EVStats> | null | undefined): EVStats {
    const rawValues = EV_KEYS.map((key) => Math.max(0, Math.floor(evs?.[key] ?? 0)));
    const isLegacyScale = rawValues.some((value) => value > EV_STAT_MAX);
    const normalizedValues = rawValues.map((value) => {
        const convertedValue = isLegacyScale ? Math.round(value / 8) : value;
        return Math.min(EV_STAT_MAX, Math.max(0, convertedValue));
    });

    let overflow = normalizedValues.reduce((sum, value) => sum + value, 0) - EV_TOTAL_MAX;
    while (overflow > 0) {
        const largestIndex = normalizedValues.reduce((largest, value, index) => (
            value > normalizedValues[largest] ? index : largest
        ), 0);
        if (normalizedValues[largestIndex] <= 0) break;
        normalizedValues[largestIndex] -= 1;
        overflow -= 1;
    }

    return EV_KEYS.reduce<EVStats>((result, key, index) => ({
        ...result,
        [key]: normalizedValues[index],
    }), { ...EMPTY_EVS });
}
