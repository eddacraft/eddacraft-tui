# Shopping Cart Module

**Scope:** CART **Owner:** @carol **Priority:** medium

> Session-based shopping cart with Redis caching.

## Tasks

### CART-001: Design cart data structure

**Intent:** Define cart schema for Redis with items, quantities, and expiry
**Expected Outcome:** Documented cart data structure with TTL strategy
**Confidence:** high **Scopes:** CART, CACHE **Tags:** design, redis, data-model

### CART-002: Implement add to cart endpoint

**Intent:** Create POST /cart/items endpoint that adds products to user cart
**Expected Outcome:** Endpoint validating product exists and updating cart in
Redis **Confidence:** high **Scopes:** CART, API, CACHE **Tags:** api, shopping
**Dependencies:** CART-001, PROD-001, AUTH-003

### CART-003: Implement view cart endpoint

**Intent:** Create GET /cart endpoint returning current cart with product
details **Expected Outcome:** Endpoint enriching cart with product info (name,
price, availability) **Confidence:** high **Scopes:** CART, API, CACHE **Tags:**
api, shopping **Dependencies:** CART-001, CART-002

### CART-004: Implement update cart quantity endpoint

**Intent:** Create PATCH /cart/items/:productId endpoint for quantity changes
**Expected Outcome:** Endpoint handling quantity updates and item removal (qty
= 0) **Confidence:** high **Scopes:** CART, API, CACHE **Tags:** api, shopping
**Dependencies:** CART-002

### CART-005: Add cart total calculation

**Intent:** Calculate cart subtotal, tax, and total in cart response **Expected
Outcome:** Cart endpoint including pricing breakdown **Confidence:** medium
**Scopes:** CART, API **Tags:** pricing, calculations **Dependencies:** CART-003
**Inputs:**

- Tax calculation rules (by region or flat rate?)

## Dependencies

- Product module for validation
- Authentication module for user context

## Notes

- Cart persistence (save for later) deferred to phase 2
- Guest cart migration on login deferred to phase 2
