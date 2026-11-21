import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Layers, Loader2 } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  listGroups,
  listProxies,
  type GroupItem,
  type ProxyItem,
} from '../../../environment-manager/src/api';
import { MAX_BATCH_CREATE, createEnvironmentsForApi, missingProxyCount } from '../utils/batch-create';

interface BatchCreateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const DEFAULT_COUNT = 5;

/**
 * 批量创建环境：选数量、名称前缀、分组和代理。每个环境用默认设置，设备指纹各自随机；
 * 代理按勾选顺序一个环境一个，代理比环境少时先提示，再由用户决定。
 */
export function BatchCreateDialog({ open, onOpenChange }: BatchCreateDialogProps) {
  const { t } = useTranslation('create-window');
  const [count, setCount] = useState(DEFAULT_COUNT);
  const [prefix, setPrefix] = useState('');
  const [groupUuid, setGroupUuid] = useState('');
  const [proxyUuids, setProxyUuids] = useState<string[]>([]);
  const [groups, setGroups] = useState<GroupItem[]>([]);
  const [proxies, setProxies] = useState<ProxyItem[]>([]);
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    if (!open) return;
    void listGroups().then(setGroups).catch(() => setGroups([]));
    void listProxies().then(setProxies).catch(() => setProxies([]));
  }, [open]);

  const reset = () => {
    setCount(DEFAULT_COUNT);
    setPrefix('');
    setGroupUuid('');
    setProxyUuids([]);
  };

  const missing = missingProxyCount(count, proxyUuids.length);

  const toggleProxy = (uuid: string) =>
    setProxyUuids((current) =>
      current.includes(uuid) ? current.filter((item) => item !== uuid) : [...current, uuid]
    );

  const create = async () => {
    setCreating(true);
    try {
      // 代理不够的情况已经在对话框里提示过，用户点了“仍然创建”就是同意
      const result = await createEnvironmentsForApi({
        count,
        namePrefix: prefix,
        groupUuid,
        proxyUuids,
        allowFewerProxies: true,
      });
      toast.success(t('batch.success', { count: result.items.length }));
      reset();
      onOpenChange(false);
    } catch (error) {
      toast.error(error instanceof Error ? error.message : t('batch.failed'));
    } finally {
      setCreating(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!creating) onOpenChange(next);
      }}
    >
      <DialogContent className="sm:max-w-[520px]">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Layers className="h-4 w-4 text-primary" />
            {t('batch.title')}
          </DialogTitle>
          <DialogDescription>{t('batch.description')}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4 text-sm">
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="batch-count">{t('batch.count')}</Label>
              <Input
                id="batch-count"
                type="number"
                min={1}
                max={MAX_BATCH_CREATE}
                value={count}
                onChange={(event) => {
                  const value = Math.round(Number(event.target.value));
                  setCount(Number.isFinite(value) ? Math.min(MAX_BATCH_CREATE, Math.max(1, value)) : 1);
                }}
              />
              <p className="text-[11px] text-muted-foreground">{t('batch.countHint', { max: MAX_BATCH_CREATE })}</p>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="batch-prefix">{t('batch.namePrefix')}</Label>
              <Input
                id="batch-prefix"
                value={prefix}
                placeholder={t('batch.namePrefixPlaceholder')}
                onChange={(event) => setPrefix(event.target.value)}
              />
              <p className="text-[11px] text-muted-foreground">
                {t('batch.namePrefixHint', { prefix: prefix.trim() || t('batch.namePrefixPlaceholder') })}
              </p>
            </div>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="batch-group">{t('batch.group')}</Label>
            <select
              id="batch-group"
              value={groupUuid}
              onChange={(event) => setGroupUuid(event.target.value)}
              className="h-9 w-full rounded-md border border-input bg-background px-3 text-sm"
            >
              <option value="">{t('batch.noGroup')}</option>
              {groups.map((group) => (
                <option key={group.uuid} value={group.uuid}>
                  {group.name}
                </option>
              ))}
            </select>
          </div>

          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <Label>{t('batch.proxies')}</Label>
              <span className="text-[11px] text-muted-foreground">
                {t('batch.proxiesSelected', { count: proxyUuids.length })}
              </span>
            </div>
            {proxies.length === 0 ? (
              <p className="rounded-md border border-dashed p-3 text-xs text-muted-foreground">{t('batch.proxiesNone')}</p>
            ) : (
              <ul className="max-h-44 space-y-0.5 overflow-auto rounded-md border p-1.5">
                {proxies.map((proxy) => {
                  const order = proxyUuids.indexOf(proxy.uuid);
                  return (
                    <li key={proxy.uuid}>
                      <label className="flex cursor-pointer items-center gap-2 rounded px-1.5 py-1 text-xs hover:bg-muted/60">
                        <Checkbox checked={order >= 0} onCheckedChange={() => toggleProxy(proxy.uuid)} />
                        <span className="w-5 text-center font-mono text-[10px] text-primary">
                          {order >= 0 ? order + 1 : ''}
                        </span>
                        <span className="truncate text-foreground">{proxy.name}</span>
                        <span className="ml-auto shrink-0 font-mono text-[10px] text-muted-foreground">
                          {proxy.host}:{proxy.port}
                        </span>
                      </label>
                    </li>
                  );
                })}
              </ul>
            )}
            <p className="text-[11px] text-muted-foreground">{t('batch.proxyOrderHint')}</p>
          </div>

          <p className="rounded-md bg-muted/50 p-2.5 text-[11px] text-muted-foreground">{t('batch.fingerprintNote')}</p>

          {missing > 0 && (
            <p className="rounded-md border border-amber-500/40 bg-amber-500/10 p-2.5 text-xs text-amber-700 dark:text-amber-400">
              {t('batch.shortage', { count, proxies: proxyUuids.length, missing })}
            </p>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" size="sm" disabled={creating} onClick={() => onOpenChange(false)}>
            {t('batch.cancel')}
          </Button>
          <Button size="sm" disabled={creating} onClick={() => void create()}>
            {creating ? <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" /> : <Layers className="mr-1.5 h-3.5 w-3.5" />}
            {missing > 0 ? t('batch.createAnyway', { count }) : t('batch.create', { count })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
