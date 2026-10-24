// SPDX-License-Identifier: MPL-2.0
import {
  Alert,
  Button,
  Card,
  Col,
  Row,
  Table,
  Tabs,
  Tooltip,
  theme,
} from "antd";
import { EditOutlined, PlusOutlined } from "@ant-design/icons";
import { useNavigate, useSearchParams } from "react-router";
import { useDoorways, useRooms } from "../api/queries";
import BackTitle from "../components/BackTitle";
import { byNumber, byText, sortDirections } from "../sort";
import { ago, exact, useMinuteTick } from "../time";

/** Descriptions of the fault codes a sensor reports, by code; 0 (no fault) is not listed. */
const FAULTS: Record<number, string> = {};

/** The description of a fault code; empty for 0 (no fault). */
function faultText(code: number) {
  if (code === 0) return "";
  return FAULTS[code] ?? `Unknown fault ${code}`;
}

/** The Configuration page (`/admin`), with the Doorways and Rooms tabs. */
export default function Configuration() {
  const { token } = theme.useToken();
  const navigate = useNavigate();
  // The tab shown, kept in the URL (`?tab=rooms`), so that the room details can return to it.
  const [params, setParams] = useSearchParams();
  const tab = params.get("tab") === "rooms" ? "rooms" : "doorways";
  const rooms = useRooms();
  const doorways = useDoorways();
  const roomName = (id: number | null | undefined) =>
    rooms.data?.find((r) => r.id === id)?.name ?? "";
  const error = rooms.error ?? doorways.error;
  useMinuteTick();
  // A value of the sensor status, red when it is beyond its limit; the tooltip gives `detail`
  // (e.g. the exact time) and why it is red.
  const status = (text: string, bad: boolean, why: string, detail?: string) => (
    <span
      style={bad ? { color: token.colorError } : undefined}
      title={[detail, bad && why].filter(Boolean).join("\n") || undefined}
    >
      {text}
    </span>
  );

  const doorwayTable = (
    <Table
      size="small"
      rowKey="id"
      loading={doorways.isPending || rooms.isPending}
      dataSource={doorways.data}
      pagination={false}
      scroll={{ x: "max-content" }}
      sortDirections={sortDirections}
      columns={[
        {
          title: "DevEUI",
          dataIndex: "dev_eui",
          sorter: byText((d) => d.dev_eui),
        },
        {
          title: "Inside room",
          key: "inside",
          render: (_, d) => roomName(d.inside_room),
          sorter: byText((d) => roomName(d.inside_room)),
          // As listed: by inside room, then by outside room.
          defaultSortOrder: "ascend",
        },
        {
          title: "Outside room",
          key: "outside",
          render: (_, d) => roomName(d.outside_room),
          // The outside corridor (no room) first.
          sorter: byText((d) => roomName(d.outside_room)),
        },
        {
          title: "Fault",
          key: "fault",
          sorter: byNumber((d) => d.fault),
          render: (_, d) => (
            <span style={{ color: token.colorError }}>
              {faultText(d.fault)}
            </span>
          ),
        },
        {
          title: "Voltage",
          key: "voltage",
          align: "right",
          // No report yet counts as the lowest.
          sorter: byNumber((d) => d.voltage_mv),
          render: (_, d) =>
            d.voltage_mv != null &&
            status(
              `${(d.voltage_mv / 1000).toFixed(2)} V`,
              d.low_voltage,
              "Below the lowest acceptable input voltage",
            ),
        },
        {
          title: "Last passage",
          key: "passage",
          // "never" counts as the oldest.
          sorter: byNumber((d) => d.last_passage),
          render: (_, d) =>
            status(
              ago(d.last_passage),
              d.no_passage,
              "No passage within the configured period",
              exact(d.last_passage),
            ),
        },
        {
          title: "Last report",
          key: "report",
          sorter: byNumber((d) => d.last_report),
          render: (_, d) =>
            status(
              ago(d.last_report),
              d.no_report,
              "No sensor report within the configured period",
              exact(d.last_report),
            ),
        },
        {
          key: "edit",
          align: "right",
          render: (_, d) => (
            <Button
              size="small"
              icon={<EditOutlined />}
              title="Edit"
              aria-label="Edit"
              onClick={() => navigate(`/admin/doorways/${d.id}`)}
            />
          ),
        },
      ]}
    />
  );
  const roomTable = (
    <Table
      size="small"
      rowKey="id"
      loading={rooms.isPending}
      dataSource={rooms.data}
      pagination={false}
      sortDirections={sortDirections}
      columns={[
        {
          title: "Name",
          dataIndex: "name",
          sorter: byText((r) => r.name),
          defaultSortOrder: "ascend",
        },
        {
          title: "Occupancy threshold",
          dataIndex: "threshold",
          align: "right",
          // No threshold counts as the highest: no limit.
          sorter: byNumber((r) => r.threshold, Infinity),
        },
        {
          key: "edit",
          align: "right",
          render: (_, room) => (
            <Button
              size="small"
              icon={<EditOutlined />}
              title="Edit"
              aria-label="Edit"
              onClick={() => navigate(`/admin/rooms/${room.id}`)}
            />
          ),
        },
      ]}
    />
  );
  const addDoorway = (
    <Tooltip title={rooms.data?.length === 0 && "Add a room first"}>
      <Button
        size="small"
        icon={<PlusOutlined />}
        disabled={!rooms.data?.length}
        title={rooms.data?.length ? "Add a doorway" : undefined}
        aria-label="Add a doorway"
        onClick={() => navigate("/admin/doorways/new")}
      />
    </Tooltip>
  );
  const addRoom = (
    <Button
      size="small"
      icon={<PlusOutlined />}
      title="Add a room"
      aria-label="Add a room"
      onClick={() => navigate("/admin/rooms/new")}
    />
  );

  return (
    <Row gutter={[12, 12]}>
      {error && (
        <Col span={24}>
          <Alert type="error" title={error.message} />
        </Col>
      )}
      <Col span={24}>
        <Card
          size="small"
          title={
            <BackTitle to="/" label="Back to monitoring">
              Configuration
            </BackTitle>
          }
        >
          <Tabs
            size="small"
            activeKey={tab}
            onChange={(key) => setParams(key === "rooms" ? { tab: key } : {})}
            tabBarExtraContent={tab === "rooms" ? addRoom : addDoorway}
            items={[
              { key: "doorways", label: "Doorways", children: doorwayTable },
              { key: "rooms", label: "Rooms", children: roomTable },
            ]}
          />
        </Card>
      </Col>
    </Row>
  );
}
