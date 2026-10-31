export type PinTarget =
  | { type: "entry" | "feed" | "topic"; id: string }
  | { type: "site"; hostname: string };
export type PinChange = { target: PinTarget; pinned: boolean };
export function pinKey(target: PinTarget): string {
  return `${target.type}:${target.type === "site" ? target.hostname : target.id}`;
}
