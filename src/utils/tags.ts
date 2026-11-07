import type { TagMerge } from "../domain";

// Match Rust's per-character Unicode lowercase conversion for tag identity.
export function tagNameComparison(name: string) {
  return Array.from(name.trim(), (character) => character.toLowerCase()).join(
    ""
  );
}

// Retained committed mappings carry history selections through successive merges.
export function resolveMergedTagIds(ids: Iterable<string>, merges: TagMerge[]) {
  const redirects = new Map<string, string>();
  for (const merge of merges) {
    for (const id of [merge.sourceId, merge.destinationId]) {
      if (id !== merge.survivorId) redirects.set(id, merge.survivorId);
    }
  }
  return [
    ...new Set(
      [...ids].map((id) => {
        const seen = new Set<string>();
        while (redirects.has(id) && !seen.has(id)) {
          seen.add(id);
          id = redirects.get(id)!;
        }
        return id;
      })
    ),
  ];
}

export function retainTagMerges(current: TagMerge[], incoming: TagMerge[]) {
  const losingId = (merge: TagMerge) =>
    merge.sourceId === merge.survivorId ? merge.destinationId : merge.sourceId;
  const merges = new Map(current.map((merge) => [losingId(merge), merge]));
  for (const merge of incoming) merges.set(losingId(merge), merge);
  return [...merges.values()];
}
