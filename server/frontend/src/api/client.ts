// SPDX-License-Identifier: MPL-2.0
import createClient from "openapi-fetch";
import type { components, paths } from "./schema";

export type Timeline = components["schemas"]["Timeline"];

export const client = createClient<paths>();

/** Resolves to the response data, or rejects with the server's error message. */
export async function call<D>(
  request: Promise<{ data?: D; error?: unknown; response: Response }>,
): Promise<D> {
  const { data, error, response } = await request;
  if (error !== undefined || !response.ok) {
    const message = (error as { error?: string } | undefined)?.error;
    throw new Error(message ?? `${response.status} ${response.statusText}`);
  }
  return data as D;
}
