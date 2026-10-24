// SPDX-License-Identifier: MPL-2.0
import dayjs, { type Dayjs } from "dayjs";

/** A range of local days, both inclusive (each at the start of its day). */
export interface DayRange {
  from: Dayjs;
  to: Dayjs;
}

export type Preset = "day" | "week" | "month" | "year";

const format = "YYYY-MM-DD";

/** Number of days in the range. */
export function days(range: DayRange): number {
  return range.to.diff(range.from, "day") + 1;
}

/** Monday of the week containing the day. */
function monday(day: Dayjs): Dayjs {
  return day.subtract((day.day() + 6) % 7, "day");
}

/** The day, week (Monday to Sunday), month or year containing the day. */
export function presetRange(preset: Preset, day: Dayjs): DayRange {
  const d = day.startOf("day");
  switch (preset) {
    case "day":
      return { from: d, to: d };
    case "week":
      return { from: monday(d), to: monday(d).add(6, "day") };
    case "month":
      return { from: d.startOf("month"), to: d.endOf("month").startOf("day") };
    case "year":
      return { from: d.startOf("year"), to: d.endOf("year").startOf("day") };
  }
}

/** The preset the range is exactly, if any. */
export function presetOf(range: DayRange): Preset | undefined {
  return (["day", "week", "month", "year"] as Preset[]).find((p) => {
    const r = presetRange(p, range.from);
    return r.from.isSame(range.from, "day") && r.to.isSame(range.to, "day");
  });
}

/** The previous (-1) or next (+1) range: the neighbouring preset period, or the same length. */
export function shift(range: DayRange, direction: -1 | 1): DayRange {
  const preset = presetOf(range);
  if (preset && preset !== "day") {
    return presetRange(preset, range.from.add(direction, preset));
  }
  const n = days(range) * direction;
  return { from: range.from.add(n, "day"), to: range.to.add(n, "day") };
}

/** The range shortened to at most `maxDays`, keeping its start. */
export function limit(range: DayRange, maxDays: number): DayRange {
  return days(range) > maxDays
    ? { from: range.from, to: range.from.add(maxDays - 1, "day") }
    : range;
}

/** `YYYY-MM-DD..YYYY-MM-DD`, as kept in the URL. */
export function formatRange(range: DayRange): string {
  return `${range.from.format(format)}..${range.to.format(format)}`;
}

function parseDay(text: string | undefined): Dayjs | undefined {
  if (!text || !/^\d{4}-\d{2}-\d{2}$/.test(text)) return undefined;
  const day = dayjs(text);
  return day.format(format) === text ? day : undefined;
}

/** The range in the URL, if it is valid: two dates, the second not before the first. */
export function parseRange(text: string | null): DayRange | undefined {
  const parts = (text ?? "").split("..");
  if (parts.length !== 2) return undefined;
  const [from, to] = parts.map(parseDay);
  return from && to && !to.isBefore(from) ? { from, to } : undefined;
}

/** Milliseconds of the start of `from` and of the end of `to`. */
export function bounds(range: DayRange): { min: number; max: number } {
  return { min: range.from.valueOf(), max: range.to.add(1, "day").valueOf() };
}

/** The whole days covering a time span in milliseconds. */
export function fromBounds(min: number, max: number): DayRange {
  return {
    from: dayjs(min).startOf("day"),
    to: dayjs(Math.max(min, max - 1)).startOf("day"),
  };
}
