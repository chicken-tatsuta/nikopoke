import Peer, { type DataConnection } from 'peerjs';
import type { DeckPokemon } from '../types/pokemon';
import type { ActionWire, BattleStateWire } from './engine';

export type OnlineRole = 'host' | 'guest';
export type OnlineStatus =
    | 'idle'
    | 'hosting'
    | 'joining'
    | 'connected'
    | 'ready'
    | 'in_battle'
    | 'disconnected'
    | 'error';

export interface OnlineSessionSnapshot {
    role: OnlineRole | null;
    status: OnlineStatus;
    localPeerId: string | null;
    hostPeerId: string | null;
    remotePeerId: string | null;
    remoteUserId: string | null;
    localDeck: DeckPokemon[] | null;
    remoteDeck: DeckPokemon[] | null;
    localSelectedDeck: DeckPokemon[] | null;
    remoteSelectedDeck: DeckPokemon[] | null;
    latestState: BattleStateWire | null;
    latestActions: ActionWire[] | null;
    error: string | null;
}

type OnlineMessage =
    | {
        type: 'hello';
        role: OnlineRole;
        peerId: string;
        userId?: string | null;
        deck: DeckPokemon[];
    }
    | {
        type: 'start_battle';
    }
    | {
        type: 'team_selected';
        deck: DeckPokemon[];
    }
    | {
        type: 'battle_init';
        state: BattleStateWire;
    }
    | {
        type: 'action_submit';
        actorId: OnlineRole;
        action: ActionWire;
    }
    | {
        type: 'battle_update';
        state: BattleStateWire;
        actions: ActionWire[];
    };

type OnlineSessionEvent =
    | {
        type: 'snapshot';
        snapshot: OnlineSessionSnapshot;
    }
    | {
        type: 'start_battle';
    }
    | {
        type: 'team_selected';
        deck: DeckPokemon[];
    }
    | {
        type: 'battle_init';
        state: BattleStateWire;
    }
    | {
        type: 'remote_action';
        actorId: OnlineRole;
        action: ActionWire;
    }
    | {
        type: 'battle_update';
        state: BattleStateWire;
        actions: ActionWire[];
    }
    | {
        type: 'peer_left';
    }
    | {
        type: 'error';
        message: string;
    };

interface OnlineSessionState {
    role: OnlineRole | null;
    status: OnlineStatus;
    peer: Peer | null;
    connection: DataConnection | null;
    localPeerId: string | null;
    hostPeerId: string | null;
    remotePeerId: string | null;
    localUserId: string | null;
    remoteUserId: string | null;
    localDeck: DeckPokemon[] | null;
    remoteDeck: DeckPokemon[] | null;
    localSelectedDeck: DeckPokemon[] | null;
    remoteSelectedDeck: DeckPokemon[] | null;
    latestState: BattleStateWire | null;
    latestActions: ActionWire[] | null;
    error: string | null;
}

const listeners = new Set<(event: OnlineSessionEvent) => void>();

const PEER_OPTIONS = {
    debug: 2,
    config: {
        iceServers: [
            { urls: 'stun:stun.l.google.com:19302' },
            { urls: 'stun:global.stun.twilio.com:3478' },
        ],
    },
};
const JOIN_TIMEOUT_MS = 30000;
const ROOM_CODE_CHARS = 'abcdefghjkmnpqrstuvwxyz23456789';

export function normalizeRoomCode(code: string): string {
    return code.trim().toLowerCase().replace(/\s+/g, '-');
}

export function validateRoomCode(code: string): string | null {
    if (code.length < 4) {
        return 'ルームコードは4文字以上で入力してください。';
    }
    if (code.length > 24) {
        return 'ルームコードは24文字以内で入力してください。';
    }
    if (!/^[a-z0-9-]+$/.test(code)) {
        return 'ルームコードは半角英数字とハイフンだけ使えます。';
    }
    if (code.startsWith('-') || code.endsWith('-') || code.includes('--')) {
        return 'ハイフンは先頭・末尾・連続では使えません。';
    }
    return null;
}

function createReadableRoomCode(): string {
    const suffix = Array.from({ length: 4 }, () => (
        ROOM_CODE_CHARS[Math.floor(Math.random() * ROOM_CODE_CHARS.length)]
    )).join('');
    return `niko-${suffix}`;
}

function createInitialState(): OnlineSessionState {
    return {
        role: null,
        status: 'idle',
        peer: null,
        connection: null,
        localPeerId: null,
        hostPeerId: null,
        remotePeerId: null,
        localUserId: null,
        remoteUserId: null,
        localDeck: null,
        remoteDeck: null,
        localSelectedDeck: null,
        remoteSelectedDeck: null,
        latestState: null,
        latestActions: null,
        error: null,
    };
}

let session = createInitialState();

function toPlainData<T>(value: T): T {
    if (value instanceof Map) {
        const plainObject: Record<string, unknown> = {};
        for (const [key, entryValue] of value.entries()) {
            plainObject[String(key)] = toPlainData(entryValue);
        }
        return plainObject as T;
    }

    if (Array.isArray(value)) {
        return value.map((entry) => toPlainData(entry)) as T;
    }

    if (value && typeof value === 'object') {
        const plainObject: Record<string, unknown> = {};
        for (const [key, entryValue] of Object.entries(value)) {
            plainObject[key] = toPlainData(entryValue);
        }
        return plainObject as T;
    }

    return value;
}

function cloneDeck(deck: DeckPokemon[]): DeckPokemon[] {
    return deck.map((pokemon) => ({
        ...pokemon,
        moves: [...pokemon.moves],
        evs: pokemon.evs ? { ...pokemon.evs } : undefined,
    }));
}

function getSnapshot(): OnlineSessionSnapshot {
    return {
        role: session.role,
        status: session.status,
        localPeerId: session.localPeerId,
        hostPeerId: session.hostPeerId,
        remotePeerId: session.remotePeerId,
        remoteUserId: session.remoteUserId,
        localDeck: session.localDeck ? cloneDeck(session.localDeck) : null,
        remoteDeck: session.remoteDeck ? cloneDeck(session.remoteDeck) : null,
        localSelectedDeck: session.localSelectedDeck ? cloneDeck(session.localSelectedDeck) : null,
        remoteSelectedDeck: session.remoteSelectedDeck ? cloneDeck(session.remoteSelectedDeck) : null,
        latestState: session.latestState,
        latestActions: session.latestActions ? [...session.latestActions] : null,
        error: session.error,
    };
}

function emit(event: OnlineSessionEvent): void {
    listeners.forEach((listener) => listener(event));
}

function emitSnapshot(): void {
    emit({
        type: 'snapshot',
        snapshot: getSnapshot(),
    });
}

function setError(message: string): void {
    session.error = message;
    session.status = 'error';
    emitSnapshot();
    emit({
        type: 'error',
        message,
    });
}

function sendMessage(message: OnlineMessage): void {
    if (!session.connection || !session.connection.open) {
        throw new Error('接続が確立していません。');
    }
    session.connection.send(toPlainData(message));
}

function sendHello(): void {
    if (!session.role || !session.localPeerId || !session.localDeck) {
        return;
    }
    sendMessage({
        type: 'hello',
        role: session.role,
        peerId: session.localPeerId,
        userId: session.localUserId,
        deck: cloneDeck(session.localDeck),
    });
}

function handleIncomingMessage(raw: unknown): void {
    if (!raw || typeof raw !== 'object' || !('type' in raw)) {
        return;
    }

    const message = raw as OnlineMessage;
    switch (message.type) {
        case 'hello':
            session.remotePeerId = message.peerId;
            session.remoteUserId = message.userId ?? null;
            session.remoteDeck = cloneDeck(message.deck);
            if (session.status !== 'in_battle') {
                session.status = 'ready';
            }
            emitSnapshot();
            return;
        case 'start_battle':
            session.status = 'in_battle';
            emitSnapshot();
            emit({ type: 'start_battle' });
            return;

            case 'team_selected':
    session.remoteSelectedDeck = cloneDeck(message.deck);
    emitSnapshot();
    emit({
        type: 'team_selected',
        deck: cloneDeck(message.deck),
    });
    return;

        case 'battle_init':
            session.status = 'in_battle';
            session.latestState = toPlainData(message.state);
            session.latestActions = null;
            emitSnapshot();
            emit({
                type: 'battle_init',
                state: toPlainData(message.state),
            });
            return;
        case 'action_submit':
            emit({
                type: 'remote_action',
                actorId: message.actorId,
                action: message.action,
            });
            return;
        case 'battle_update':
            session.status = 'in_battle';
            session.latestState = toPlainData(message.state);
            session.latestActions = toPlainData([...message.actions]);
            emitSnapshot();
            emit({
                type: 'battle_update',
                state: toPlainData(message.state),
                actions: toPlainData([...message.actions]),
            });
            return;
    }
}

function attachConnection(connection: DataConnection): void {
    console.log('[p2p] attach connection', connection.peer);

    session.connection = connection;
    session.remotePeerId = connection.peer;
    emitSnapshot();

    connection.on('open', () => {
        console.log('[p2p] connection open', connection.peer);

        session.connection = connection;
        session.remotePeerId = connection.peer;
        session.status = 'connected';
        session.error = null;
        emitSnapshot();
        sendHello();
    });

    connection.on('data', (message) => {
        console.log('[p2p] data received', message);
        handleIncomingMessage(message);
    });

    connection.on('close', () => {
        console.warn('[p2p] connection closed', connection.peer);

        if (session.connection !== connection) {
            return;
        }
        session.connection = null;
        session.status = 'disconnected';
        emitSnapshot();
        emit({ type: 'peer_left' });
    });

    connection.on('error', (error) => {
        console.error('[p2p] connection error', error);
        setError(error.message);
    });
}

function setupPeerCommon(peer: Peer): void {
    peer.on('open', (peerId) => {
        console.log('[p2p] peer open', peerId);
    });

    peer.on('error', (error) => {
        console.error('[p2p] peer error', error);
        setError(error.message);
    });

    peer.on('disconnected', () => {
        console.warn('[p2p] peer disconnected');

        if (session.peer !== peer || peer.destroyed || session.status === 'error') {
            return;
        }

        if (!session.connection?.open) {
            session.status = session.role === 'host'
                ? 'hosting'
                : session.role === 'guest'
                  ? 'joining'
                  : 'disconnected';
            emitSnapshot();
        }

        try {
            peer.reconnect();
        } catch (error) {
            const message = error instanceof Error ? error.message : 'PeerJS の再接続に失敗しました。';
            setError(message);
        }
    });

    peer.on('close', () => {
        console.warn('[p2p] peer closed');
        if (session.peer !== peer || session.connection?.open || session.status === 'error') {
            return;
        }
        session.status = 'disconnected';
        emitSnapshot();
    });
}

export function subscribeOnlineSession(
    listener: (event: OnlineSessionEvent) => void,
): () => void {
    listeners.add(listener);
    listener({
        type: 'snapshot',
        snapshot: getSnapshot(),
    });
    return () => {
        listeners.delete(listener);
    };
}

export function getOnlineSessionSnapshot(): OnlineSessionSnapshot {
    return getSnapshot();
}

export function clearOnlineSession(): void {
    try {
        session.connection?.close();
    } catch {
        // ignore cleanup errors
    }
    try {
        session.peer?.destroy();
    } catch {
        // ignore cleanup errors
    }
    session = createInitialState();
    emitSnapshot();
}

export async function createHostSession(
    deck: DeckPokemon[],
    userId?: string | null,
    requestedRoomCode?: string,
): Promise<string> {
    clearOnlineSession();
    const roomCode = normalizeRoomCode(requestedRoomCode || createReadableRoomCode());
    const validationError = validateRoomCode(roomCode);
    if (validationError) {
        throw new Error(validationError);
    }
    session.role = 'host';
    session.status = 'hosting';
    session.localDeck = cloneDeck(deck);
    session.localUserId = userId ?? null;
    emitSnapshot();

    return await new Promise<string>((resolve, reject) => {
        const peer = new Peer(roomCode, PEER_OPTIONS);
        session.peer = peer;
        emitSnapshot();
        setupPeerCommon(peer);

        peer.on('open', (peerId) => {
            session.localPeerId = peerId;
            session.hostPeerId = peerId;
            session.error = null;
            emitSnapshot();
            resolve(peerId);
        });

        peer.on('connection', (connection) => {
            if (session.connection && session.connection.open) {
                connection.on('open', () => {
                    connection.close();
                });
                return;
            }
            attachConnection(connection);
        });

        peer.on('error', (error) => {
            reject(error);
        });
    });
}

export async function joinHostSession(
    hostPeerId: string,
    deck: DeckPokemon[],
    userId?: string | null,
): Promise<void> {
    clearOnlineSession();
    const roomCode = normalizeRoomCode(hostPeerId);
    const validationError = validateRoomCode(roomCode);
    if (validationError) {
        throw new Error(validationError);
    }
    session.role = 'guest';
    session.status = 'joining';
    session.hostPeerId = roomCode;
    session.localDeck = cloneDeck(deck);
    session.localUserId = userId ?? null;
    emitSnapshot();

    return await new Promise<void>((resolve, reject) => {
        let resolved = false;

        const rejectOnce = (error: Error): void => {
            if (resolved) {
                return;
            }
            resolved = true;
            setError(error.message);
            reject(error);
        };

        const timeoutId = window.setTimeout(() => {
            rejectOnce(
                new Error('接続がタイムアウトしました。部屋IDが正しいか、ホスト側の画面が開いたままか確認してください。'),
            );
        }, JOIN_TIMEOUT_MS);

        const peer = new Peer(PEER_OPTIONS);
        session.peer = peer;
        emitSnapshot();
        setupPeerCommon(peer);

        peer.on('open', (peerId) => {
            console.log('[p2p] guest peer open', peerId);
            session.localPeerId = peerId;
            emitSnapshot();

            const connection = peer.connect(session.hostPeerId!, {
                reliable: true,
            });

            console.log('[p2p] connecting to host', session.hostPeerId);
            attachConnection(connection);

            connection.on('open', () => {
                if (resolved) {
                    return;
                }
                resolved = true;
                window.clearTimeout(timeoutId);
                resolve();
            });

            connection.on('error', (error) => {
                rejectOnce(error);
            });
        });

        peer.on('error', (error) => {
            rejectOnce(error);
        });
    });
}

export function startOnlineBattle(): void {
    if (session.role !== 'host') {
        throw new Error('ホストのみ対戦を開始できます。');
    }
    if (!session.connection?.open || !session.remoteDeck) {
        throw new Error('相手の接続完了後に対戦を開始してください。');
    }
    session.status = 'in_battle';
    session.latestActions = null;
    emitSnapshot();
    sendMessage({ type: 'start_battle' });
}

export function sendTeamSelected(deck: DeckPokemon[]): void {
    if (!session.role) {
        throw new Error('オンラインセッションが初期化されていません。');
    }
    if (deck.length !== 3) {
        throw new Error('選出は3匹である必要があります。');
    }

    session.localSelectedDeck = cloneDeck(deck);
    emitSnapshot();

    sendMessage({
        type: 'team_selected',
        deck: cloneDeck(deck),
    });
}

export function sendBattleInit(state: BattleStateWire): void {
    session.status = 'in_battle';
    const plainState = toPlainData(state);
    session.latestState = plainState;
    session.latestActions = null;
    emitSnapshot();
    sendMessage({
        type: 'battle_init',
        state: plainState,
    });
}

export function sendBattleUpdate(
    state: BattleStateWire,
    actions: ActionWire[],
): void {
    const plainState = toPlainData(state);
    const plainActions = toPlainData([...actions]);
    session.status = 'in_battle';
    session.latestState = plainState;
    session.latestActions = plainActions;
    emitSnapshot();
    sendMessage({
        type: 'battle_update',
        state: plainState,
        actions: plainActions,
    });
}

export function sendPlayerAction(action: ActionWire): void {
    if (!session.role) {
        throw new Error('オンラインセッションが初期化されていません。');
    }
    sendMessage({
        type: 'action_submit',
        actorId: session.role,
        action,
    });
}
