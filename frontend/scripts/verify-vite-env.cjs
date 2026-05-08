#!/usr/bin/env node

const REQUIRED_ENV = ['VITE_SUPABASE_URL', 'VITE_SUPABASE_ANON_KEY'];

function getSource(name, viteEnv) {
  if (process.env[name]) return 'process.env';
  if (viteEnv[name]) return 'vite env file';
  return 'missing';
}

function describeUrl(value) {
  if (!value) return { ok: false, reason: 'missing' };

  try {
    const url = new URL(value);
    return {
      ok: url.protocol === 'https:' && url.hostname.endsWith('.supabase.co'),
      protocol: url.protocol,
      host: url.hostname,
    };
  } catch {
    return { ok: false, reason: 'invalid URL' };
  }
}

function describeKey(value) {
  if (!value) return { ok: false, reason: 'missing' };

  const knownPrefix = value.startsWith('sb_publishable_') || value.startsWith('eyJ');

  return {
    ok: value.length >= 32 && knownPrefix,
    length: value.length,
    prefixType: value.startsWith('sb_publishable_')
      ? 'sb_publishable'
      : value.startsWith('eyJ')
        ? 'jwt'
        : 'unknown',
  };
}

async function main() {
  const mode = process.argv[2] || 'production';
  const { loadEnv } = await import('vite');
  const viteEnv = loadEnv(mode, process.cwd(), 'VITE_');
  const env = {
    ...viteEnv,
    ...Object.fromEntries(
      REQUIRED_ENV.map((name) => [name, process.env[name] ?? viteEnv[name]]),
    ),
  };

  console.log(`[env-check] mode=${mode}`);
  console.log(`[env-check] cwd=${process.cwd()}`);
  console.log(`[env-check] node=${process.version}`);
  console.log(`[env-check] vercel_env=${process.env.VERCEL_ENV ?? 'local'}`);
  console.log(`[env-check] vercel_git_ref=${process.env.VERCEL_GIT_COMMIT_REF ?? 'n/a'}`);
  console.log(`[env-check] vercel_url_present=${Boolean(process.env.VERCEL_URL)}`);

  for (const name of REQUIRED_ENV) {
    const value = env[name];
    console.log(`[env-check] ${name}.present=${Boolean(value)}`);
    console.log(`[env-check] ${name}.source=${getSource(name, viteEnv)}`);
  }

  const supabaseUrl = describeUrl(env.VITE_SUPABASE_URL);
  console.log(`[env-check] VITE_SUPABASE_URL.host=${supabaseUrl.host ?? 'n/a'}`);
  console.log(`[env-check] VITE_SUPABASE_URL.valid=${supabaseUrl.ok}`);

  const anonKey = describeKey(env.VITE_SUPABASE_ANON_KEY);
  console.log(`[env-check] VITE_SUPABASE_ANON_KEY.length=${anonKey.length ?? 0}`);
  console.log(`[env-check] VITE_SUPABASE_ANON_KEY.prefix=${anonKey.prefixType ?? 'n/a'}`);
  console.log(`[env-check] VITE_SUPABASE_ANON_KEY.valid_shape=${anonKey.ok}`);

  const missing = REQUIRED_ENV.filter((name) => !env[name]);
  const invalid = [];

  if (!supabaseUrl.ok) invalid.push('VITE_SUPABASE_URL');
  if (!anonKey.ok) invalid.push('VITE_SUPABASE_ANON_KEY');

  if (missing.length > 0 || invalid.length > 0) {
    console.error(`[env-check] failed missing=${missing.join(',') || 'none'} invalid=${invalid.join(',') || 'none'}`);
    process.exit(1);
  }

  console.log('[env-check] ok');
}

main().catch((error) => {
  console.error('[env-check] unexpected failure');
  console.error(error);
  process.exit(1);
});
