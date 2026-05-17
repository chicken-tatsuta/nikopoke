import { Link } from 'react-router-dom';
import { ArrowLeft, CalendarDays, Newspaper } from 'lucide-react';
import { useAuth } from '../contexts/AuthContext';
import { HomeHeader } from './HomePage';
import newsImageUrl from '../../image/news.jpg';

const NEWS_ITEMS = [
    {
        date: '2026.05.17',
        title: 'いたずらごころに関するバグを修正しました',
        body: '悪タイプの敵を対象に変化技を打てていたバグを修正しました。',
        tag: 'BATTLE',
    },
    {
        date: '2026.05.17',
        title: '種族値のバグを修正しました',
        body: 'NIKIDAN"ふうと"に関して、内部処理において想定よりもこうげきの値が低くなっていたのを修正しました。',
        tag: 'MOVE',
    },
    {
        date: '2026.05.17',
        title: 'バトルUIの改善を行いました',
        body: '場の状態を確認しやすくなりました。',
        tag: 'UI',
    },
];

export default function NewsPage() {
    const { profile } = useAuth();

    return (
        <div className="min-h-dvh bg-white text-[#111111]">
            <HomeHeader profile={profile} />

            <main className="mx-auto max-w-6xl px-5 py-6 sm:px-8 lg:px-10">
                <Link
                    to="/home"
                    className="mb-5 inline-flex items-center gap-2 text-sm font-bold tracking-[0.08em] text-[#555555] transition-colors hover:text-[#111111]"
                >
                    <ArrowLeft className="size-4" strokeWidth={1.8} />
                    ホームへ戻る
                </Link>

                <section className="grid gap-6 lg:grid-cols-[minmax(0,1.08fr)_minmax(360px,0.92fr)] lg:items-stretch">
                    <div className="relative isolate min-h-[360px] overflow-hidden rounded-lg border border-[#111111] bg-[#111111] sm:min-h-[440px]">
                        <img
                            src={newsImageUrl}
                            alt="ニキダンのおしらせ"
                            className="absolute inset-0 -z-20 size-full object-cover"
                            draggable={false}
                        />
                        <div className="absolute inset-0 -z-10 bg-black/45" />
                        <div className="flex h-full flex-col justify-end p-6 text-white sm:p-8">
                            <div className="mb-5 flex items-center gap-3">
                                <span className="h-8 w-4 rounded-l-full border border-r-0 border-white bg-[#F5EEE4]" />
                                <span className="text-xs font-bold tracking-[0.22em]">NIKIDAN NEWS</span>
                            </div>
                            <h1 className="text-balance text-4xl font-bold leading-[1.35] tracking-[0.14em] sm:text-5xl">
                                おしらせ
                            </h1>
                            <p className="mt-4 max-w-xl text-sm font-semibold leading-7 tracking-[0.08em] text-white/90">
                                ニキダンの更新、バトル調整、図鑑まわりの変更をまとめて確認できます。
                                「あいつ頑張ってんなあ」と温かい目でみてください。
                            </p>
                        </div>
                    </div>

                    <aside className="rounded-lg border border-[#111111] bg-[#FAFAFA] p-5 sm:p-6">
                        <div className="flex items-center justify-between border-b border-[#111111] pb-4">
                            <div>
                                <p className="text-[10px] font-bold tracking-[0.24em] text-[#555555]">LATEST</p>
                                <h2 className="mt-1 text-xl font-bold tracking-[0.12em]">更新ノート</h2>
                            </div>
                            <span className="grid size-11 place-items-center rounded-full border border-[#111111] bg-[#F5EEE4]">
                                <Newspaper className="size-5" strokeWidth={1.8} />
                            </span>
                        </div>

                        <div className="mt-5 space-y-4">
                            {NEWS_ITEMS.map((item, index) => (
                                <article key={item.title} className="rounded-md border border-[#111111] bg-white p-4">
                                    <div className="mb-3 flex items-center justify-between gap-3">
                                        <div className="flex items-center gap-2 text-xs font-bold tracking-[0.08em] text-[#555555]">
                                            <CalendarDays className="size-4" strokeWidth={1.8} />
                                            <time>{item.date}</time>
                                        </div>
                                        <span className="rounded border border-[#111111] bg-[#F5EEE4] px-2 py-1 text-[10px] font-bold leading-none tracking-[0.14em]">
                                            {item.tag}
                                        </span>
                                    </div>
                                    <div className="grid grid-cols-[2.5rem_1fr] gap-3">
                                        <span className="font-bold tabular-nums tracking-[0.12em]">
                                            {String(index + 1).padStart(2, '0')}
                                        </span>
                                        <div>
                                            <h3 className="text-base font-bold leading-relaxed tracking-[0.05em]">{item.title}</h3>
                                            <p className="mt-2 text-sm leading-7 text-[#555555]">{item.body}</p>
                                        </div>
                                    </div>
                                </article>
                            ))}
                        </div>
                    </aside>
                </section>
            </main>
        </div>
    );
}
