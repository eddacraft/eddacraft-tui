---
id: websocket-realtime
name: WebSocket Real-time
description: Implement real-time communication using WebSockets
category: integration
tags: [websocket, realtime, socket.io, events, live]
variables:
  - name: feature_name
    description: Name of the real-time feature
    required: true
  - name: event_types
    description: Comma-separated list of event types
    default: connect,disconnect,message
    required: false
---

# WebSocket Real-time: {{ feature_name }}

## Intent

Implement real-time {{ feature_name }} functionality using WebSockets with
proper connection management and error handling.

## Changes

### 1. Create WebSocket Server

- **File**: `src/websocket/server.ts`
- **Action**: Create
- **Description**: WebSocket server setup and configuration

### 2. Create Event Handlers

- **File**: `src/websocket/handlers/{{ feature_name }}.handler.ts`
- **Action**: Create
- **Description**: Handle {{ feature_name }} events: {{ event_types }}

### 3. Create Client Hook

- **File**: `src/hooks/use{{ feature_name }}Socket.ts`
- **Action**: Create
- **Description**: React hook for WebSocket connection

### 4. Create Types

- **File**: `src/types/websocket.types.ts`
- **Action**: Create
- **Description**: TypeScript types for WebSocket events

### 5. Add Tests

- **File**: `src/websocket/__tests__/{{ feature_name }}.test.ts`
- **Action**: Create
- **Description**: WebSocket integration tests

### 6. Update Server Entry

- **File**: `src/server.ts`
- **Action**: Modify
- **Description**: Integrate WebSocket server with HTTP server

## Event Structure

```typescript
interface WebSocketEvent<T = unknown> {
  type: string;
  payload: T;
  timestamp: number;
  userId?: string;
}
```

## Events

| Event                     | Direction       | Description            |
| ------------------------- | --------------- | ---------------------- |
| connect                   | Server → Client | Connection established |
| disconnect                | Server → Client | Connection closed      |
| error                     | Server → Client | Error occurred         |
| {{ feature_name }}:update | Server → Client | Data update            |
| {{ feature_name }}:action | Client → Server | User action            |

## Client Usage

```typescript
const { isConnected, send, data } = use{{ feature_name }}Socket();

// Send event
send('{{ feature_name }}:action', { ... });
```

## Connection Management

- [ ] Automatic reconnection on disconnect
- [ ] Heartbeat/ping-pong for connection health
- [ ] Graceful degradation when WebSocket unavailable
- [ ] Connection state management

## Security

- [ ] Authentication on connection
- [ ] Message validation
- [ ] Rate limiting
- [ ] Origin verification

## Acceptance Criteria

- [ ] Real-time updates working
- [ ] Reconnection handling functional
- [ ] Error handling complete
- [ ] Tests passing
- [ ] No memory leaks
