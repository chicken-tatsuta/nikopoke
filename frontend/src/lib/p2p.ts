import Peer, { type DataConnection, type PeerJSOption } from 'peerjs';
import type { DeckPokemon } from '../types/pokemon';
import type { ActionWire, BattleStateWire } from './engine';
import { registerRoomCode, lookupRoomCode } from './roomCodes';

export type OnlineRole = 'host' | 'guest';
export type OnlineStatus =
    | 'idle'
    | 'hosting'
    | 'joining'
    | 'connected'
    | 'ready'
    | 'in_battle'
    | 'disconnected'
    | 'reconnecting'
    | 'error';

export interface OnlineSessionSnapshot {
    role: OnlineRole | null;
    status: OnlineStatus;
    localPeerId: string | null;
    hostPeerId: string | null;
    remotePeerId: string | null;
    remoteUserId: string | null;
    remoteUserName: string | null;
    localDeck: DeckPokemon[] | null;
    remoteDeck: DeckPokemon[] | null;
    localSelectedDeck: DeckPokemon[] | null;
    remoteSelectedDeck: DeckPokemon[] | null;
    latestState: BattleStateWire | null;
    latestActions: ActionWire[] | null;
    error: string | null;
}

type OnlineMessage =
    | { type: 'hello'; role: OnlineRole; peerId: string; userId?: string | null; userName?: string | null; deck: DeckPokemon[] }
    | { type: 'start_battle' }
    | { type: 'team_selected'; deck: DeckPokemon[] }
    | { type: 'battle_init'; state: BattleStateWire }
    | { type: 'action_submit'; actorId: OnlineRole; action: ActionWire }
    | { type: 'battle_update'; state: BattleStateWire; actions: ActionWire[] }
    | { type: 'ping'; ts: number }
    | { type: 'pong'; ts: number };

type OnlineSessionEvent =
    | { type: 'snapshot'; snapshot: OnlineSessionSnapshot }
    | { type: 'start_battle' }
    | { type: 'team_selected'; deck: DeckPokemon[] }
    | { type: 'battle_init'; state: BattleStateWire }
    | { type: 'remote_action'; actorId: OnlineRole; action: ActionWire }
    | { type: 'battle_update'; state: BattleStateWire; actions: ActionWire[] }
    | { type: 'peer_left' }
    | { type: 'reconnected' }
    | { type: 'error'; message: string };

interface OnlineSessionState {
    role: OnlineRole | null;
    status: OnlineStatus;
    peer: Peer | null;
    connection: DataConnection | null;
    localPeerId: string | null;
    hostPeerId: string | null;
    remotePeerId: string | null;
    localUserId: string | null;
    localUserName: string | null;
    remoteUserId: string | null;
    remoteUserName: string | null;
    localDeck: DeckPokemon[] | null;
    remoteDeck: DeckPokemon[] | null;
    localSelectedDeck: DeckPokemon[] | null;
    remoteSelectedDeck: DeckPokemon[] | null;
    latestState: BattleStateWire | null;
    latestActions: ActionWire[] | null;
    error: string | null;
}

const listeners = new Set<(event: OnlineSessionEvent) => void>();
const JOIN_TIMEOUT_MS = 45000;
const ROOM_LOOKUP_RETRY_COUNT = 12;
const ROOM_LOOKUP_RETRY_DELAY_MS = 500;
const PEER_CONNECT_RETRY_COUNT = 5;
const PEER_CONNECT_RETRY_DELAY_MS = 1200;
const CREATE_ROOM_CODE_RETRY_COUNT = 12;

function delay(ms: number): Promise<void> {
    return new Promise((resolve) => {
        window.setTimeout(resolve, ms);
    });
}

// --- ICE Servers (TURN / STUN) ---

const CLOUDFLARE_TURN_KEY_ID = import.meta.env.VITE_CLOUDFLARE_TURN_KEY_ID as string | undefined;
const CLOUDFLARE_TURN_API_TOKEN = import.meta.env.VITE_CLOUDFLARE_TURN_API_TOKEN as string | undefined;

async function buildIceServers(): Promise<RTCIceServer[]> {
    const servers: RTCIceServer[] = [
        { urls: 'stun:stun.l.google.com:19302' },
        { urls: 'stun:stun.cloudflare.com:3478' },
        { urls: 'stun:global.stun.twilio.com:3478' },
    ];

    if (CLOUDFLARE_TURN_KEY_ID && CLOUDFLARE_TURN_API_TOKEN) {
        try {
            const response = await fetch(
                `https://rtc.live.cloudflare.com/v1/turn/keys/${CLOUDFLARE_TURN_KEY_ID}/credentials/generate`,
                {
                    method: 'POST',
                    headers: { Authorization: `Bearer ${CLOUDFLARE_TURN_API_TOKEN}` },
                    body: JSON.stringify({}),
                },
            );
            if (response.ok) {
                const data = (await response.json()) as {
                    iceServers: RTCIceServer[];
                };
                servers.push(...data.iceServers);
            } else {
                console.warn('[p2p] Cloudflare TURN credential fetch failed:', response.status);
            }
        } catch (error) {
            console.warn('[p2p] Cloudflare TURN credential fetch error:', error);
        }
    }

    return servers;
}

// --- PeerJS signaling server config ---

function buildPeerOptions(iceServers: RTCIceServer[]): PeerJSOption {
    const host = import.meta.env.VITE_PEERJS_HOST as string | undefined;
    const port = import.meta.env.VITE_PEERJS_PORT as string | undefined;
    const path = import.meta.env.VITE_PEERJS_PATH as string | undefined;
    const key = import.meta.env.VITE_PEERJS_KEY as string | undefined;

    const options: PeerJSOption = {
        debug: 2,
        config: { iceServers },
    };

    if (host) {
        options.host = host;
        options.port = port ? Number(port) : 443;
        if (path) options.path = path;
        if (key) options.key = key;
    }

    return options;
}

// --- Keepalive ---

const KEEPALIVE_INTERVAL_MS = 10_000;
const KEEPALIVE_TIMEOUT_MS = 30_000;
let keepaliveTimer: ReturnType<typeof setInterval> | null = null;
let lastPongTs = 0;

function startKeepalive(): void {
    stopKeepalive();
    lastPongTs = Date.now();
    keepaliveTimer = setInterval(() => {
        if (!session.connection?.open) {
            return;
        }
        if (Date.now() - lastPongTs > KEEPALIVE_TIMEOUT_MS) {
            console.warn('[p2p] keepalive timeout, closing connection');
            session.connection?.close();
            return;
        }
        try {
            session.connection.send({ type: 'ping', ts: Date.now() } satisfies OnlineMessage);
        } catch {
            // will be caught by connection.on('close')
        }
    }, KEEPALIVE_INTERVAL_MS);
}

function stopKeepalive(): void {
    if (keepaliveTimer !== null) {
        clearInterval(keepaliveTimer);
        keepaliveTimer = null;
    }
}

// --- Reconnection ---

const MAX_RECONNECT_ATTEMPTS = 3;
const RECONNECT_BASE_DELAY_MS = 1_000;
let reconnectAttempts = 0;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleReconnect(): void {
    if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS || session.role === null) {
        return;
    }

    const delay = RECONNECT_BASE_DELAY_MS * Math.pow(2, reconnectAttempts);
    reconnectAttempts += 1;
    session.status = 'reconnecting';
    emitSnapshot();

    console.log(`[p2p] reconnect attempt ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS} in ${delay}ms`);

    reconnectTimer = setTimeout(async () => {
        if (!session.peer || !session.role) {
            return;
        }
        try {
            if (session.peer.disconnected) {
                await new Promise<void>((resolve, reject) => {
                    session.peer!.reconnect();
                    const onOpen = () => {
                        cleanup();
                        resolve();
                    };
                    const onError = (error: Error) => {
                        cleanup();
                        reject(error);
                    };
                    const cleanup = () => {
                        session.peer?.off('open', onOpen);
                        session.peer?.off('error', onError);
                    };
                    session.peer?.on('open', onOpen);
                    session.peer?.on('error', onError);
                });
            }

            if (session.role === 'guest' && session.hostPeerId) {
                const targetPeerId = session.remotePeerId ?? await lookupRoomCodeWithRetry(session.hostPeerId);
                if (!targetPeerId) {
                    throw new Error('ホストの接続先が見つかりませんでした。');
                }
                const conn = session.peer.connect(targetPeerId, { reliable: true });
                attachConnection(conn);
                await new Promise<void>((resolve, reject) => {
                    const timeoutId = setTimeout(() => reject(new Error('timeout')), 10_000);
                    conn.on('open', () => {
                        clearTimeout(timeoutId);
                        resolve();
                    });
                    conn.on('error', (error) => {
                        clearTimeout(timeoutId);
                        reject(error);
                    });
                });
            }

            reconnectAttempts = 0;
            session.error = null;
            emitSnapshot();
            emit({ type: 'reconnected' });
        } catch (error) {
            console.warn('[p2p] reconnect failed:', error);
            scheduleReconnect();
        }
    }, delay);
}

function cancelReconnect(): void {
    if (reconnectTimer !== null) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
    }
    reconnectAttempts = 0;
}

// --- Core utilities ---

let session: OnlineSessionState = createInitialState();

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
        localUserName: null,
        remoteUserId: null,
        remoteUserName: null,
        localDeck: null,
        remoteDeck: null,
        localSelectedDeck: null,
        remoteSelectedDeck: null,
        latestState: null,
        latestActions: null,
        error: null,
    };
}

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
        remoteUserName: session.remoteUserName,
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
    emit({ type: 'snapshot', snapshot: getSnapshot() });
}

function setError(message: string): void {
    session.error = message;
    session.status = 'error';
    cancelReconnect();
    emitSnapshot();
    emit({ type: 'error', message });
}

// --- Send with backpressure handling ---

function sendMessage(message: OnlineMessage): boolean {
    if (!session.connection || !session.connection.open) {
        return false;
    }
    try {
        session.connection.send(toPlainData(message));
        return true;
    } catch (error) {
        console.error('[p2p] send error:', error);
        return false;
    }
}

// --- Handlers ---

function sendHello(): void {
    if (!session.role || !session.localPeerId || !session.localDeck) {
        return;
    }
    sendMessage({
        type: 'hello',
        role: session.role,
        peerId: session.localPeerId,
        userId: session.localUserId,
        userName: session.localUserName,
        deck: cloneDeck(session.localDeck),
    });
}

function handleIncomingMessage(raw: unknown): void {
    if (!raw || typeof raw !== 'object' || !('type' in raw)) {
        return;
    }

    const message = raw as OnlineMessage;

    if (message.type === 'ping') {
        sendMessage({ type: 'pong', ts: message.ts });
        return;
    }
    if (message.type === 'pong') {
        lastPongTs = Date.now();
        return;
    }

    switch (message.type) {
        case 'hello':
            session.remotePeerId = message.peerId;
            session.remoteUserId = message.userId ?? null;
            session.remoteUserName = message.userName ?? null;
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
            emit({ type: 'team_selected', deck: cloneDeck(message.deck) });
            return;
        case 'battle_init':
            session.status = 'in_battle';
            session.latestState = toPlainData(message.state);
            session.latestActions = null;
            emitSnapshot();
            emit({ type: 'battle_init', state: toPlainData(message.state) });
            return;
        case 'action_submit':
            emit({ type: 'remote_action', actorId: message.actorId, action: message.action });
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

    cancelReconnect();

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
        startKeepalive();
    });

    connection.on('data', (data) => {
        handleIncomingMessage(data);
    });

    connection.on('close', () => {
        console.warn('[p2p] connection closed', connection.peer);

        stopKeepalive();

        if (session.connection !== connection) {
            return;
        }
        session.connection = null;

        if (session.status === 'joining') {
            emitSnapshot();
            return;
        }

        if (session.status === 'in_battle' || session.status === 'connected' || session.status === 'ready') {
            session.status = 'disconnected';
            emitSnapshot();
            emit({ type: 'peer_left' });
            scheduleReconnect();
        } else {
            session.status = 'disconnected';
            emitSnapshot();
            emit({ type: 'peer_left' });
        }
    });

    connection.on('error', (error) => {
        console.error('[p2p] connection error', error);
        if (session.status === 'joining' && isRetryablePeerError(error)) {
            return;
        }
        if (session.connection === connection) {
            setError(error.message);
        }
    });
}

function setupPeerCommon(peer: Peer): void {
    peer.on('open', (peerId) => {
        console.log('[p2p] peer open', peerId);
    });

    peer.on('error', (error) => {
        console.error('[p2p] peer error', error);
        if (session.status === 'joining' && isRetryablePeerError(error)) {
            return;
        }
        setError(error.message);
    });

    peer.on('disconnected', () => {
        console.warn('[p2p] peer disconnected');

        if (session.peer !== peer || peer.destroyed || session.status === 'error' || session.status === 'reconnecting') {
            return;
        }

        if (!session.connection?.open) {
            session.status = session.role === 'host'
                ? 'hosting'
                : session.role === 'guest'
                  ? 'joining'
                  : 'disconnected';
            emitSnapshot();
            scheduleReconnect();
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

// --- Public API ---

export function normalizeRoomCode(code: string): string {
    return code.trim().toLowerCase().replace(/\s+/g, '-');
}

export function validateRoomCode(code: string): string | null {
    if (code.length < 4) return 'ルームコードは4文字以上で入力してください。';
    if (code.length > 24) return 'ルームコードは24文字以内で入力してください。';
    if (!/^[a-z0-9-]+$/.test(code)) return 'ルームコードは半角英数字とハイフンだけ使えます。';
    if (code.startsWith('-') || code.endsWith('-') || code.includes('--')) return 'ハイフンは先頭・末尾・連続では使えません。';
    return null;
}

function createReadableRoomCode(): string {
    return Math.floor(Math.random() * 10000).toString().padStart(4, '0');
}

async function lookupRoomCodeWithRetry(roomCode: string): Promise<string | null> {
    for (let attempt = 0; attempt <= ROOM_LOOKUP_RETRY_COUNT; attempt += 1) {
        const peerId = await lookupRoomCode(roomCode);
        if (peerId) {
            if (attempt > 0) {
                console.log('[p2p] room lookup succeeded after retry', { roomCode, attempt });
            }
            return peerId;
        }
        if (attempt < ROOM_LOOKUP_RETRY_COUNT) {
            await delay(ROOM_LOOKUP_RETRY_DELAY_MS);
        }
    }
    return null;
}

function isRetryablePeerError(error: Error): boolean {
    const type = 'type' in error ? String((error as Error & { type?: unknown }).type ?? '') : '';
    const message = error.message.toLowerCase();
    return (
        type === 'peer-unavailable' ||
        type === 'network' ||
        message.includes('could not connect') ||
        message.includes('peer-unavailable') ||
        message.includes('is not available')
    );
}

function isRoomCodeTakenError(error: unknown): boolean {
    const message = error instanceof Error ? error.message : String(error);
    return message.includes('既に使われています') || message.toLowerCase().includes('already');
}



export function subscribeOnlineSession(listener: (event: OnlineSessionEvent) => void): () => void {
    listeners.add(listener);
    listener({ type: 'snapshot', snapshot: getSnapshot() });
    return () => { listeners.delete(listener); };
}

export function getOnlineSessionSnapshot(): OnlineSessionSnapshot {
    return getSnapshot();
}

export function clearOnlineSession(): void {
    stopKeepalive();
    cancelReconnect();
    try { session.connection?.close(); } catch { /* ignore */ }
    try { session.peer?.destroy(); } catch { /* ignore */ }
    session = createInitialState();
    emitSnapshot();
}

export async function createHostSession(
    deck: DeckPokemon[],
    userId?: string | null,
    userName?: string | null,
    requestedRoomCode?: string,
): Promise<string> {
    clearOnlineSession();

    const fixedRoomCode = requestedRoomCode ? normalizeRoomCode(requestedRoomCode) : null;
    if (fixedRoomCode) {
        const validationError = validateRoomCode(fixedRoomCode);
        if (validationError) throw new Error(validationError);
    }

    session.role = 'host';
    session.status = 'hosting';
    session.localDeck = cloneDeck(deck);
    session.localUserId = userId ?? null;
    session.localUserName = userName ?? null;
    emitSnapshot();

    const iceServers = await buildIceServers();
    const peerOptions = buildPeerOptions(iceServers);

    const attempts = fixedRoomCode ? 1 : CREATE_ROOM_CODE_RETRY_COUNT;
    let lastError: Error | null = null;

    for (let attempt = 0; attempt < attempts; attempt += 1) {
        const roomCode = fixedRoomCode ?? createReadableRoomCode();
        const validationError = validateRoomCode(roomCode);
        if (validationError) throw new Error(validationError);
        session.status = 'hosting';
        session.error = null;
        emitSnapshot();

        // Use a random UUID as the actual PeerID to avoid collisions.
        const peerId = crypto.randomUUID();

        const peer = new Peer(peerId, peerOptions);
        session.peer = peer;
        emitSnapshot();
        setupPeerCommon(peer);

        try {
            return await new Promise<string>((resolve, reject) => {
                peer.on('open', async () => {
                    try {
                        // Register the code → peerId mapping (no-op if Supabase not configured).
                        await registerRoomCode(roomCode, peerId);
                    } catch (error) {
                        peer.destroy();
                        const message = error instanceof Error ? error.message : 'ルーム登録に失敗しました。';
                        reject(new Error(message));
                        return;
                    }

                    session.localPeerId = peerId;
                    session.hostPeerId = roomCode; // share the user-friendly code, not the UUID.
                    session.error = null;
                    emitSnapshot();
                    resolve(roomCode); // return the user-friendly code.
                });

                peer.on('connection', (connection) => {
                    if (session.connection && session.connection.open) {
                        connection.on('open', () => connection.close());
                        return;
                    }
                    attachConnection(connection);
                });

                peer.on('error', (error) => {
                    reject(error);
                });
            });
        } catch (error) {
            lastError = error instanceof Error ? error : new Error('ルーム作成に失敗しました。');
            try { peer.destroy(); } catch { /* ignore */ }
            if (!fixedRoomCode && isRoomCodeTakenError(lastError) && attempt < attempts - 1) {
                console.warn('[p2p] generated room code was already taken, retrying', { roomCode, attempt });
                continue;
            }
            setError(lastError.message);
            throw lastError;
        }
    }

    const message = lastError?.message ?? 'ルームコードの自動生成に失敗しました。もう一度試してください。';
    setError(message);
    throw new Error(message);
}

export async function joinHostSession(
    hostPeerId: string,
    deck: DeckPokemon[],
    userId?: string | null,
    userName?: string | null,
): Promise<void> {
    clearOnlineSession();

    const roomCode = normalizeRoomCode(hostPeerId);
    const validationError = validateRoomCode(roomCode);
    if (validationError) throw new Error(validationError);

    session.role = 'guest';
    session.status = 'joining';
    session.hostPeerId = roomCode;
    session.localDeck = cloneDeck(deck);
    session.localUserId = userId ?? null;
    session.localUserName = userName ?? null;
    emitSnapshot();

    const iceServers = await buildIceServers();
    const peerOptions = buildPeerOptions(iceServers);

    // Resolve the room code to an actual PeerID via Supabase.
    const lookedUpPeerId = await lookupRoomCodeWithRetry(roomCode);
    if (lookedUpPeerId) {
        console.log('[p2p] room code resolved', { roomCode, peerId: lookedUpPeerId });
    } else {
        throw new Error(
            `ルームコード "${roomCode}" が見つかりませんでした。コードが正しいか、ホストがルームを作成したままかを確認してください。`,
        );
    }
    const resolvedPeerId = lookedUpPeerId;

    return await new Promise<void>((resolve, reject) => {
        let resolved = false;
        let connectionAttempt = 0;
        let connectToHost: (() => void) | null = null;
        let retryScheduledForAttempt = 0;

        const rejectOnce = (error: Error): void => {
            if (resolved) return;
            resolved = true;
            cancelReconnect();
            window.clearTimeout(timeoutId);
            setError(error.message);
            reject(error);
        };

        const timeoutId = window.setTimeout(() => {
            rejectOnce(
                new Error('接続がタイムアウトしました。部屋IDが正しいか、ホスト側の画面が開いたままか確認してください。'),
            );
        }, JOIN_TIMEOUT_MS);

        const peer = new Peer(peerOptions);
        session.peer = peer;
        emitSnapshot();
            setupPeerCommon(peer);

        peer.on('open', (peerId) => {
            console.log('[p2p] guest peer open', peerId);
            session.localPeerId = peerId;
            emitSnapshot();

            const connect = () => {
                if (resolved || peer.destroyed) {
                    return;
                }
                connectionAttempt += 1;
                const connection = peer.connect(resolvedPeerId, {
                    reliable: true,
                });

                console.log('[p2p] connecting to host peer', {
                    roomCode,
                    resolvedPeerId,
                    attempt: connectionAttempt,
                });
                attachConnection(connection);

                let attemptFinished = false;
                const retryConnection = (reason: unknown): void => {
                    if (resolved || attemptFinished) {
                        return;
                    }
                    attemptFinished = true;
                    if (connectionAttempt < PEER_CONNECT_RETRY_COUNT) {
                        retryScheduledForAttempt = connectionAttempt;
                        console.warn('[p2p] connection attempt failed, retrying', {
                            roomCode,
                            resolvedPeerId,
                            attempt: connectionAttempt,
                            reason,
                        });
                        window.setTimeout(connect, PEER_CONNECT_RETRY_DELAY_MS);
                        return;
                    }
                    const message = reason instanceof Error
                        ? reason.message
                        : 'ホストへの接続に失敗しました。もう一度ルームを作り直して試してください。';
                    rejectOnce(new Error(message));
                };

                connection.on('open', () => {
                    if (resolved) return;
                    attemptFinished = true;
                    resolved = true;
                    window.clearTimeout(timeoutId);
                    resolve();
                });

                connection.on('error', (error) => {
                    if (isRetryablePeerError(error)) {
                        retryConnection(error);
                        return;
                    }
                    rejectOnce(error);
                });

                connection.on('close', () => {
                    if (!resolved && session.status === 'joining') {
                        retryConnection(new Error('接続が開く前に切断されました。'));
                    }
                });
            };

            connectToHost = connect;
            connect();
        });

        peer.on('error', (error) => {
            if (
                !resolved &&
                connectToHost &&
                connectionAttempt > 0 &&
                connectionAttempt < PEER_CONNECT_RETRY_COUNT &&
                retryScheduledForAttempt !== connectionAttempt &&
                isRetryablePeerError(error)
            ) {
                retryScheduledForAttempt = connectionAttempt;
                console.warn('[p2p] peer error during connect, retrying', {
                    roomCode,
                    resolvedPeerId,
                    attempt: connectionAttempt,
                    error,
                });
                window.setTimeout(connectToHost, PEER_CONNECT_RETRY_DELAY_MS);
                return;
            }
            rejectOnce(error);
        });
    });
}

export function startOnlineBattle(): void {
    if (session.role !== 'host') throw new Error('ホストのみ対戦を開始できます。');
    if (!session.connection?.open || !session.remoteDeck) throw new Error('相手の接続完了後に対戦を開始してください。');
    session.status = 'in_battle';
    session.latestActions = null;
    emitSnapshot();
    sendMessage({ type: 'start_battle' });
}

export function sendTeamSelected(deck: DeckPokemon[]): void {
    if (!session.role) throw new Error('オンラインセッションが初期化されていません。');
    if (deck.length !== 3) throw new Error('選出は3匹である必要があります。');
    session.localSelectedDeck = cloneDeck(deck);
    emitSnapshot();
    sendMessage({ type: 'team_selected', deck: cloneDeck(deck) });
}

export function sendBattleInit(state: BattleStateWire): void {
    session.status = 'in_battle';
    const plainState = toPlainData(state);
    session.latestState = plainState;
    session.latestActions = null;
    emitSnapshot();
    sendMessage({ type: 'battle_init', state: plainState });
}

export function sendBattleUpdate(state: BattleStateWire, actions: ActionWire[]): void {
    const plainState = toPlainData(state);
    const plainActions = toPlainData([...actions]);
    session.status = 'in_battle';
    session.latestState = plainState;
    session.latestActions = plainActions;
    emitSnapshot();
    sendMessage({ type: 'battle_update', state: plainState, actions: plainActions });
}

export function sendPlayerAction(action: ActionWire): void {
    if (!session.role) throw new Error('オンラインセッションが初期化されていません。');
    sendMessage({ type: 'action_submit', actorId: session.role, action });
}

/** for testing: exposes current session state */
export function __debugSession(): Readonly<OnlineSessionState> {
    return session;
}
