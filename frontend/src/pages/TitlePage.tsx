import { Link } from 'react-router-dom';
import heroBackgroundUrl from '../../image/bg.jpg';

export default function TitlePage() {
    return (
        <div className="relative min-h-dvh overflow-hidden bg-white text-[#111111]">
            <div className="absolute inset-0 bg-cover bg-center" style={{ backgroundImage: `url(${heroBackgroundUrl})` }} />
            <div className="absolute inset-0 bg-black/45" />

            <main className="relative z-10 mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-12">
                <div className="max-w-2xl text-white">
                    <div className="mb-8 flex items-center gap-4">
                        <span className="h-9 w-16 rounded-t-full border border-b-0 border-white" />
                        <span className="text-sm font-bold tracking-[0.28em]">NIKIDAN</span>
                    </div>

                    <h1 className="text-balance text-5xl font-black leading-[1.25] tracking-[0.16em] sm:text-7xl">
                        二期男が、<br />
                        ニキダンに。
                    </h1>

                    <p className="mt-8 max-w-lg text-sm font-semibold leading-8 tracking-[0.1em] text-white/90">
                        学生証みたいに集めて、研究ノートみたいに記録して、対戦ログとして残していく。
                    </p>

                    <div className="mt-10">
                        <Link
                            to="/home"
                            className="inline-flex min-w-52 items-center justify-between gap-8 rounded-md border border-[#111111] bg-white px-6 py-3 text-sm font-bold tracking-[0.1em] text-[#111111] transition-colors hover:bg-[#F5EEE4]"
                        >
                            はじめる
                            <span className="text-xl leading-none">→</span>
                        </Link>
                    </div>
                </div>
            </main>
        </div>
    );
}
