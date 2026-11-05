import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
import { canonicalizeMoveId, loadAllData } from '../lib/data';
import { getPokemonPreset, resolvePresetMoveIds } from '../lib/pokemonPresets';
import type { DeckPokemon, ItemData, Learnset, MoveData, Species, SpeciesData } from '../types/pokemon';

type Direction = 'up' | 'down' | 'left' | 'right';
type Position = { x: number; y: number };

type Npc = {
    id: string;
    name: string;
    x: number;
    y: number;
    color: string;
    dialogue: string[];
};

type WildEncounter = {
    lead: Species;
    pokemon: DeckPokemon;
};

const TILE_SIZE = 40;
const MAP_WIDTH = 16;
const MAP_HEIGHT = 10;
const ENCOUNTER_RATE = 0.22;

const npcList: Npc[] = [
    {
        id: 'mentor',
        name: '悠斗',
        x: 8,
        y: 1,
        color: '#9db39a',
        dialogue: [
            '草むらは危ないぞ。急にニキダンが飛び出してくる。',
            'でもバトル自体はいつものポケモン対戦画面につながるようにしておいた。',
        ],
    },
    {
        id: 'friend',
        name: '春樹',
        x: 9,
        y: 1,
        color: '#a7bea4',
        dialogue: [
            '草の上を歩くとエンカウントするよ。',
            'デッキを作ってあるなら、その手持ちでそのまま戦える。',
        ],
    },
];

const blockedTiles = new Set([
    '0,0', '1,0', '2,0', '3,0', '4,0',
    '0,1', '1,1', '2,1', '3,1', '4,1',
    '0,2', '1,2', '2,2', '3,2', '4,2',
    '13,0', '14,0', '15,0',
    '13,1', '14,1', '15,1',
    '13,2', '14,2', '15,2',
    '13,3', '14,3', '15,3',
    '1,8', '2,8', '3,8',
    '1,9', '2,9', '3,9',
    '11,1', '11,2', '11,3', '12,1', '12,2', '12,3',
]);

const pathTiles = new Set([
    '6,0', '6,1', '6,2', '6,3', '6,4', '6,5', '6,6', '6,7',
    '7,0', '7,1', '7,2', '7,3', '7,4', '7,5', '7,6', '7,7',
    '4,5', '5,5', '6,5', '7,5', '8,5', '9,5', '10,5', '11,5', '12,5', '13,5', '14,5', '15,5',
    '3,4', '3,5', '3,6',
]);

const hedgeTiles = new Set([
    '10,1', '10,2', '10,3', '10,4',
    '11,4', '12,4',
    '8,8', '9,8', '10,8', '11,8',
]);

const tallGrassTiles = new Set([
    '8,2', '9,2', '10,2',
    '8,3', '9,3', '10,3',
    '8,4', '9,4', '10,4',
    '12,7', '13,7', '14,7',
    '12,8', '13,8', '14,8',
]);

const crystalTile = { x: 7, y: 1 };
const playerStart = { x: 7, y: 3 };
const fallbackDeckSpeciesIds = ['eiraku', 'tatuta', 'morimitu', 'takaho', 'ume', 'machida'];
const wildSpeciesPool = ['haruta', 'macchan', 'michii', 'nisiki', 'sena', 'ikkun', 'futo', 'makocchan', 'reosan', 'ayuma'];

function isSamePosition(a: Position, b: Position) {
    return a.x === b.x && a.y === b.y;
}

function directionVector(direction: Direction) {
    switch (direction) {
        case 'up':
            return { x: 0, y: -1 };
        case 'down':
            return { x: 0, y: 1 };
        case 'left':
            return { x: -1, y: 0 };
        case 'right':
            return { x: 1, y: 0 };
    }
}

function tileKey(x: number, y: number) {
    return `${x},${y}`;
}

function getTileType(x: number, y: number) {
    const key = tileKey(x, y);

    if (pathTiles.has(key)) return 'path';
    if (tallGrassTiles.has(key)) return 'tall-grass';
    if (hedgeTiles.has(key)) return 'hedge';
    if (blockedTiles.has(key)) return 'wall';
    return 'grass';
}

function uniqueRandomItems<T>(items: T[], count: number): T[] {
    return [...items].sort(() => Math.random() - 0.5).slice(0, count);
}

function readLocalDeck(): DeckPokemon[] | null {
    const saved = localStorage.getItem('savedDeck');
    if (!saved) {
        return null;
    }

    try {
        return JSON.parse(saved) as DeckPokemon[];
    } catch {
        return null;
    }
}

function sanitizeDeckPokemon(
    pokemon: DeckPokemon,
    species: SpeciesData,
    moves: MoveData,
    items: ItemData,
    learnsets: Learnset,
    moveIdMigrations: Map<string, string>,
): DeckPokemon | null {
    const mon = species[pokemon.speciesId];
    if (!mon) {
        return null;
    }

    const sanitizedMoves = pokemon.moves
        .map((moveId) => canonicalizeMoveId(moveId, moves, moveIdMigrations))
        .filter((moveId, index, self) => self.indexOf(moveId) === index)
        .filter((moveId) => moves[moveId])
        .slice(0, 4);

    const fallbackMoves = (learnsets[pokemon.speciesId] ?? []).filter((moveId) => moves[moveId]).slice(0, 4);

    return {
        ...pokemon,
        ability: mon.abilities.includes(pokemon.ability) ? pokemon.ability : (mon.abilities[0] ?? 'none'),
        item: pokemon.item && items[pokemon.item] ? pokemon.item : null,
        moves: sanitizedMoves.length > 0 ? sanitizedMoves : fallbackMoves,
    };
}

function sanitizeDeck(
    deck: DeckPokemon[] | null | undefined,
    species: SpeciesData,
    moves: MoveData,
    items: ItemData,
    learnsets: Learnset,
    moveIdMigrations: Map<string, string>,
): DeckPokemon[] {
    if (!deck) {
        return [];
    }

    const usedItems = new Set<string>();

    return deck
        .map((pokemon) => sanitizeDeckPokemon(pokemon, species, moves, items, learnsets, moveIdMigrations))
        .filter((pokemon): pokemon is DeckPokemon => Boolean(pokemon))
        .map((pokemon) => {
            if (!pokemon.item) {
                return pokemon;
            }
            if (pokemon.item !== 'kamidama' && usedItems.has(pokemon.item)) {
                return { ...pokemon, item: null };
            }
            usedItems.add(pokemon.item);
            return pokemon;
        })
        .slice(0, 6);
}

function buildPokemonFromSpecies(
    speciesId: string,
    loadedSpecies: SpeciesData,
    loadedMoves: MoveData,
    loadedLearnsets: Learnset,
    moveIdMigrations: Map<string, string>,
): DeckPokemon | null {
    const mon = loadedSpecies[speciesId];
    if (!mon) {
        return null;
    }

    const learnset = loadedLearnsets[speciesId] ?? [];
    const preset = getPokemonPreset(speciesId);
    const moveIds = resolvePresetMoveIds(preset, loadedMoves, learnset, moveIdMigrations);

    return {
        speciesId,
        moves: moveIds.slice(0, 4),
        ability: preset?.abilityId && mon.abilities.includes(preset.abilityId)
            ? preset.abilityId
            : (mon.abilities[0] ?? 'none'),
        item: preset?.itemId ?? null,
        evs: preset?.evs,
    };
}

export default function RpgPage() {
    const navigate = useNavigate();
    const { profile } = useAuth();
    const [player, setPlayer] = useState<Position>(playerStart);
    const [direction, setDirection] = useState<Direction>('up');
    const [messageIndex, setMessageIndex] = useState(0);
    const [messageQueue, setMessageQueue] = useState<string[]>([
        '矢印キーで移動。X または Enter で調べる。',
        '草むらを歩くと、野生のニキダンが飛び出してきます。',
    ]);
    const [savePromptOpen, setSavePromptOpen] = useState(false);
    const [species, setSpecies] = useState<SpeciesData>({});
    const [moves, setMoves] = useState<MoveData>({});
    const [items, setItems] = useState<ItemData>({});
    const [learnsets, setLearnsets] = useState<Learnset>({});
    const [moveIdMigrations, setMoveIdMigrations] = useState<Map<string, string>>(new Map());
    const [dataReady, setDataReady] = useState(false);
    const [wildEncounter, setWildEncounter] = useState<WildEncounter | null>(null);

    const currentMessage = useMemo(() => {
        return messageQueue[messageIndex] ?? '';
    }, [messageIndex, messageQueue]);

    const interactiveTarget = useMemo(() => {
        const vector = directionVector(direction);
        const target = { x: player.x + vector.x, y: player.y + vector.y };

        const npc = npcList.find((entry) => entry.x === target.x && entry.y === target.y);
        if (npc) {
            return { type: 'npc' as const, npc };
        }

        if (isSamePosition(target, crystalTile)) {
            return { type: 'crystal' as const };
        }

        return null;
    }, [direction, player]);

    useEffect(() => {
        loadAllData()
            .then((data) => {
                setSpecies(data.species);
                setMoves(data.moves);
                setItems(data.items);
                setLearnsets(data.learnsets);
                setMoveIdMigrations(data.moveIdMigrations);
                setDataReady(true);
            })
            .catch((error) => {
                console.error('Failed to load RPG data:', error);
            });
    }, []);

    const openDialogue = (lines: string[]) => {
        setMessageQueue(lines);
        setMessageIndex(0);
    };

    const triggerWildEncounter = () => {
        if (!dataReady) {
            return;
        }

        const candidates = uniqueRandomItems(wildSpeciesPool, 1)
            .map((speciesId) => buildPokemonFromSpecies(speciesId, species, moves, learnsets, moveIdMigrations))
            .filter((pokemon): pokemon is DeckPokemon => Boolean(pokemon));

        if (candidates.length < 1) {
            return;
        }

        const leadSpecies = species[candidates[0].speciesId];
        if (!leadSpecies) {
            return;
        }

        setWildEncounter({
            lead: leadSpecies,
            pokemon: candidates[0],
        });
        openDialogue([
            `あっ！ やせいの ${leadSpecies.name} が とびだしてきた！`,
            'バトルをはじめる を押すと、既存のポケモンバトル画面に移動します。',
        ]);
    };

    useEffect(() => {
        const handleKeyDown = (event: KeyboardEvent) => {
            if (savePromptOpen || wildEncounter) {
                if (event.key === 'Escape') {
                    setSavePromptOpen(false);
                    setWildEncounter(null);
                }
                return;
            }

            if (event.key === 'Enter' || event.key.toLowerCase() === 'x') {
                event.preventDefault();
                handleInspect();
                return;
            }

            const keyToDirection: Record<string, Direction> = {
                ArrowUp: 'up',
                ArrowDown: 'down',
                ArrowLeft: 'left',
                ArrowRight: 'right',
            };

            const nextDirection = keyToDirection[event.key];
            if (!nextDirection) {
                return;
            }

            event.preventDefault();
            setDirection(nextDirection);

            const vector = directionVector(nextDirection);
            const nextPosition = { x: player.x + vector.x, y: player.y + vector.y };
            const key = tileKey(nextPosition.x, nextPosition.y);
            const occupiedByNpc = npcList.some((npc) => npc.x === nextPosition.x && npc.y === nextPosition.y);
            const outOfBounds =
                nextPosition.x < 0 ||
                nextPosition.x >= MAP_WIDTH ||
                nextPosition.y < 0 ||
                nextPosition.y >= MAP_HEIGHT;

            if (outOfBounds || blockedTiles.has(key) || occupiedByNpc) {
                return;
            }

            setPlayer(nextPosition);

            if (tallGrassTiles.has(key) && Math.random() < ENCOUNTER_RATE) {
                window.setTimeout(() => {
                    triggerWildEncounter();
                }, 80);
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [dataReady, direction, learnsets, moveIdMigrations, moves, player, savePromptOpen, species, wildEncounter]);

    const handleInspect = () => {
        if (interactiveTarget?.type === 'npc') {
            openDialogue(interactiveTarget.npc.dialogue);
            return;
        }

        if (interactiveTarget?.type === 'crystal') {
            setSavePromptOpen(true);
            openDialogue(['セーブしますか？']);
            return;
        }

        if (tallGrassTiles.has(tileKey(player.x, player.y))) {
            openDialogue(['草むらが さわさわ している。', 'この中では野生のニキダンと出会うことがある。']);
            return;
        }

        openDialogue(['そのまま先へ進んだ。', '次はマップ切り替えや町の出口も足せます。']);
    };

    const advanceMessage = () => {
        setMessageIndex((prev) => {
            if (prev >= messageQueue.length - 1) {
                return prev;
            }
            return prev + 1;
        });
    };

    const performSave = () => {
        setSavePromptOpen(false);
        openDialogue(['レポートを しっかり かきのこした！', '草むらに入れば、そのままポケモンバトルにつながる。']);
    };

    const startWildBattle = () => {
        if (!wildEncounter || !dataReady) {
            return;
        }

        const savedDeck = sanitizeDeck(
            profile?.current_deck ?? readLocalDeck(),
            species,
            moves,
            items,
            learnsets,
            moveIdMigrations,
        );

        const fallbackDeck = fallbackDeckSpeciesIds
            .map((speciesId) => buildPokemonFromSpecies(speciesId, species, moves, learnsets, moveIdMigrations))
            .filter((pokemon): pokemon is DeckPokemon => Boolean(pokemon));

        const playerDeck = (savedDeck.length >= 3 ? savedDeck : fallbackDeck).slice(0, 6);
        if (playerDeck.length < 3) {
            openDialogue(['バトルを始めるための手持ちが足りません。', '先にデッキを作るか、データを確認してください。']);
            setWildEncounter(null);
            return;
        }

        sessionStorage.setItem('battleMode', 'ai');
        sessionStorage.setItem('aiLevel', 'lv1');
        sessionStorage.setItem('playerDeck', JSON.stringify(playerDeck));
        sessionStorage.setItem('selectedPlayerDeck', JSON.stringify(playerDeck.slice(0, 3)));
        sessionStorage.setItem('selectedOpponentDeck', JSON.stringify([wildEncounter.pokemon]));
        sessionStorage.setItem('opponentPreviewDeck', JSON.stringify([wildEncounter.pokemon]));
        sessionStorage.setItem('rpgBattleKind', 'wild');
        navigate('/battle');
    };

    return (
        <div className="min-h-dvh bg-[#dfeedd] px-4 py-6 text-[#46624a]">
            <div className="mx-auto max-w-6xl">
                <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
                    <div>
                        <p className="text-xs font-bold tracking-[0.24em] text-[#6d8a72]">NIKIDAN RPG VIEW</p>
                        <h1 className="text-2xl font-black tracking-[0.08em]">草むらでエンカウントする町</h1>
                    </div>
                    <div className="flex flex-wrap gap-2">
                        <Link
                            to="/home"
                            className="rounded-full bg-[#cfe3cd] px-4 py-2 text-sm font-bold text-[#46624a] shadow-[0_6px_20px_rgba(92,132,98,0.12)]"
                        >
                            ホームへ
                        </Link>
                        <Link
                            to="/deck-builder?mode=ai"
                            className="rounded-full bg-[#b9d8b5] px-4 py-2 text-sm font-bold text-[#46624a] shadow-[0_6px_20px_rgba(92,132,98,0.12)]"
                        >
                            手持ちを作る
                        </Link>
                    </div>
                </div>

                <div className="mx-auto max-w-[860px]">
                    <section className="overflow-hidden rounded-[28px] bg-[#cfe3cd] p-4 shadow-[0_24px_80px_rgba(92,132,98,0.14)]">
                        <div
                            className="relative overflow-hidden rounded-[20px] bg-[#b8dcae]"
                            style={{
                                width: MAP_WIDTH * TILE_SIZE,
                                height: MAP_HEIGHT * TILE_SIZE,
                                maxWidth: '100%',
                            }}
                        >
                            {Array.from({ length: MAP_HEIGHT }).map((_, y) =>
                                Array.from({ length: MAP_WIDTH }).map((__, x) => (
                                    <Tile key={`${x}-${y}`} x={x} y={y} />
                                )),
                            )}

                            <Building x={0} y={0} width={5} height={3} roof="#a8cd68" />
                            <Building x={13} y={0} width={3} height={4} roof="#a8cd68" />
                            <Building x={1} y={8} width={3} height={2} roof="#88a35c" />

                            <Barrel x={1} y={4} />
                            <Crystal x={crystalTile.x} y={crystalTile.y} />

                            {npcList.map((npc) => (
                                <Character key={npc.id} x={npc.x} y={npc.y} color={npc.color} />
                            ))}

                            <Character x={player.x} y={player.y} color="#8fa78b" facing={direction} player />
                        </div>

                        <div className="mt-4 rounded-[22px] bg-[#b9d8b5] px-5 py-4 text-[#46624a]">
                            <div className="mb-2 flex items-center justify-between gap-4">
                                <span className="text-sm font-bold tracking-[0.16em] text-[#537257]">メッセージ</span>
                                <span className="text-xs text-[#739176]">Enter で調べる / クリックで次へ</span>
                            </div>
                            <button
                                type="button"
                                onClick={advanceMessage}
                                className="min-h-24 w-full text-left text-lg leading-8 text-[#46624a] outline-none"
                            >
                                {currentMessage}
                            </button>
                        </div>
                    </section>
                </div>
            </div>

            {savePromptOpen ? (
                <div className="fixed inset-0 z-50 grid place-items-center bg-[#dfeedd]/75 p-4 backdrop-blur-sm">
                    <div className="w-full max-w-md rounded-[24px] bg-[#cfe3cd] p-6 text-[#46624a] shadow-[0_24px_80px_rgba(92,132,98,0.18)]">
                        <h2 className="text-xl font-black">セーブしますか？</h2>
                        <p className="mt-3 text-sm leading-7 text-[#6d8a72]">
                            現在の進行をレポートとして保存する想定のUIです。
                        </p>
                        <div className="mt-5 flex gap-3">
                            <button
                                type="button"
                                onClick={performSave}
                                className="rounded-full bg-[#b9d8b5] px-4 py-2 text-sm font-bold text-[#46624a]"
                            >
                                はい
                            </button>
                            <button
                                type="button"
                                onClick={() => setSavePromptOpen(false)}
                                className="rounded-full bg-[#d7e8d4] px-4 py-2 text-sm font-bold text-[#46624a]"
                            >
                                いいえ
                            </button>
                        </div>
                    </div>
                </div>
            ) : null}

            {wildEncounter ? (
                <div className="fixed inset-0 z-50 grid place-items-center bg-[#dfeedd]/78 p-4 backdrop-blur-sm">
                    <div className="w-full max-w-lg rounded-[28px] bg-[#cfe3cd] p-6 text-[#46624a] shadow-[0_24px_80px_rgba(92,132,98,0.18)]">
                        <p className="text-xs font-bold tracking-[0.24em] text-[#6d8a72]">WILD ENCOUNTER</p>
                        <h2 className="mt-2 text-3xl font-black">
                            やせいの {wildEncounter.lead.name} が あらわれた！
                        </h2>
                        <p className="mt-3 text-sm leading-7 text-[#6d8a72]">
                            このまま既存のポケモンバトル画面へ移動します。相手はこの1体だけです。
                        </p>
                        <div className="mt-5 inline-flex rounded-full bg-[#b9d8b5] px-4 py-3 text-sm font-semibold text-[#46624a]">
                            {species[wildEncounter.pokemon.speciesId]?.name ?? wildEncounter.pokemon.speciesId}
                        </div>
                        <div className="mt-6 flex gap-3">
                            <button
                                type="button"
                                onClick={startWildBattle}
                                className="rounded-full bg-[#b9d8b5] px-4 py-2 text-sm font-bold text-[#46624a]"
                            >
                                バトルをはじめる
                            </button>
                            <button
                                type="button"
                                onClick={() => {
                                    setWildEncounter(null);
                                    openDialogue(['うまく にげきれた！', 'また草むらを歩くと野生ポケモンに出会う。']);
                                }}
                                className="rounded-full bg-[#d7e8d4] px-4 py-2 text-sm font-bold text-[#46624a]"
                            >
                                にげる
                            </button>
                        </div>
                    </div>
                </div>
            ) : null}
        </div>
    );
}

function Tile({ x, y }: Position) {
    const type = getTileType(x, y);

    const styleByType: Record<string, string> = {
        grass: 'bg-[#b8dcae]',
        'tall-grass': 'bg-[#a9cf9f]',
        path: 'bg-[#c8d9c4]',
        hedge: 'bg-[#9ec494]',
        wall: 'bg-[#adc7a7]',
    };

    const overlay =
        type === 'tall-grass'
            ? (
                <>
                    <div className="absolute inset-x-1.5 bottom-2 h-2.5 rounded-[4px] bg-[#8ebb85]" />
                </>
            )
            : null;

    return (
        <div
            className={`absolute ${styleByType[type]}`}
            style={{
                left: x * TILE_SIZE,
                top: y * TILE_SIZE,
                width: TILE_SIZE,
                height: TILE_SIZE,
            }}
        >
            {overlay}
        </div>
    );
}

function Building({
    x,
    y,
    width,
    height,
    roof,
}: {
    x: number;
    y: number;
    width: number;
    height: number;
    roof: string;
}) {
    return (
        <div
            className="absolute"
            style={{
                left: x * TILE_SIZE,
                top: y * TILE_SIZE,
                width: width * TILE_SIZE,
                height: height * TILE_SIZE,
            }}
        >
            <div
                className="h-[38%] opacity-90"
                style={{ backgroundColor: roof }}
            />
            <div className="relative h-[62%] bg-[#b5caae]">
                <div className="absolute bottom-0 left-[38%] h-10 w-8 bg-[#93aa8c]" />
            </div>
        </div>
    );
}

function Barrel({ x, y }: Position) {
    return (
        <div
            className="absolute rounded-full bg-[#a7bd9f]"
            style={{
                left: x * TILE_SIZE + 11,
                top: y * TILE_SIZE + 11,
                width: 18,
                height: 18,
            }}
        />
    );
}

function Crystal({ x, y }: Position) {
    return (
        <div
            className="absolute rotate-12 rounded-[10px] bg-[#d7ead4] opacity-85"
            style={{
                left: x * TILE_SIZE + 10,
                top: y * TILE_SIZE + 6,
                width: 20,
                height: 28,
            }}
        />
    );
}

function Character({
    x,
    y,
    color,
    facing,
    player = false,
}: {
    x: number;
    y: number;
    color: string;
    facing?: Direction;
    player?: boolean;
}) {
    const eyeOffset =
        facing === 'left' ? 8 : facing === 'right' ? 16 : 12;

    return (
        <div
            className="absolute"
            style={{
                left: x * TILE_SIZE + 8,
                top: y * TILE_SIZE + 6,
                width: 24,
                height: 28,
            }}
        >
            <div
                className="mx-auto h-3.5 w-3.5 rounded-full border border-black/20"
                style={{ backgroundColor: player ? '#c7d7c3' : '#ceddc9' }}
            />
            <div
                className="relative mt-[-1px] h-5 rounded-sm border border-black/20"
                style={{ backgroundColor: color }}
            >
                <div
                    className="absolute top-1 h-1 w-1 rounded-full bg-black/55"
                    style={{ left: eyeOffset }}
                />
            </div>
        </div>
    );
}
