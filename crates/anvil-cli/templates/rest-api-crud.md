---
id: rest-api-crud
name: REST API CRUD
description: Create a RESTful API with full CRUD operations for a resource
category: api
tags: [rest, api, crud, express, controller]
variables:
  - name: resource_name
    description: Name of the resource (singular, lowercase)
    required: true
  - name: resource_name_plural
    description: Plural form of the resource name
    required: true
  - name: fields
    description: Comma-separated list of fields
    default: name,description,status
    required: false
---

# REST API CRUD: {{ resource_name }}

## Intent

Implement full CRUD (Create, Read, Update, Delete) operations for the
{{ resource_name }} resource with proper validation and error handling.

## Changes

### 1. Create Model

- **File**: `src/models/{{ resource_name }}.model.ts`
- **Action**: Create
- **Description**: Define {{ resource_name }} schema with fields: {{ fields }}

### 2. Create Controller

- **File**: `src/controllers/{{ resource_name }}.controller.ts`
- **Action**: Create
- **Description**: CRUD handlers for {{ resource_name }}

### 3. Create Service

- **File**: `src/services/{{ resource_name }}.service.ts`
- **Action**: Create
- **Description**: Business logic for {{ resource_name }} operations

### 4. Create Routes

- **File**: `src/routes/{{ resource_name }}.routes.ts`
- **Action**: Create
- **Description**: RESTful routes for {{ resource_name }}

### 5. Create Validation

- **File**: `src/validators/{{ resource_name }}.validator.ts`
- **Action**: Create
- **Description**: Input validation schemas

### 6. Add Tests

- **File**: `src/__tests__/{{ resource_name }}.test.ts`
- **Action**: Create
- **Description**: Unit and integration tests

## API Endpoints

| Method | Endpoint                            | Description                         |
| ------ | ----------------------------------- | ----------------------------------- |
| GET    | /api/{{ resource_name_plural }}     | List all {{ resource_name_plural }} |
| GET    | /api/{{ resource_name_plural }}/:id | Get single {{ resource_name }}      |
| POST   | /api/{{ resource_name_plural }}     | Create new {{ resource_name }}      |
| PUT    | /api/{{ resource_name_plural }}/:id | Update {{ resource_name }}          |
| DELETE | /api/{{ resource_name_plural }}/:id | Delete {{ resource_name }}          |

## Response Format

```json
{
  "success": true,
  "data": { ... },
  "message": "Operation successful"
}
```

## Error Response

```json
{
  "success": false,
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid input",
    "details": [...]
  }
}
```

## Validation

- All inputs validated before processing
- Appropriate HTTP status codes returned
- Pagination supported for list endpoint
- Soft delete implemented (optional)

## Acceptance Criteria

- [ ] All CRUD operations functional
- [ ] Input validation working
- [ ] Error handling complete
- [ ] Pagination implemented
- [ ] Tests passing with >80% coverage
