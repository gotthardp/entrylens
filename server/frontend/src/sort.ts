// SPDX-License-Identifier: MPL-2.0

/** Compares rows by a text, in the browser's language order; a missing text sorts first. */
export function byText<T>(get: (row: T) => string | null | undefined) {
  return (a: T, b: T) => (get(a) ?? "").localeCompare(get(b) ?? "");
}

/** Compares rows by a number; a missing one counts as `missing` (by default the lowest). */
export function byNumber<T>(
  get: (row: T) => number | null | undefined,
  missing = -Infinity,
) {
  return (a: T, b: T) => {
    const [x, y] = [get(a) ?? missing, get(b) ?? missing];
    return x < y ? -1 : x > y ? 1 : 0;
  };
}

/** A header click sorts ascending, the next one descending, and so on (never unsorted). */
export const sortDirections: ("ascend" | "descend")[] = ["ascend", "descend"];
