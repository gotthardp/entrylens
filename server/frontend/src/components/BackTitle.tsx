// SPDX-License-Identifier: MPL-2.0
import type { ReactNode } from "react";
import { Button, Flex } from "antd";
import { LeftOutlined } from "@ant-design/icons";
import { useNavigate } from "react-router";

/** A card title after a "<" button that goes to `to` (labelled `label`), changing nothing. */
export default function BackTitle(props: {
  to: string;
  label: string;
  children: ReactNode;
}) {
  const { to, label, children } = props;
  const navigate = useNavigate();
  return (
    <Flex align="center" gap="small">
      <Button
        type="text"
        size="small"
        icon={<LeftOutlined />}
        title={label}
        aria-label={label}
        onClick={() => navigate(to)}
      />
      {children}
    </Flex>
  );
}
