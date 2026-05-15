import { Link } from 'react-router-dom';
import type { ReactNode } from 'react';
import { ArrowLeft, Bot, UsersRound } from 'lucide-react';

export default function BattleModePage() {
    return (
        <div className="min-h-dvh bg-white text-[#111111]">
            <header className="border-b border-[#111111] bg-white">
                <div className="mx-auto flex max-w-6xl items-center justify-between px-5 py-3 sm:px-8 lg:px-10">
                    <Link to="/home" className="flex items-center gap-3 text-sm font-bold tracking-[0.12em]">
                        <ArrowLeft className="size-4" strokeWidth={1.8} />
                        ホーム
                    </Link>
                    <span className="text-xl font-bold tracking-[0.2em]">Nikidan</span>
                </div>
            </header>

            <main className="mx-auto flex min-h-[calc(100dvh-53px)] max-w-6xl flex-col justify-center px-5 py-8 sm:px-8 lg:px-10">
                <section className="grid gap-8 lg:grid-cols-[0.86fr_1.14fr] lg:items-end">
                    <div className="space-y-5">
                        <div className="flex items-center gap-3">
                            <span className="h-8 w-4 rounded-l-full border border-r-0 border-[#111111] bg-[#F5EEE4]" />
                            <p className="text-xs font-bold tracking-[0.24em]">BATTLE MODE</p>
                        </div>
                        <div className="space-y-4">
                            <h1 className="text-4xl font-bold leading-[1.35] tracking-[0.18em] sm:text-5xl">
                                対戦方法を<br />
                                選ぶ。
                            </h1>
                            <p className="max-w-md text-sm font-semibold leading-7 tracking-[0.08em] text-[#333333]">
                                AIと研究するか、プレイヤーとぶつかるか。<br />
                                デッキを作って、戦おう。
                            </p>
                        </div>
                    </div>

                    <div className="grid gap-4 md:grid-cols-2">
                        <BattleModeCard
                            to="/deck-builder?mode=ai"
                            icon={<Bot className="size-7" strokeWidth={1.7} />}
                            title="AI戦"
                            label="VS AI"
                            body="デッキを準備して、AI相手に対戦を始める。"
                        />
                        <BattleModeCard
                            to="/deck-builder?mode=player"
                            icon={<UsersRound className="size-7" strokeWidth={1.7} />}
                            title="プレイヤー戦"
                            label="VS PLAYER"
                            body="デッキを準備して、オンラインロビーから対戦する。(レート戦)"
                        />
                    </div>
                </section>
            </main>
        </div>
    );
}

function BattleModeCard({
    to,
    icon,
    title,
    label,
    body,
}: {
    to: string;
    icon: ReactNode;
    title: string;
    label: string;
    body: string;
}) {
    return (
        <Link
            to={to}
            className="group grid min-h-64 grid-rows-[auto_1fr_auto] rounded-lg border border-[#111111] bg-white p-5 transition-colors hover:bg-[#F5EEE4]"
        >
            <div className="flex items-center justify-between border-b border-[#111111] pb-4">
                <span className="grid size-12 place-items-center rounded-full bg-[#F5EEE4]">
                    {icon}
                </span>
                <span className="text-[10px] font-bold tracking-[0.24em]">{label}</span>
            </div>

            <div className="py-5">
                <h2 className="text-2xl font-bold tracking-[0.12em]">{title}</h2>
                <p className="mt-3 text-sm font-semibold leading-7 tracking-[0.06em] text-[#333333]">{body}</p>
            </div>

            <div className="flex items-center justify-between border-t border-dashed border-[#111111]/50 pt-4 text-sm font-bold tracking-[0.08em]">
                <span>デッキ選択へ</span>
                <span className="text-2xl leading-none transition-transform group-hover:translate-x-1">→</span>
            </div>
        </Link>
    );
}
