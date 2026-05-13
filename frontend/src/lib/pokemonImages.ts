const POKEMON_IMAGE_MODULES = import.meta.glob('../../image/*.{png,jpg,jpeg,webp,avif}', {
    eager: true,
    query: '?url',
    import: 'default',
}) as Record<string, string>;

const POKEMON_IMAGE_BY_ID = Object.fromEntries(
    Object.entries(POKEMON_IMAGE_MODULES).map(([path, url]) => {
        const filename = path.split('/').pop() ?? '';
        const id = filename.replace(/\.(png|jpe?g|webp|avif)$/i, '').toLowerCase();
        return [id, url];
    }),
);

function pokemonPortraitFallback(speciesId: string, name?: string): string {
    const seed = speciesId.split('').reduce((sum, char) => sum + char.charCodeAt(0), 0);
    const hue = seed % 360;
    const label = (name ?? speciesId).slice(0, 2);

    return `data:image/svg+xml;utf8,${encodeURIComponent(`
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 160">
            <rect width="160" height="160" rx="28" fill="hsl(${hue} 42% 24%)"/>
            <circle cx="80" cy="64" r="42" fill="hsl(${hue} 54% 42%)"/>
            <path d="M38 126c12-30 72-30 84 0" fill="hsl(${hue} 48% 34%)"/>
            <text x="80" y="92" text-anchor="middle" font-family="sans-serif" font-size="34" font-weight="700" fill="white">${label}</text>
        </svg>
    `)}`;
}

export function getPokemonPortraitSrc(speciesId: string, name?: string): string {
    return POKEMON_IMAGE_BY_ID[speciesId.toLowerCase()] ?? pokemonPortraitFallback(speciesId, name);
}
