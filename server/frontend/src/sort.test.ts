// SPDX-License-Identifier: MPL-2.0
import { expect, test } from "vitest";
import { byNumber, byText } from "./sort";

type Row = { name: string | null; value: number | null };
const rows: Row[] = [
  { name: "b", value: 2 },
  { name: null, value: null },
  { name: "a", value: 10 },
];
const names = (r: Row[]) => r.map((row) => row.name);

test("texts sort in order, a missing one first", () => {
  expect(names([...rows].sort(byText((r) => r.name)))).toEqual([
    null,
    "a",
    "b",
  ]);
});

test("numbers sort by value, not as text; a missing one first by default", () => {
  expect(names([...rows].sort(byNumber((r) => r.value)))).toEqual([
    null,
    "b",
    "a",
  ]);
});

test("a missing number can sort last", () => {
  const sorted = [...rows].sort(byNumber((r) => r.value, Infinity));
  expect(names(sorted)).toEqual(["b", "a", null]);
});
