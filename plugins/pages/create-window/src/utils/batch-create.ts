import { notifyEnvironmentsChanged } from '@/lib/app-events';
import { batchCreateEnvironments } from '../api';
import { getDefaultWindowConfig } from './default-config';

/** 一次最多创建的环境数 */
export const MAX_BATCH_CREATE = 25;

/**
 * 新建环境的名称。只创建一个时原样使用输入的名称（没填则叫"窗口"）；
 * 一次创建多个时才加 _1、_2… 序号来区分。
 */
export function buildEnvironmentName(name: string | undefined, index: number, total: number): string {
  const base = name?.trim() || '窗口';
  return total > 1 ? `${base}_${index + 1}` : base;
}

/** 选了代理但比环境少时，有几个环境分不到代理；没选代理或代理够用时为 0 */
export function missingProxyCount(environments: number, proxies: number): number {
  return proxies > 0 && proxies < environments ? environments - proxies : 0;
}

export interface BatchCreateRequest {
  count?: unknown;
  namePrefix?: unknown;
  groupUuid?: unknown;
  tagUuids?: unknown;
  proxyUuids?: unknown;
  allowFewerProxies?: unknown;
}

const stringList = (value: unknown): string[] =>
  Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string' && item.trim() !== '') : [];

/**
 * 批量创建环境（“批量创建环境”对话框和 HTTP 接口共用）。每个环境都用一份新生成的默认配置，
 * 设备（显卡、核心数、内存、屏幕、字体、设备名、MAC）各自从样本库抽取，同一批环境不共用指纹。
 * 代理按顺序一个环境一个；代理比环境少时，没有明确允许就拒绝，避免悄悄创建出没有代理的环境。
 */
export async function createEnvironmentsForApi(request: BatchCreateRequest) {
  const count = Number(request.count);
  if (!Number.isInteger(count) || count < 1 || count > MAX_BATCH_CREATE) {
    throw new Error(`count must be an integer from 1 to ${MAX_BATCH_CREATE}`);
  }
  const proxyUuids = stringList(request.proxyUuids);
  const missing = missingProxyCount(count, proxyUuids.length);
  if (missing > 0 && request.allowFewerProxies !== true) {
    throw new Error(
      `Only ${proxyUuids.length} proxies for ${count} environments; ${missing} would have no proxy. ` +
        'Pass allow_fewer_proxies=true to create them anyway, or provide more proxies.'
    );
  }
  const prefix = typeof request.namePrefix === 'string' ? request.namePrefix : undefined;
  const configs = Array.from({ length: count }, (_, index) => {
    const config = getDefaultWindowConfig();
    config.windowInfo.name = buildEnvironmentName(prefix, index, count);
    return config;
  });
  const tagUuids = stringList(request.tagUuids);
  const results = await batchCreateEnvironments(configs, proxyUuids, {
    groupUuid: typeof request.groupUuid === 'string' && request.groupUuid ? request.groupUuid : undefined,
    tagUuids: tagUuids.length > 0 ? tagUuids : undefined,
  });
  notifyEnvironmentsChanged();
  return {
    items: results.map((result, index) => ({
      uuid: result.uuid,
      name: configs[index].windowInfo.name,
      proxy_uuid: proxyUuids[index] ?? null,
    })),
  };
}
