---
id: multi-module
title: Multi-Module Plan
description: A realistic multi-module APS plan structure.
sidebar_position: 2
---

# Multi-Module Plan

A realistic example showing a multi-module APS structure for an e-commerce
platform.

## Directory Structure

```
plans/
├── index.aps.md
├── modules/
│   ├── auth.aps.md
│   ├── products.aps.md
│   ├── cart.aps.md
│   └── payments.aps.md
└── execution/
    └── PAY-001.steps.md
```

## Index

```markdown
---
format: aps
version: 1.0
---

# ShopCraft E-Commerce

An online store platform with authentication, product catalog, shopping cart,
and payment processing.

## Modules

- [auth](modules/auth.aps.md) — User authentication
- [products](modules/products.aps.md) — Product catalog
- [cart](modules/cart.aps.md) — Shopping cart
- [payments](modules/payments.aps.md) — Payment processing
```

## Auth Module

```markdown
---
format: aps
module: auth
files:
  - src/auth/**
  - src/middleware/auth.ts
---

# Authentication Module

User registration, login, and session management.

## Task: AUTH-001 — User registration

**Status:** complete

**Intent:** New users can create accounts with email and password. Passwords
are hashed. Email uniqueness enforced.

**Validation:** `pnpm test src/auth/register.test.ts`

**Steps:**

1. [x] POST /auth/register endpoint
2. [x] Email format validation
3. [x] Password hashing (bcrypt)
4. [x] Duplicate email rejection

---

## Task: AUTH-002 — User login

**Status:** complete

**Intent:** Users authenticate with email/password and receive JWT.

**Validation:** `pnpm test src/auth/login.test.ts`

**Steps:**

1. [x] POST /auth/login endpoint
2. [x] Credential verification
3. [x] JWT generation
4. [x] Refresh token support

---

## Task: AUTH-003 — Auth middleware

**Status:** complete

**Intent:** Protected routes require valid JWT. Invalid tokens return 401.

**Validation:** `pnpm test src/middleware/auth.test.ts`
```

## Products Module

```markdown
---
format: aps
module: products
files:
  - src/products/**
---

# Products Module

Product catalog management.

## Task: PROD-001 — List products

**Status:** complete

**Intent:** GET /products returns paginated product list with filtering.

**Validation:** `pnpm test src/products/list.test.ts`

**Steps:**

1. [x] Pagination (limit, offset)
2. [x] Category filtering
3. [x] Price range filtering
4. [x] Search by name

---

## Task: PROD-002 — Product details

**Status:** complete

**Intent:** GET /products/:id returns full product details.

**Validation:** `pnpm test src/products/details.test.ts`

---

## Task: PROD-003 — Admin product management

**Status:** pending

**Intent:** Admins can create, update, and delete products.

**Validation:** `pnpm test src/products/admin.test.ts`

**Dependencies:**

- AUTH-003 (auth middleware for admin check)
```

## Cart Module

```markdown
---
format: aps
module: cart
depends_on:
  - auth
  - products
files:
  - src/cart/**
---

# Cart Module

Shopping cart functionality.

## Task: CART-001 — Add to cart

**Status:** in_progress

**Intent:** Authenticated users can add products to their cart. Cart persists
across sessions.

**Validation:** `pnpm test src/cart/add.test.ts`

**Dependencies:**

- AUTH-002 (user must be logged in)
- PROD-002 (product must exist)

**Steps:**

1. [x] POST /cart/items endpoint
2. [x] Quantity validation
3. [ ] Stock check
4. [ ] Cart persistence

---

## Task: CART-002 — View cart

**Status:** pending

**Intent:** Users see their cart with items, quantities, and totals.

**Validation:** `pnpm test src/cart/view.test.ts`

**Dependencies:**

- CART-001

---

## Task: CART-003 — Update cart

**Status:** pending

**Intent:** Users can update quantities or remove items.

**Validation:** `pnpm test src/cart/update.test.ts`

**Dependencies:**

- CART-001
```

## Payments Module

```markdown
---
format: aps
module: payments
depends_on:
  - auth
  - cart
files:
  - src/payments/**
---

# Payments Module

Payment processing and order creation.

## Task: PAY-001 — Stripe integration

**Status:** pending

**Intent:** System can process payments via Stripe. Supports card payments. PCI
compliant.

**Validation:** `pnpm test src/payments/stripe.test.ts`

**Dependencies:**

- AUTH-003 (user context)
- CART-002 (cart contents)

**Steps:** See [PAY-001.steps.md](../execution/PAY-001.steps.md)

---

## Task: PAY-002 — Checkout flow

**Status:** pending

**Intent:** Users complete checkout: review → payment → confirmation. Cart
cleared on success. Order created.

**Validation:** `pnpm test src/payments/checkout.test.ts`

**Dependencies:**

- PAY-001
- CART-002
```

## Detailed Steps File

`plans/execution/PAY-001.steps.md`:

```markdown
# PAY-001 — Stripe Integration Steps

## Phase 1: Setup

1. [ ] Stripe SDK installed
2. [ ] API keys configured (env vars)
3. [ ] Webhook endpoint created

## Phase 2: Payment Intent

4. [ ] Create payment intent endpoint
5. [ ] Amount calculated from cart
6. [ ] Currency handling (GBP, USD)

## Phase 3: Payment Confirmation

7. [ ] Handle successful payment
8. [ ] Handle failed payment
9. [ ] Handle payment requiring action (3DS)

## Phase 4: Webhooks

10. [ ] Webhook signature verification
11. [ ] Handle payment_intent.succeeded
12. [ ] Handle payment_intent.failed
13. [ ] Idempotency checks

## Phase 5: Testing

14. [ ] Unit tests for payment logic
15. [ ] Integration tests with Stripe test mode
16. [ ] E2E test of full checkout
```

## Dependency Graph

```
AUTH-001 ──▶ AUTH-002 ──▶ AUTH-003 ──┬──▶ PROD-003
                                      │
PROD-001                              │
                                      │
PROD-002 ─────────────────────────────┼──▶ CART-001 ──▶ CART-002 ──┬──▶ CART-003
                                      │                             │
                                      └─────────────────────────────┼──▶ PAY-001 ──▶ PAY-002
                                                                    │
                                                                    └───────────────────────
```

---

**Next:** [Validation tooling →](/docs/aps/tooling/validation)
