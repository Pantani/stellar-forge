import { loadApiConfig } from '../lib/config.js';
import { HttpError, requireConfigured } from '../lib/errors.js';

type RelayerPayload = Record<string, unknown>;

export function relayerStatus() {
  const config = loadApiConfig();
  return {
    configured:
      config.relayer_base_url.length > 0 &&
      config.relayer_api_key.length > 0,
    relayer_base_url: config.relayer_base_url || null,
    relayer_submit_path: config.relayer_submit_path,
  };
}

export async function submitSponsoredTransaction(payload: RelayerPayload) {
  const config = loadApiConfig();
  const baseUrl = requireConfigured(config.relayer_base_url, 'RELAYER_BASE_URL');
  const apiKey = requireConfigured(config.relayer_api_key, 'RELAYER_API_KEY');
  const url = new URL(config.relayer_submit_path, baseUrl);

  const response = await fetch(url, {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify(payload),
  });

  const rawBody = await response.text();
  let parsed: unknown = null;
  if (rawBody.length > 0) {
    try {
      parsed = JSON.parse(rawBody);
    } catch {
      parsed = { raw: rawBody };
    }
  }

  if (!response.ok) {
    throw new HttpError(
      response.status,
      `relayer request failed with status ${response.status}`,
      parsed,
    );
  }

  return {
    accepted: true,
    status_code: response.status,
    upstream_url: url.toString(),
    upstream_response: parsed,
  };
}
