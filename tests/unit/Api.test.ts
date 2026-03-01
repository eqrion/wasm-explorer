import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { storePayload, retrievePayload, proxyFetch } from "../../src/Api.js";

const API_BASE = "http://localhost:8081";

function mockOkResponse(body: string | ArrayBuffer): Response {
  if (typeof body === "string") {
    return new Response(body, { status: 200 });
  }
  return new Response(body, { status: 200 });
}

function mockErrorResponse(status: number, statusText: string): Response {
  return new Response(null, { status, statusText });
}

describe("storePayload", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("POSTs body and returns response text on 200", async () => {
    const key = "abc123";
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(mockOkResponse(key)),
    );

    const payload = new ArrayBuffer(4);
    const result = await storePayload(payload);
    expect(result).toBe(key);

    const fetchMock = vi.mocked(globalThis.fetch);
    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toBe(`${API_BASE}/store`);
    expect((options as RequestInit).method).toBe("POST");
    expect((options as RequestInit).body).toBe(payload);
  });

  it("throws ApiError on non-ok response", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(mockErrorResponse(500, "Internal Server Error")),
    );

    await expect(storePayload(new ArrayBuffer(4))).rejects.toMatchObject({
      name: "ApiError",
      status: 500,
    });
  });

  it("thrown error is instanceof Error", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(mockErrorResponse(503, "Service Unavailable")),
    );

    try {
      await storePayload(new ArrayBuffer(4));
      expect.fail("should have thrown");
    } catch (e) {
      expect(e instanceof Error).toBe(true);
      expect((e as Error).name).toBe("ApiError");
    }
  });
});

describe("retrievePayload", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns ArrayBuffer on 200", async () => {
    const buf = new ArrayBuffer(8);
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(mockOkResponse(buf)),
    );

    const result = await retrievePayload("mykey");
    expect(result).toBeInstanceOf(ArrayBuffer);
  });

  it("throws ApiError with 'Key not found' message on 404", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(mockErrorResponse(404, "Not Found")),
    );

    await expect(retrievePayload("missing")).rejects.toMatchObject({
      name: "ApiError",
      status: 404,
      message: expect.stringContaining("Key not found"),
    });
  });

  it("throws ApiError on other non-ok responses", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(mockErrorResponse(403, "Forbidden")),
    );

    await expect(retrievePayload("key")).rejects.toMatchObject({
      name: "ApiError",
      status: 403,
    });
  });
});

describe("proxyFetch", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns direct response when fetch succeeds", async () => {
    const directResponse = mockOkResponse("data");
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(directResponse));

    const result = await proxyFetch("https://example.com/data");
    expect(result).toBe(directResponse);
    expect(vi.mocked(globalThis.fetch)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(globalThis.fetch).mock.calls[0][0]).toBe(
      "https://example.com/data",
    );
  });

  it("retries via proxy URL when direct fetch throws", async () => {
    const proxyResponse = mockOkResponse("proxied");
    vi.stubGlobal(
      "fetch",
      vi.fn()
        .mockRejectedValueOnce(new TypeError("Failed to fetch"))
        .mockResolvedValueOnce(proxyResponse),
    );

    const targetUrl = "https://cors-blocked.example.com/data";
    const result = await proxyFetch(targetUrl);
    expect(result).toBe(proxyResponse);

    const fetchMock = vi.mocked(globalThis.fetch);
    expect(fetchMock).toHaveBeenCalledTimes(2);
    const proxyCall = fetchMock.mock.calls[1][0] as string;
    expect(proxyCall).toContain(`${API_BASE}/proxy?url=`);
    expect(proxyCall).toContain(encodeURIComponent(targetUrl));
  });
});
