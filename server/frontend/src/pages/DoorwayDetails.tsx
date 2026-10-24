// SPDX-License-Identifier: MPL-2.0
import { useEffect } from "react";
import {
  Alert,
  Button,
  Card,
  Flex,
  Form,
  Input,
  Popconfirm,
  Result,
  Select,
  Spin,
  Tooltip,
} from "antd";
import { useNavigate, useParams } from "react-router";
import {
  useDeleteDoorway,
  useDoorways,
  useRooms,
  useSaveDoorway,
} from "../api/queries";
import BackTitle from "../components/BackTitle";

interface Fields {
  dev_eui?: string;
  inside_room: number;
  outside_room?: number;
}

/** Edits the doorway `/admin/doorways/{id}`, or creates one at `/admin/doorways/new`. */
export default function DoorwayDetails() {
  const navigate = useNavigate();
  const params = useParams();
  const id = params.id === undefined ? undefined : Number(params.id);
  const [form] = Form.useForm<Fields>();

  const rooms = useRooms();
  const doorways = useDoorways();
  const doorway = doorways.data?.find((d) => d.id === id);
  useEffect(() => {
    if (doorway) {
      form.setFieldsValue({
        dev_eui: doorway.dev_eui ?? "",
        inside_room: doorway.inside_room,
        outside_room: doorway.outside_room ?? undefined,
      });
    }
  }, [doorway, form]);
  const roomOptions = rooms.data?.map((r) => ({ value: r.id, label: r.name }));

  const save = useSaveDoorway(id, "/admin");
  const remove = useDeleteDoorway("/admin");

  // The passages and the occupancy counted through a doorway belong to its rooms.
  const fixed = doorway?.has_passages;

  const error = rooms.error ?? doorways.error;
  if (error) return <Alert type="error" title={error.message} />;
  if (rooms.isPending || (id !== undefined && doorways.isPending))
    return <Spin />;
  if (id !== undefined && !doorway)
    return <Result status="404" title="Unknown doorway" />;

  return (
    <Card
      size="small"
      title={
        <BackTitle to="/admin" label="Back to configuration">
          {doorway ? "Doorway" : "New doorway"}
        </BackTitle>
      }
      style={{ maxWidth: 600 }}
    >
      <Form
        form={form}
        layout="vertical"
        onFinish={(fields: Fields) =>
          save.mutate({
            dev_eui: fields.dev_eui ?? null,
            inside_room: fields.inside_room,
            outside_room: fields.outside_room ?? null,
          })
        }
      >
        {fixed && (
          <Form.Item>
            Passages were counted through this doorway, so its rooms cannot be
            changed.
          </Form.Item>
        )}
        <Form.Item
          name="dev_eui"
          label="Sensor DevEUI"
          extra="16 hexadecimal digits, configured in the sensor's LoRaWAN module. Leave empty while no sensor is installed."
          rules={[
            {
              validator: (_, value?: string) =>
                !value || /^[0-9a-f]{16}$/i.test(value.replace(/[\s:-]/g, ""))
                  ? Promise.resolve()
                  : Promise.reject(
                      new Error("The DevEUI must have 16 hexadecimal digits"),
                    ),
            },
          ]}
        >
          <Input placeholder="e.g. AC1F09FFFE0A0001" />
        </Form.Item>
        <Form.Item
          name="inside_room"
          label="Inside room"
          rules={[{ required: true }]}
        >
          <Select options={roomOptions} disabled={fixed} />
        </Form.Item>
        <Form.Item
          name="outside_room"
          label="Outside room"
          extra="Leave empty for a doorway to the outside corridor."
          dependencies={["inside_room"]}
          rules={[
            ({ getFieldValue }) => ({
              validator: (_, value) =>
                value !== undefined && value === getFieldValue("inside_room")
                  ? Promise.reject(
                      new Error("Inside and outside room must differ"),
                    )
                  : Promise.resolve(),
            }),
          ]}
        >
          <Select options={roomOptions} allowClear disabled={fixed} />
        </Form.Item>
        <Flex gap="small" justify="space-between">
          <Flex gap="small">
            <Button type="primary" htmlType="submit" loading={save.isPending}>
              Save
            </Button>
            <Button onClick={() => navigate("/admin")}>Cancel</Button>
          </Flex>
          {doorway && (
            <Tooltip
              title={
                doorway.has_passages &&
                "Passages were counted through this doorway, so it cannot be deleted."
              }
            >
              <Popconfirm
                title="Delete this doorway?"
                disabled={doorway.has_passages}
                onConfirm={() => remove.mutate(doorway.id)}
              >
                <Button
                  danger
                  disabled={doorway.has_passages}
                  loading={remove.isPending}
                >
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
