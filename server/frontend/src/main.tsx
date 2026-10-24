// SPDX-License-Identifier: MPL-2.0
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App as AntApp, ConfigProvider } from "antd";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createBrowserRouter, RouterProvider } from "react-router";
import AppLayout from "./AppLayout";
import Monitoring from "./pages/Monitoring";
import Configuration from "./pages/Configuration";
import RoomDetails from "./pages/RoomDetails";
import DoorwayDetails from "./pages/DoorwayDetails";

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1 } },
});

const router = createBrowserRouter([
  {
    path: "/",
    element: <AppLayout />,
    children: [
      { index: true, element: <Monitoring /> },
      { path: "admin", element: <Configuration /> },
      { path: "admin/rooms/new", element: <RoomDetails /> },
      { path: "admin/rooms/:id", element: <RoomDetails /> },
      { path: "admin/doorways/new", element: <DoorwayDetails /> },
      { path: "admin/doorways/:id", element: <DoorwayDetails /> },
    ],
  },
]);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ConfigProvider
      theme={{
        // Denser forms: less space after each field and under its label.
        components: {
          Form: { itemMarginBottom: 12, verticalLabelPadding: "0 0 4px" },
        },
      }}
    >
      <AntApp>
        <QueryClientProvider client={queryClient}>
          <RouterProvider router={router} />
        </QueryClientProvider>
      </AntApp>
    </ConfigProvider>
  </StrictMode>,
);
