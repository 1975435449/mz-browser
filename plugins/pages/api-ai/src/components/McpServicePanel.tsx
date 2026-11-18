import { useMemo, useState } from 'react';
import { ExternalLink, Zap } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Switch } from '@/components/ui/switch';
import type { McpConfigSnapshot, UpdateMcpConfigRequest } from '../types';
import { ApiServiceSkeleton } from './api-service-skeleton';
import { McpAccessSettings } from './McpAccessSettings';

interface McpServicePanelProps {
  config: McpConfigSnapshot | null;
  loading: boolean;
  error: string | null;
  onToggle: (enabled: boolean) => void;
  onUpdate: (updates: UpdateMcpConfigRequest) => Promise<void>;
  onRefresh: () => Promise<void>;
}

export function McpServicePanel({
  config,
  loading,
  error,
  onToggle,
  onUpdate,
  onRefresh,
}: McpServicePanelProps) {
  if (loading) {
    return <ApiServiceSkeleton />;
  }

  return (
    <div className="space-y-4">
      {config ? <McpServiceControl config={config} onToggle={onToggle} /> : null}

      {config && (
        <McpAccessSettings
          key={JSON.stringify([
            config.allowedEnvironmentUuids,
            config.pageToolsEnabled,
            config.fullPageAccessEnabled,
          ])}
          config={config}
          onUpdate={onUpdate}
          onRefresh={onRefresh}
        />
      )}

      {error ? (
        <div className="px-4 py-2 text-xs text-destructive border border-destructive/20 bg-destructive/10">
          {error}
        </div>
      ) : null}

      {config ? (
        <HttpExamples config={config} />
      ) : null}
    </div>
  );
}

function McpServiceControl({
  config,
  onToggle,
}: {
  config: McpConfigSnapshot;
  onToggle: (enabled: boolean) => void;
}) {
  const { t } = useTranslation('apiConsole');

  return (
    <div
      className={`flex items-center justify-between gap-4 rounded-lg border p-4 shadow-xs ${
        config.running ? 'border-success/30 bg-success/5' : 'border-border bg-card'
      }`}
    >
      <div className="flex items-center gap-4">
        <div
          className={`flex h-10 w-10 items-center justify-center rounded-lg ${
            config.running ? 'bg-success/10 text-success' : 'bg-muted text-muted-foreground'
          }`}
        >
          <Zap className="h-5 w-5" />
        </div>
        <div>
          <div className="flex items-center gap-2">
            <h3 className="text-[15px] font-semibold text-foreground">
              {config.running
                ? t('mcp.statusRunning')
                : t('mcp.control')}
            </h3>
            {config.running ? <span className="h-2 w-2 rounded-full bg-success" /> : null}
          </div>
          <p className="mt-0.5 font-mono text-xs text-muted-foreground">{config.endpoint}</p>
        </div>
      </div>
      <Switch data-testid="mcp-service-toggle" checked={config.running} onCheckedChange={onToggle} className="ml-4" />
    </div>
  );
}

function HttpExamples({ config }: { config: McpConfigSnapshot }) {
  const { t } = useTranslation('apiConsole');
  const [active, setActive] = useState<'curl' | 'powershell' | 'python'>('curl');

  const examples = useMemo(
    () => ({
      curl: {
        title: 'curl',
        snippet: `# 列出当前权限下可用的接口
curl ${config.endpoint}/tools -H "Authorization: Bearer <TOKEN>"

# 读取页面快照
curl -X POST ${config.endpoint}/tools/mz_browser_snapshot \\
  -H "Authorization: Bearer <TOKEN>" -H "Content-Type: application/json" \\
  -d '{"env_uuid": "<ENV_UUID>", "page_id": "<PAGE_ID>"}'`,
      },
      powershell: {
        title: 'PowerShell',
        snippet: `$headers = @{ Authorization = "Bearer $env:MZ_TOKEN" }
Invoke-RestMethod -Uri ${config.endpoint}/tools -Headers $headers

$body = @{ env_uuid = "<ENV_UUID>"; page_id = "<PAGE_ID>"; node_id = "<NODE_ID>"; text = "hello" } | ConvertTo-Json
Invoke-RestMethod -Method Post -Uri ${config.endpoint}/tools/mz_browser_type -Headers $headers -ContentType "application/json" -Body $body`,
      },
      python: {
        title: 'Python',
        snippet: `import os, requests

BASE = "${config.endpoint}"
HEADERS = {"Authorization": f"Bearer {os.environ['MZ_TOKEN']}"}

def call(name, **args):
    return requests.post(f"{BASE}/tools/{name}", headers=HEADERS, json=args).json()

print(requests.get(f"{BASE}/tools", headers=HEADERS).json())
print(call("mz_browser_tabs", env_uuid="<ENV_UUID>"))`,
      },
    }),
    [config.endpoint]
  );
  const selected = examples[active];

  return (
    <div className="grid grid-cols-1 gap-4 xl:grid-cols-[1.05fr_1.45fr]">
      <div className="rounded-lg border border-border bg-card p-4 shadow-xs">
        <div className="mb-4">
          <h4 className="text-[15px] font-semibold text-foreground">{t('http.examplesTitle')}</h4>
          <p className="mt-1 text-xs text-muted-foreground">{t('http.examplesDesc')}</p>
        </div>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          {Object.entries(examples).map(([key, example]) => {
            const isActive = key === active;
            return (
              <button
                key={key}
                type="button"
                onClick={() => {
                  setActive(key as keyof typeof examples);
                }}
                className={`border p-3 h-16 text-center rounded-sm transition-colors ${
                  isActive
                    ? 'border-primary bg-primary/5 shadow-sm'
                    : 'border-border bg-background hover:bg-muted/40'
                }`}
              >
                <div className="text-sm font-semibold text-foreground">{example.title}</div>
              </button>
            );
          })}
        </div>
      </div>

      <div className="space-y-3 rounded-lg border border-border bg-card p-4 shadow-xs">
        <div className="flex items-start justify-between gap-3">
          <h4 className="text-xs font-semibold text-foreground mb-1">{selected.title}</h4>
          <a
            href={t('http.docsUrl')}
            target="_blank"
            rel="noreferrer"
            className="text-xs text-primary hover:underline inline-flex items-center gap-1 shrink-0"
          >
            {t('http.docs')}
            <ExternalLink className="w-3 h-3" />
          </a>
        </div>
        <pre className="bg-muted border border-border rounded p-3 text-[11px] leading-5 overflow-auto whitespace-pre-wrap break-all font-mono">
          {selected.snippet}
        </pre>
      </div>
    </div>
  );
}
