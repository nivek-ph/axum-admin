import { AxiosError, type InternalAxiosRequestConfig } from 'axios'

export function adapterResponse(
  config: InternalAxiosRequestConfig,
  data: unknown,
  status = 200,
) {
  return Promise.resolve({
    data,
    status,
    statusText: status === 200 ? 'OK' : 'Unauthorized',
    headers: {},
    config,
  })
}

export function rejectEnvelope(
  config: InternalAxiosRequestConfig,
  code: string,
  status = 401,
) {
  const envelope = { code, message: 'session expired', data: null }
  return Promise.reject(
    new AxiosError('request failed', 'ERR_BAD_REQUEST', config, undefined, {
      data: envelope,
      status,
      statusText: 'Unauthorized',
      headers: {},
      config,
    }),
  )
}
