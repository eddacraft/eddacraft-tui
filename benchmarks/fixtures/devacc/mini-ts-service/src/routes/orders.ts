import { createOrder } from '../services/orderService.js';

export function handleCreateOrder(body: { sku: string; qty: number }) {
  return createOrder(body.sku, body.qty);
}
