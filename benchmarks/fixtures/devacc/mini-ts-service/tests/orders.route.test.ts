import { handleCreateOrder } from '../src/routes/orders.js';

export function testHandleCreateOrder() {
  const order = handleCreateOrder({ sku: 'A1', qty: 1 });
  if (order.sku !== 'A1') throw new Error('sku mismatch');
}
