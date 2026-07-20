import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Plus, Search } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { toast } from 'sonner'

import {
  createDictionary,
  createDictionaryDetail,
  deleteDictionary,
  deleteDictionaryDetail,
  fetchDictionaries,
  fetchDictionaryDetails,
  updateDictionary,
  updateDictionaryDetail,
  type DictionaryDetailPayload,
  type DictionaryDetailRecord,
  type DictionaryPayload,
  type DictionaryRecord,
} from '@/api/dictionaries'
import { Button } from '@/components/ui/Button'
import { useConfirm } from '@/components/ui/ConfirmProvider'
import { Modal } from '@/components/ui/Modal'

type FlatDetail = DictionaryDetailRecord & { displayLevel: number }
const emptyDictionary: DictionaryPayload = { name: '', type: '', status: true, desc: '', parentId: null }
const emptyDetail: DictionaryDetailPayload = {
  label: '',
  value: '',
  extend: '',
  status: true,
  sort: 0,
  sysDictionaryId: 0,
  parentId: null,
}
function flatten(items: DictionaryDetailRecord[], level = 0): FlatDetail[] {
  return items.flatMap((item) => [{ ...item, displayLevel: level }, ...flatten(item.children ?? [], level + 1)])
}

export function DictionariesPage() {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const confirmAction = useConfirm()
  const [search, setSearch] = useState('')
  const [filter, setFilter] = useState('')
  const [selectedId, setSelectedId] = useState<number | null>(null)
  const [dictionaryOpen, setDictionaryOpen] = useState(false)
  const [dictionaryForm, setDictionaryForm] = useState<DictionaryPayload>(emptyDictionary)
  const [detailOpen, setDetailOpen] = useState(false)
  const [detailForm, setDetailForm] = useState<DictionaryDetailPayload>(emptyDetail)
  const dictionaries = useQuery({ queryKey: ['dictionaries', filter], queryFn: () => fetchDictionaries(filter) })
  const details = useQuery({
    queryKey: ['dictionary-details', selectedId],
    queryFn: () => fetchDictionaryDetails(selectedId as number),
    enabled: selectedId !== null,
  })

  useEffect(() => {
    if (!selectedId && dictionaries.data?.length) setSelectedId(dictionaries.data[0].id)
    if (selectedId && dictionaries.data && !dictionaries.data.some((item) => item.id === selectedId))
      setSelectedId(dictionaries.data[0]?.id ?? null)
  }, [dictionaries.data, selectedId])

  const selected = dictionaries.data?.find((item) => item.id === selectedId) ?? null
  const invalidateDictionaries = () => queryClient.invalidateQueries({ queryKey: ['dictionaries'] })
  const invalidateDetails = () => queryClient.invalidateQueries({ queryKey: ['dictionary-details', selectedId] })
  const dictionaryMutation = useMutation({
    mutationFn: () =>
      dictionaryForm.id
        ? updateDictionary(dictionaryForm as DictionaryPayload & { id: number })
        : createDictionary(dictionaryForm),
    onSuccess: async () => {
      toast.success(t('Dictionary saved'))
      setDictionaryOpen(false)
      await invalidateDictionaries()
    },
    onError: () => toast.error(t('Failed to save dictionary')),
  })
  const detailMutation = useMutation({
    mutationFn: () =>
      detailForm.id
        ? updateDictionaryDetail(detailForm as DictionaryDetailPayload & { id: number })
        : createDictionaryDetail(detailForm),
    onSuccess: async () => {
      toast.success(t('Dictionary detail saved'))
      setDetailOpen(false)
      await invalidateDetails()
    },
    onError: () => toast.error(t('Failed to save dictionary detail')),
  })
  const deleteDictionaryMutation = useMutation({
    mutationFn: deleteDictionary,
    onSuccess: async () => {
      toast.success(t('Dictionary deleted'))
      setSelectedId(null)
      await invalidateDictionaries()
    },
  })
  const deleteDetailMutation = useMutation({
    mutationFn: ({ dictionaryId, id }: { dictionaryId: number; id: number }) =>
      deleteDictionaryDetail(dictionaryId, id),
    onSuccess: async () => {
      toast.success(t('Dictionary detail deleted'))
      await invalidateDetails()
    },
  })

  function editDictionary(item?: DictionaryRecord) {
    setDictionaryForm(item ? { ...item } : { ...emptyDictionary })
    setDictionaryOpen(true)
  }
  function editDetail(item?: DictionaryDetailRecord, parentId: number | null = null) {
    if (!selectedId) return
    setDetailForm(
      item
        ? {
            id: item.id,
            label: item.label,
            value: item.value,
            extend: item.extend,
            status: item.status,
            sort: item.sort,
            sysDictionaryId: item.sysDictionaryId,
            parentId: item.parentId ?? null,
          }
        : { ...emptyDetail, sysDictionaryId: selectedId, parentId },
    )
    setDetailOpen(true)
  }
  function saveDictionary() {
    if (!dictionaryForm.name.trim() || !dictionaryForm.type.trim()) {
      toast.error(t('Name and type are required'))
      return
    }
    dictionaryMutation.mutate()
  }
  function saveDetail() {
    if (!detailForm.label.trim() || !detailForm.value.trim()) {
      toast.error(t('Label and value are required'))
      return
    }
    detailMutation.mutate()
  }

  return (
    <div className="page-stack">
      <header className="page-header">
        <div>
          <p className="eyebrow">{t('System configuration')}</p>
          <h1>{t('Dictionaries')}</h1>
          <p>{t('Manage dictionaries and their hierarchical detail values.')}</p>
        </div>
        <Button onClick={() => editDictionary()} variant="primary">
          <Plus size={16} />
          {t('New dictionary')}
        </Button>
      </header>
      <section className="dictionary-workbench">
        <aside className="dictionary-list">
          <div className="toolbar compact">
            <input
              aria-label="Search dictionaries"
              onChange={(event) => setSearch(event.target.value)}
              placeholder={t('Dictionary name')}
              value={search}
            />
            <Button aria-label={t('Search')} onClick={() => setFilter(search)}>
              <Search size={15} />
            </Button>
          </div>
          {dictionaries.data?.map((item) => (
            <button
              className={selectedId === item.id ? 'active' : ''}
              key={item.id}
              onClick={() => setSelectedId(item.id)}
              type="button"
            >
              <strong>{item.name}</strong>
              <small>{item.type}</small>
            </button>
          ))}
        </aside>
        <div className="dictionary-details">
          <div className="content-toolbar">
            <div>
              <h2>{selected?.name ?? t('Dictionaries')}</h2>
              {selected && (
                <p>
                  <code>{selected.type}</code> · {t(selected.status ? 'Enabled' : 'Disabled')}
                </p>
              )}
            </div>
            {selected && (
              <div className="header-actions">
                <Button onClick={() => editDictionary(selected)}>{t('Edit dictionary')}</Button>
                <Button
                  onClick={() =>
                    void confirmAction(t('Delete this dictionary?')).then((yes) => {
                      if (yes) deleteDictionaryMutation.mutate(selected.id)
                    })
                  }
                >
                  {t('Delete')}
                </Button>
                <Button onClick={() => editDetail()} variant="primary">
                  <Plus size={15} />
                  {t('New root detail')}
                </Button>
              </div>
            )}
          </div>
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>{t('Label')}</th>
                  <th>{t('Value')}</th>
                  <th>{t('Sort')}</th>
                  <th>{t('Status')}</th>
                  <th>{t('Actions')}</th>
                </tr>
              </thead>
              <tbody>
                {flatten(details.data ?? []).map((item) => (
                  <tr key={item.id}>
                    <td style={{ paddingLeft: 14 + item.displayLevel * 20 }}>
                      <strong>{item.label}</strong>
                      <small>{item.path}</small>
                    </td>
                    <td>
                      <code>{item.value}</code>
                    </td>
                    <td>{item.sort}</td>
                    <td>
                      <span className={item.status ? 'status enabled' : 'status'}>
                        {t(item.status ? 'Enabled' : 'Disabled')}
                      </span>
                    </td>
                    <td>
                      <div className="row-actions">
                        <Button onClick={() => editDetail(undefined, item.id)} variant="ghost">
                          {t('Add child')}
                        </Button>
                        <Button onClick={() => editDetail(item)} variant="ghost">
                          {t('Edit')}
                        </Button>
                        <Button
                          onClick={() =>
                            void confirmAction(t('Delete this dictionary detail?')).then((yes) => {
                              if (yes && selectedId)
                                deleteDetailMutation.mutate({ dictionaryId: selectedId, id: item.id })
                            })
                          }
                          variant="ghost"
                        >
                          {t('Delete')}
                        </Button>
                      </div>
                    </td>
                  </tr>
                ))}
                {selectedId && !details.isLoading && !details.data?.length && (
                  <tr>
                    <td className="empty-cell" colSpan={5}>
                      {t('No dictionary details')}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      </section>
      <Modal
        footer={
          <>
            <Button onClick={() => setDictionaryOpen(false)}>{t('Cancel')}</Button>
            <Button disabled={dictionaryMutation.isPending} onClick={saveDictionary} variant="primary">
              {t('Save')}
            </Button>
          </>
        }
        onOpenChange={setDictionaryOpen}
        open={dictionaryOpen}
        title={t(dictionaryForm.id ? 'Edit dictionary' : 'New dictionary')}
      >
        <div className="form-grid">
          <label>
            {t('Name')}
            <input
              onChange={(event) => setDictionaryForm((current) => ({ ...current, name: event.target.value }))}
              value={dictionaryForm.name}
            />
          </label>
          <label>
            {t('Type')}
            <input
              onChange={(event) => setDictionaryForm((current) => ({ ...current, type: event.target.value }))}
              value={dictionaryForm.type}
            />
          </label>
          <label>
            {t('Description')}
            <input
              onChange={(event) => setDictionaryForm((current) => ({ ...current, desc: event.target.value }))}
              value={dictionaryForm.desc}
            />
          </label>
          <label>
            {t('Status')}
            <select
              onChange={(event) =>
                setDictionaryForm((current) => ({ ...current, status: event.target.value === 'enabled' }))
              }
              value={dictionaryForm.status ? 'enabled' : 'disabled'}
            >
              <option value="enabled">{t('Enabled')}</option>
              <option value="disabled">{t('Disabled')}</option>
            </select>
          </label>
        </div>
      </Modal>
      <Modal
        footer={
          <>
            <Button onClick={() => setDetailOpen(false)}>{t('Cancel')}</Button>
            <Button disabled={detailMutation.isPending} onClick={saveDetail} variant="primary">
              {t('Save')}
            </Button>
          </>
        }
        onOpenChange={setDetailOpen}
        open={detailOpen}
        title={t(detailForm.id ? 'Edit dictionary detail' : 'New dictionary detail')}
      >
        <div className="form-grid">
          <label>
            {t('Label')}
            <input
              onChange={(event) => setDetailForm((current) => ({ ...current, label: event.target.value }))}
              value={detailForm.label}
            />
          </label>
          <label>
            {t('Value')}
            <input
              onChange={(event) => setDetailForm((current) => ({ ...current, value: event.target.value }))}
              value={detailForm.value}
            />
          </label>
          <label>
            {t('Extend')}
            <input
              onChange={(event) => setDetailForm((current) => ({ ...current, extend: event.target.value }))}
              value={detailForm.extend}
            />
          </label>
          <label>
            {t('Sort')}
            <input
              onChange={(event) => setDetailForm((current) => ({ ...current, sort: Number(event.target.value) }))}
              type="number"
              value={detailForm.sort}
            />
          </label>
          <label>
            {t('Status')}
            <select
              onChange={(event) =>
                setDetailForm((current) => ({ ...current, status: event.target.value === 'enabled' }))
              }
              value={detailForm.status ? 'enabled' : 'disabled'}
            >
              <option value="enabled">{t('Enabled')}</option>
              <option value="disabled">{t('Disabled')}</option>
            </select>
          </label>
        </div>
      </Modal>
    </div>
  )
}
