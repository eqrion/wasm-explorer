export function debounce(f: (...args: unknown[]) => void, ms: number) {
  let timeout: number | null;
  return function (this: unknown, ...args: unknown[]) {
    let fresh = timeout == null;
    if (!fresh) clearTimeout(timeout!);
    timeout = setTimeout(() => {
      timeout = null;
      if (!fresh) f.apply(this, args);
    }, ms);
    if (fresh) f.apply(this, args);
  };
}

export function fuzzy<T extends { displayName: string }>(
  items: T[],
  search: string,
): T[] {
  if (!search.trim()) return items;

  const searchLower = search.toLowerCase();

  const scored = items
    .map((item) => {
      const nameLower = item.displayName.toLowerCase();
      let score = 0;
      let searchIndex = 0;
      let consecutiveBonus = 0;

      for (
        let i = 0;
        i < nameLower.length && searchIndex < searchLower.length;
        i++
      ) {
        if (nameLower[i] === searchLower[searchIndex]) {
          score += 1;
          consecutiveBonus += 1;
          score += consecutiveBonus; // Bonus for consecutive matches

          // Bonus for word boundary matches
          if (
            i === 0 ||
            nameLower[i - 1] === " " ||
            nameLower[i - 1] === "-" ||
            nameLower[i - 1] === "_"
          ) {
            score += 3;
          }

          searchIndex++;
        } else {
          consecutiveBonus = 0;
        }
      }

      // Only include if all search characters were matched
      return searchIndex === searchLower.length ? { item, score } : null;
    })
    .filter(Boolean) as { item: T; score: number }[];

  return scored.sort((a, b) => b.score - a.score).map(({ item }) => item);
}
