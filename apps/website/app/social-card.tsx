export function SocialCard() {
  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        background: '#0D0D0F',
        color: '#EBEBEB',
        fontFamily: 'monospace',
        padding: '54px 64px',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          borderBottom: '1px solid #2A2A2E',
          paddingBottom: 22,
          fontSize: 20,
        }}
      >
        <div style={{ display: 'flex', gap: 30 }}>
          <span style={{ color: '#EBEBEB' }}>eddacraft</span>
          <span style={{ color: '#CC5500' }}>anvil</span>
        </div>
        <span style={{ color: '#EBEBEB' }}>v0.9.5-beta</span>
      </div>

      <div style={{ flex: 1, display: 'flex', alignItems: 'center', gap: 64 }}>
        <div style={{ width: '48%', display: 'flex', flexDirection: 'column' }}>
          <div style={{ display: 'flex', flexDirection: 'column', fontSize: 52, lineHeight: 1.08 }}>
            <span>TRUST THE CODE</span>
            <span style={{ color: '#CC5500' }}>YOUR AI WRITES.</span>
          </div>
          <div style={{ marginTop: 30, color: '#EBEBEB', fontSize: 20, lineHeight: 1.45 }}>
            Independent, deterministic control for AI-assisted software engineering.
          </div>
        </div>

        <div
          style={{
            width: '52%',
            display: 'flex',
            flexDirection: 'column',
            border: '1px solid #2A2A2E',
            background: '#0D0D0F',
            padding: 26,
            fontSize: 18,
            lineHeight: 1.65,
          }}
        >
          <div style={{ color: '#CC5500' }}>MCP REQUEST :: anvil_validate_write</div>
          <div>target :: src/secret.ts</div>
          <div style={{ marginTop: 22, color: '#CC5500' }}>[ = ] PRE_WRITE_VALIDATION</div>
          <div style={{ marginTop: 16 }}>[ ERR ] secret-detection</div>
          <div style={{ color: '#EBEBEB' }}>credential-like token in proposed content</div>
          <div
            style={{
              display: 'flex',
              gap: 8,
              borderTop: '1px solid #2A2A2E',
              marginTop: 18,
              paddingTop: 18,
            }}
          >
            <span>decision</span>
            <span style={{ color: '#CC5500' }}>interrupt</span>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <span>safe_default</span>
            <span style={{ color: '#EBEBEB' }}>do-not-write</span>
          </div>
        </div>
      </div>
    </div>
  );
}
