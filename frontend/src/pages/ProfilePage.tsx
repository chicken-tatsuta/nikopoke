import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ArrowLeft, Edit3, FolderOpen, Save, Trash2, UserRound } from 'lucide-react';
import { useAuth, type SavedDeck } from '../contexts/AuthContext';
import { loadAllData } from '../lib/data';
import { getPokemonPortraitSrc } from '../lib/pokemonImages';
import { getAbilityLabel } from './PokemonDetailPage';
import type { DeckPokemon, MoveData, SpeciesData } from '../types/pokemon';

export default function ProfilePage() {
    const navigate = useNavigate();
    const { profile, updateProfile, signOut } = useAuth();
    const [species, setSpecies] = useState<SpeciesData>({});
    const [moves, setMoves] = useState<MoveData>({});
    const [username, setUsername] = useState(profile?.username ?? '');
    const [savingName, setSavingName] = useState(false);
    const [message, setMessage] = useState('');

    useEffect(() => {
        loadAllData().then(({ species, moves }) => {
            setSpecies(species);
            setMoves(moves);
        }).catch((error) => {
            console.error('[profile] Failed to load deck data:', error);
        });
    }, []);

    useEffect(() => {
        setUsername(profile?.username ?? '');
    }, [profile?.username]);

    const wins = profile?.win_count ?? 0;
    const losses = profile?.loss_count ?? 0;
    const total = wins + losses;
    const winRate = total > 0 ? Math.round((wins / total) * 1000) / 10 : 0;
    const savedDecks = useMemo(() => profile?.saved_decks ?? [], [profile?.saved_decks]);

    const handleSaveName = async () => {
        const nextName = username.trim();
        if (!nextName || nextName === profile?.username) return;

        setSavingName(true);
        setMessage('');
        try {
            await updateProfile({ username: nextName });
            setMessage('名前を更新しました。');
        } catch (error) {
            console.error('[profile] Failed to update username:', error);
            setMessage('名前の更新に失敗しました。');
        } finally {
            setSavingName(false);
        }
    };

    const editDeck = async (deck: DeckPokemon[]) => {
        if (deck.length === 0) return;
        localStorage.setItem('savedDeck', JSON.stringify(deck));
        try {
            await updateProfile({ current_deck: deck });
        } catch (error) {
            console.error('[profile] Failed to set current deck:', error);
        }
        navigate('/deck-builder');
    };

    const deleteSavedDeck = async (deckName: string) => {
        const ok = window.confirm(`「${deckName}」を削除しますか？`);
        if (!ok) return;

        const nextDecks = savedDecks.filter((deck) => deck.name !== deckName);
        try {
            await updateProfile({ saved_decks: nextDecks });
            setMessage('保存済みデッキを削除しました。');
        } catch (error) {
            console.error('[profile] Failed to delete saved deck:', error);
            setMessage('デッキの削除に失敗しました。');
        }
    };

    return (
        <div className="min-h-dvh bg-white text-[#111111]">
            <header className="border-b border-[#111111] bg-white">
                <div className="mx-auto flex max-w-6xl items-center justify-between px-5 py-3 sm:px-8 lg:px-10">
                    <Link to="/home" className="inline-flex items-center gap-3 text-sm font-bold tracking-[0.12em]">
                        <ArrowLeft className="size-4" />
                        トップへ戻る
                    </Link>
                    <button
                        onClick={() => void signOut().then(() => navigate('/login'))}
                        className="rounded-md border border-[#111111] bg-white px-3 py-2 text-xs font-bold tracking-[0.12em] transition-colors hover:bg-[#F5EEE4]"
                    >
                        ログアウト
                    </button>
                </div>
            </header>

            <main className="mx-auto max-w-6xl px-5 py-8 sm:px-8 lg:px-10">
                <section className="grid gap-5 border-b border-[#111111] pb-8 lg:grid-cols-[minmax(0,1fr)_360px]">
                    <div>
                        <div className="mb-4 inline-flex items-center gap-3">
                            <span className="h-7 w-14 rounded-t-full border border-b-0 border-[#111111]" />
                            <span className="text-xs font-bold tracking-[0.24em]">PROFILE NOTE</span>
                        </div>
                        <h1 className="text-4xl font-black tracking-[0.12em] sm:text-5xl">
                            {profile?.username ?? 'ユーザー'}
                        </h1>
                        <p className="mt-4 max-w-xl text-sm font-bold leading-7 tracking-[0.08em] text-[#333333]">
                            作成したデッキを確認して、必要なら編集へ戻る。あなたの輝かしい戦績はこの右。
                        </p>
                    </div>

                    <div className="rounded-lg border border-[#111111] bg-[#FAFAFA] p-4">
                        <div className="mb-4 flex items-center gap-3 border-b border-[#111111] pb-3">
                            <span className="grid size-10 place-items-center rounded-full border border-[#111111] bg-white">
                                <UserRound className="size-5" />
                            </span>
                            <div className="min-w-0">
                                <p className="truncate text-sm font-black tracking-[0.12em]">{profile?.username ?? '-'}</p>
                                <p className="text-xs font-bold tracking-[0.12em] text-[#555555]">R{profile?.rating ?? 1500}</p>
                            </div>
                        </div>
                        <div className="grid grid-cols-3 gap-2 text-center">
                            <ProfileStat label="勝率" value={`${winRate.toFixed(1)}%`} />
                            <ProfileStat label="勝ち" value={String(wins)} />
                            <ProfileStat label="負け" value={String(losses)} />
                        </div>
                    </div>
                </section>

                <section className="grid gap-6 py-8 lg:grid-cols-[360px_minmax(0,1fr)]">
                    <div className="space-y-4">
                        <div className="rounded-lg border border-[#111111] bg-white p-4">
                            <h2 className="mb-4 text-base font-black tracking-[0.16em]">名前の変更</h2>
                            <label className="mb-2 block text-xs font-bold tracking-[0.12em] text-[#333333]">ユーザー名</label>
                            <input
                                value={username}
                                onChange={(event) => setUsername(event.target.value)}
                                className="w-full rounded-md border border-[#111111] bg-white px-3 py-2 text-sm font-bold outline-none transition-colors focus:bg-[#F5EEE4]"
                            />
                            <button
                                onClick={handleSaveName}
                                disabled={savingName || !username.trim() || username.trim() === profile?.username}
                                className="mt-3 inline-flex w-full items-center justify-between rounded-md border border-[#111111] bg-[#F5EEE4] px-4 py-2 text-sm font-black tracking-[0.12em] transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-50"
                            >
                                {savingName ? '保存中' : '保存する'}
                                <Save className="size-4" />
                            </button>
                            {message && <p className="mt-3 text-xs font-bold text-[#333333]">{message}</p>}
                        </div>

                        <div className="rounded-lg border border-[#111111] bg-[#FAFAFA] p-4">
                            <h2 className="mb-3 text-base font-black tracking-[0.16em]">デッキ管理</h2>
                            <Link
                                to="/deck-builder"
                                className="inline-flex w-full items-center justify-between rounded-md border border-[#111111] bg-white px-4 py-2 text-sm font-black tracking-[0.12em] transition-colors hover:bg-[#F5EEE4]"
                            >
                                新しく編集する
                                <Edit3 className="size-4" />
                            </Link>
                        </div>
                    </div>

                    <div className="min-w-0 space-y-6">
                        <DeckSection
                            title="現在のデッキ"
                            emptyText="現在のデッキはありません。"
                            decks={profile?.current_deck ? [{ name: 'CURRENT DECK', deck: profile.current_deck }] : []}
                            species={species}
                            moves={moves}
                            onEdit={(deck) => void editDeck(deck)}
                        />

                        <DeckSection
                            title="保存済みデッキ"
                            emptyText="保存済みデッキはありません。"
                            decks={savedDecks}
                            species={species}
                            moves={moves}
                            onEdit={(deck) => void editDeck(deck)}
                            onDelete={deleteSavedDeck}
                        />
                    </div>
                </section>
            </main>
        </div>
    );
}

function ProfileStat({ label, value }: { label: string; value: string }) {
    return (
        <div className="rounded-md border border-[#111111] bg-white px-2 py-3">
            <p className="text-[10px] font-bold tracking-[0.16em] text-[#555555]">{label}</p>
            <p className="mt-1 text-lg font-black tabular-nums">{value}</p>
        </div>
    );
}

function DeckSection({
    title,
    emptyText,
    decks,
    species,
    moves,
    onEdit,
    onDelete,
}: {
    title: string;
    emptyText: string;
    decks: SavedDeck[];
    species: SpeciesData;
    moves: MoveData;
    onEdit: (deck: DeckPokemon[]) => void;
    onDelete?: (name: string) => void;
}) {
    return (
        <section>
            <div className="mb-3 flex items-center justify-between border-b border-[#111111] pb-2">
                <h2 className="text-lg font-black tracking-[0.16em]">{title}</h2>
                <span className="text-xs font-bold tracking-[0.12em]">{decks.length}件</span>
            </div>
            {decks.length === 0 ? (
                <div className="rounded-lg border border-dashed border-[#111111] bg-[#FAFAFA] p-6 text-sm font-bold text-[#555555]">
                    {emptyText}
                </div>
            ) : (
                <div className="grid gap-4">
                    {decks.map((deck) => (
                        <DeckCard
                            key={deck.name}
                            savedDeck={deck}
                            species={species}
                            moves={moves}
                            onEdit={() => onEdit(deck.deck)}
                            onDelete={onDelete ? () => onDelete(deck.name) : undefined}
                        />
                    ))}
                </div>
            )}
        </section>
    );
}

function DeckCard({
    savedDeck,
    species,
    moves,
    onEdit,
    onDelete,
}: {
    savedDeck: SavedDeck;
    species: SpeciesData;
    moves: MoveData;
    onEdit: () => void;
    onDelete?: () => void;
}) {
    return (
        <article className="rounded-lg border border-[#111111] bg-white">
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[#111111] bg-[#FAFAFA] px-4 py-3">
                <div className="min-w-0">
                    <h3 className="truncate text-base font-black tracking-[0.12em]">{savedDeck.name}</h3>
                    <p className="mt-1 text-xs font-bold tracking-[0.1em] text-[#555555]">{savedDeck.deck.length} NIKIDAN</p>
                </div>
                <div className="flex gap-2">
                    <button
                        onClick={onEdit}
                        className="inline-flex items-center gap-2 rounded-md border border-[#111111] bg-[#F5EEE4] px-3 py-2 text-xs font-black tracking-[0.12em] transition-colors hover:bg-white"
                    >
                        <FolderOpen className="size-4" />
                        編集
                    </button>
                    {onDelete && (
                        <button
                            onClick={onDelete}
                            className="grid size-9 place-items-center rounded-md border border-[#111111] bg-white transition-colors hover:bg-[#F5EEE4]"
                            aria-label={`${savedDeck.name}を削除`}
                        >
                            <Trash2 className="size-4" />
                        </button>
                    )}
                </div>
            </div>
            <div className="grid gap-3 p-4 sm:grid-cols-2 xl:grid-cols-3">
                {savedDeck.deck.map((pokemon, index) => (
                    <MiniDeckPokemon
                        key={`${pokemon.speciesId}-${index}`}
                        pokemon={pokemon}
                        species={species[pokemon.speciesId]}
                        moves={moves}
                    />
                ))}
            </div>
        </article>
    );
}

function MiniDeckPokemon({ pokemon, species, moves }: { pokemon: DeckPokemon; species: SpeciesData[string] | undefined; moves: MoveData }) {
    const portraitSrc = getPokemonPortraitSrc(pokemon.speciesId, species?.name);

    return (
        <div className="grid grid-cols-[56px_minmax(0,1fr)] gap-3 rounded-md border border-[#111111] bg-white p-2">
            <div className="aspect-square overflow-hidden rounded-md border border-[#111111] bg-[#F3F3F3]">
                <img src={portraitSrc} alt={species?.name ?? pokemon.speciesId} className="size-full object-cover" draggable={false} />
            </div>
            <div className="min-w-0">
                <div className="flex items-start justify-between gap-2">
                    <p className="truncate text-sm font-black tracking-[0.08em]">{species?.name ?? pokemon.speciesId}</p>
                    <span className="text-[10px] font-bold tabular-nums">{String(indexLabel(pokemon)).padStart(2, '0')}</span>
                </div>
                <p className="mt-1 truncate text-[10px] font-bold tracking-[0.08em] text-[#555555]">
                    {getAbilityLabel(pokemon.ability)}
                </p>
                <div className="mt-2 grid grid-cols-2 gap-1">
                    {pokemon.moves.slice(0, 4).map((moveId) => (
                        <span key={moveId} className="truncate border-t border-[#111111] pt-1 text-[10px] font-bold text-[#333333]">
                            {moves[moveId]?.name ?? moveId}
                        </span>
                    ))}
                </div>
            </div>
        </div>
    );
}

function indexLabel(pokemon: DeckPokemon): number {
    return pokemon.speciesId.split('').reduce((sum, char) => sum + char.charCodeAt(0), 0) % 99 + 1;
}
