import { afterEach, describe, expect, it } from 'vitest';

import { clearAuthSession, readAuthSession, writeAuthSession } from './authStorage';

describe('authStorage', () => {
  afterEach(() => {
    clearAuthSession();
  });

  it('persists and restores the token pair with user info', () => {
    writeAuthSession({
      accessToken: 'access-token',
      refreshToken: 'refresh-token',
      userInfo: {
        id: 1,
        userName: 'admin',
        nickName: 'System Administrator',
      },
    });

    expect(readAuthSession()).toEqual({
      accessToken: 'access-token',
      refreshToken: 'refresh-token',
      userInfo: {
        id: 1,
        userName: 'admin',
        nickName: 'System Administrator',
      },
    });
  });

  it('keeps an otherwise valid session when the optional display nickname is empty', () => {
    writeAuthSession({
      accessToken: 'access-token',
      refreshToken: 'refresh-token',
      userInfo: {
        id: 1,
        userName: 'admin',
        nickName: '',
      },
    });

    expect(readAuthSession().userInfo?.nickName).toBe('');
  });

  it('clears invalid, legacy, and incomplete sessions', () => {
    localStorage.setItem('axum-vue-admin.auth', '{not-json');
    expect(readAuthSession()).toEqual({ accessToken: '', refreshToken: '', userInfo: null });
    expect(localStorage.getItem('axum-vue-admin.auth')).toBeNull();

    localStorage.setItem('axum-vue-admin.auth', JSON.stringify({ token: 'legacy-token' }));
    expect(readAuthSession()).toEqual({ accessToken: '', refreshToken: '', userInfo: null });
    expect(localStorage.getItem('axum-vue-admin.auth')).toBeNull();

    localStorage.setItem(
      'axum-vue-admin.auth',
      JSON.stringify({ accessToken: 'access-token', refreshToken: 'refresh-token', userInfo: null })
    );
    expect(readAuthSession()).toEqual({ accessToken: '', refreshToken: '', userInfo: null });
    expect(localStorage.getItem('axum-vue-admin.auth')).toBeNull();

    writeAuthSession({ accessToken: 'access-token', refreshToken: '   ', userInfo: null });
    expect(localStorage.getItem('axum-vue-admin.auth')).toBeNull();
  });
});
