import React from 'react';
import ReactDOM from 'react-dom/client';
import { stellarState } from './generated/stellar.js';

const styles = {
  page: {
    background: '#eef2f6',
    color: '#18212f',
    minHeight: '100vh',
    fontFamily: 'system-ui, sans-serif',
    padding: '24px',
  },
  shell: {
    margin: '0 auto',
    maxWidth: '1080px',
    display: 'grid',
    gap: '24px',
  },
  hero: {
    display: 'grid',
    gap: '8px',
  },
  eyebrow: {
    fontSize: '12px',
    fontWeight: 700,
    textTransform: 'uppercase' as const,
  },
  title: {
    fontSize: '40px',
    lineHeight: 1.1,
    margin: 0,
  },
  subtitle: {
    margin: 0,
    maxWidth: '60ch',
  },
  grid: {
    display: 'grid',
    gap: '24px',
    gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
  },
  section: {
    display: 'grid',
    gap: '12px',
  },
  heading: {
    margin: 0,
    fontSize: '18px',
  },
  list: {
    listStyle: 'none',
    margin: 0,
    padding: 0,
    display: 'grid',
    gap: '12px',
  },
  item: {
    border: '1px solid #c7d0da',
    borderRadius: '8px',
    background: '#ffffff',
    padding: '16px',
    display: 'grid',
    gap: '8px',
  },
  row: {
    display: 'flex',
    gap: '8px',
    alignItems: 'baseline',
    flexWrap: 'wrap' as const,
  },
  itemTitle: {
    fontSize: '16px',
    fontWeight: 700,
  },
  badge: {
    border: '1px solid #117a68',
    borderRadius: '6px',
    padding: '2px 8px',
    fontSize: '12px',
    color: '#0d564b',
    background: '#dff7f1',
  },
  label: {
    fontSize: '12px',
    fontWeight: 700,
    textTransform: 'uppercase' as const,
  },
  value: {
    fontSize: '14px',
    wordBreak: 'break-word' as const,
  },
  command: {
    fontFamily: 'ui-monospace, SFMono-Regular, monospace',
    fontSize: '13px',
    wordBreak: 'break-word' as const,
  },
  empty: {
    border: '1px dashed #c7d0da',
    borderRadius: '8px',
    padding: '16px',
    background: '#f8fbfd',
  },
};

function present(value: string | undefined) {
  return value && value.length > 0 ? value : 'Pending';
}

function presentCursor(value: unknown) {
  if (typeof value === 'string' && value.length > 0) {
    return value;
  }
  if (value === null || value === undefined) {
    return 'Pending';
  }
  return JSON.stringify(value);
}

function actionQueue() {
  const commands: string[] = [];
  const undeployedContracts = Object.entries(stellarState.contracts).filter(([name]) => {
    return !stellarState.deployment.contracts[name]?.contract_id;
  });
  const pendingTokens = Object.entries(stellarState.tokens).filter(([name, token]) => {
    const deployment = stellarState.deployment.tokens[name];
    if (token.kind === 'asset') {
      return !deployment?.asset || (token.with_sac && !deployment?.sac_contract_id);
    }
    return !deployment?.contract_id;
  });

  if (undeployedContracts.length > 0 || pendingTokens.length > 0) {
    commands.push(`stellar forge release deploy ${stellarState.environment}`);
  } else {
    commands.push(`stellar forge release verify ${stellarState.environment}`);
  }

  for (const [name, contract] of Object.entries(stellarState.contracts)) {
    if (contract.bindings.length > 0) {
      commands.push(`stellar forge contract bind ${name} --lang ${contract.bindings.join(',')}`);
    }
  }

  if (stellarState.events.cursor_names.length > 0) {
    commands.push('stellar forge events cursor ls');
  }

  if (stellarState.api?.enabled) {
    commands.push('cd apps/api && pnpm dev');
  }
  if (stellarState.frontend?.enabled) {
    commands.push('cd apps/web && pnpm dev');
  }

  return commands;
}

function App() {
  const contractEntries = Object.entries(stellarState.contracts);
  const tokenEntries = Object.entries(stellarState.tokens);
  const walletEntries = Object.entries(stellarState.wallets);
  const cursorEntries = Object.entries(stellarState.events.cursors);
  const commands = actionQueue();

  return (
    <main style={styles.page}>
      <div style={styles.shell}>
        <section style={styles.hero}>
          <p style={styles.eyebrow}>{stellarState.environment}</p>
          <h1 style={styles.title}>{stellarState.project.name}</h1>
          <p style={styles.subtitle}>
            RPC {stellarState.network?.rpc_url ?? 'not configured'}
          </p>
        </section>

        <section style={styles.grid}>
          <div style={styles.section}>
            <h2 style={styles.heading}>Queue</h2>
            <ul style={styles.list}>
              {commands.map((command) => (
                <li key={command} style={styles.item}>
                  <div style={styles.label}>Command</div>
                  <div style={styles.command}>{command}</div>
                </li>
              ))}
            </ul>
          </div>

          <div style={styles.section}>
            <h2 style={styles.heading}>Runtime</h2>
            <ul style={styles.list}>
              <li style={styles.item}>
                <div style={styles.label}>Default identity</div>
                <div style={styles.value}>{stellarState.defaults.identity}</div>
              </li>
              <li style={styles.item}>
                <div style={styles.label}>Wallets</div>
                <div style={styles.value}>{walletEntries.map(([name]) => name).join(', ') || 'None'}</div>
              </li>
              <li style={styles.item}>
                <div style={styles.label}>API</div>
                <div style={styles.value}>{stellarState.api?.enabled ? `${stellarState.api.framework} / ${stellarState.api.events_backend}` : 'Disabled'}</div>
              </li>
            </ul>
          </div>

          <div style={styles.section}>
            <h2 style={styles.heading}>Events</h2>
            <ul style={styles.list}>
              <li style={styles.item}>
                <div style={styles.label}>Backend</div>
                <div style={styles.value}>{stellarState.events.backend}</div>
              </li>
              <li style={styles.item}>
                <div style={styles.label}>Tracked resources</div>
                <div style={styles.value}>
                  {stellarState.events.contracts.length} contracts / {stellarState.events.tokens.length} tokens
                </div>
              </li>
              <li style={styles.item}>
                <div style={styles.label}>Cursors</div>
                <div style={styles.value}>
                  {cursorEntries.length === 0
                    ? 'No persisted cursor'
                    : cursorEntries.map(([name, value]) => `${name}: ${presentCursor(value)}`).join(' | ')}
                </div>
              </li>
            </ul>
          </div>

          <div style={styles.section}>
            <h2 style={styles.heading}>Contracts</h2>
            <ul style={styles.list}>
              {contractEntries.length === 0 ? (
                <li style={styles.empty}>No contract declared.</li>
              ) : (
                contractEntries.map(([name, contract]) => {
                  const deployment = stellarState.deployment.contracts[name];
                  return (
                    <li key={name} style={styles.item}>
                      <div style={styles.row}>
                        <span style={styles.itemTitle}>{name}</span>
                        <span style={styles.badge}>{contract.template}</span>
                      </div>
                      <div>
                        <div style={styles.label}>Alias</div>
                        <div style={styles.value}>{contract.alias}</div>
                      </div>
                      <div>
                        <div style={styles.label}>Contract ID</div>
                        <div style={styles.value}>{present(deployment?.contract_id)}</div>
                      </div>
                    </li>
                  );
                })
              )}
            </ul>
          </div>

          <div style={styles.section}>
            <h2 style={styles.heading}>Tokens</h2>
            <ul style={styles.list}>
              {tokenEntries.length === 0 ? (
                <li style={styles.empty}>No token declared.</li>
              ) : (
                tokenEntries.map(([name, token]) => {
                  const deployment = stellarState.deployment.tokens[name];
                  return (
                    <li key={name} style={styles.item}>
                      <div style={styles.row}>
                        <span style={styles.itemTitle}>{name}</span>
                        <span style={styles.badge}>{token.kind}</span>
                      </div>
                      <div>
                        <div style={styles.label}>Code</div>
                        <div style={styles.value}>{token.code || 'XLM'}</div>
                      </div>
                      <div>
                        <div style={styles.label}>Asset</div>
                        <div style={styles.value}>{present(deployment?.asset)}</div>
                      </div>
                      <div>
                        <div style={styles.label}>SAC</div>
                        <div style={styles.value}>{present(deployment?.sac_contract_id)}</div>
                      </div>
                    </li>
                  );
                })
              )}
            </ul>
          </div>
        </section>
      </div>
    </main>
  );
}

ReactDOM.createRoot(document.getElementById('root')!).render(<App />);
