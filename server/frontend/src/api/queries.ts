// SPDX-License-Identifier: MPL-2.0
import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { App } from "antd";
import { useNavigate } from "react-router";
import { call, client } from "./client";
import type { components } from "./schema";

/** What a room is saved with. */
export type RoomInput = components["schemas"]["RoomInput"];
/** What a doorway is saved with. */
export type DoorwayInput = components["schemas"]["DoorwayInput"];

/** The query keys; a change of the configuration invalidates them all. */
const keys = {
  rooms: ["rooms"],
  doorways: ["doorways"],
  occupancy: ["occupancy"],
  timeline: ["timeline"],
};

/** The rooms, sorted by name. */
export function useRooms() {
  return useQuery({
    queryKey: keys.rooms,
    queryFn: () => call(client.GET("/api/v1/rooms")),
  });
}

/** The doorways with their sensor status. */
export function useDoorways() {
  return useQuery({
    queryKey: keys.doorways,
    queryFn: () => call(client.GET("/api/v1/doorways")),
  });
}

/** The current occupancy and entries today. */
export function useOccupancy() {
  return useQuery({
    queryKey: keys.occupancy,
    queryFn: () => call(client.GET("/api/v1/occupancy")),
  });
}

/** The timeline of an interval (Unix times); the previous one is kept while it loads. */
export function useTimeline(interval: { start: number; end: number }) {
  return useQuery({
    queryKey: [...keys.timeline, interval.start, interval.end],
    queryFn: () =>
      call(client.GET("/api/v1/timeline", { params: { query: interval } })),
    placeholderData: keepPreviousData,
  });
}

/**
 * The options of a mutation that changes the configuration: on success, it reloads all data and
 * goes to `returnTo`; on failure, it shows the server's message.
 */
function useChange(returnTo: string) {
  const { message } = App.useApp();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  return {
    onSuccess: () => {
      Object.values(keys).forEach((queryKey) =>
        queryClient.invalidateQueries({ queryKey }),
      );
      navigate(returnTo);
    },
    onError: (e: Error) => message.error(e.message),
  };
}

/** Saves the room `id`, or creates one without an `id`; then goes to `returnTo`. */
export function useSaveRoom(id: number | undefined, returnTo: string) {
  return useMutation({
    mutationFn: (body: RoomInput) =>
      id === undefined
        ? call(client.POST("/api/v1/rooms", { body }))
        : call(
            client.PUT("/api/v1/rooms/{id}", {
              params: { path: { id } },
              body,
            }),
          ),
    ...useChange(returnTo),
  });
}

/** Deletes a room; then goes to `returnTo`. */
export function useDeleteRoom(returnTo: string) {
  return useMutation({
    mutationFn: (id: number) =>
      call(client.DELETE("/api/v1/rooms/{id}", { params: { path: { id } } })),
    ...useChange(returnTo),
  });
}

/** Saves the doorway `id`, or creates one without an `id`; then goes to `returnTo`. */
export function useSaveDoorway(id: number | undefined, returnTo: string) {
  return useMutation({
    mutationFn: (body: DoorwayInput) =>
      id === undefined
        ? call(client.POST("/api/v1/doorways", { body }))
        : call(
            client.PUT("/api/v1/doorways/{id}", {
              params: { path: { id } },
              body,
            }),
          ),
    ...useChange(returnTo),
  });
}

/** Deletes a doorway; then goes to `returnTo`. */
export function useDeleteDoorway(returnTo: string) {
  return useMutation({
    mutationFn: (id: number) =>
      call(
        client.DELETE("/api/v1/doorways/{id}", { params: { path: { id } } }),
      ),
    ...useChange(returnTo),
  });
}
