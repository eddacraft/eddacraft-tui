# E-commerce Platform MVP

## Overview

Build the minimum viable product for an e-commerce platform with authentication,
product catalog, shopping cart, and payment processing.

## Modules

### auth

- **Path:** [./modules/auth.aps.md](./modules/auth.aps.md)
- **Scope:** AUTH
- **Owner:** @alice
- **Priority:** high
- **Tags:** security, core, backend
- **Dependencies:** (none)

### products

- **Path:** [./modules/products.aps.md](./modules/products.aps.md)
- **Scope:** PROD
- **Owner:** @bob
- **Priority:** high
- **Tags:** catalog, backend, api
- **Dependencies:** auth

### cart

- **Path:** [./modules/cart.aps.md](./modules/cart.aps.md)
- **Scope:** CART
- **Owner:** @carol
- **Priority:** medium
- **Tags:** shopping, backend, api
- **Dependencies:** auth, products

### payments

- **Path:** [./modules/payments.aps.md](./modules/payments.aps.md)
- **Scope:** PAY
- **Owner:** @david
- **Priority:** high
- **Tags:** billing, stripe, backend
- **Dependencies:** auth, cart

## Open Questions

- Do we need inventory management in MVP or can we assume unlimited stock?
- Should we support guest checkout or require authentication?
- What payment methods beyond credit card (Stripe)?

## Decisions

- Using Stripe for payment processing (decided 2025-12-15)
- PostgreSQL for primary database (decided 2025-12-10)
- JWT for authentication tokens (decided 2025-12-10)
- Redis for session/cart caching (decided 2025-12-12)
