import { lazy, Suspense } from 'react';
import { Link, useParams } from 'react-router-dom';
import { ThemeProvider } from '@mui/material/styles';
import { Calculator, Users, Gauge, Swords, ArrowLeft } from 'lucide-react';
import { toolsTheme } from '../tools/theme';

const pages = {
  damage: lazy(() => import('../tools/pages/DamageCalculatorPage')),
  party: lazy(() => import('../tools/pages/PartyTypePage')),
  speed: lazy(() => import('../tools/pages/SpeedLinePage')),
  matchup: lazy(() => import('../tools/pages/MatchupSimulatorPage')),
};
const tabs = [
  { id: 'damage', name: 'ダメージ計算', icon: Calculator },
  { id: 'party', name: 'タイプ補完', icon: Users },
  { id: 'speed', name: '素早さライン', icon: Gauge },
  { id: 'matchup', name: '対面シミュレーター', icon: Swords },
];

export default function ToolsPage() {
  const { tool = 'damage' } = useParams();
  const Page = pages[tool as keyof typeof pages] ?? pages.damage;
  return (
    <div className="min-h-dvh bg-white text-[#111111]">
      <header className="border-b border-[#111111] bg-white">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-5 py-3 sm:px-8 lg:px-10">
          <Link to="/home" className="inline-flex items-center gap-2 text-sm font-bold"><ArrowLeft size={16} strokeWidth={1.8} />ホーム</Link>
          <span className="text-xl font-bold tracking-[0.2em]">Nikidan</span>
        </div>
      </header>
      <div className="mx-auto max-w-6xl px-5 pt-7 sm:px-8 lg:px-10">
        <div className="mb-3 flex items-center gap-3">
          <span className="h-7 w-3.5 rounded-l-full border border-r-0 border-[#111111] bg-[#F5EEE4]" />
          <p className="text-xs font-bold tracking-[0.22em]">NIKIDAN TOOLS</p>
        </div>
        <h1 className="mb-6 text-3xl font-black">ツール</h1>
      <nav aria-label="ツール" className="grid grid-cols-2 border-b border-[#111111] sm:flex">
        {tabs.map(({ id, name, icon: Icon }) => (
          <Link key={id} to={`/tools/${id}`} aria-current={tool === id ? 'page' : undefined}
            className={`inline-flex min-h-12 items-center justify-center gap-2 border-b-2 px-3 py-3 text-xs font-bold sm:text-sm ${tool === id ? 'border-[#111111] bg-[#F5EEE4]' : 'border-transparent hover:bg-[#fafafa]'}`}>
            <Icon size={17} />{name}
          </Link>
        ))}
      </nav>
      {tool === 'matchup' && <p className="mt-4 border-l-2 border-[#111111] pl-3 text-xs leading-6 text-[#666666]">参考値：簡易シミュレーターによる推定です。実際のバトルとは特性・技効果の対応範囲が異なります。</p>}
      </div>
      <main className="nikipoke-tools mx-auto max-w-6xl px-5 pb-12 sm:px-8 lg:px-10">
      <ThemeProvider theme={toolsTheme}>
        <Suspense fallback={<p role="status" className="p-8">読み込み中...</p>}><Page /></Suspense>
      </ThemeProvider>
      </main>
    </div>
  );
}
