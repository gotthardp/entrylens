// SPDX-License-Identifier: MPL-2.0
import type { CSSProperties } from "react";
import "./Toggle.css";

/** Relative luminance of a `#rrggbb` colour, as defined by WCAG. */
function luminance(color: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(color.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** Black or white, whichever contrasts more with the `#rrggbb` background. */
function textOn(background: string): string {
  // Contrast with white (1.05 / (L + 0.05)) exceeds that with black ((L + 0.05) / 0.05) below this.
  return luminance(background) > 0.179 ? "#000" : "#fff";
}

/**
 * A number that shows or hides a line of the chart: filled with the line's colour when shown,
 * underlined in it when hidden (dashed for the dashed entries lines). See `Toggle.css`.
 */
export default function Toggle(props: {
  value: number;
  color: string;
  shown: boolean;
  onToggle: () => void;
  large?: boolean;
  dashed?: boolean;
}) {
  const { value, color, shown, onToggle, large, dashed } = props;
  return (
    <span
      role="switch"
      aria-checked={shown}
      tabIndex={0}
      title={shown ? "Hide in the chart" : "Show in the chart"}
      onClick={onToggle}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onToggle();
        }
      }}
      className={[
        "toggle",
        shown && "shown",
        large && "large",
        dashed && "dashed",
      ]
        .filter(Boolean)
        .join(" ")}
      style={
        {
          "--color": color,
          "--text": textOn(color),
          "--line": `${large ? 3 : 1.5}px`,
        } as CSSProperties
      }
    >
      {value}
    </span>
  );
}
