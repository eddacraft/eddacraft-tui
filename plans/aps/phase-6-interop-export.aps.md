# Phase 6: Interop & Export - APS Document

**Phase:** 6 of 7
**Duration:** 2 weeks (10 working days)
**Dependencies:** All previous phases (0-5)
**Status:** Not Started
**Owner:** TBD

---

## Phase Overview

### Purpose
Implement REST API, export/import capabilities, and external integration points to make Edda accessible to external tools, CI/CD pipelines, and other systems.

### Scope
This phase delivers programmatic access to Edda through a comprehensive REST API, data portability through export/import, and integration hooks for external systems.

**Note:** Based on OPEN-QUESTIONS.md recommendation, this phase may be deferred to v1.1 (CLI-only for v1.0). If deferred, v1.0 ships in Week 14, v1.1 (with API) ships in Week 17.

### Success Criteria
- ✅ REST API operational with full CRUD operations
- ✅ OpenAPI 3.0 specification published
- ✅ Export to multiple formats (JSON, YAML, Markdown)
- ✅ Import from JSON/YAML with validation
- ✅ Webhook support for event notifications
- ✅ API authentication via JWT tokens
- ✅ <100ms API response time (95th percentile)
- ✅ 100% API test coverage

---

## Epic Breakdown

### Epic 1: REST API Foundation
**Duration:** 3 days
**Priority:** P0 (Blocking)

#### Epic 1.1: HTTP Server & Routing
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Set up Express.js HTTP server with routing and middleware.

**Acceptance Criteria:**
- Express server with TypeScript
- Request logging middleware
- Error handling middleware
- CORS configuration
- Health check endpoint
- API versioning (/api/v1/...)

**Implementation:**

```typescript
// packages/edda-api/src/server.ts

import express, { Express, Request, Response, NextFunction } from 'express'
import cors from 'cors'
import helmet from 'helmet'
import morgan from 'morgan'

export class EddaAPIServer {
  private app: Express
  private port: number

  constructor(
    private config: APIConfig,
    private eddaCore: IEddaCoreServices,
  ) {
    this.app = express()
    this.port = config.port || 3000
    this.setupMiddleware()
    this.setupRoutes()
    this.setupErrorHandling()
  }

  private setupMiddleware(): void {
    // Security
    this.app.use(helmet())

    // CORS
    this.app.use(cors({
      origin: this.config.cors_origins || '*',
      credentials: true,
    }))

    // Body parsing
    this.app.use(express.json({ limit: '10mb' }))
    this.app.use(express.urlencoded({ extended: true }))

    // Logging
    this.app.use(morgan('combined'))

    // Request ID
    this.app.use((req, res, next) => {
      req.id = ulid()
      res.setHeader('X-Request-ID', req.id)
      next()
    })
  }

  private setupRoutes(): void {
    // Health check
    this.app.get('/health', (req, res) => {
      res.json({
        status: 'ok',
        version: '1.0.0',
        timestamp: new Date().toISOString(),
      })
    })

    // API version 1
    const v1Router = express.Router()

    // Memory routes
    v1Router.use('/memories', this.createMemoryRoutes())

    // Query routes
    v1Router.use('/query', this.createQueryRoutes())

    // Promotion routes
    v1Router.use('/promotions', this.createPromotionRoutes())

    // Enforcement routes
    v1Router.use('/enforcement', this.createEnforcementRoutes())

    // Lifecycle routes
    v1Router.use('/lifecycle', this.createLifecycleRoutes())

    // Export routes
    v1Router.use('/export', this.createExportRoutes())

    // Mount v1 router
    this.app.use('/api/v1', v1Router)

    // 404 handler
    this.app.use((req, res) => {
      res.status(404).json({
        error: 'Not Found',
        message: `Route ${req.method} ${req.path} not found`,
      })
    })
  }

  private setupErrorHandling(): void {
    this.app.use((error: Error, req: Request, res: Response, next: NextFunction) => {
      console.error('API Error:', error)

      const status = this.getErrorStatus(error)
      const message = error.message || 'Internal Server Error'

      res.status(status).json({
        error: error.name,
        message,
        request_id: req.id,
      })
    })
  }

  private getErrorStatus(error: Error): number {
    if (error instanceof UnauthorizedError) return 401
    if (error instanceof ForbiddenError) return 403
    if (error instanceof NotFoundError) return 404
    if (error instanceof ValidationError) return 400
    return 500
  }

  async start(): Promise<void> {
    return new Promise((resolve) => {
      this.app.listen(this.port, () => {
        console.log(`Edda API server listening on port ${this.port}`)
        resolve()
      })
    })
  }
}

export interface APIConfig {
  port?: number
  cors_origins?: string[]
  auth_enabled?: boolean
  rate_limit?: {
    window_ms: number
    max_requests: number
  }
}
```

**File Structure:**
```
packages/edda-api/
├── src/
│   ├── server.ts
│   ├── middleware/
│   │   ├── auth.ts
│   │   ├── validation.ts
│   │   └── rate-limit.ts
│   ├── routes/
│   │   ├── memories.ts
│   │   ├── query.ts
│   │   ├── promotions.ts
│   │   ├── enforcement.ts
│   │   ├── lifecycle.ts
│   │   └── export.ts
│   ├── controllers/
│   │   ├── memory-controller.ts
│   │   ├── query-controller.ts
│   │   └── ...
│   └── __tests__/
│       ├── server.test.ts
│       └── integration/
│           └── api.integration.test.ts
├── package.json
└── tsconfig.json
```

**Tests:**
- Server starts successfully
- Health check returns 200
- 404 for unknown routes
- Error handling returns correct status codes
- CORS headers present
- Request ID in response headers

---

#### Epic 1.2: Authentication Middleware
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement JWT-based authentication for API requests.

**Acceptance Criteria:**
- JWT token validation
- Extract principal from token
- Auth middleware protects routes
- Public routes (health check) don't require auth
- 401 for missing/invalid token

**Implementation:**

```typescript
// packages/edda-api/src/middleware/auth.ts

import jwt from 'jsonwebtoken'
import { Request, Response, NextFunction } from 'express'

export interface JWTPayload {
  sub: string           // Principal identifier
  principal_type: string
  roles: string[]
  iat: number
  exp: number
}

export class AuthMiddleware {
  constructor(
    private jwtSecret: string,
    private principalRepo: IPrincipalRepository,
  ) {}

  authenticate = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      // Extract token from Authorization header
      const authHeader = req.headers.authorization
      if (!authHeader) {
        throw new UnauthorizedError('Missing Authorization header')
      }

      const [scheme, token] = authHeader.split(' ')
      if (scheme !== 'Bearer') {
        throw new UnauthorizedError('Invalid authentication scheme')
      }

      // Verify JWT
      const payload = jwt.verify(token, this.jwtSecret) as JWTPayload

      // Load principal
      const principal = await this.principalRepo.get(payload.sub)

      // Attach to request
      req.principal = principal

      next()
    } catch (error) {
      if (error instanceof jwt.JsonWebTokenError) {
        next(new UnauthorizedError('Invalid token'))
      } else {
        next(error)
      }
    }
  }

  /**
   * Middleware to require specific permission
   */
  requirePermission = (permission: Permission) => {
    return async (req: Request, res: Response, next: NextFunction): Promise<void> => {
      if (!req.principal) {
        return next(new UnauthorizedError('Authentication required'))
      }

      const hasPermission = PermissionChecker.principalHasPermission(
        req.principal,
        permission,
      )

      if (!hasPermission) {
        return next(new ForbiddenError(`Missing permission: ${permission}`))
      }

      next()
    }
  }
}

// Extend Express Request type
declare global {
  namespace Express {
    interface Request {
      id?: string
      principal?: Principal
    }
  }
}
```

**Tests:**
- Valid token authenticates successfully
- Invalid token returns 401
- Missing token returns 401
- Expired token returns 401
- Principal loaded and attached to request
- Permission check works

---

#### Epic 1.3: Memory CRUD Endpoints
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
Implement REST endpoints for memory CRUD operations.

**Acceptance Criteria:**
- GET /api/v1/memories - List memories
- GET /api/v1/memories/:id - Get memory by ID
- POST /api/v1/memories - Create memory
- PUT /api/v1/memories/:id - Update memory
- DELETE /api/v1/memories/:id - Delete memory
- Pagination, filtering, sorting support
- Authorization enforced
- Input validation

**Implementation:**

```typescript
// packages/edda-api/src/routes/memories.ts

import { Router } from 'express'
import { MemoryController } from '../controllers/memory-controller'
import { AuthMiddleware } from '../middleware/auth'
import { ValidationMiddleware } from '../middleware/validation'

export function createMemoryRoutes(
  auth: AuthMiddleware,
  controller: MemoryController,
): Router {
  const router = Router()

  // List memories
  router.get(
    '/',
    auth.authenticate,
    auth.requirePermission(Permission.MEMORY_READ),
    controller.list,
  )

  // Get memory by ID
  router.get(
    '/:id',
    auth.authenticate,
    auth.requirePermission(Permission.MEMORY_READ),
    controller.get,
  )

  // Create memory
  router.post(
    '/',
    auth.authenticate,
    auth.requirePermission(Permission.MEMORY_CREATE),
    ValidationMiddleware.validateBody(CreateMemorySchema),
    controller.create,
  )

  // Update memory
  router.put(
    '/:id',
    auth.authenticate,
    auth.requirePermission(Permission.MEMORY_UPDATE),
    ValidationMiddleware.validateBody(UpdateMemorySchema),
    controller.update,
  )

  // Delete memory
  router.delete(
    '/:id',
    auth.authenticate,
    auth.requirePermission(Permission.MEMORY_DELETE),
    controller.delete,
  )

  return router
}

// packages/edda-api/src/controllers/memory-controller.ts

export class MemoryController {
  constructor(
    private memoryManager: IMemoryManager,
    private queryService: IQueryService,
  ) {}

  list = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const query: EddaQuery = {
        filters: {
          type: req.query.type as any,
          status: req.query.status as any,
          tags: req.query.tags ? (req.query.tags as string).split(',') : undefined,
        },
        sort: {
          field: (req.query.sort_by as any) || 'created_at',
          direction: (req.query.sort_dir as any) || 'desc',
        },
        pagination: {
          limit: parseInt(req.query.limit as string) || 50,
          offset: parseInt(req.query.offset as string) || 0,
        },
      }

      const result = await this.queryService.query(query)

      res.json({
        memories: result.memories,
        total: result.total_count,
        page_info: result.page_info,
      })
    } catch (error) {
      next(error)
    }
  }

  get = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { id } = req.params

      const memory = await this.memoryManager.get(id)
      if (!memory) {
        throw new NotFoundError(`Memory ${id} not found`)
      }

      res.json(memory)
    } catch (error) {
      next(error)
    }
  }

  create = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const data = req.body as CreateMemoryData

      const memory = await this.memoryManager.create(req.principal!, data)

      res.status(201).json(memory)
    } catch (error) {
      next(error)
    }
  }

  update = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { id } = req.params
      const updates = req.body as UpdateMemoryData

      const memory = await this.memoryManager.update(req.principal!, id, updates)

      res.json(memory)
    } catch (error) {
      next(error)
    }
  }

  delete = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { id } = req.params

      await this.memoryManager.delete(req.principal!, id)

      res.status(204).send()
    } catch (error) {
      next(error)
    }
  }
}
```

**Tests:**
- GET /memories returns list
- GET /memories/:id returns memory
- POST /memories creates memory (returns 201)
- PUT /memories/:id updates memory
- DELETE /memories/:id deletes memory (returns 204)
- Pagination works (limit, offset)
- Filtering works (type, status, tags)
- Sorting works (field, direction)
- Authorization enforced (401 without token)
- Permission checks work (403 without permission)
- Validation errors return 400

---

### Epic 2: Query & Search APIs
**Duration:** 2 days
**Priority:** P0 (Blocking)

#### Epic 2.1: Query Endpoints
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement query and search API endpoints.

**Acceptance Criteria:**
- POST /api/v1/query - Structured query
- GET /api/v1/search?q=... - Full-text search
- GET /api/v1/memories/:id/provenance - Provenance chain
- GET /api/v1/memories/:id/related - Related memories
- Returns query metadata (execution time)

**Implementation:**

```typescript
// packages/edda-api/src/routes/query.ts

export function createQueryRoutes(
  auth: AuthMiddleware,
  controller: QueryController,
): Router {
  const router = Router()

  // Structured query
  router.post(
    '/',
    auth.authenticate,
    auth.requirePermission(Permission.MEMORY_READ),
    ValidationMiddleware.validateBody(EddaQuerySchema),
    controller.query,
  )

  // Full-text search
  router.get(
    '/search',
    auth.authenticate,
    auth.requirePermission(Permission.MEMORY_READ),
    controller.search,
  )

  return router
}

// packages/edda-api/src/controllers/query-controller.ts

export class QueryController {
  constructor(
    private queryService: IQueryService,
    private provenanceService: IProvenanceService,
    private relationshipService: IRelationshipService,
  ) {}

  query = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const query = req.body as EddaQuery

      const result = await this.queryService.query(query)

      res.json(result)
    } catch (error) {
      next(error)
    }
  }

  search = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { q, limit } = req.query

      if (!q) {
        throw new ValidationError('Query parameter "q" is required')
      }

      const memories = await this.queryService.search(
        q as string,
        parseInt(limit as string) || 50,
      )

      res.json({
        memories,
        total: memories.length,
      })
    } catch (error) {
      next(error)
    }
  }
}

// Add provenance route to memory routes
router.get(
  '/:id/provenance',
  auth.authenticate,
  auth.requirePermission(Permission.MEMORY_READ),
  async (req, res, next) => {
    try {
      const { id } = req.params
      const provenance = await provenanceService.traceProvenance(id)
      res.json(provenance)
    } catch (error) {
      next(error)
    }
  },
)

// Add related route to memory routes
router.get(
  '/:id/related',
  auth.authenticate,
  auth.requirePermission(Permission.MEMORY_READ),
  async (req, res, next) => {
    try {
      const { id } = req.params
      const related = await relationshipService.findRelated(id)
      res.json({ memories: related })
    } catch (error) {
      next(error)
    }
  },
)
```

**Tests:**
- POST /query with filters works
- GET /search?q=... returns results
- GET /memories/:id/provenance returns chain
- GET /memories/:id/related returns related memories
- Query metadata included (execution time)

---

### Epic 3: Promotion & Enforcement APIs
**Duration:** 2 days
**Priority:** P1 (Important)

#### Epic 3.1: Promotion Endpoints
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement promotion workflow API endpoints.

**Acceptance Criteria:**
- GET /api/v1/promotions - List pending promotions
- POST /api/v1/promotions - Create promotion request
- POST /api/v1/promotions/:id/approve - Approve promotion
- POST /api/v1/promotions/:id/reject - Reject promotion
- Returns diff, conflicts, trust adjustments

**Implementation:**

```typescript
// packages/edda-api/src/routes/promotions.ts

export function createPromotionRoutes(
  auth: AuthMiddleware,
  controller: PromotionController,
): Router {
  const router = Router()

  // List promotions
  router.get(
    '/',
    auth.authenticate,
    auth.requirePermission(Permission.PROPOSAL_APPROVE),
    controller.list,
  )

  // Create promotion request
  router.post(
    '/',
    auth.authenticate,
    auth.requirePermission(Permission.PROPOSAL_SUBMIT),
    controller.create,
  )

  // Approve promotion
  router.post(
    '/:id/approve',
    auth.authenticate,
    auth.requirePermission(Permission.PROPOSAL_APPROVE),
    controller.approve,
  )

  // Reject promotion
  router.post(
    '/:id/reject',
    auth.authenticate,
    auth.requirePermission(Permission.PROPOSAL_REJECT),
    controller.reject,
  )

  return router
}

// packages/edda-api/src/controllers/promotion-controller.ts

export class PromotionController {
  constructor(
    private promotionService: IPromotionService,
  ) {}

  list = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const status = req.query.status as PromotionStatus || 'pending'

      const promotions = await this.promotionService.listRequests({ status })

      res.json({ promotions })
    } catch (error) {
      next(error)
    }
  }

  create = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { proposal_id, agent_id } = req.body

      // Fetch proposal from Ember
      const proposal = await this.emberPort.getProposal(proposal_id)

      const request = await this.promotionService.createPromotionRequest(
        proposal,
        agent_id,
      )

      res.status(201).json(request)
    } catch (error) {
      next(error)
    }
  }

  approve = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { id } = req.params

      const memory = await this.promotionService.approvePromotion(
        id,
        req.principal!,
      )

      res.json(memory)
    } catch (error) {
      next(error)
    }
  }

  reject = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const { id } = req.params
      const { reason } = req.body

      await this.promotionService.rejectPromotion(
        id,
        req.principal!,
        reason,
      )

      res.status(204).send()
    } catch (error) {
      next(error)
    }
  }
}
```

**Tests:**
- GET /promotions lists pending requests
- POST /promotions creates request
- POST /promotions/:id/approve approves and creates memory
- POST /promotions/:id/reject rejects with reason
- Authorization enforced

---

#### Epic 3.2: Enforcement Endpoints
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement enforcement check API endpoint.

**Acceptance Criteria:**
- POST /api/v1/enforcement/check - Check action
- Returns violations, warnings, suggestions
- Performance: <100ms

**Implementation:**

```typescript
// packages/edda-api/src/routes/enforcement.ts

export function createEnforcementRoutes(
  auth: AuthMiddleware,
  controller: EnforcementController,
): Router {
  const router = Router()

  // Check action
  router.post(
    '/check',
    auth.authenticate,
    ValidationMiddleware.validateBody(ActionContextSchema),
    controller.checkAction,
  )

  return router
}

// packages/edda-api/src/controllers/enforcement-controller.ts

export class EnforcementController {
  constructor(
    private enforcementService: IEnforcementService,
  ) {}

  checkAction = async (req: Request, res: Response, next: NextFunction): Promise<void> => {
    try {
      const action = req.body as ActionContext

      const result = await this.enforcementService.checkAction(
        action,
        req.principal!,
      )

      res.json(result)
    } catch (error) {
      next(error)
    }
  }
}
```

**Tests:**
- POST /enforcement/check returns result
- Blocking violations returned
- Warnings and suggestions included
- Performance: <100ms

---

### Epic 4: Export & Import
**Duration:** 2 days
**Priority:** P1 (Important)

#### Epic 4.1: Export Service
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement export service supporting multiple formats.

**Acceptance Criteria:**
- Export to JSON (machine-readable)
- Export to YAML (human-readable)
- Export to Markdown (documentation)
- Filter by type, status, scope, tags
- Include/exclude relationships, provenance
- Streaming export for large datasets

**Implementation:**

```typescript
// packages/edda-core/src/export/export-service.ts

export enum ExportFormat {
  JSON = 'json',
  YAML = 'yaml',
  MARKDOWN = 'markdown',
}

export interface ExportOptions {
  format: ExportFormat
  filters?: EddaQuery['filters']
  include_provenance?: boolean
  include_relationships?: boolean
  stream?: boolean
}

export interface IExportService {
  /**
   * Export memories to specified format
   */
  export(options: ExportOptions): Promise<string | ReadableStream>

  /**
   * Export single memory
   */
  exportMemory(memoryId: MemoryId, format: ExportFormat): Promise<string>
}

export class ExportService implements IExportService {
  constructor(
    private queryService: IQueryService,
    private provenanceService: IProvenanceService,
    private relationshipService: IRelationshipService,
  ) {}

  async export(options: ExportOptions): Promise<string | ReadableStream> {
    // Query memories
    const result = await this.queryService.query({
      filters: options.filters,
      pagination: {
        limit: 10000, // Export all
        offset: 0,
      },
    })

    // Optionally include extra data
    const enriched = await Promise.all(
      result.memories.map(async memory => {
        const data: any = { ...memory }

        if (options.include_provenance) {
          data.provenance_chain = await this.provenanceService.traceProvenance(memory.id)
        }

        if (options.include_relationships) {
          data.related_memories = await this.relationshipService.findRelated(memory.id)
        }

        return data
      })
    )

    // Format
    return this.formatExport(enriched, options.format)
  }

  async exportMemory(memoryId: MemoryId, format: ExportFormat): Promise<string> {
    const memory = await this.storage.fetch(memoryId)
    if (!memory) {
      throw new MemoryNotFoundError(memoryId)
    }

    return this.formatMemory(memory, format)
  }

  private formatExport(memories: any[], format: ExportFormat): string {
    switch (format) {
      case ExportFormat.JSON:
        return JSON.stringify({
          version: '1.0.0',
          exported_at: new Date().toISOString(),
          count: memories.length,
          memories,
        }, null, 2)

      case ExportFormat.YAML:
        return yaml.stringify({
          version: '1.0.0',
          exported_at: new Date().toISOString(),
          count: memories.length,
          memories,
        })

      case ExportFormat.MARKDOWN:
        return this.formatMarkdown(memories)

      default:
        throw new Error(`Unsupported format: ${format}`)
    }
  }

  private formatMarkdown(memories: MemoryObject[]): string {
    const lines: string[] = [
      '# Edda Memories Export',
      '',
      `**Exported:** ${new Date().toISOString()}`,
      `**Count:** ${memories.length}`,
      '',
      '---',
      '',
    ]

    for (const memory of memories) {
      lines.push(`## ${memory.id}`)
      lines.push('')
      lines.push(`**Type:** ${memory.type}`)
      lines.push(`**Status:** ${memory.status}`)
      lines.push(`**Confidence:** ${memory.confidence}`)
      lines.push(`**Created:** ${memory.authority.created_at}`)
      lines.push('')
      lines.push(`### Statement`)
      lines.push('')
      lines.push(memory.statement)
      lines.push('')

      if (memory.context.reasoning) {
        lines.push(`### Reasoning`)
        lines.push('')
        lines.push(memory.context.reasoning)
        lines.push('')
      }

      if (memory.tags.length > 0) {
        lines.push(`**Tags:** ${memory.tags.join(', ')}`)
        lines.push('')
      }

      lines.push('---')
      lines.push('')
    }

    return lines.join('\n')
  }

  private formatMemory(memory: MemoryObject, format: ExportFormat): string {
    switch (format) {
      case ExportFormat.JSON:
        return JSON.stringify(memory, null, 2)

      case ExportFormat.YAML:
        return yaml.stringify(memory)

      case ExportFormat.MARKDOWN:
        return this.formatMarkdown([memory])

      default:
        throw new Error(`Unsupported format: ${format}`)
    }
  }
}
```

**File Structure:**
```
packages/edda-core/src/export/
├── export-service.ts
├── import-service.ts
└── __tests__/
    ├── export-service.test.ts
    └── import-service.test.ts
```

**Tests:**
- Export to JSON format
- Export to YAML format
- Export to Markdown format
- Export with filters (type, status)
- Export includes provenance (when requested)
- Export includes relationships (when requested)

---

#### Epic 4.2: Import Service
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Implement import service with validation.

**Acceptance Criteria:**
- Import from JSON
- Import from YAML
- Schema validation before import
- Dry-run mode (validate without importing)
- Conflict detection (duplicate IDs)
- Bulk import

**Implementation:**

```typescript
// packages/edda-core/src/export/import-service.ts

export interface ImportOptions {
  format: ExportFormat
  dry_run?: boolean
  overwrite_existing?: boolean
  skip_validation?: boolean
}

export interface ImportResult {
  success: boolean
  imported_count: number
  skipped_count: number
  errors: Array<{
    memory_id?: string
    error: string
  }>
}

export interface IImportService {
  /**
   * Import memories from data
   */
  import(data: string, options: ImportOptions): Promise<ImportResult>

  /**
   * Validate import data without importing
   */
  validate(data: string, format: ExportFormat): Promise<ValidationResult>
}

export class ImportService implements IImportService {
  constructor(
    private memoryManager: IMemoryManager,
    private storage: IMemoryStorage,
  ) {}

  async import(data: string, options: ImportOptions): Promise<ImportResult> {
    // Parse data
    const parsed = this.parseData(data, options.format)

    // Validate
    if (!options.skip_validation) {
      const validation = await this.validateParsed(parsed)
      if (!validation.valid) {
        throw new ValidationError(`Import data invalid: ${validation.errors.join(', ')}`)
      }
    }

    const result: ImportResult = {
      success: true,
      imported_count: 0,
      skipped_count: 0,
      errors: [],
    }

    // Dry run?
    if (options.dry_run) {
      console.log(`[DRY RUN] Would import ${parsed.memories.length} memories`)
      return result
    }

    // Import memories
    for (const memoryData of parsed.memories) {
      try {
        // Check if exists
        const existing = await this.storage.fetch(memoryData.id)

        if (existing && !options.overwrite_existing) {
          result.skipped_count++
          continue
        }

        // Import (create or update)
        if (existing) {
          await this.storage.store(memoryData)
        } else {
          await this.storage.store(memoryData)
        }

        result.imported_count++
      } catch (error) {
        result.errors.push({
          memory_id: memoryData.id,
          error: error.message,
        })
      }
    }

    return result
  }

  async validate(data: string, format: ExportFormat): Promise<ValidationResult> {
    try {
      const parsed = this.parseData(data, format)
      return await this.validateParsed(parsed)
    } catch (error) {
      return {
        valid: false,
        errors: [error.message],
      }
    }
  }

  private parseData(data: string, format: ExportFormat): any {
    switch (format) {
      case ExportFormat.JSON:
        return JSON.parse(data)

      case ExportFormat.YAML:
        return yaml.parse(data)

      default:
        throw new Error(`Unsupported format for import: ${format}`)
    }
  }

  private async validateParsed(parsed: any): Promise<ValidationResult> {
    const errors: string[] = []

    if (!parsed.memories || !Array.isArray(parsed.memories)) {
      errors.push('Missing or invalid "memories" array')
    }

    for (const memory of parsed.memories || []) {
      try {
        MemoryObjectSchema.parse(memory)
      } catch (error) {
        errors.push(`Memory ${memory.id}: ${error.message}`)
      }
    }

    return {
      valid: errors.length === 0,
      errors,
    }
  }
}

export interface ValidationResult {
  valid: boolean
  errors: string[]
}
```

**Tests:**
- Import from JSON
- Import from YAML
- Dry run doesn't import
- Validation detects invalid data
- Conflict detection (existing IDs)
- Overwrite existing (when flag set)
- Bulk import performance

---

### Epic 5: OpenAPI Specification & Documentation
**Duration:** 1 day
**Priority:** P1 (Important)

#### Epic 5.1: OpenAPI Spec Generation
**Estimate:** 4 hours
**Owner:** TBD

**Description:**
Generate OpenAPI 3.0 specification for the REST API.

**Acceptance Criteria:**
- OpenAPI 3.0 spec covers all endpoints
- Schema definitions for all types
- Authentication documented (JWT Bearer)
- Examples included
- Served at /api/docs
- Swagger UI available

**Implementation:**

```typescript
// packages/edda-api/src/openapi/spec.ts

export const openAPISpec = {
  openapi: '3.0.0',
  info: {
    title: 'Edda API',
    version: '1.0.0',
    description: 'Curated memory layer for institutional knowledge',
    contact: {
      name: 'EddaCraft',
      url: 'https://github.com/EddaCraft',
    },
  },
  servers: [
    {
      url: 'http://localhost:3000/api/v1',
      description: 'Development server',
    },
  ],
  security: [
    {
      bearerAuth: [],
    },
  ],
  paths: {
    '/memories': {
      get: {
        summary: 'List memories',
        tags: ['Memories'],
        parameters: [
          {
            name: 'type',
            in: 'query',
            schema: { type: 'string', enum: ['decision', 'pattern', 'warning', 'constraint', 'doctrine', 'lesson'] },
          },
          {
            name: 'status',
            in: 'query',
            schema: { type: 'string', enum: ['active', 'deprecated', 'superseded'] },
          },
          {
            name: 'limit',
            in: 'query',
            schema: { type: 'integer', default: 50 },
          },
          {
            name: 'offset',
            in: 'query',
            schema: { type: 'integer', default: 0 },
          },
        ],
        responses: {
          '200': {
            description: 'List of memories',
            content: {
              'application/json': {
                schema: {
                  type: 'object',
                  properties: {
                    memories: {
                      type: 'array',
                      items: { $ref: '#/components/schemas/MemoryObject' },
                    },
                    total: { type: 'integer' },
                    page_info: { $ref: '#/components/schemas/PageInfo' },
                  },
                },
              },
            },
          },
        },
      },
      post: {
        summary: 'Create memory',
        tags: ['Memories'],
        requestBody: {
          required: true,
          content: {
            'application/json': {
              schema: { $ref: '#/components/schemas/CreateMemoryData' },
            },
          },
        },
        responses: {
          '201': {
            description: 'Memory created',
            content: {
              'application/json': {
                schema: { $ref: '#/components/schemas/MemoryObject' },
              },
            },
          },
        },
      },
    },
    // ... more paths
  },
  components: {
    securitySchemes: {
      bearerAuth: {
        type: 'http',
        scheme: 'bearer',
        bearerFormat: 'JWT',
      },
    },
    schemas: {
      MemoryObject: {
        type: 'object',
        properties: {
          id: { type: 'string' },
          type: { type: 'string', enum: ['decision', 'pattern', 'warning', 'constraint', 'doctrine', 'lesson'] },
          status: { type: 'string', enum: ['active', 'deprecated', 'superseded'] },
          statement: { type: 'string' },
          confidence: { type: 'string', enum: ['high', 'medium', 'low'] },
          // ... more fields
        },
      },
      // ... more schemas
    },
  },
}

// Serve spec and Swagger UI
app.get('/api/docs/spec.json', (req, res) => {
  res.json(openAPISpec)
})

app.use('/api/docs', swaggerUi.serve, swaggerUi.setup(openAPISpec))
```

**Tests:**
- OpenAPI spec is valid
- All endpoints documented
- Swagger UI loads
- Authentication documented

---

### Epic 6: Integration & Testing
**Duration:** 2 days (end of phase)
**Priority:** P0 (Blocking)

#### Epic 6.1: API Integration Tests
**Estimate:** 6 hours
**Owner:** TBD

**Description:**
End-to-end API integration tests.

**Test Scenarios:**

```typescript
describe('API Integration', () => {
  let apiClient: APIClient
  let authToken: string

  beforeAll(async () => {
    // Start API server
    await server.start()

    // Get auth token
    authToken = await getAuthToken('test-user')
    apiClient = new APIClient('http://localhost:3000', authToken)
  })

  it('should perform full CRUD workflow via API', async () => {
    // Create memory
    const created = await apiClient.post('/api/v1/memories', {
      type: 'decision',
      statement: 'Use TypeScript for all new projects',
      tags: ['typescript', 'standards'],
    })

    expect(created.id).toBeDefined()

    // Get memory
    const fetched = await apiClient.get(`/api/v1/memories/${created.id}`)
    expect(fetched.statement).toBe('Use TypeScript for all new projects')

    // Update memory
    const updated = await apiClient.put(`/api/v1/memories/${created.id}`, {
      statement: 'Use TypeScript for all new projects (updated)',
    })
    expect(updated.statement).toContain('updated')

    // List memories
    const list = await apiClient.get('/api/v1/memories?type=decision')
    expect(list.memories.length).toBeGreaterThan(0)

    // Delete memory
    await apiClient.delete(`/api/v1/memories/${created.id}`)

    // Verify deleted
    await expect(
      apiClient.get(`/api/v1/memories/${created.id}`)
    ).rejects.toThrow('404')
  })

  it('should perform query and search', async () => {
    // Full-text search
    const searchResults = await apiClient.get('/api/v1/search?q=typescript')
    expect(searchResults.memories.length).toBeGreaterThan(0)

    // Structured query
    const queryResults = await apiClient.post('/api/v1/query', {
      filters: {
        type: ['decision'],
        status: ['active'],
      },
    })
    expect(queryResults.memories.length).toBeGreaterThan(0)
  })

  it('should export and import memories', async () => {
    // Export
    const exported = await apiClient.get('/api/v1/export?format=json')
    expect(exported.memories).toBeDefined()

    // Import
    const importResult = await apiClient.post('/api/v1/import', {
      data: JSON.stringify(exported),
      format: 'json',
      dry_run: false,
    })
    expect(importResult.imported_count).toBeGreaterThan(0)
  })

  it('should enforce authentication', async () => {
    const unauthClient = new APIClient('http://localhost:3000')

    await expect(
      unauthClient.get('/api/v1/memories')
    ).rejects.toThrow('401')
  })

  it('should enforce authorization', async () => {
    const readonlyToken = await getAuthToken('readonly-user')
    const readonlyClient = new APIClient('http://localhost:3000', readonlyToken)

    await expect(
      readonlyClient.post('/api/v1/memories', {})
    ).rejects.toThrow('403')
  })
})
```

**Tests:**
- Full CRUD workflow via API
- Query and search work
- Export and import work
- Authentication enforced (401 without token)
- Authorization enforced (403 without permission)
- Performance: <100ms response time (95th percentile)
- 100% API test coverage

---

## Timeline

### Week 1 (Days 1-5)
- **Day 1-3:** Epic 1 (REST API Foundation)
- **Day 4-5:** Epic 2 (Query & Search APIs)

### Week 2 (Days 6-10)
- **Day 6-7:** Epic 3 (Promotion & Enforcement APIs)
- **Day 8-9:** Epic 4 (Export & Import)
- **Day 10:** Epic 5 (OpenAPI Spec) + Epic 6 (Integration & Testing)

---

## Deliverables

### Package Structure
```
packages/edda-api/
├── src/
│   ├── server.ts
│   ├── middleware/
│   ├── routes/
│   ├── controllers/
│   ├── openapi/
│   └── __tests__/
├── package.json
└── tsconfig.json

packages/edda-core/src/export/
├── export-service.ts
└── import-service.ts
```

### API Documentation
- OpenAPI 3.0 specification
- Swagger UI at /api/docs
- Authentication guide
- API usage examples

### Tests
- Unit tests: 40+ tests
- Integration tests: 20+ scenarios
- Performance tests: <100ms response time
- Test coverage: 100%

---

## Success Metrics

### Functional
- ✅ All CRUD operations work via API
- ✅ Query and search operational
- ✅ Export/import work correctly
- ✅ Authentication and authorization enforced

### Performance
- ✅ API response time: <100ms (95th percentile)
- ✅ Export: <1s for 1000 memories
- ✅ Import: <2s for 1000 memories

### Quality
- ✅ 100% test coverage
- ✅ OpenAPI spec complete
- ✅ Clear error messages
- ✅ Security best practices followed

---

## Risks & Mitigation

### Risk 1: API Performance Under Load
**Probability:** Medium
**Impact:** Medium

**Mitigation:**
- Proper indexing for fast queries
- Response caching where appropriate
- Rate limiting to prevent abuse
- Load testing before release

### Risk 2: Authentication/Authorization Complexity
**Probability:** Low
**Impact:** High

**Mitigation:**
- Use proven JWT library
- Thorough security testing
- Clear permission model
- Audit all auth decisions

---

## Dependencies

### Upstream (Must Complete First)
- All previous phases (0-5)

### Downstream (Blocked By This Phase)
- External integrations (CI/CD, webhooks)

---

## Open Questions

### Q1: REST API Priority (from OPEN-QUESTIONS.md)
**Status:** 🟡 Pending Stakeholder Decision
**Recommended:** CLI-only for v1.0, API in v1.1

**Impact on Phase 6:**
- CLI-only: Skip Phase 6 for v1.0 (14 weeks total)
- API in v1.0: Include Phase 6 (16 weeks total)
- API in v1.1: Phase 6 after v1.0 (v1.1 at Week 17)

**Decision Required By:** Before Phase 6 starts

---

## Next Steps

1. ✅ Complete Phases 0-5
2. **Review this APS document** with team
3. **Decide on API timing** (v1.0 vs v1.1)
4. **Assign owners** to epics and tasks
5. **Kick off Phase 6** implementation (or defer to v1.1)

---

**Document Version:** 1.0
**Last Updated:** 2026-01-19
**Status:** Ready for Review (May be deferred to v1.1)
**Estimated Completion:** 2 weeks after Phase 5 completion (or 3 weeks after v1.0 for v1.1)
