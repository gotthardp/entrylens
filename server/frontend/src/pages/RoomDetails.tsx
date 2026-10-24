// SPDX-License-Identifier: MPL-2.0
import { useEffect } from "react";
import {
  Alert,
  Button,
  Card,
  Flex,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Result,
  Spin,
  Tooltip,
} from "antd";
import { useNavigate, useParams } from "react-router";
import {
  useDeleteRoom,
  useDoorways,
  useRooms,
  useSaveRoom,
} from "../api/queries";
import BackTitle from "../components/BackTitle";

/** The room's form; an empty threshold is none. */
type Fields = { name: string; threshold?: number | null };

/** Edits the room `/admin/rooms/{id}`, or creates one at `/admin/rooms/new`. */
export default function RoomDetails() {
  const navigate = useNavigate();
  const params = useParams();
  const id = params.id === undefined ? undefined : Number(params.id);
  const [form] = Form.useForm<Fields>();

  const rooms = useRooms();
  const doorways = useDoorways();
  const room = rooms.data?.find((r) => r.id === id);
  // A room used by a doorway cannot be deleted.
  const used = doorways.data?.some(
    (d) => d.inside_room === id || d.outside_room === id,
  );
  useEffect(() => {
    if (room)
      form.setFieldsValue({ name: room.name, threshold: room.threshold });
  }, [room, form]);

  const back = "/admin?tab=rooms";
  const save = useSaveRoom(id, back);
  const remove = useDeleteRoom(back);

  const error = rooms.error ?? doorways.error;
  if (error) return <Alert type="error" title={error.message} />;
  if (id !== undefined && (rooms.isPending || doorways.isPending))
    return <Spin />;
  if (id !== undefined && !room)
    return <Result status="404" title="Unknown room" />;

  return (
    <Card
      size="small"
      title={
        <BackTitle to="/admin?tab=rooms" label="Back to configuration">
          {room ? `Room ${room.name}` : "New room"}
        </BackTitle>
      }
      style={{ maxWidth: 600 }}
    >
      <Form
        form={form}
        layout="vertical"
        onFinish={({ name, threshold }: Fields) =>
          save.mutate({ name, threshold: threshold ?? null })
        }
      >
        <Form.Item
          name="name"
          label="Name"
          rules={[{ required: true, whitespace: true, max: 100 }]}
        >
          <Input autoFocus />
        </Form.Item>
        <Form.Item
          name="threshold"
          label="Occupancy threshold"
          extra="The room is crowded when more people are in it. Empty for none."
        >
          <InputNumber min={1} precision={0} />
        </Form.Item>
        <Flex gap="small" justify="space-between">
          <Flex gap="small">
            <Button type="primary" htmlType="submit" loading={save.isPending}>
              Save
            </Button>
            <Button onClick={() => navigate("/admin?tab=rooms")}>Cancel</Button>
          </Flex>
          {room && (
            <Tooltip
              title={
                used && "A doorway uses this room, so it cannot be deleted."
              }
            >
              <Popconfirm
                title={`Delete room ${room.name}?`}
                disabled={used}
                onConfirm={() => remove.mutate(room.id)}
              >
                <Button danger disabled={used} loading={remove.isPending}>
                  Delete
                </Button>
              </Popconfirm>
            </Tooltip>
          )}
        </Flex>
      </Form>
    </Card>
  );
}
