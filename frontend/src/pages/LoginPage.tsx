import { useState } from 'react';
import type { FormEvent } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import { ArrowRight } from 'lucide-react';
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
        <div className="flex min-h-dvh items-center justify-center bg-white px-6 py-10 text-[#111111]">
            <main className="grid w-full max-w-5xl overflow-hidden rounded-lg border border-[#111111] bg-white md:grid-cols-[0.95fr_1.05fr]">
                <section className="border-b border-[#111111] bg-[#FAFAFA] p-7 md:border-b-0 md:border-r">
                    <div className="flex items-center gap-4">
                        <span className="h-7 w-14 rounded-t-full border border-b-0 border-[#111111]" />
                        <span className="text-xl font-bold tracking-[0.22em]">Nikidan</span>
                    </div>
                    <h1 className="mt-12 text-4xl font-black leading-[1.35] tracking-[0.14em]">
                        記録を<br />
                        再開する。
                    </h1>
                    <p className="mt-5 text-sm font-semibold leading-7 tracking-[0.08em] text-[#333333]">
                        デッキ、レート、対戦ログをそのまま引き継いで、ニキダンの研究を続けます。
                    </p>
                </section>

                <form onSubmit={handleSubmit} className="space-y-4 p-7">
                    <label className="block">
                        <span className="mb-1.5 block text-xs font-bold tracking-[0.12em] text-[#333333]">メールアドレス</span>
                        <input
                            type="email"
                            value={email}
                            onChange={(event) => setEmail(event.target.value)}
                            required
                            autoComplete="email"
                            className="w-full rounded-md border border-[#111111] bg-white px-4 py-3 text-[#111111] outline-none transition-colors placeholder:text-[#777777] focus:bg-[#F5EEE4]"
                        />
                    </label>

                    <label className="block">
                        <span className="mb-1.5 block text-xs font-bold tracking-[0.12em] text-[#333333]">パスワード</span>
                        <input
                            type="password"
                            value={password}
                            onChange={(event) => setPassword(event.target.value)}
                            required
                            autoComplete="current-password"
                            className="w-full rounded-md border border-[#111111] bg-white px-4 py-3 text-[#111111] outline-none transition-colors placeholder:text-[#777777] focus:bg-[#F5EEE4]"
                        />
                    </label>

                    {error && (
                        <div className="rounded-md border border-[#111111] bg-[#F5EEE4] px-4 py-3 text-sm font-semibold text-[#111111]">
                            {error}
                        </div>
                    )}

                    <button
                        type="submit"
                        disabled={busy}
                        className="inline-flex w-full items-center justify-between rounded-md border border-[#111111] bg-[#F5EEE4] px-4 py-3 font-bold tracking-[0.08em] text-[#111111] transition-colors hover:bg-white disabled:cursor-not-allowed disabled:opacity-60"
                    >
                        {busy ? 'ログイン中...' : 'ログイン'}
                        <ArrowRight className="size-4" />
                    </button>

                    <p className="pt-2 text-center text-sm text-[#666666]">
                        アカウントがない場合は{' '}
                        <Link to="/signup" className="font-bold text-[#111111] underline underline-offset-4">
                            新規登録
                        </Link>
                    </p>
                </form>
            </main>
        </div>
    );
}
