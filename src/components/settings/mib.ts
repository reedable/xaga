const MIB = 1048576n;

export function bytesToMib(value: string): string {
  const bytes = BigInt(value);
  const fraction = ((bytes % MIB) * 5n ** 20n)
    .toString()
    .padStart(20, "0")
    .replace(/0+$/, "");
  return `${bytes / MIB}${fraction ? `.${fraction}` : ""}`;
}

export function mibToBytes(value: string): string | null {
  if (!/^\d+(\.\d+)?$/.test(value)) return null;
  const [whole, fraction = ""] = value.split(".");
  const digits = fraction.replace(/0+$/, "");
  if (digits.length > 20) return null;
  const denominator = 10n ** BigInt(digits.length);
  const bytes = (BigInt(whole) * denominator + BigInt(digits || "0")) * MIB;
  return bytes % denominator === 0n ? (bytes / denominator).toString() : null;
}
