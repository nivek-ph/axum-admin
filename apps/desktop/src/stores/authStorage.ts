import type { AuthUserInfo } from './auth';

const STORAGE_KEY = 'axum-vue-admin.auth';

export interface PersistedAuthSession {
  accessToken: string;
  refreshToken: string;
  userInfo: AuthUserInfo | null;
}

const emptySession = (): PersistedAuthSession => ({
  accessToken: '',
  refreshToken: '',
  userInfo: null,
});

export function readAuthSession(): PersistedAuthSession {
  if (typeof localStorage === 'undefined') {
    return emptySession();
  }

  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      return emptySession();
    }

    const parsed = JSON.parse(raw) as Partial<PersistedAuthSession>;
    const accessToken = typeof parsed.accessToken === 'string' ? parsed.accessToken.trim() : '';
    const refreshToken = typeof parsed.refreshToken === 'string' ? parsed.refreshToken.trim() : '';
    const userInfo = parsed.userInfo && typeof parsed.userInfo === 'object' ? (parsed.userInfo as AuthUserInfo) : null;

    if (!accessToken || !refreshToken) {
      localStorage.removeItem(STORAGE_KEY);
      return emptySession();
    }

    return { accessToken, refreshToken, userInfo };
  } catch {
    localStorage.removeItem(STORAGE_KEY);
    return emptySession();
  }
}

export function writeAuthSession(session: PersistedAuthSession) {
  if (typeof localStorage === 'undefined') {
    return;
  }

  const accessToken = session.accessToken.trim();
  const refreshToken = session.refreshToken.trim();
  if (!accessToken || !refreshToken) {
    localStorage.removeItem(STORAGE_KEY);
    return;
  }

  localStorage.setItem(
    STORAGE_KEY,
    JSON.stringify({
      accessToken,
      refreshToken,
      userInfo: session.userInfo,
    } satisfies PersistedAuthSession)
  );
}

export function clearAuthSession() {
  if (typeof localStorage === 'undefined') {
    return;
  }

  localStorage.removeItem(STORAGE_KEY);
}
