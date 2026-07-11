import { http } from './http';
import { withAuthHeaders, type ApiResponse } from './core';

export interface SystemConfig {
  system: {
    env: string;
    addr: string;
    'db-type': string;
    'use-multipoint'?: boolean;
    'use-strict-auth'?: boolean;
  };
  captcha: {
    openCaptcha: number;
    openCaptchaTimeOut: number;
  };
  local: {
    storePath: string;
  };
}

export interface ServerInfo {
  os: {
    goos: string;
    numCpu: number;
    compiler: string;
    goVersion: string;
    numGoroutine: number;
  };
  cpu: {
    cores: number;
    cpus: number[];
  };
  ram: {
    totalMb: number;
    usedMb: number;
    usedPercent: number;
  };
  disk: Array<{
    mountPoint: string;
    totalMb?: number;
    usedMb?: number;
    totalGb: number;
    usedGb: number;
    usedPercent: number;
  }>;
}

export function normalizeSystemConfigResponse(payload: ApiResponse<{ config: SystemConfig }>) {
  return (
    payload?.data?.config || {
      system: { env: '', addr: '', 'db-type': '' },
      captcha: { openCaptcha: 1, openCaptchaTimeOut: 300 },
      local: { storePath: '' },
    }
  );
}

export function normalizeServerInfoResponse(payload: ApiResponse<{ server: ServerInfo }>) {
  return (
    payload?.data?.server || {
      os: { goos: '', numCpu: 0, compiler: '', goVersion: '', numGoroutine: 0 },
      cpu: { cores: 0, cpus: [] },
      ram: { totalMb: 0, usedMb: 0, usedPercent: 0 },
      disk: [],
    }
  );
}

export async function fetchSystemConfig() {
  const response = await http.get('/system/config', withAuthHeaders());
  return normalizeSystemConfigResponse(response);
}

export async function fetchServerInfo() {
  const response = await http.get('/system/server-info', withAuthHeaders());
  return normalizeServerInfoResponse(response);
}
