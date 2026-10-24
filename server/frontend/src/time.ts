// SPDX-License-Identifier: MPL-2.0
import { useEffect, useState } from "react";
import dayjs from "dayjs";
import relativeTime from "dayjs/plugin/relativeTime";

dayjs.extend(relativeTime);

/** How long ago a Unix time was, e.g. "3 minutes ago"; "never" for none. */
export function ago(t: number | null | undefined) {
  return t == null ? "never" : dayjs.unix(t).fromNow();
}

/** A Unix time as local date and time. */
export function exact(t: number | null | undefined) {
  return t == null ? undefined : dayjs.unix(t).format("YYYY-MM-DD HH:mm");
}

/** Re-renders the component every minute, so that texts like "3 minutes ago" stay current. */
export function useMinuteTick() {
  const [, setMinute] = useState(0);
  useEffect(() => {
    const timer = setInterval(() => setMinute((m) => m + 1), 60_000);
    return () => clearInterval(timer);
  }, []);
}
