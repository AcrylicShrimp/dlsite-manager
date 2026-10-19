import { describe, expect, it } from "vitest";
import {
  buildLanguagePriority,
  languagePriorityPatch,
  mergeLanguageDiscovery,
  moveLanguagePriority,
} from "./name-language-priority";

describe("name language priority drafts", () => {
  it("matches backend fallback order and preserves a saved subset on path-only saves", () => {
    const baseline = buildLanguagePriority(
      ["ja_JP"],
      ["pt_BR", "aa_AA", "en_US"],
    );
    expect(baseline).toEqual([
      "ja_JP",
      "aa_AA",
      "en_US",
      "ko_KR",
      "pt_BR",
      "zh_CN",
      "zh_TW",
    ]);
    expect(languagePriorityPatch(baseline, baseline)).toEqual({});
    const moved = moveLanguagePriority(baseline, "aa_AA", -1);
    expect(languagePriorityPatch(moved, baseline)).toEqual({
      languagePriority: moved,
    });
    const undone = moveLanguagePriority(moved, "aa_AA", 1);
    expect(languagePriorityPatch(undone, baseline)).toEqual({});
  });

  it("retains saved codes absent from the cache and sorts unknown keys like SQLite", () => {
    const result = buildLanguagePriority(
      ["missing_code"],
      ["😀", "\ue000", "😀"],
    );
    expect(result[0]).toBe("missing_code");
    expect(result.indexOf("\ue000")).toBeLessThan(result.indexOf("😀"));
    expect(moveLanguagePriority(result, "missing_code", -1)).toEqual(result);
  });

  it("adds discovered codes without marking untouched order dirty or discarding edits", () => {
    const saved = ["ja_JP"],
      baseline = buildLanguagePriority(saved, []);
    const untouched = mergeLanguageDiscovery(saved, baseline, baseline, [
      "pt_BR",
    ]);
    expect(languagePriorityPatch(untouched.draft, untouched.baseline)).toEqual(
      {},
    );
    const edited = moveLanguagePriority(baseline, "en_US", -1);
    const discovered = mergeLanguageDiscovery(saved, edited, baseline, [
      "pt_BR",
    ]);
    expect(discovered.draft.slice(0, 2)).toEqual(edited.slice(0, 2));
    expect(discovered.draft).toContain("pt_BR");
    expect(
      languagePriorityPatch(discovered.draft, discovered.baseline)
        .languagePriority,
    ).toEqual(discovered.draft);
  });
});
