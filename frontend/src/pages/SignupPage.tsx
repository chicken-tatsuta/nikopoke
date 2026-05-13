import { useState } from 'react';
import type { FormEvent } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ArrowRight } from 'lucide-react';
import { useAuth } from '../contexts/AuthContext';

export default function SignupPage() {
    const navigate = useNavigate();
    const { signUp } = useAuth();
    const [username, setUsername] = useState('');
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [error, setError] = useState<string | null>(null);
    const [busy, setBusy] = useState(false);

    const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
        event.preventDefault();
        setBusy(true);
        setError(null);

        try {
            await signUp(email, password, username.trim());
            navigate('/home');
        } catch (signUpError) {
            const message = signUpError instanceof Error ? signUpError.message : '登録に失敗しました。';
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
                        アカウントを<br />
                        作る。
                    </h1>
                    <p className="mt-5 text-sm font-semibold leading-7 tracking-[0.08em] text-[#333333]">
                        ユーザー名とデッキを紐づけて、ニキダンと戦えるようにします。
                    </p>
                </section>

                <form onSubmit={handleSubmit} className="space-y-4 p-7">
                    <label className="block">
                        <span className="mb-1.5 block text-xs font-bold tracking-[0.12em] text-[#333333]">ユーザー名</span>
                        <input
                            value={username}
                            onChange={(event) => setUsername(event.target.value)}
                            required
                            minLength={2}
                            autoComplete="username"
                            className="w-full rounded-md border border-[#111111] bg-white px-4 py-3 text-[#111111] outline-none transition-colors placeholder:text-[#777777] focus:bg-[#F5EEE4]"
                        />
                    </label>

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
                            minLength={6}
                            autoComplete="new-password"
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
                        {busy ? '登録中...' : '新規登録'}
                        <ArrowRight className="size-4" />
                    </button>

                    <p className="pt-2 text-center text-sm text-[#666666]">
                        すでにアカウントがある場合は{' '}
                        <Link to="/login" className="font-bold text-[#111111] underline underline-offset-4">
                            ログイン
                        </Link>
                    </p>
                </form>
            </main>
        </div>
    );
}
