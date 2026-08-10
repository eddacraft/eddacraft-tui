export type Order = { sku: string; qty: number; status: string };

const orders: Order[] = [];

export function insertOrder(order: Order): Order {
  orders.push(order);
  return order;
}

export function listOrders(): Order[] {
  return [...orders];
}
