/**
 * 页面之间的轻量通知（同一个窗口内，用 window 事件）。
 */

/** 环境被创建、删除等在别处改动了，环境列表收到后重新加载 */
export const ENVIRONMENTS_CHANGED_EVENT = 'mz:environments-changed';

export function notifyEnvironmentsChanged(): void {
  window.dispatchEvent(new Event(ENVIRONMENTS_CHANGED_EVENT));
}
