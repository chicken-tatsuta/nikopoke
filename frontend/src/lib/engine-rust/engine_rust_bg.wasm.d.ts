/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const createBattleState: (a: any) => [number, number, number];
export const createCreature: (a: number, b: number, c: any) => [number, number, number];
export const getBestMoveMCTS: (a: any, b: number, c: number, d: number) => [number, number, number];
export const getBestMoveMinimax: (a: any, b: number, c: number, d: number) => [number, number, number];
export const getBestMoveVega: (a: any, b: number, c: number, d: number) => [number, number, number];
export const getBestMoveVegaIterative: (a: any, b: number, c: number, d: number, e: number) => [number, number, number];
export const getBestMoveVegaWithBranch: (a: any, b: number, c: number, d: number, e: number) => [number, number, number];
export const isBattleOver: (a: any) => [number, number, number];
export const replaceFaintedPokemon: (a: any, b: number, c: number, d: number) => [number, number, number];
export const stepBattle: (a: any, b: any, c: any) => [number, number, number];
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_exn_store: (a: number) => void;
export const __externref_table_alloc: () => number;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __externref_table_dealloc: (a: number) => void;
export const __wbindgen_start: () => void;
