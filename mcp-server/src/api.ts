const DEFAULT_URL = "http://localhost:8080";

export class ApiClient {
  constructor(
    private baseUrl = process.env.SOLHUB_API_URL ?? DEFAULT_URL,
    private apiKey = process.env.SOLHUB_API_KEY ?? "",
  ) {}

  private async request(path: string, init: RequestInit = {}): Promise<unknown> {
    const headers = new Headers(init.headers as HeadersInit | undefined);
    if (this.apiKey) headers.set("Authorization", `Bearer ${this.apiKey}`);
    headers.set("Content-Type", "application/json");
    const res = await fetch(`${this.baseUrl}${path}`, { ...init, headers });
    if (!res.ok) {
      const text = await res.text().catch(() => "");
      throw new Error(`SolHub API ${res.status}: ${text || res.statusText}`);
    }
    return res.json();
  }

  get<T>(path: string): Promise<T> {
    return this.request(path) as Promise<T>;
  }

  post<T>(path: string, body: unknown): Promise<T> {
    return this.request(path, { method: "POST", body: JSON.stringify(body) }) as Promise<T>;
  }

  patch<T>(path: string, body: unknown): Promise<T> {
    return this.request(path, { method: "PATCH", body: JSON.stringify(body) }) as Promise<T>;
  }

  del<T>(path: string): Promise<T> {
    return this.request(path, { method: "DELETE" }) as Promise<T>;
  }
}
