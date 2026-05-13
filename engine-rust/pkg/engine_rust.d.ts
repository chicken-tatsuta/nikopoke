/* tslint:disable */
/* eslint-disable */

export function createBattleState(players: any): any;

export function createCreature(species_id: string, options: any): any;

export function getBestMoveMCTS(state: any, player_id: string, iterations: number): any;

export function getBestMoveMinimax(state: any, player_id: string, depth: number): any;

export function getBestMoveVega(state: any, player_id: string, depth: number): any;

export function getBestMoveVegaIterative(state: any, player_id: string, max_depth: number, node_budget: number): any;

export function getBestMoveVegaWithBranch(state: any, player_id: string, depth: number, branch_limit: number): any;

export function isBattleOver(state: any): boolean;

export function replaceFaintedPokemon(state: any, player_id: string, slot: number): any;

export function stepBattle(state: any, actions: any, options: any): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly createBattleState: (a: any) => [number, number, number];
  readonly createCreature: (a: number, b: number, c: any) => [number, number, number];
  readonly getBestMoveMCTS: (a: any, b: number, c: number, d: number) => [number, number, number];
  readonly getBestMoveMinimax: (a: any, b: number, c: number, d: number) => [number, number, number];
  readonly getBestMoveVega: (a: any, b: number, c: number, d: number) => [number, number, number];
  readonly getBestMoveVegaIterative: (a: any, b: number, c: number, d: number, e: number) => [number, number, number];
  readonly getBestMoveVegaWithBranch: (a: any, b: number, c: number, d: number, e: number) => [number, number, number];
  readonly isBattleOver: (a: any) => [number, number, number];
  readonly replaceFaintedPokemon: (a: any, b: number, c: number, d: number) => [number, number, number];
  readonly stepBattle: (a: any, b: any, c: any) => [number, number, number];
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
