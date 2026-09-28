import type {
  Product,
  ProductDetail,
  ProductFilterFacets,
} from "$lib/model/types";
import { productIsLocalOnly, productType } from "$lib/utils/products";

// Local Storybook projections only. Production queries remain owned by Rust.
export type PreviewFilters = {
  sort: string;
  accounts: string[];
  sources: string[];
  ages: string[];
  types: string[];
  makers: string[];
  tags: string[];
  excludedTags: string[];
};
export function emptyFilters(): PreviewFilters {
  return {
    sort: "latestPurchaseDesc",
    accounts: [],
    sources: [],
    ages: [],
    types: [],
    makers: [],
    tags: [],
    excludedTags: [],
  };
}
export function toggle(values: string[], value: string) {
  return values.includes(value)
    ? values.filter((x) => x !== value)
    : [...values, value];
}
export function cycleTag(filters: PreviewFilters, name: string) {
  if (filters.tags.includes(name)) {
    filters.tags = filters.tags.filter((x) => x !== name);
    filters.excludedTags = [...filters.excludedTags, name];
  } else if (filters.excludedTags.includes(name))
    filters.excludedTags = filters.excludedTags.filter((x) => x !== name);
  else filters.tags = [...filters.tags, name];
}
export function filterCount(filters: PreviewFilters) {
  return (
    filters.accounts.length +
    filters.sources.length +
    filters.ages.length +
    filters.types.length +
    filters.makers.length +
    filters.tags.length +
    filters.excludedTags.length
  );
}
export function queryProducts(
  products: Product[],
  search: string,
  filters: PreviewFilters,
) {
  const query = search.trim().toLowerCase();
  const any = (selected: string[], value: string) =>
    !selected.length || selected.includes(value);
  return products
    .filter((p) => {
      const local = productIsLocalOnly(p);
      const tags = p.customTags.map((t) => t.name);
      const haystack = [
        p.title,
        p.makerName,
        p.workId,
        ...p.creditGroups.flatMap((g) => g.names),
        ...tags,
        ...p.owners.map((o) => o.label),
        local ? "local only not owned" : "owned",
      ]
        .join(" ")
        .toLowerCase();
      const tone = productType(p).tone;
      return (
        haystack.includes(query) &&
        (!filters.accounts.length ||
          p.owners.some((o) => filters.accounts.includes(o.accountId))) &&
        any(filters.sources, local ? "localOnly" : "owned") &&
        any(filters.ages, p.ageCategory ?? "") &&
        any(filters.types, tone === "voice-comic" ? "image" : tone) &&
        any(filters.makers, p.makerName ?? "") &&
        (!filters.tags.length || filters.tags.some((t) => tags.includes(t))) &&
        !filters.excludedTags.some((t) => tags.includes(t))
      );
    })
    .sort((a, b) => {
      const dateKey =
        filters.sort === "publishedAtDesc"
          ? "publishedAt"
          : "latestPurchasedAt";
      const byDate =
        filters.sort === "titleAsc"
          ? 0
          : (b[dateKey] ?? "").localeCompare(a[dateKey] ?? "");
      return (
        byDate ||
        a.title.localeCompare(b.title) ||
        a.workId.localeCompare(b.workId)
      );
    });
}
export function facetsFor(
  products: Product[],
  search = "",
  filters = emptyFilters(),
): ProductFilterFacets {
  const count = (values: string[]) =>
    [...new Set(values)]
      .sort()
      .map((name) => ({
        name,
        count: values.filter((x) => x === name).length,
      }))
      .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name));
  // Match the production facet boundary: omit only the facet's own selection.
  const makerProducts = queryProducts(products, search, {
    ...filters,
    makers: [],
  });
  const tagProducts = queryProducts(products, search, {
    ...filters,
    tags: [],
    excludedTags: [],
  });
  return {
    makers: count(
      makerProducts.flatMap((p) => (p.makerName ? [p.makerName] : [])),
    ),
    customTags: count(
      tagProducts.flatMap((p) => p.customTags.map((t) => t.name)),
    ),
  };
}
export function makeDetail(product: Product): ProductDetail {
  const local = productIsLocalOnly(product);
  return {
    ...product,
    titleVariants: local
      ? []
      : [
          { language: "en_US", value: product.title },
          { language: "ja_JP", value: "静かな夜の物語 — サンプル作品" },
        ],
    makerId: local ? null : "RG01234567",
    makerNames: product.makerName
      ? [
          { language: "en_US", value: product.makerName },
          { language: "ja_JP", value: "北窓スタジオ" },
        ]
      : [],
    contentSizeBytes: local ? null : 482_000_000,
    registeredAt: local ? null : "2026-07-20T00:00:00Z",
    lastDetailSyncAt: "2026-09-28T05:00:00Z",
    tags: local
      ? []
      : [
          { class: "genre", name: "Slice of life" },
          { class: "genre", name: "Relaxing" },
        ],
  };
}
export function parseTags(value: string, existing: string[]): string[] {
  const names = new Map<string, string>();
  for (const part of [...existing, ...value.split(/[,\n]/)]) {
    const name = part.trim().replace(/\s+/g, " ");
    if ([...name].length > 64)
      throw new Error("Each tag must be 64 characters or fewer.");
    if (name && !names.has(name.toLowerCase()))
      names.set(name.toLowerCase(), name);
  }
  return [...names.values()].sort((a, b) => a.localeCompare(b));
}
