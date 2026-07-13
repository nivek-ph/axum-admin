import { http } from './http';
import { withAuthHeaders, type ApiResponse } from './core';

export interface MenuMeta {
  activeName: string;
  keepAlive: boolean;
  defaultMenu: boolean;
  title: string;
  icon: string;
  closeTab: boolean;
  transitionType: string;
}

export interface MenuParameter {
  id: number;
  sysBaseMenuId: number;
  type: string;
  key: string;
  value: string;
}

export interface MenuButton {
  id: number;
  name: string;
  desc: string;
}

export interface MenuRecord {
  id: number;
  parentId: number;
  path: string;
  name: string;
  hidden: boolean;
  component: string;
  sort: number;
  meta: MenuMeta;
  parameters: MenuParameter[];
  menuBtn: MenuButton[];
  menuType?: 'directory' | 'page' | 'action' | string;
  permission?: string | null;
  status?: string;
  apiBindings?: Array<{ method: string; pathPattern: string }>;
  children: MenuRecord[];
}

export interface MenuRoleSelection {
  roleIds: number[];
}

export function normalizeMenuListResponse(payload: ApiResponse<MenuRecord[]>) {
  if (Array.isArray(payload?.data)) {
    return payload.data;
  }
  if (Array.isArray((payload?.data as any)?.menus)) {
    return (payload.data as any).menus;
  }
  return [];
}

export function normalizeMenuRoleSelection(payload: ApiResponse<MenuRoleSelection>) {
  return {
    roleIds: payload?.data?.roleIds || [],
  };
}

export async function fetchMenuList() {
  const response = await http.get('/menus/tree', withAuthHeaders());
  return normalizeMenuListResponse(response);
}
