import { listOrders } from '../store/orderStore.js';

export function reservedQty(sku: string): number {
  return listOrders()
    .filter((o) => o.sku === sku)
    .reduce((n, o) => n + o.qty, 0);
}
