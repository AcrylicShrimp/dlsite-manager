import languages from "$lib/data/cldr-48.2.3/languages.json";
import territories from "$lib/data/cldr-48.2.3/territories.json";
import scripts from "$lib/data/cldr-48.2.3/scripts.json";

const languageNames: Record<string, string> =
  languages.main.en.localeDisplayNames.languages;
const territoryNames: Record<string, string> =
  territories.main.en.localeDisplayNames.territories;
const scriptNames: Record<string, string> =
  scripts.main.en.localeDisplayNames.scripts;
const lookup = (table: Record<string, string>, code: string) =>
  Object.hasOwn(table, code) ? table[code] : undefined;

/** Label lookup only: the original cache/settings code is never normalized. */
export function languageDisplayName(code: string): string {
  const match = /^([a-z]{2,8})(?:-([a-z]{4}))?(?:-([a-z]{2}|[0-9]{3}))?$/i.exec(
    code.replaceAll("_", "-"),
  );
  if (!match) return code;
  const language = match[1].toLowerCase();
  const script = match[2]
    ? match[2][0].toUpperCase() + match[2].slice(1).toLowerCase()
    : null;
  const region = match[3]?.toUpperCase() ?? null;
  const normalized = [language, script, region].filter(Boolean).join("-");
  const exact = lookup(languageNames, normalized);
  if (exact) return exact;
  const base = lookup(languageNames, language);
  if (!base) return code;
  const scriptLabel = script ? lookup(scriptNames, script) : null;
  const regionLabel = region ? lookup(territoryNames, region) : null;
  if ((script && !scriptLabel) || (region && !regionLabel)) return code;
  const scriptLanguage = script
    ? lookup(languageNames, `${language}-${script}`)
    : null;
  const modifiers = [scriptLanguage ? null : scriptLabel, regionLabel].filter(
    Boolean,
  );
  const label = scriptLanguage ?? base;
  return modifiers.length ? `${label} (${modifiers.join(", ")})` : label;
}

export function languageCodeLabel(code: string): string {
  const name = languageDisplayName(code);
  return name === code ? code : `${name} (${code})`;
}
