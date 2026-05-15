import { useEffect, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { Trophy, Home, RotateCcw } from 'lucide-react';
import { clearOnlineSession } from '../lib/p2p';

interface BattleResult {
    winner: string | null;
    logs: string[];
    localPlayerId?: string;
}

interface RatingDelta {
    winnerDelta: number;
    loserDelta: number;
    winnerEloDelta?: number;
    loserEloDelta?: number;
    winnerBonus?: number;
    loserBonus?: number;
}

interface ResultLocationState {
    battleMode?: 'ai' | 'player';
    result?: BattleResult;
    ratingDelta?: RatingDelta | null;
}

export default function ResultPage() {
    const navigate = useNavigate();
    const location = useLocation();
    const [result] = useState<BattleResult | null>(() => {
        const state = location.state as ResultLocationState | null;
        return state?.result ?? null;
    });
    const ratingDelta = ((location.state as ResultLocationState | null)?.ratingDelta ?? null) as RatingDelta | null;
    const battleMode = ((location.state as ResultLocationState | null)?.battleMode ?? 'ai') as 'ai' | 'player';

    useEffect(() => {
        if (!result) {
            navigate('/home');
        }
    }, [navigate, result]);

    if (!result) {
        return (
            <div className="min-h-dvh bg-white flex items-center justify-center">
                <div className="text-[var(--text-muted)] text-lg">読み込み中...</div>
            </div>
        );
    }

    const localPlayerId = result.localPlayerId ?? 'player';
    const isVictory = result.winner === localPlayerId;
    const totalDelta = ratingDelta ? (isVictory ? ratingDelta.winnerDelta : ratingDelta.loserDelta) : 0;
    const eloDelta = ratingDelta ? (isVictory ? ratingDelta.winnerEloDelta : ratingDelta.loserEloDelta) ?? totalDelta : 0;
    const bonusDelta = ratingDelta ? (isVictory ? ratingDelta.winnerBonus : ratingDelta.loserBonus) ?? 0 : 0;
    const formatDelta = (value: number) => value > 0 ? `+${value}` : String(value);

    return (
        <div className="min-h-dvh bg-white bg-grid-pattern flex flex-col items-center justify-center p-6 text-[#111111]">
            <div className="bg-white border border-[#111111] rounded-lg p-7 text-center max-w-md w-full">
                <div className={`mx-auto size-16 rounded-full border border-[#111111] flex items-center justify-center mb-6
                    ${isVictory ? 'bg-[#F5EEE4]' : 'bg-white'}`}
                >
                    <Trophy className="size-8 text-[#111111]" />
                </div>

                <h1 className="text-balance text-4xl font-black mb-2 tracking-[0.16em]">
                    {isVictory ? '勝利' : '敗北'}
                </h1>
                <p className="text-[var(--text-secondary)] mb-2 font-semibold">
                    {isVictory ? '対戦記録に勝利として保存されました。' : '対戦記録に敗北として保存されました。'}
                </p>

                {ratingDelta && (
                    <div className="mb-6 rounded-md border border-[#111111] bg-[#FAFAFA] p-4 text-left">
                        <p className="mb-3 text-center text-xl font-bold tabular-nums">
                            レート変動: {formatDelta(totalDelta)}
                        </p>
                        <div className="grid grid-cols-2 gap-2 text-sm font-bold tabular-nums text-[#333333]">
                            <span>Elo: {formatDelta(eloDelta)}</span>
                            <span className="text-right">ボーナス: {formatDelta(bonusDelta)}</span>
                        </div>
                    </div>
                )}

                <div className="bg-[#FAFAFA] rounded-md border border-[#111111] p-4 mb-6 max-h-48 overflow-y-auto text-left">
                    <h3 className="text-xs font-bold text-[var(--text-muted)] mb-2 uppercase tracking-[0.16em]">バトルログ</h3>
                    {result.logs.slice(-10).map((log, i) => (
                        <p key={i} className="text-xs text-[var(--text-secondary)] py-1.5 border-b border-[var(--border)] last:border-0">
                            {log}
                        </p>
                    ))}
                </div>

                <div className="space-y-3">
                    <button
                        onClick={() => {
                            if (battleMode === 'player') {
                                clearOnlineSession();
                                navigate('/online-lobby');
                                return;
                            }
                            navigate('/deck-builder?mode=ai');
                        }}
                        className="w-full py-3.5 rounded-md border border-[#111111] font-bold flex items-center justify-center gap-2
                            bg-[#F5EEE4] text-[#111111] hover:bg-white transition-all"
                    >
                        <RotateCcw className="size-5" />
                        もう一度バトル
                    </button>
                    <button
                        onClick={() => {
                            sessionStorage.removeItem('playerDeck');
                            if (battleMode === 'player') {
                                clearOnlineSession();
                            }
                            navigate('/home');
                        }}
                        className="w-full py-3.5 rounded-md border border-[#111111] font-bold flex items-center justify-center gap-2
                            bg-white text-[var(--text-primary)] hover:bg-[#F5EEE4] transition-all"
                    >
                        <Home className="size-5" />
                        ホームに戻る
                    </button>
                </div>
            </div>
        </div>
    );
}
