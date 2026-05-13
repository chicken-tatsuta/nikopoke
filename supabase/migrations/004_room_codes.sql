-- Room code → PeerID mapping for P2P online battles.
-- Host registers a friendly room code pointing to their random PeerID.
-- Guest looks up the PeerID by room code.

create table if not exists public.room_codes (
  code text primary key,
  peer_id text not null,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null default now() + interval '5 minutes'
);

alter table public.room_codes enable row level security;

-- Anyone (including anonymous) can read room codes
drop policy if exists "Anyone can read room_codes" on public.room_codes;
create policy "Anyone can read room_codes"
  on public.room_codes
  for select
  to anon, authenticated
  using (true);

-- Anyone (including anonymous) can insert room codes
-- (the RPC handles duplicate-check internally)
drop policy if exists "Anyone can insert room_codes" on public.room_codes;
create policy "Anyone can insert room_codes"
  on public.room_codes
  for insert
  to anon, authenticated
  with check (true);

-- RPC: register a room code (fails if code is taken and not expired)
create or replace function public.register_room_code(p_code text, p_peer_id text)
returns text
language plpgsql
security definer
set search_path = public
as $$
declare
  existing record;
begin
  -- Clean up expired codes for the requested code
  delete from public.room_codes
  where code = p_code and expires_at < now();

  -- Check if the code is still taken (non-expired)
  select * into existing
  from public.room_codes
  where code = p_code;

  if found then
    raise exception 'Room code "%" is already in use.', p_code;
  end if;

  -- Register
  insert into public.room_codes (code, peer_id, expires_at)
  values (p_code, p_peer_id, now() + interval '5 minutes');

  return p_peer_id;
end;
$$;

-- RPC: look up a peer ID by room code (returns null if not found or expired)
create or replace function public.lookup_room_code(p_code text)
returns text
language plpgsql
security definer
set search_path = public
as $$
declare
  result text;
begin
  -- Clean up expired
  delete from public.room_codes
  where code = p_code and expires_at < now();

  select peer_id into result
  from public.room_codes
  where code = p_code;

  return result;
end;
$$;

-- RPC: periodically clean up all expired codes (can be called by cron or client)
create or replace function public.cleanup_expired_room_codes()
returns int
language plpgsql
security definer
set search_path = public
as $$
declare
  deleted_count int;
begin
  delete from public.room_codes
  where expires_at < now();
  get diagnostics deleted_count = row_count;
  return deleted_count;
end;
$$;

grant execute on function public.register_room_code to anon, authenticated;
grant execute on function public.lookup_room_code to anon, authenticated;
grant execute on function public.cleanup_expired_room_codes to anon, authenticated;
