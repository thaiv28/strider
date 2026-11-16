import "server-only";
export type AuthorizationQuery = Record<string, string | string[] | undefined>;
export function parseAuthorization(query: AuthorizationQuery) {
  const challenge = query.code_challenge;
  const deviceName = query.device_name;
  const uri = query.redirect_uri;
  const state = query.state;
  if (typeof challenge !== "string" || !/^[A-Za-z0-9_-]{43}$/.test(challenge) || query.code_challenge_method !== "S256" || typeof deviceName !== "string" || deviceName.length < 1 || deviceName.length > 60 || !deviceName.trim() || typeof uri !== "string" || typeof state !== "string" || !state || state.length > 2048) return null;
  // Compare the entire input as well as URL components: no credentials, query,
  // fragments, alternate loopback spellings or implicit ports are accepted.
  const match = uri.match(/^http:\/\/127\.0\.0\.1:(\d{4,5})\/callback$/);
  if (!match || Number(match[1]) < 1024 || Number(match[1]) > 65535) return null;
  return { challenge, deviceName, redirectUri: uri, state };
}
