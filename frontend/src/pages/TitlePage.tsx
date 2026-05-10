import { Zap } from 'lucide-react';
import { Link } from 'react-router-dom';

export default function TitlePage() {
    return (
        <div className="min-h-dvh flex flex-col items-center justify-center bg-[var(--surface-0)] bg-grid-pattern relative overflow-hidden">
            {/* Subtle radial gradient overlay */}
            <div
                className="absolute inset-0 pointer-events-none"
                style={{
                    background: 'radial-gradient(ellipse at 50% 30%, rgba(99, 102, 241, 0.08) 0%, transparent 60%)'
                }}
            />

            {/* Main content */}
            <div className="relative z-10 text-center px-6">
                {/* Logo */}
                <div className="flex items-center justify-center gap-4 mb-6">
                    <Zap className="size-10 text-[var(--accent)]" strokeWidth={2.5} />
                    <h1 className="text-balance text-6xl sm:text-7xl font-black tracking-tight text-[var(--text-primary)]">
                        Nikimon
                    </h1>
                    <Zap className="size-10 text-[var(--accent)]" strokeWidth={2.5} />
                </div>

                {/* Subtitle */}
                <p className="text-lg text-[var(--text-secondary)] max-w-sm mx-auto leading-relaxed">
                    2期生男子
                    <br />
                    <span className="text-[var(--text-muted)]">ポケモンバトルシミュレーター</span>
                </p>

                {/* Start Button */}
                <div className="mt-12">
                    <Link
                        to="/home"
                        className="inline-flex items-center gap-3 px-10 py-4 text-lg font-semibold
                            text-white bg-[var(--accent)] hover:bg-[var(--accent-hover)]
                            rounded-xl
                            transition-all duration-150 ease-out
                            hover:scale-[1.02] active:scale-[0.98]
                            shadow-lg shadow-[var(--accent)]/20"
                    >
                        ゲームスタート
                    </Link>
                </div>

                {/* Version info */}
                <p className="text-xs text-[var(--text-muted)] mt-16 tracking-wide">
                    v1.0.0 — Powered by engine-rust
                </p>
            </div>
        </div>
    );
}
