import { useState } from 'react';
import type { FormEvent } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import { Zap } from 'lucide-react';
import { useAuth } from '../contexts/AuthContext';

export default function LoginPage() {
    const navigate = useNavigate();
    const location = useLocation();
    const { signIn } = useAuth();
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [error, setError] = useState<string | null>(null);
    const [busy, setBusy] = useState(false);

    const from = (location.state as { from?: { pathname?: string; search?: string } } | null)?.from;
    const redirectTo = from?.pathname ? `${from.pathname}${from.search ?? ''}` : '/home';

    const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault();
        setBusy(true);
        setError(null);

        try {
            await signIn(email, password);
            navigate(redirectTo, { replace: true });
        } catch (signInError) {
            const raw = signInError instanceof Error ? signInError.message : '';
            const message = raw.toLowerCase().includes('email not confirmed')
                ? 'メールアドレスの確認が完了していません。届いたメールのリンクをクリックしてから再度ログインしてください。'
                : raw || 'ログインに失敗しました。';
            setError(message);
        } finally {
            setBusy(false);
        }
    };

    return (
        <div className="min-h-dvh bg-[var(--surface-1)] flex items-center justify-center px-6">
            <main className="w-full max-w-sm rounded-xl border border-[var(--border)] bg-[var(--surface-2)] p-7 shadow-xl shadow-black/10">
                <div className="mb-7 text-center">
                    <div className="mb-3 flex items-center justify-center gap-2">
                        <Zap className="size-7 text-[var(--accent)]" />
                        <h1 className="text-3xl font-black text-[var(--text-primary)]">Nikipoke</h1>
                    </div>
                    <p className="text-sm text-[var(--text-muted)]">ログインしてバトルを始めましょう</p>
                </div>

                <form onSubmit={handleSubmit} className="space-y-4">
                    <label className="block">
                        <span className="mb-1.5 block text-sm font-medium text-[var(--text-secondary)]">メールアドレス</span>
                        <input
                            type="email"
                            value={email}
                            onChange={(event) => setEmail(event.target.value)}
                            required
                            autoComplete="email"
                            className="w-full rounded-xl border border-[var(--border)] bg-[var(--surface-1)] px-4 py-3 text-[var(--text-primary)] outline-none transition-colors placeholder:text-[var(--text-muted)] focus:border-[var(--accent)]"
                        />
                    </label>

                    <label className="block">
                        <span className="mb-1.5 block text-sm font-medium text-[var(--text-secondary)]">パスワード</span>
                        <input
                            type="password"
                            value={password}
                            onChange={(event) => setPassword(event.target.value)}
                            required
                            autoComplete="current-password"
                            className="w-full rounded-xl border border-[var(--border)] bg-[var(--surface-1)] px-4 py-3 text-[var(--text-primary)] outline-none transition-colors placeholder:text-[var(--text-muted)] focus:border-[var(--accent)]"
                        />
                    </label>

                    {error && (
                        <div className="rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-300">
                            {error}
                        </div>
                    )}

                    <button
                        type="submit"
                        disabled={busy}
                        className="w-full rounded-xl bg-[var(--accent)] px-4 py-3 font-semibold text-white transition-colors hover:bg-[var(--accent-hover)] disabled:cursor-not-allowed disabled:opacity-60"
                    >
                        {busy ? 'ログイン中...' : 'ログイン'}
                    </button>
                </form>

                <p className="mt-6 text-center text-sm text-[var(--text-muted)]">
                    アカウントがない場合は{' '}
                    <Link to="/signup" className="font-semibold text-[var(--accent)] hover:text-[var(--accent-hover)]">
                        新規登録
                    </Link>
                </p>
            </main>
        </div>
    );
}
