import { describe, expect, it } from "vitest";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import {
  languageCodeLabel,
  languageDisplayName,
} from "./language-display-name";

describe("bundled CLDR language names", () => {
  it("matches every byte of the verified pinned source files", () => {
    for (const [file, count, hash] of [
      [
        "languages.json",
        693,
        "a5960d2371bc7d982da7fcfd9a0e237429d3467a5cc76f680bc3af5bcef9fb4e",
      ],
      [
        "territories.json",
        316,
        "158c1d575308f7e46912edbeda435c8fe2ef5dad280798231f3a432e406b1807",
      ],
      [
        "scripts.json",
        220,
        "48ce7e6cf85ca719ec04850786e778bb93fb7c83067a417f56f02d2fa882448b",
      ],
      [
        "LICENSE",
        0,
        "220ba0e1c43b99530d2d5bdb892a99dca0989414f51ab695ecd90163eaa1ec3b",
      ],
    ] as const) {
      const raw = readFileSync(
        new URL(`../data/cldr-48.2.3/${file}`, import.meta.url),
      );
      expect(createHash("sha256").update(raw).digest("hex"), file).toBe(hash);
      if (count)
        expect(
          Object.keys(
            JSON.parse(raw.toString()).main.en.localeDisplayNames[
              file.replace(".json", "")
            ],
          ),
        ).toHaveLength(count);
    }
  });

  it("uses direct names, composed names and script names without changing codes", () => {
    expect(languageCodeLabel("pt_BR")).toBe("Brazilian Portuguese (pt_BR)");
    expect(languageCodeLabel("en_US")).toBe("American English (en_US)");
    expect(languageCodeLabel("ja_JP")).toBe("Japanese (Japan) (ja_JP)");
    expect(languageCodeLabel("zh_TW")).toBe("Chinese (Taiwan) (zh_TW)");
    expect(languageDisplayName("zh_Hant")).toBe("Traditional Chinese");
    expect(languageDisplayName("zh_Hant_TW")).toBe(
      "Traditional Chinese (Taiwan)",
    );
    expect(languageDisplayName("sr_Latn_RS")).toBe("Serbian (Latin, Serbia)");
    expect(languageDisplayName("und")).toBe("Unknown language");
  });

  it("falls back to the exact raw code for unsupported or malformed values", () => {
    for (const code of [
      "xx_XX",
      "en_XX",
      "not/a/locale",
      "x-custom",
      "en-US-u-ca-test",
      "",
      "__proto__",
    ]) {
      expect(languageCodeLabel(code)).toBe(code);
    }
  });
});
