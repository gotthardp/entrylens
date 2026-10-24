// SPDX-License-Identifier: MPL-2.0
import { useState } from "react";
import {
  Alert,
  Badge,
  Button,
  Card,
  Col,
  Empty,
  Flex,
  Table,
  Row,
  Statistic,
  theme,
} from "antd";
import { SettingOutlined, WarningFilled } from "@ant-design/icons";
import { Link, useNavigate, useSearchParams } from "react-router";
import dayjs from "dayjs";
import type { Timeline } from "../api/client";
import { useOccupancy, useTimeline } from "../api/queries";
import { siteTitle } from "../site";
import Toggle from "../components/Toggle";
import { byNumber, byText, sortDirections } from "../sort";
import TimeChart, {
  roomColor,
  type Point,
  type TimeData,
  type View,
} from "../components/TimeChart";
import {
  bounds,
  formatRange,
  parseRange,
  presetRange,
  type DayRange,
} from "../range";

/** Longest range of the chart: `MAX_DAILY_DAYS` of the backend's timeline API. */
const MAX_DAYS = 3660;

/** The timeline's `bucket` of a point per day, for ranges over 31 days: the backend's `DAY`. */
const DAY = 86400;

/** The colour of the total's line: black, emphasised, apart from every room's colour. */
const TOTAL_COLOR = "#000";

/**
 * `[time, count]` pairs as chart points. The count is `null` for a bucket after now, which the
 * generated types cannot express.
 */
function chartPoints(points: number[][]): Point[] {
  return points.map(([t, count]) => ({ x: t * 1000, y: count ?? null }));
}

/** Title of an axis, saying what a bucketed point shows. */
function axisTitle(title: string, bucket: number | undefined): string {
  if (!bucket) return title;
  if (bucket === DAY) return `${title}, maximum per day`;
  const minutes = bucket / 60;
  const length =
    minutes < 60
      ? `${minutes} minutes`
      : minutes === 60
        ? "hour"
        : `${minutes / 60} hours`;
  return `${title}, maximum per ${length}`;
}

/** A room's line in the chart: its occupancy or its entries. */
type Line = "occupancy" | "entries";

/**
 * The occupancy of the total and of each room on the left axis, and the daily entries into each
 * room as dashed lines on the right axis (a point per day, at the day's last entry); `totalShown`
 * and `roomShown` tell which lines are shown.
 */
function chartData(
  timeline: Timeline,
  totalShown: boolean,
  roomShown: (room: number, line: Line) => boolean,
): TimeData {
  return {
    datasets: [
      {
        label: "Total occupancy",
        data: chartPoints(timeline.total),
        borderColor: TOTAL_COLOR,
        backgroundColor: TOTAL_COLOR,
        borderWidth: 3,
        hidden: !totalShown,
      },
      ...timeline.rooms.map((room, i) => ({
        label: `Occupancy ${room.name}`,
        data: chartPoints(room.occupancy),
        borderWidth: 1.5,
        hidden: !roomShown(room.id, "occupancy"),
        ...roomColor(i),
      })),
      ...timeline.rooms.map((room, i) => ({
        label: `Entries ${room.name}`,
        data: chartPoints(room.entries),
        yAxisID: "y2",
        borderWidth: 1.5,
        borderDash: [6, 4],
        // One point per day, so the points are drawn.
        pointRadius: 3,
        hidden: !roomShown(room.id, "entries"),
        ...roomColor(i),
      })),
    ],
  };
}

/** The Monitoring page (`/`). */
export default function Monitoring() {
  const { token } = theme.useToken();
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  // The zoomed-in interval of the chart, if any; otherwise its whole range of days.
  const [view, setView] = useState<View | undefined>();
  // The total line, shown at first, and per room which of its lines is shown (`null` for none):
  // never both, to keep the chart readable; the occupancy at first.
  const [totalShown, setTotalShown] = useState(true);
  const [roomLines, setRoomLines] = useState<Map<number, Line | null>>(
    new Map(),
  );
  const lineOf = (lines: Map<number, Line | null>, room: number) =>
    lines.has(room) ? lines.get(room) : "occupancy";
  const roomShown = (room: number, line: Line) =>
    lineOf(roomLines, room) === line;
  /** Shows the room's line, hiding its other one, or hides it if it is shown. */
  const toggle = (room: number, line: Line) =>
    setRoomLines((lines) =>
      new Map(lines).set(room, lineOf(lines, room) === line ? null : line),
    );
  const range = parseRange(params.get("range")) ?? presetRange("day", dayjs());
  const setRange = (r: DayRange) =>
    setParams(
      (p) => {
        p.set("range", formatRange(r));
        return p;
      },
      { replace: true },
    );

  const shownInterval = view ?? bounds(range);
  const interval = {
    start: Math.floor(shownInterval.min / 1000),
    end: Math.ceil(shownInterval.max / 1000),
  };
  const occupancy = useOccupancy();
  const timeline = useTimeline(interval);

  // Colours follow the rooms sorted by name, as in the chart; every listed room has one.
  const colors = new Map(
    occupancy.data?.rooms.map((r, i) => [r.id, roomColor(i).borderColor]),
  );
  const color = (room: number) => colors.get(room)!;
  const faults = occupancy.data?.sensor_faults ?? 0;
  const crowded = occupancy.data?.crowded_rooms ?? 0;
  const rooms = [...(occupancy.data?.rooms ?? [])].sort(
    (a, b) => b.occupancy - a.occupancy || a.name.localeCompare(b.name),
  );

  return (
    <Row gutter={[12, 12]}>
      <Col xs={24} lg={8}>
        {/* Wide: beside the chart, one below the other; medium: side by side above the chart;
            narrow: one below the other. Side by side, both cards are as high as the higher one. */}
        <Row gutter={[12, 12]} align="stretch">
          <Col xs={24} md={12} lg={24}>
            <Card
              size="small"
              style={{ height: "100%" }}
              title={siteTitle}
              extra={
                <Button
                  type="text"
                  icon={<SettingOutlined />}
                  title="Configuration"
                  aria-label="Configuration"
                  onClick={() => navigate("/admin")}
                />
              }
            >
              <Flex gap={24} align="flex-end">
                <Statistic
                  title="Total occupancy"
                  value={occupancy.data?.total ?? 0}
                  loading={occupancy.isPending}
                  formatter={(value) => (
                    <Toggle
                      large
                      value={Number(value)}
                      color={TOTAL_COLOR}
                      shown={totalShown}
                      onToggle={() => setTotalShown((s) => !s)}
                    />
                  )}
                />
                {occupancy.data &&
                  (faults || crowded ? (
                    <Flex vertical style={{ color: token.colorError }}>
                      {!!faults && (
                        <Link
                          to="/admin"
                          title="Show the doorways with a fault"
                          style={{ color: "inherit" }}
                        >
                          {faults} sensor {faults === 1 ? "fault" : "faults"}
                        </Link>
                      )}
                      {!!crowded && (
                        <span>
                          {crowded} crowded {crowded === 1 ? "room" : "rooms"}
                        </span>
                      )}
                    </Flex>
                  ) : (
                    <span style={{ color: token.colorTextSecondary }}>
                      No warnings
                    </span>
                  ))}
              </Flex>
            </Card>
          </Col>
          <Col xs={24} md={12} lg={24}>
            <Card size="small" style={{ height: "100%" }}>
              {occupancy.error && (
                <Alert type="error" title={occupancy.error.message} />
              )}
              <Table
                rowKey="id"
                size="small"
                loading={occupancy.isPending}
                dataSource={rooms}
                pagination={false}
                sortDirections={sortDirections}
                locale={{
                  emptyText: (
                    <Empty description="No rooms yet - add them on the Configuration page" />
                  ),
                }}
                columns={[
                  {
                    title: "Room",
                    dataIndex: "name",
                    sorter: byText((room) => room.name),
                    render: (name: string, room) => {
                      const badge = (
                        <Badge
                          count={room.sensor_faults}
                          color={token.colorError}
                          size="small"
                          offset={[6, 0]}
                        >
                          {name}
                        </Badge>
                      );
                      return room.sensor_faults > 0 ? (
                        <Link
                          to="/admin"
                          title="Show the doorways with a fault"
                          style={{ color: "inherit" }}
                        >
                          {badge}
                        </Link>
                      ) : (
                        badge
                      );
                    },
                  },
                  {
                    title: "Occupancy",
                    dataIndex: "occupancy",
                    align: "right",
                    sorter: byNumber((room) => room.occupancy),
                    // As listed: the fullest first, then by name.
                    defaultSortOrder: "descend",
                    render: (value: number, room) => (
                      <Badge
                        count={
                          room.crowded && (
                            <WarningFilled
                              title="Over the occupancy threshold"
                              style={{ color: token.colorError }}
                            />
                          )
                        }
                        offset={[-2, 2]}
                        // The badge sets line-height 1, which would shrink the number.
                        styles={{ root: { lineHeight: "inherit" } }}
                      >
                        <Toggle
                          value={value}
                          color={color(room.id)}
                          shown={roomShown(room.id, "occupancy")}
                          onToggle={() => toggle(room.id, "occupancy")}
                        />
                      </Badge>
                    ),
                  },
                  {
                    title: "Entries today",
                    dataIndex: "entries",
                    align: "right",
                    sorter: byNumber((room) => room.entries),
                    render: (value: number, room) => (
                      <Toggle
                        dashed
                        value={value}
                        color={color(room.id)}
                        shown={roomShown(room.id, "entries")}
                        onToggle={() => toggle(room.id, "entries")}
                      />
                    ),
                  },
                ]}
              />
            </Card>
          </Col>
        </Row>
      </Col>
      <Col xs={24} lg={16}>
        <TimeChart
          title={axisTitle("Occupancy", timeline.data?.bucket)}
          title2="Entries per day (dashed)"
          range={range}
          onRange={setRange}
          maxDays={MAX_DAYS}
          data={
            timeline.data && chartData(timeline.data, totalShown, roomShown)
          }
          error={timeline.error}
          view={view}
          onView={setView}
        />
      </Col>
    </Row>
  );
}
