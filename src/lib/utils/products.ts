import { creditFieldDefinitions, productTypeCodeDetails } from "$lib/model/constants";
import type {
  Product,
  ProductCreditField,
  ProductCreditGroup,
  ProductTypeInfo,
} from "$lib/model/types";

export function productType(product: Product): ProductTypeInfo {
  return productTypeFromCode(product.workType);
}

export function productTypeFromCode(workType: string | null): ProductTypeInfo {
  const raw = workType?.trim() || "";
  const upper = raw.toUpperCase();
  const knownType = productTypeCodeDetails[upper];

  if (knownType) {
    return {
      label: knownType.label,
      tone: knownType.tone,
      tooltip: `${knownType.label}: ${knownType.description}. DLsite code ${upper}.`,
    };
  }

  const normalized = raw.toLowerCase().replace(/[\s_-]+/g, "");

  if (matchesAny(normalized, ["voicecomic", "vcomic"])) {
    return productTypeFallback(
      raw,
      "Voice Comic",
      "voice-comic",
      "Comic with voice/audio presentation",
    );
  }

  if (matchesAny(normalized, ["sou", "audio", "voice", "asmr", "music", "sound"])) {
    return productTypeFallback(raw, "Audio", "audio", "Audio-like product type");
  }

  if (matchesAny(normalized, ["mov", "movie", "video", "anime"])) {
    return productTypeFallback(raw, "Video", "video", "Video-like product type");
  }

  if (
    matchesAny(normalized, [
      "gam",
      "game",
      "rpg",
      "adv",
      "action",
      "acn",
      "puzzle",
      "puz",
      "quiz",
      "simulation",
      "slg",
      "shooter",
      "stg",
      "tabletop",
      "typing",
    ])
  ) {
    return productTypeFallback(raw, "Game", "game", "Game-like product type");
  }

  if (
    matchesAny(normalized, [
      "cg",
      "icg",
      "image",
      "illust",
      "comic",
      "com",
      "manga",
      "mng",
      "gekiga",
      "pdf",
      "novel",
      "digitalnovel",
      "book",
    ])
  ) {
    return productTypeFallback(
      raw,
      "Image / Comic",
      "image",
      "Image, comic, manga, or reading-material product type",
    );
  }

  if (matchesAny(normalized, ["software", "tool", "utility", "etc", "other"])) {
    return productTypeFallback(raw, "Other", "other", "Tool or other product type");
  }

  return {
    label: raw || "Other",
    tone: "other",
    tooltip: raw
      ? `Unrecognized product type from DLsite: ${raw}.`
      : "Product type is not available from DLsite.",
  };
}

function productTypeFallback(
  raw: string,
  fallbackLabel: string,
  tone: string,
  description: string,
): ProductTypeInfo {
  const label = raw || fallbackLabel;

  return {
    label,
    tone,
    tooltip: raw ? `${label}: ${description}.` : `${fallbackLabel}: ${description}.`,
  };
}

function matchesAny(value: string, needles: string[]) {
  return needles.some((needle) => value.includes(needle));
}

export function ageTone(value: string | null) {
  switch (value) {
    case "all":
      return "all";
    case "r15":
      return "r15";
    case "r18":
      return "r18";
    default:
      return "unknown";
  }
}

export function ageLabel(value: string | null) {
  switch (value) {
    case "all":
      return "All Ages";
    case "r15":
      return "R-15";
    case "r18":
      return "R-18";
    default:
      return "";
  }
}

export function ageTooltip(value: string | null) {
  switch (value) {
    case "all":
      return "DLsite rating: all ages.";
    case "r15":
      return "DLsite rating: R-15.";
    case "r18":
      return "DLsite rating: R-18.";
    default:
      return "DLsite rating is unknown.";
  }
}

export function creditText(group: ProductCreditGroup) {
  return group.names.join(", ");
}

export function productCreditFields(product: {
  makerName: string | null;
  creditGroups: ProductCreditGroup[];
}): ProductCreditField[] {
  return creditFieldDefinitions.map((definition) => {
    const value =
      definition.key === "maker"
        ? product.makerName?.trim() || ""
        : creditTextForKind(product, definition.key);

    return {
      ...definition,
      value: value || "-",
      missing: !value,
    };
  });
}

export function creditTextForKind(product: { creditGroups: ProductCreditGroup[] }, kind: string) {
  const group = product.creditGroups?.find((item) => item.kind === kind);
  return group ? creditText(group).trim() : "";
}

export function creditTooltip(field: ProductCreditField) {
  return field.missing ? `${field.label}: Not available` : `${field.label}: ${field.value}`;
}
