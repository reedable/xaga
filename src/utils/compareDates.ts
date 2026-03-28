export default function compareDates(
  a?: Date | number | string,
  b?: Date | number | string,
) {
  if (a === undefined) return 1;
  if (b === undefined) return -1;
  const _a = a instanceof Date ? a : new Date(a);
  const _b = b instanceof Date ? b : new Date(b);
  if (isNaN(_a.getTime())) return 1;
  if (isNaN(_b.getTime())) return -1;
  return _a.getTime() - _b.getTime();
}
