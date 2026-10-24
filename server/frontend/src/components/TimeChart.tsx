// SPDX-License-Identifier: MPL-2.0
import { Alert, Button, Card, DatePicker, Flex, Spin, Typography } from "antd";
import { LeftOutlined, RightOutlined } from "@ant-design/icons";
import {
  Chart as ChartJS,
  LinearScale,
  LineElement,
  PointElement,
  TimeScale,
  Tooltip,
  type ChartData,
} from "chart.js";
import "chartjs-adapter-dayjs-4";
import zoomPlugin from "chartjs-plugin-zoom";
import { Line } from "react-chartjs-2";
import {
  bounds,
  formatRange,
  fromBounds,
  limit,
  shift,
  type DayRange,
} from "../range";

ChartJS.register(
  TimeScale,
  LinearScale,
  PointElement,
  LineElement,
  Tooltip,
  zoomPlugin,
);

export type Point = { x: number; y: number | null };

/** Line colours of the rooms; no red, which marks faults. */
const palette = [
  "#1677ff",
  "#fa8c16",
  "#52c41a",
  "#722ed1",
  "#13c2c2",
  "#eb2f96",
  "#a0d911",
  "#faad14",
  "#2f54eb",
];

/** The colour of the i-th room (rooms sorted by name), the same in every chart. */
export function roomColor(i: number) {
  const color = palette[i % palette.length];
  return { borderColor: color, backgroundColor: color };
}
export type TimeData = ChartData<"line", Point[]>;

/** A displayed interval, in milliseconds. */
export interface View {
  min: number;
  max: number;
}

interface Props {
  /** Title of the vertical axis. */
  title: string;
  /** Title of a second vertical axis on the right, for datasets with `yAxisID: "y2"`. */
  title2?: string;
  range: DayRange;
  onRange: (range: DayRange) => void;
  /** Longest range, in days; zooming out stops there. */
  maxDays: number;
  data: TimeData | undefined;
  error: Error | null;
  /** The interval displayed within the range after a zoom or pan; `undefined` for the range. */
  view: View | undefined;
  onView: (view: View | undefined) => void;
}

/**
 * A line chart on a time axis over a range of local days, without a legend, with previous/next,
 * a date range picker, zooming and panning. A zoom or pan changes the displayed interval; zooming
 * out beyond `maxDays` changes the range instead (rounded to whole days).
 */
export default function TimeChart({
  title,
  title2,
  range,
  onRange,
  maxDays,
  data,
  error,
  view,
  onView,
}: Props) {
  const { min, max } = view ?? bounds(range);
  // The whole days shown, for the date picker and previous/next.
  const shownDays = view ? fromBounds(view.min, view.max) : range;
  const change = (r: DayRange) => {
    const limited = limit(r, maxDays);
    if (view || formatRange(limited) !== formatRange(range)) {
      onView(undefined);
      onRange(limited);
    }
  };
  const onZoomed = ({ chart }: { chart: ChartJS }) => {
    const zoomed = { min: chart.scales.x.min, max: chart.scales.x.max };
    if (zoomed.max - zoomed.min <= maxDays * 24 * 60 * 60 * 1000) {
      onView(zoomed);
    } else {
      change(fromBounds(zoomed.min, zoomed.max));
    }
  };

  return (
    <Card size="small">
      <Flex vertical gap="small">
        <Flex gap="small" wrap align="center">
          <Button
            icon={<LeftOutlined />}
            aria-label="Previous"
            onClick={() => change(shift(shownDays, -1))}
          />
          <Button
            icon={<RightOutlined />}
            aria-label="Next"
            onClick={() => change(shift(shownDays, 1))}
          />
          <DatePicker.RangePicker
            value={[shownDays.from, shownDays.to]}
            allowClear={false}
            onChange={(dates) => {
              const [from, to] = dates ?? [];
              if (from && to)
                change({ from: from.startOf("day"), to: to.startOf("day") });
            }}
          />
          <Typography.Text type="secondary">
            Scroll or drag to zoom, Shift+drag to pan
          </Typography.Text>
        </Flex>
        {error && <Alert type="error" title={error.message} />}
        <div style={{ position: "relative", height: 320 }}>
          {data ? (
            <Line
              // A new interval is a new chart, without the previous zoom.
              key={`${min}-${max}`}
              data={data}
              options={{
                maintainAspectRatio: false,
                animation: false,
                interaction: { mode: "nearest", axis: "x", intersect: false },
                elements: { point: { radius: 0 } },
                scales: {
                  x: {
                    type: "time",
                    min,
                    max,
                    // Day.js formats, 24-hour clock.
                    time: {
                      tooltipFormat: "D MMM YYYY HH:mm",
                      displayFormats: {
                        minute: "HH:mm",
                        hour: "HH:mm",
                        day: "D MMM",
                        week: "D MMM",
                        month: "MMM YYYY",
                        year: "YYYY",
                      },
                    },
                    ticks: { maxRotation: 0, autoSkipPadding: 16 },
                  },
                  y: {
                    beginAtZero: true,
                    title: { display: true, text: title },
                  },
                  ...(title2 && {
                    y2: {
                      position: "right" as const,
                      beginAtZero: true,
                      grid: { drawOnChartArea: false },
                      title: { display: true, text: title2 },
                    },
                  }),
                },
                plugins: {
                  legend: { display: false },
                  zoom: {
                    // Zooming out further than maxDays is cut back to maxDays when it ends.
                    limits: { x: { minRange: 60 * 60 * 1000 } },
                    pan: {
                      enabled: true,
                      mode: "x",
                      modifierKey: "shift",
                      onPanComplete: onZoomed,
                    },
                    zoom: {
                      mode: "x",
                      wheel: { enabled: true },
                      pinch: { enabled: true },
                      drag: { enabled: true },
                      onZoomComplete: onZoomed,
                    },
                  },
                },
              }}
            />
          ) : (
            !error && <Spin />
          )}
        </div>
      </Flex>
    </Card>
  );
}
