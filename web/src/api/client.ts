export class ApiError extends Error {
  readonly status: number;
  readonly body: string;

  constructor(status: number, body: string) {
    super(body || `API request failed with status ${status}`);
    this.name = 'ApiError';
    this.status = status;
    this.body = body;
  }
}

type ApiQueryValue = string | number | boolean | null | undefined;

export function buildApiPath(
  path: string,
  query?:
    | URLSearchParams
    | Record<string, ApiQueryValue>,
): string {
  if (!query) {
    return path;
  }

  const searchParams =
    query instanceof URLSearchParams ? new URLSearchParams(query) : new URLSearchParams();

  if (!(query instanceof URLSearchParams)) {
    Object.entries(query).forEach(([key, value]) => {
      if (value === null || value === undefined || value === '') {
        return;
      }

      searchParams.set(key, String(value));
    });
  }

  const suffix = searchParams.toString();
  return suffix ? `${path}?${suffix}` : path;
}

async function readErrorBody(response: Response) {
  const text = await response.text();

  if (!text) {
    return '';
  }

  try {
    const parsed = JSON.parse(text) as { error?: string };
    return parsed.error ?? text;
  } catch {
    return text;
  }
}

export async function apiGet<T>(path: string): Promise<T> {
  const response = await fetch(`/api${path}`, {
    headers: {
      Accept: 'application/json',
    },
  });

  if (!response.ok) {
    throw new ApiError(response.status, await readErrorBody(response));
  }

  return (await response.json()) as T;
}

export async function apiPost<T, TBody = unknown>(
  path: string,
  body?: TBody,
): Promise<T> {
  const response = await fetch(`/api${path}`, {
    method: 'POST',
    headers: {
      Accept: 'application/json',
      'Content-Type': 'application/json',
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (!response.ok) {
    throw new ApiError(response.status, await readErrorBody(response));
  }

  return (await response.json()) as T;
}
