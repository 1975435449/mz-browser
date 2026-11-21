import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ batchCreate: vi.fn(), notify: vi.fn() }));
vi.mock('../../api', () => ({ batchCreateEnvironments: mocks.batchCreate }));
vi.mock('@/lib/app-events', () => ({ notifyEnvironmentsChanged: mocks.notify }));

import { getDefaultWindowConfig } from '../default-config';
import { buildEnvironmentName, createEnvironmentsForApi, missingProxyCount } from '../batch-create';

/** 一台设备的指纹：批量里每个环境都应该不同 */
const device = (config: ReturnType<typeof getDefaultWindowConfig>) =>
  JSON.stringify([
    config.advancedFingerprintSettings.webglRenderer,
    config.advancedFingerprintSettings.resolution,
    config.advancedFingerprintSettings.devicePixelRatio,
    config.deviceSettings.hardwareConcurrency,
    config.deviceSettings.deviceMemory,
    config.advancedFingerprintSettings.fontList.fonts,
    config.deviceSettings.macAddress,
  ]);

describe('batch environment creation', () => {
  beforeEach(() => {
    mocks.batchCreate.mockReset();
    mocks.notify.mockReset();
    mocks.batchCreate.mockImplementation(async (configs: unknown[]) =>
      configs.map((_, index) => ({ uuid: `env-${index}` }))
    );
  });

  it('names only get a number when more than one is created', () => {
    expect(buildEnvironmentName('Shop', 0, 1)).toBe('Shop');
    expect(buildEnvironmentName('Shop', 2, 5)).toBe('Shop_3');
    expect(buildEnvironmentName('  ', 0, 2)).toBe('窗口_1');
  });

  it('counts the environments that would get no proxy', () => {
    expect(missingProxyCount(5, 0)).toBe(0);
    expect(missingProxyCount(5, 2)).toBe(3);
    expect(missingProxyCount(5, 5)).toBe(0);
    expect(missingProxyCount(5, 8)).toBe(0);
  });

  it('gives every environment its own device and assigns proxies in order', async () => {
    const result = await createEnvironmentsForApi({
      count: 20,
      namePrefix: 'Api',
      proxyUuids: Array.from({ length: 20 }, (_, index) => `p${index + 1}`),
      tagUuids: ['t1'],
      groupUuid: 'g1',
    });
    const [configs, proxies, options] = mocks.batchCreate.mock.calls[0];
    expect(proxies).toHaveLength(20);
    expect(options).toEqual({ groupUuid: 'g1', tagUuids: ['t1'] });
    // 设备样本是随机抽的，20 台里允许偶尔撞上，但绝大多数必须不同
    expect(new Set(configs.map(device)).size).toBeGreaterThan(15);
    expect(result.items.slice(0, 2)).toEqual([
      { uuid: 'env-0', name: 'Api_1', proxy_uuid: 'p1' },
      { uuid: 'env-1', name: 'Api_2', proxy_uuid: 'p2' },
    ]);
    // 环境列表收到通知后重新加载
    expect(mocks.notify).toHaveBeenCalledTimes(1);
  });

  it('refuses a bad count and too few proxies unless allowed', async () => {
    await expect(createEnvironmentsForApi({ count: 0 })).rejects.toThrow('from 1 to 25');
    await expect(createEnvironmentsForApi({ count: 26 })).rejects.toThrow('from 1 to 25');
    await expect(createEnvironmentsForApi({ count: 4, proxyUuids: ['p1'] })).rejects.toThrow('3 would have no proxy');
    expect(mocks.batchCreate).not.toHaveBeenCalled();

    const result = await createEnvironmentsForApi({ count: 2, proxyUuids: ['p1'], allowFewerProxies: true });
    expect(result.items.map((item) => item.proxy_uuid)).toEqual(['p1', null]);
  });
});
