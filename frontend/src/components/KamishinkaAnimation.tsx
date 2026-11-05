import type { CreatureStateWire } from '../lib/engine';
import { getBattlePortrait, kamishinkaForms } from '../lib/kamishinka';

export interface KamishinkaAnimationState {
    creature: CreatureStateWire;
    formId: string;
    phase: 'orb' | 'light' | 'reveal';
}

export function KamishinkaAnimation({ state }: { state: KamishinkaAnimationState }) {
    const form = kamishinkaForms.find((f) => f.id === state.formId);
    return (
        <div className={`kami-overlay kami-phase-${state.phase}`} role="status" aria-live="polite" aria-label="カミシンカ演出中">
            <div className="kami-orb" aria-hidden="true" />
            <div className="kami-portrait">
                <img src={getBattlePortrait(state.creature)} alt={state.creature.name} />
                <div className="kami-light" />
            </div>
            <p className="mt-6 text-center text-xl font-bold text-white">
                {state.phase === 'reveal' ? `${form?.name ?? state.creature.name}は カミシンカした！` : 'カミダマが 光り輝く…'}
            </p>
            {state.phase === 'reveal' && !form?.image && <p className="mt-2 text-sm text-white/70">進化後の画像は未設定です</p>}
        </div>
    );
}
