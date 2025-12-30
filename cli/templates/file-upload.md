---
id: file-upload
name: File Upload
description: Implement secure file upload with validation and storage
category: api
tags: [upload, file, storage, multer, s3]
variables:
  - name: storage_type
    description: Storage backend (local, s3, gcs)
    default: local
    required: false
  - name: max_file_size
    description: Maximum file size in MB
    default: 10
    required: false
  - name: allowed_types
    description: Comma-separated allowed MIME types
    default: image/jpeg,image/png,application/pdf
    required: false
---

# File Upload Implementation

## Intent

Implement secure file upload functionality with validation, {{ storage_type }}
storage, and proper error handling.

## Changes

### 1. Create Upload Controller

- **File**: `src/controllers/upload.controller.ts`
- **Action**: Create
- **Description**: Handle file upload endpoints

### 2. Create Upload Service

- **File**: `src/services/upload.service.ts`
- **Action**: Create
- **Description**: File processing and storage logic

### 3. Create Storage Adapter

- **File**: `src/services/storage/{{ storage_type }}.adapter.ts`
- **Action**: Create
- **Description**: {{ storage_type }} storage implementation

### 4. Create Upload Middleware

- **File**: `src/middleware/upload.middleware.ts`
- **Action**: Create
- **Description**: Multer configuration with validation

### 5. Create Types

- **File**: `src/types/upload.types.ts`
- **Action**: Create
- **Description**: TypeScript types for upload operations

### 6. Add Routes

- **File**: `src/routes/upload.routes.ts`
- **Action**: Create
- **Description**: Upload API endpoints

### 7. Add Tests

- **File**: `src/__tests__/upload.test.ts`
- **Action**: Create
- **Description**: Upload functionality tests

## Configuration

```typescript
const uploadConfig = {
  maxFileSize: {{ max_file_size }} * 1024 * 1024, // {{ max_file_size }}MB
  allowedTypes: ['{{ allowed_types }}'],
  storage: '{{ storage_type }}',
};
```

## API Endpoints

| Method | Endpoint             | Description           |
| ------ | -------------------- | --------------------- |
| POST   | /api/upload          | Upload single file    |
| POST   | /api/upload/multiple | Upload multiple files |
| DELETE | /api/upload/:id      | Delete uploaded file  |
| GET    | /api/upload/:id      | Get file metadata     |

## Validation Rules

- Maximum file size: {{ max_file_size }}MB
- Allowed types: {{ allowed_types }}
- Filename sanitisation
- Virus scanning (optional)

## Security Measures

- [ ] File type validation (magic bytes, not just extension)
- [ ] Size limits enforced
- [ ] Filename sanitisation
- [ ] Secure storage paths
- [ ] Access control on downloads
- [ ] No execution permissions on uploads

## Error Handling

| Error          | Status | Message                                  |
| -------------- | ------ | ---------------------------------------- |
| File too large | 413    | File exceeds {{ max_file_size }}MB limit |
| Invalid type   | 415    | File type not allowed                    |
| Upload failed  | 500    | Upload failed, please retry              |

## Acceptance Criteria

- [ ] Single file upload works
- [ ] Multiple file upload works
- [ ] Size validation working
- [ ] Type validation working
- [ ] Storage integration complete
- [ ] Delete functionality working
- [ ] Tests passing
