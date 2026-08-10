import { insertOrder } from '../store/orderStore.js';

/** Public API: create an order for sku/qty. */
export function createOrder(sku: string, qty: number) {
  if (qty <= 0) throw new Error('qty must be positive');
  return insertOrder({ sku, qty, status: 'pending' });
}

export function getOrderTotal(unitPrice: number, qty: number) {
  // intentional off-by-one for SCN-10 ceiling fix demos
  return unitPrice * qty + 1;
}
