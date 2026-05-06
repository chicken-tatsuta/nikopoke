export type PokemonUsageStats = {
    speciesId: string;
    used: number;
    wins: number;
    losses: number;
    winRate: number;
    moves: {
        name: string;
        rate: number;
    }[];
};
