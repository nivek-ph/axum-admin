import type { StorageDriver, StoragePayload, StorageRecord } from '@/api/storages'

const commonDefaults = {
  name: '',
  code: '',
  enabled: true,
  sort: 999,
  description: '',
}

export function emptyStoragePayload(): StoragePayload {
  return { ...commonDefaults, driver: 'local', root: './uploads' }
}

export function storagePayloadFor(record: StorageRecord): StoragePayload {
  const common = {
    name: record.name,
    code: record.code,
    enabled: record.enabled,
    sort: record.sort,
    description: record.description,
  }
  if (record.driver === 'local') {
    return { ...common, driver: 'local', root: record.root }
  }
  return {
    ...common,
    driver: 's3',
    root: record.root ?? '',
    bucket: record.bucket,
    region: record.region,
    endpoint: record.endpoint ?? '',
    publicBaseUrl: record.publicBaseUrl,
    accessKey: '',
    secretKey: '',
    virtualHostStyle: record.virtualHostStyle,
  }
}

export function storagePayloadWithDriver(form: StoragePayload, driver: StorageDriver): StoragePayload {
  const common = {
    name: form.name,
    code: form.code,
    enabled: form.enabled,
    sort: form.sort,
    description: form.description,
  }
  if (driver === 'local') return { ...common, driver, root: './uploads' }
  return {
    ...common,
    driver,
    root: '',
    bucket: '',
    region: '',
    endpoint: '',
    publicBaseUrl: '',
    accessKey: '',
    secretKey: '',
    virtualHostStyle: false,
  }
}

export type StorageValidationError =
  | 'Name and code are required'
  | 'Root directory is required'
  | 'Bucket, region, and public URL are required'

export function validateStoragePayload(form: StoragePayload): StorageValidationError | null {
  if (!form.name.trim() || !form.code.trim()) return 'Name and code are required'
  if (form.driver === 'local' && !form.root.trim()) return 'Root directory is required'
  if (form.driver === 's3' && (!form.bucket.trim() || !form.region.trim() || !form.publicBaseUrl.trim())) {
    return 'Bucket, region, and public URL are required'
  }
  return null
}
