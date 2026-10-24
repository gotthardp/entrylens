// SPDX-License-Identifier: MPL-2.0
import { Layout, Typography } from "antd";
import { GithubOutlined } from "@ant-design/icons";
import { Outlet } from "react-router";
import { serverVersion } from "./site";

/** The project's source code and documentation. */
const REPOSITORY = "https://github.com/gotthardp/entrylens";

/** The page, without a header or a menu, and a footer with the project's references. */
export default function AppLayout() {
  return (
    <Layout style={{ minHeight: "100vh" }}>
      <Layout.Content style={{ padding: 12 }}>
        <Outlet />
      </Layout.Content>
      {/* Left, in line with the cards' edge, like the rest of the page. */}
      <Layout.Footer style={{ padding: "4px 12px 12px" }}>
        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
          Entrylens server {serverVersion}, licensed under the{" "}
          <Typography.Link
            href="https://mozilla.org/MPL/2.0/"
            target="_blank"
            style={{ fontSize: 12 }}
          >
            MPL 2.0
          </Typography.Link>{" "}
          ·{" "}
          <Typography.Link
            href={REPOSITORY}
            target="_blank"
            style={{ fontSize: 12 }}
          >
            <GithubOutlined /> GitHub
          </Typography.Link>
        </Typography.Text>
      </Layout.Footer>
    </Layout>
  );
}
