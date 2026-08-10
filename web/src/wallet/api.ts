import type { Challenge } from './kasware'

const apiBaseUrl = import.meta.env.VITE_API_BASE_URL ?? ''

async function requestJson<T>(path: string, body: object): Promise<T> {
  const response = await fetch(`${apiBaseUrl}${path}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
  })
  if (!response.ok) {
    const error = await response.json().catch(() => null) as { error?: string } | null
    throw new Error(error?.error ?? 'The API request failed.')
  }
  return response.json() as Promise<T>
}

export const walletAuthApi = {
  createChallenge(wallet: string) {
    return requestJson<Challenge>('/v1/auth/challenge', { wallet })
  },
  verifyChallenge(wallet: string, nonce: string, signature: string, publicKey: string) {
    return requestJson<{ session_token: string }>('/v1/auth/verify', {
      wallet,
      nonce,
      signature,
      public_key: publicKey,
    })
  },
}
