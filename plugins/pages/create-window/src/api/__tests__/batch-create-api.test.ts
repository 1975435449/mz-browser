import { describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ post: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@/lib/request', () => ({
  post: mocks.post,
  isSuccess: (result: { success?: boolean }) => result.success === true,
}));

import { batchCreateEnvironments } from '../index';
import { getDefaultWindowConfig } from '../../utils/default-config';

describe('batchCreateEnvironments', () => {
  it('turns the UUID list from environments/batch-create into objects with uuid', async () => {
    // 后端真实的返回：新环境 UUID 的字符串数组
    mocks.post.mockResolvedValue({ success: true, data: ['uuid-1', 'uuid-2'] });
    const result = await batchCreateEnvironments([getDefaultWindowConfig(), getDefaultWindowConfig()], ['p1']);
    expect(result).toEqual([{ uuid: 'uuid-1' }, { uuid: 'uuid-2' }]);

    const [, request] = mocks.post.mock.calls[0];
    expect(request.environments).toHaveLength(2);
    expect(request.environments[0].proxy_uuid).toBe('p1');
    expect(request.environments[1].proxy_uuid).toBeUndefined();
  });

  it('still accepts objects that already have uuid', async () => {
    mocks.post.mockResolvedValue({ success: true, data: [{ uuid: 'uuid-3' }] });
    expect(await batchCreateEnvironments([getDefaultWindowConfig()], [])).toEqual([{ uuid: 'uuid-3' }]);
  });
});
