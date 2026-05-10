import { cn } from '../lib/cn';
import { useEffect, useRef } from 'react';
import { Zap, Shield, Heart, AlertTriangle, Sparkles, ArrowRightLeft, Badge } from 'lucide-react';

interface LogEntry {
    text: string;
    type: 'player-move' | 'ai-move' | 'damage' | 'effect' | 'status' | 'switch' | 'ability' | 'info';
}

interface BattleLogProps {
    logs: string[];
    currentTurn: number;
    className?: string;
    compact?: boolean;
}

// Parse log message to determine its type and enhance display
function parseLogEntry(log: string): LogEntry {
    const lowerLog = log.toLowerCase();

    // Detect switch actions
    if (log.includes('交代') || log.includes('を繰り出した') || log.includes('を引っ込めた')) {
        return { text: log, type: 'switch' };
    }

    if (log.includes('特性')) {
        return { text: log, type: 'ability' };
    }

    // Detect player moves - look for patterns like 'playerの' or player pokemon names
    if (log.includes('🔵') || log.match(/^.+は\s+.+\s+を使った！$/)) {
        // Check if it seems like player move
        return { text: log, type: 'player-move' };
    }

    // Detect AI moves
    if (log.includes('🔴')) {
        return { text: log, type: 'ai-move' };
    }

    // Detect damage
    if (log.includes('ダメージ') || log.includes('HP') || log.includes('倒れた') || log.includes('瀕死')) {
        return { text: log, type: 'damage' };
    }

    // Detect status effects
    if (log.includes('状態になった') || log.includes('やけど') || log.includes('まひ') ||
        log.includes('どく') || log.includes('こおり') || log.includes('ねむり') ||
        log.includes('こんらん') || log.includes('ひるみ')) {
        return { text: log, type: 'status' };
    }

    // Detect stat changes and effects
    if (log.includes('上がった') || log.includes('下がった') || log.includes('効果') ||
        lowerLog.includes('急所') || log.includes('ばつぐん') || log.includes('いまひとつ')) {
        return { text: log, type: 'effect' };
    }

    // Default to info for everything else  
    return { text: log, type: 'info' };
}

// Get icon for log entry type
function getLogIcon(type: LogEntry['type']) {
    switch (type) {
        case 'player-move':
        case 'ai-move':
            return <Zap className="size-3.5 shrink-0" />;
        case 'damage':
            return <Heart className="size-3.5 shrink-0" />;
        case 'effect':
            return <Sparkles className="size-3.5 shrink-0" />;
        case 'status':
            return <AlertTriangle className="size-3.5 shrink-0" />;
        case 'switch':
            return <ArrowRightLeft className="size-3.5 shrink-0" />;
        case 'ability':
            return <Badge className="size-3.5 shrink-0" />;
        default:
            return <Shield className="size-3.5 shrink-0" />;
    }
}

// Get styling for log entry type
function getLogStyle(type: LogEntry['type']) {
    switch (type) {
        case 'player-move':
            return 'bg-blue-500/10 border-l-blue-500 text-blue-100';
        case 'ai-move':
            return 'bg-red-500/10 border-l-red-500 text-red-100';
        case 'damage':
            return 'bg-orange-500/10 border-l-orange-400 text-orange-100';
        case 'effect':
            return 'bg-yellow-500/10 border-l-yellow-400 text-yellow-100';
        case 'status':
            return 'bg-purple-500/10 border-l-purple-400 text-purple-100';
        case 'switch':
            return 'bg-emerald-500/10 border-l-emerald-400 text-emerald-100';
        case 'ability':
            return 'bg-cyan-500/10 border-l-cyan-300 text-cyan-100';
        default:
            return 'bg-slate-500/10 border-l-slate-400 text-slate-200';
    }
}

export function BattleLog({ logs, currentTurn, className, compact = false }: BattleLogProps) {
    const entriesRef = useRef<HTMLDivElement>(null);
    // Parse all logs
    const parsedEntries = logs.map(parseLogEntry);

    useEffect(() => {
        if (entriesRef.current) {
            entriesRef.current.scrollTop = entriesRef.current.scrollHeight;
        }
    }, [logs]);

    return (
        <div className={cn(
            'flex min-h-0 flex-col overflow-hidden rounded-xl border border-slate-700/50 bg-slate-800/30',
            className
        )}>
            {/* Header */}
            <div className={cn(
                'flex items-center justify-between border-b border-slate-700/50 bg-slate-800/50',
                compact ? 'px-2 py-1.5' : 'px-4 py-2',
            )}>
                <h3 className={cn('font-medium text-slate-200', compact ? 'text-xs' : 'text-sm')}>バトルログ</h3>
                <span className={cn(
                    'rounded-full bg-indigo-500/20 font-medium tabular-nums text-indigo-300',
                    compact ? 'px-1.5 py-0.5 text-[10px]' : 'px-2 py-0.5 text-xs',
                )}>
                    ターン {currentTurn}
                </span>
            </div>

            {/* Log entries */}
            <div ref={entriesRef} className={cn('min-h-0 flex-1 overflow-y-auto', compact ? 'space-y-1 p-1.5' : 'space-y-1 p-2')}>
                {parsedEntries.length === 0 ? (
                    <p className={cn('px-2 py-4 text-center text-slate-500', compact ? 'text-xs' : 'text-sm')}>
                        バトル開始！
                    </p>
                ) : (
                    parsedEntries.map((entry, i) => (
                        <div
                            key={i}
                            className={cn(
                                'flex items-start rounded-lg border-l-2',
                                compact ? 'gap-1 px-1.5 py-1 text-[11px] leading-snug' : 'gap-2 px-3 py-1.5 text-sm',
                                getLogStyle(entry.type)
                            )}
                        >
                            {getLogIcon(entry.type)}
                            <span className="text-pretty">{entry.text}</span>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
}

// Compact action summary for showing last turn's actions
interface ActionSummaryProps {
    playerMove?: { name: string; type: string };
    aiMove?: { name: string; type: string };
    getTypeColor: (type: string) => string;
    className?: string;
}

export function ActionSummary({ playerMove, aiMove, getTypeColor, className }: ActionSummaryProps) {
    if (!playerMove && !aiMove) return null;

    return (
        <div className={cn(
            'flex items-center justify-center gap-6 rounded-lg border border-slate-700/50 bg-slate-800/40 px-4 py-2',
            className
        )}>
            {/* Player action */}
            {playerMove && (
                <div className="flex items-center gap-2">
                    <span className="text-xs text-blue-400">🔵 あなた</span>
                    <span
                        className="rounded px-1.5 py-0.5 text-xs text-white"
                        style={{ backgroundColor: getTypeColor(playerMove.type) }}
                    >
                        {playerMove.type}
                    </span>
                    <span className="font-medium text-white">{playerMove.name}</span>
                </div>
            )}

            {playerMove && aiMove && (
                <span className="text-slate-600">vs</span>
            )}

            {/* AI action */}
            {aiMove && (
                <div className="flex items-center gap-2">
                    <span className="text-xs text-red-400">🔴 相手</span>
                    <span
                        className="rounded px-1.5 py-0.5 text-xs text-white"
                        style={{ backgroundColor: getTypeColor(aiMove.type) }}
                    >
                        {aiMove.type}
                    </span>
                    <span className="font-medium text-white">{aiMove.name}</span>
                </div>
            )}
        </div>
    );
}
