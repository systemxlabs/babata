import { type SystemResponse } from '../api/types';
import { apiGet } from '../api/client';
import { Panel } from '../components/Panel';
import { StatusBadge } from '../components/StatusBadge';
import { usePolling } from '../hooks/usePolling';

export function SystemPage() {
  const system = usePolling(() => apiGet<SystemResponse>('/system'), {
    intervalMs: 5000,
  });

  if (system.error) {
    return (
      <Panel eyebrow="System" title="Service ledger" titleLevel={1}>
        <p className="panel__lede">Failed to load system metadata: {system.error.message}</p>
      </Panel>
    );
  }

  if (!system.data) {
    return (
      <Panel eyebrow="System" title="Service ledger" titleLevel={1}>
        <p className="panel__lede">Loading service metadata...</p>
      </Panel>
    );
  }

  const dashboardUrl = `http://${system.data.http_addr}`;

  return (
    <div className="page-stack">
      <Panel
        className="panel--hero"
        eyebrow="System"
        meta={<StatusBadge status="running" />}
        title="Service ledger"
        titleLevel={1}
      >
        <p className="panel__lede">
          Operational context stays narrow in v1: enough runtime metadata to orient the
          operator without turning the dashboard into a settings product.
        </p>
      </Panel>

      <div className="page-grid page-grid--two-up">
        <Panel eyebrow="Health" title="Service reachability">
          <dl className="fact-list">
            <div className="fact-list__row">
              <dt>Status</dt>
              <dd>Reachable</dd>
            </div>
            <div className="fact-list__row">
              <dt>Dashboard URL</dt>
              <dd>{dashboardUrl}</dd>
            </div>
          </dl>
        </Panel>

        <Panel eyebrow="Build" title="Runtime metadata">
          <dl className="fact-list">
            <div className="fact-list__row">
              <dt>Version</dt>
              <dd>{system.data.version}</dd>
            </div>
            <div className="fact-list__row">
              <dt>Listen address</dt>
              <dd>{system.data.http_addr}</dd>
            </div>
            <div className="fact-list__row">
              <dt>Data directory</dt>
              <dd>Pending backend export</dd>
            </div>
          </dl>
        </Panel>
      </div>
    </div>
  );
}
