// SPDX-License-Identifier: MPL-2.0
import { describe, expect, test } from "vitest";
import dayjs from "dayjs";
import {
  bounds,
  days,
  formatRange,
  fromBounds,
  limit,
  parseRange,
  presetOf,
  presetRange,
  shift,
  type DayRange,
} from "./range";

/** A range from two `YYYY-MM-DD` dates. */
const range = (from: string, to: string): DayRange => ({
  from: dayjs(from),
  to: dayjs(to),
});
const text = (r: DayRange) => formatRange(r);

describe("presets", () => {
  test("a week runs from Monday to Sunday, also from a Sunday", () => {
    expect(text(presetRange("week", dayjs("2026-10-25")))).toBe(
      "2026-10-19..2026-10-25",
    );
    expect(text(presetRange("week", dayjs("2026-10-26")))).toBe(
      "2026-10-26..2026-11-01",
    );
  });

  test("months and years end on their last day", () => {
    expect(text(presetRange("month", dayjs("2028-02-10")))).toBe(
      "2028-02-01..2028-02-29",
    );
    expect(text(presetRange("year", dayjs("2026-06-15 13:45")))).toBe(
      "2026-01-01..2026-12-31",
    );
  });

  test("a range is recognised as its preset", () => {
    expect(presetOf(range("2026-10-24", "2026-10-24"))).toBe("day");
    expect(presetOf(range("2026-10-19", "2026-10-25"))).toBe("week");
    expect(presetOf(range("2026-02-01", "2026-02-28"))).toBe("month");
    expect(presetOf(range("2026-10-20", "2026-10-25"))).toBeUndefined();
  });
});

describe("previous and next", () => {
  test("move by the preset, across the end of a year", () => {
    expect(text(shift(range("2026-12-01", "2026-12-31"), 1))).toBe(
      "2027-01-01..2027-01-31",
    );
    expect(text(shift(range("2027-01-01", "2027-01-31"), -1))).toBe(
      "2026-12-01..2026-12-31",
    );
    expect(text(shift(range("2026-01-01", "2026-12-31"), 1))).toBe(
      "2027-01-01..2027-12-31",
    );
    // From a 31-day month to a 30-day one.
    expect(text(shift(range("2026-03-01", "2026-03-31"), 1))).toBe(
      "2026-04-01..2026-04-30",
    );
  });

  test("move other ranges by their length", () => {
    expect(text(shift(range("2026-10-20", "2026-10-22"), 1))).toBe(
      "2026-10-23..2026-10-25",
    );
    expect(text(shift(range("2026-10-24", "2026-10-24"), -1))).toBe(
      "2026-10-23..2026-10-23",
    );
  });
});

describe("limits", () => {
  test("a range is shortened to the longest, keeping its start", () => {
    const year = range("2026-01-01", "2026-12-31");
    expect(days(year)).toBe(365);
    expect(text(limit(year, 31))).toBe("2026-01-01..2026-01-31");
    expect(limit(year, 3660)).toBe(year);
  });
});

describe("the URL", () => {
  test("a range survives the URL", () => {
    const r = range("2026-03-29", "2026-04-04");
    expect(text(parseRange(text(r))!)).toBe(text(r));
  });

  test("invalid ranges are ignored", () => {
    for (const bad of [
      null,
      "",
      "2026-10-24",
      "2026-02-30..2026-03-01",
      "2026-10-24..2026-10-23",
      "24.10.2026..25.10.2026",
      "2026-10-24..2026-10-25..2026-10-26",
    ]) {
      expect(parseRange(bad), String(bad)).toBeUndefined();
    }
  });
});

describe("bounds", () => {
  test("cover whole local days, also of 23 and 25 hours", () => {
    const hours = (r: DayRange) => {
      const { min, max } = bounds(r);
      return (max - min) / 3_600_000;
    };
    expect(hours(range("2026-03-29", "2026-03-29"))).toBe(23);
    expect(hours(range("2026-10-25", "2026-10-25"))).toBe(25);
    expect(hours(range("2026-10-24", "2026-10-26"))).toBe(73);
  });

  test("a zoomed interval is rounded to the days it touches", () => {
    const min = dayjs("2026-10-24 10:00").valueOf();
    expect(text(fromBounds(min, dayjs("2026-10-25 00:00").valueOf()))).toBe(
      "2026-10-24..2026-10-24",
    );
    expect(text(fromBounds(min, dayjs("2026-10-26 09:00").valueOf()))).toBe(
      "2026-10-24..2026-10-26",
    );
  });
});
