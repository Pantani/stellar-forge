export class HttpError extends Error {
  statusCode: number;
  detail: unknown;

  constructor(statusCode: number, message: string, detail: unknown = null) {
    super(message);
    this.statusCode = statusCode;
    this.detail = detail;
  }
}

export function requireConfigured(value: string, name: string) {
  if (!value || value.trim().length === 0) {
    throw new HttpError(503, `${name} is not configured`);
  }
  return value;
}
