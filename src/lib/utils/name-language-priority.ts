import { DEFAULT_LANGUAGE_PRIORITY } from "$lib/model/constants";

export function sameLanguagePriority(
  a: readonly string[],
  b: readonly string[],
) {
  return a.length === b.length && a.every((code, index) => code === b[index]);
}

// SQLite BINARY compares UTF-8, whose order follows Unicode scalar values.
// JS's default UTF-16 sort differs for supplementary characters in unknown keys.
function compareCodes(a: string, b: string) {
  const left = [...a],
    right = [...b];
  for (let i = 0; i < Math.min(left.length, right.length); i++) {
    const difference = left[i].codePointAt(0)! - right[i].codePointAt(0)!;
    if (difference) return difference;
  }
  return left.length - right.length;
}

export function buildLanguagePriority(
  saved: readonly string[],
  observed: readonly string[],
) {
  const unlisted = [...new Set([...DEFAULT_LANGUAGE_PRIORITY, ...observed])]
    .filter((code) => code.trim() && !saved.includes(code))
    .sort(compareCodes);
  return [...saved, ...unlisted];
}

export function moveLanguagePriority(
  codes: readonly string[],
  code: string,
  direction: -1 | 1,
) {
  const index = codes.indexOf(code),
    target = index + direction;
  const result = [...codes];
  if (index >= 0 && target >= 0 && target < codes.length) {
    [result[index], result[target]] = [result[target], result[index]];
  }
  return result;
}

export function languagePriorityPatch(
  draft: readonly string[],
  baseline: readonly string[],
): { languagePriority?: string[] } {
  return sameLanguagePriority(draft, baseline)
    ? {}
    : { languagePriority: [...draft] };
}

export function mergeLanguageDiscovery(
  saved: readonly string[],
  draft: readonly string[],
  baseline: readonly string[],
  observed: readonly string[],
) {
  const nextBaseline = buildLanguagePriority(saved, observed);
  return {
    baseline: nextBaseline,
    draft: sameLanguagePriority(draft, baseline)
      ? [...nextBaseline]
      : [...draft, ...nextBaseline.filter((code) => !draft.includes(code))],
  };
}
