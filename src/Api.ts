/**
 * API client for the wasm-explorer server
 */

export interface ApiError extends Error {
  status?: number;
}

/**
 * Base URL for the API server
 */
const API_BASE_URL = "http://localhost:8081";

/**
 * Proxy a request to bypass CORS restrictions
 */
export async function proxyFetch(
  targetUrl: string,
  options?: RequestInit,
): Promise<Response> {
  // First try direct fetch
  try {
    const response = await fetch(targetUrl, options);
    return response;
  } catch (error) {
    // If direct fetch fails (likely CORS), fall back to proxy
    const proxyUrl = `${API_BASE_URL}/proxy?url=${encodeURIComponent(targetUrl)}`;

    const response = await fetch(proxyUrl, {
      method: options?.method || "GET",
      headers: options?.headers,
      body: options?.body,
    });

    return response;
  }
}

/**
 * Store a byte payload and get a unique key
 */
export async function storePayload(payload: ArrayBuffer): Promise<string> {
  const response = await fetch(`${API_BASE_URL}/store`, {
    method: "POST",
    body: payload,
  });

  if (!response.ok) {
    throw createApiError(
      `Failed to store payload: ${response.statusText}`,
      response.status,
    );
  }

  return response.text();
}

/**
 * Retrieve a payload by its key
 */
export async function retrievePayload(key: string): Promise<ArrayBuffer> {
  const response = await fetch(`${API_BASE_URL}/retrieve/${key}`);

  if (!response.ok) {
    if (response.status === 404) {
      throw createApiError(`Key not found: ${key}`, 404);
    }
    throw createApiError(
      `Failed to retrieve payload: ${response.statusText}`,
      response.status,
    );
  }

  return response.arrayBuffer();
}

/**
 * Helper function to create ApiError with status code
 */
function createApiError(message: string, status?: number): ApiError {
  const error = new Error(message) as ApiError;
  error.name = "ApiError";
  error.status = status;
  return error;
}
