# Security Patterns Reference

This file catalogs common security vulnerabilities, how to detect them in code
review, and how to fix them. Reference this when reviewing code for security
issues.

## OWASP Top 10 Quick Reference

1. **Broken Access Control**
2. **Cryptographic Failures**
3. **Injection**
4. **Insecure Design**
5. **Security Misconfiguration**
6. **Vulnerable and Outdated Components**
7. **Identification and Authentication Failures**
8. **Software and Data Integrity Failures**
9. **Security Logging and Monitoring Failures**
10. **Server-Side Request Forgery (SSRF)**

## Injection Vulnerabilities

### SQL Injection

**Pattern to Detect:**

```javascript
// ❌ Vulnerable - String concatenation
const query = `SELECT * FROM users WHERE id = ${userId}`;
const query = "SELECT * FROM users WHERE name = '" + username + "'";

// ❌ Vulnerable - Template literals
const query = `DELETE FROM users WHERE id = ${req.params.id}`;
```

**How to Fix:**

```javascript
// ✅ Parameterized query (Node.js)
const query = 'SELECT * FROM users WHERE id = ?'
db.query(query, [userId])

// ✅ ORM (Sequelize, TypeORM, etc.)
User.findOne({ where: { id: userId } })

// ✅ Python (psycopg2)
cursor.execute("SELECT * FROM users WHERE id = %s", (user_id,))

// ✅ Python (SQLAlchemy)
session.query(User).filter(User.id == user_id).first()
```

### NoSQL Injection

**Pattern to Detect:**

```javascript
// ❌ Vulnerable - Direct object from request
db.collection.find({ username: req.body.username });

// User can send: { "$ne": null } to bypass auth
```

**How to Fix:**

```javascript
// ✅ Validate and sanitize
const username = String(req.body.username);
if (typeof req.body.username !== 'string') {
  throw new Error('Invalid username');
}
db.collection.find({ username });

// ✅ Use schema validation
const schema = Joi.object({
  username: Joi.string().required(),
});
```

### Command Injection

**Pattern to Detect:**

```javascript
// ❌ Vulnerable
exec(`ping ${userInput}`);
exec(`rm -rf ${directory}`);

// ❌ Vulnerable
import(userProvidedPath);
require(userProvidedModule);
```

**How to Fix:**

```javascript
// ✅ Avoid shell execution with user input
const { execFile } = require('child_process');
execFile('ping', [userInput]);

// ✅ Whitelist allowed values
const allowedDirs = ['uploads', 'temp', 'cache'];
if (!allowedDirs.includes(directory)) {
  throw new Error('Invalid directory');
}

// ✅ Validate and sanitize
const sanitized = userInput.replace(/[^a-zA-Z0-9]/g, '');
```

### XSS (Cross-Site Scripting)

**Pattern to Detect:**

```javascript
// ❌ Vulnerable - Direct HTML injection
element.innerHTML = userInput
document.write(userInput)

// ❌ React - dangerouslySetInnerHTML
<div dangerouslySetInnerHTML={{__html: userInput}} />

// ❌ Template injection
template = `<div>${userInput}</div>`
```

**How to Fix:**

```javascript
// ✅ Use textContent
element.textContent = userInput

// ✅ React - auto-escaping
<div>{userInput}</div>

// ✅ Sanitize HTML if necessary
import DOMPurify from 'dompurify'
const clean = DOMPurify.sanitize(userInput)

// ✅ Use templating engine with auto-escaping (Handlebars, EJS)
```

## Authentication & Authorization

### Broken Authentication

**Pattern to Detect:**

```javascript
// ❌ Weak password requirements
if (password.length < 4) {
  reject();
}

// ❌ No rate limiting on auth endpoints
app.post('/login', handleLogin);

// ❌ Passwords in logs
logger.info(`Login attempt: ${username}:${password}`);

// ❌ Weak session management
req.session.userId = userId; // No expiration

// ❌ Predictable session IDs
sessionId = userId + timestamp;
```

**How to Fix:**

```javascript
// ✅ Strong password requirements
const minLength = 12;
const hasUpperCase = /[A-Z]/.test(password);
const hasLowerCase = /[a-z]/.test(password);
const hasNumber = /[0-9]/.test(password);
const hasSpecial = /[^A-Za-z0-9]/.test(password);

// ✅ Rate limiting
const rateLimit = require('express-rate-limit');
const loginLimiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 5, // 5 requests per window
});
app.post('/login', loginLimiter, handleLogin);

// ✅ Secure logging
logger.info(`Login attempt: ${username}`); // No password

// ✅ Secure session configuration
app.use(
  session({
    secret: process.env.SESSION_SECRET,
    resave: false,
    saveUninitialized: false,
    cookie: {
      secure: true, // HTTPS only
      httpOnly: true, // No JS access
      maxAge: 3600000, // 1 hour
      sameSite: 'strict', // CSRF protection
    },
  })
);
```

### Broken Access Control

**Pattern to Detect:**

```javascript
// ❌ No authorization check
app.delete('/users/:id', (req, res) => {
  deleteUser(req.params.id); // Anyone can delete any user
});

// ❌ Client-side authorization only
if (user.role === 'admin') {
  <DeleteButton />; // Checked only in UI
}

// ❌ IDOR (Insecure Direct Object Reference)
app.get('/invoice/:id', (req, res) => {
  const invoice = getInvoice(req.params.id);
  res.json(invoice); // No ownership check
});

// ❌ Parameter tampering
app.get('/user/profile', (req, res) => {
  const userId = req.query.userId; // User can change this
  const user = getUser(userId);
  res.json(user);
});
```

**How to Fix:**

```javascript
// ✅ Server-side authorization
app.delete('/users/:id', requireAdmin, (req, res) => {
  deleteUser(req.params.id);
});

function requireAdmin(req, res, next) {
  if (req.user.role !== 'admin') {
    return res.status(403).json({ error: 'Forbidden' });
  }
  next();
}

// ✅ Resource ownership check
app.get('/invoice/:id', authenticate, async (req, res) => {
  const invoice = await getInvoice(req.params.id);
  if (invoice.userId !== req.user.id) {
    return res.status(403).json({ error: 'Forbidden' });
  }
  res.json(invoice);
});

// ✅ Use authenticated user context
app.get('/user/profile', authenticate, (req, res) => {
  const user = getUser(req.user.id); // From auth token
  res.json(user);
});
```

## Cryptographic Failures

### Weak Cryptography

**Pattern to Detect:**

```javascript
// ❌ Weak hashing
const hash = md5(password);
const hash = sha1(password);

// ❌ No salt
const hash = bcrypt.hashSync(password, 1); // Weak rounds

// ❌ Hardcoded secrets
const SECRET = 'mysecret123';
const encrypted = encrypt(data, 'hardcoded-key');

// ❌ Custom crypto (usually wrong)
function myEncrypt(data) {
  return data.split('').reverse().join('');
}
```

**How to Fix:**

```javascript
// ✅ Strong password hashing
const bcrypt = require('bcrypt');
const hash = await bcrypt.hash(password, 12); // Strong rounds

// ✅ Use argon2 (even better)
const argon2 = require('argon2');
const hash = await argon2.hash(password);

// ✅ Environment variables for secrets
const SECRET = process.env.JWT_SECRET;
if (!SECRET) throw new Error('JWT_SECRET not configured');

// ✅ Use established crypto libraries
const crypto = require('crypto');
const algorithm = 'aes-256-gcm';
const key = crypto.scryptSync(password, salt, 32);
```

### Insecure Data Storage

**Pattern to Detect:**

```javascript
// ❌ Storing sensitive data in plaintext
user.creditCard = req.body.creditCard;
user.ssn = req.body.ssn;

// ❌ Logging sensitive data
logger.info(`Payment: ${cardNumber}`);

// ❌ Exposing sensitive data in responses
res.json({
  user: {
    password: user.password, // Should never expose
    apiKey: user.apiKey,
  },
});
```

**How to Fix:**

```javascript
// ✅ Encrypt sensitive data
const encrypted = encrypt(creditCard, key);
user.creditCardEncrypted = encrypted;

// ✅ Redact sensitive data in logs
logger.info(`Payment: ${cardNumber.slice(-4)}`);

// ✅ Exclude sensitive fields
res.json({
  user: {
    id: user.id,
    username: user.username,
    // No password, no apiKey
  },
});

// ✅ Use serialization/transformation
class UserDTO {
  constructor(user) {
    this.id = user.id;
    this.username = user.username;
    // Explicitly include only safe fields
  }
}
```

## Cross-Site Request Forgery (CSRF)

**Pattern to Detect:**

```javascript
// ❌ No CSRF protection
app.post('/transfer', (req, res) => {
  transfer(req.body.amount, req.body.toAccount);
});

// ❌ CORS misconfiguration
app.use(
  cors({
    origin: '*', // Allows any origin
  })
);
```

**How to Fix:**

```javascript
// ✅ CSRF tokens (for traditional forms)
const csrf = require('csurf');
app.use(csrf({ cookie: true }));

app.get('/form', (req, res) => {
  res.render('form', { csrfToken: req.csrfToken() });
});

// ✅ SameSite cookie attribute
cookie: {
  sameSite: 'strict';
}

// ✅ Proper CORS configuration
app.use(
  cors({
    origin: ['https://yourdomain.com'],
    credentials: true,
  })
);

// ✅ Verify Origin/Referer headers
function verifyOrigin(req, res, next) {
  const origin = req.get('origin');
  if (origin !== 'https://yourdomain.com') {
    return res.status(403).json({ error: 'Forbidden' });
  }
  next();
}
```

## Server-Side Request Forgery (SSRF)

**Pattern to Detect:**

```javascript
// ❌ Unvalidated URL from user
app.post('/fetch', async (req, res) => {
  const data = await fetch(req.body.url);
  res.send(data);
});

// ❌ Can access internal services
const url = req.query.url;
// User provides: http://localhost:6379/ (Redis)
// Or: http://169.254.169.254/latest/meta-data/ (AWS metadata)
```

**How to Fix:**

```javascript
// ✅ Whitelist allowed domains
const allowedDomains = ['api.example.com', 'cdn.example.com'];
const url = new URL(req.body.url);
if (!allowedDomains.includes(url.hostname)) {
  return res.status(400).json({ error: 'Invalid URL' });
}

// ✅ Blacklist private IP ranges
function isPrivateIP(hostname) {
  return /^(10\.|172\.(1[6-9]|2[0-9]|3[01])\.|192\.168\.|127\.)/.test(hostname);
}

if (isPrivateIP(url.hostname)) {
  return res.status(400).json({ error: 'Private IP not allowed' });
}

// ✅ Use URL parsing and validation
const url = new URL(req.body.url);
if (url.protocol !== 'https:') {
  return res.status(400).json({ error: 'Only HTTPS allowed' });
}
```

## Information Disclosure

**Pattern to Detect:**

```javascript
// ❌ Verbose error messages
catch (err) {
  res.status(500).send(err.stack) // Exposes internal paths
}

// ❌ Exposing system info
res.setHeader('X-Powered-By', 'Express') // Version info

// ❌ Directory listing enabled
app.use(express.static('public', { index: false }))

// ❌ Source maps in production
// webpack.config.js
devtool: 'source-map' // Exposes original code
```

**How to Fix:**

```javascript
// ✅ Generic error messages to users
catch (err) {
  logger.error(err) // Full error in logs
  res.status(500).json({ error: 'Internal server error' })
}

// ✅ Remove version headers
app.disable('x-powered-by')

// ✅ Proper static file configuration
app.use(express.static('public', {
  index: 'index.html',
  dotfiles: 'deny'
}))

// ✅ No source maps in production
devtool: process.env.NODE_ENV === 'production' ? false : 'source-map'
```

## Dependency Vulnerabilities

**Pattern to Detect:**

```bash
# ❌ Outdated dependencies
npm audit # Shows vulnerabilities
npm outdated # Shows old versions

# ❌ No lock file
# Missing package-lock.json or yarn.lock

# ❌ Installing from untrusted sources
npm install https://github.com/random/package.git
```

**How to Fix:**

```bash
# ✅ Regular dependency updates
npm audit fix
npm update

# ✅ Use lock files
# Commit package-lock.json or yarn.lock

# ✅ Automated dependency scanning
# Use Dependabot, Snyk, or GitHub Security

# ✅ Verify package integrity
npm ci # Uses lock file

# ✅ Scan for known vulnerabilities
npm audit
```

## Security Headers

**Pattern to Detect:**

```javascript
// ❌ Missing security headers
app.use(express.json());
// No security middleware
```

**How to Fix:**

```javascript
// ✅ Use Helmet.js
const helmet = require('helmet');
app.use(helmet());

// ✅ Or configure manually
app.use((req, res, next) => {
  res.setHeader('X-Content-Type-Options', 'nosniff');
  res.setHeader('X-Frame-Options', 'DENY');
  res.setHeader('X-XSS-Protection', '1; mode=block');
  res.setHeader(
    'Strict-Transport-Security',
    'max-age=31536000; includeSubDomains'
  );
  res.setHeader('Content-Security-Policy', "default-src 'self'");
  next();
});
```

## Security Review Checklist

Use this quick checklist during code review:

### Input Validation

- [ ] All user input validated
- [ ] Whitelist validation (not blacklist)
- [ ] Type checking performed
- [ ] Length limits enforced
- [ ] Special characters handled

### Authentication

- [ ] Strong password requirements
- [ ] Secure password storage (bcrypt/argon2)
- [ ] Rate limiting on auth endpoints
- [ ] Multi-factor authentication support
- [ ] Session management secure

### Authorization

- [ ] All endpoints check authorization
- [ ] Resource ownership verified
- [ ] Role-based access control
- [ ] No client-side auth only

### Data Protection

- [ ] Sensitive data encrypted at rest
- [ ] TLS/HTTPS enforced
- [ ] Secrets in environment variables
- [ ] No sensitive data in logs
- [ ] PII handling compliant

### Injection Prevention

- [ ] Parameterized queries
- [ ] Input sanitization
- [ ] Output encoding
- [ ] No eval() or similar
- [ ] Safe deserialization

### Error Handling

- [ ] Generic error messages to users
- [ ] Detailed errors only in logs
- [ ] No stack traces exposed
- [ ] Error logging comprehensive

### Dependencies

- [ ] Dependencies up to date
- [ ] Known vulnerabilities patched
- [ ] Lock files committed
- [ ] Minimal dependencies

---

**Reference Version:** 1.0 **Based on:** OWASP Top 10 2021 **Last Updated:**
2025-11-08
