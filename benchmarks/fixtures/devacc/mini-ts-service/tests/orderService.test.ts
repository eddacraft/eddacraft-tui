import { getOrderTotal } from '../src/services/orderService.js';

export function testGetOrderTotal() {
  const got = getOrderTotal(10, 2);
  if (got !== 20) throw new Error(`expected 20, got ${got}`);
}
