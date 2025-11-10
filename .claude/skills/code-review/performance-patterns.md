# Performance Patterns Reference

This file catalogs common performance anti-patterns, how to detect them in code
review, and how to optimize them. Reference this when reviewing code for
performance issues.

## Database Performance

### N+1 Query Problem

**Pattern to Detect:**

```javascript
// ❌ N+1 queries - fetches users, then separate query for each user's posts
const users = await User.findAll()
for (const user of users) {
  user.posts = await Post.findAll({ where: { userId: user.id } })
}

// Python/Django
users = User.objects.all()
for user in users:
    posts = Post.objects.filter(user_id=user.id) # N+1
```

**How to Fix:**

```javascript
// ✅ Use eager loading / joins
const users = await User.findAll({
  include: [{ model: Post }],
});

// ✅ Sequelize with associations
const users = await User.findAll({
  include: ['posts'], // Defined association
});

// Python/Django
users = User.objects.prefetch_related('posts');

// Python/SQLAlchemy
users = session.query(User).options(joinedload(User.posts)).all();
```

**Detection in Review:**

- Loop with database query inside
- Multiple queries for related data
- Missing `include`/`join`/`prefetch_related`

### Missing Indexes

**Pattern to Detect:**

```sql
-- ❌ No index on frequently queried column
SELECT * FROM users WHERE email = 'user@example.com'
-- If email has no index, this is a full table scan

-- ❌ No index on foreign keys
SELECT * FROM posts WHERE user_id = 123
-- If user_id has no index, slow on large tables
```

**How to Fix:**

```sql
-- ✅ Add index
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_posts_user_id ON posts(user_id);

-- ✅ Composite index for common query patterns
CREATE INDEX idx_posts_user_status ON posts(user_id, status);
```

```javascript
// Sequelize migration
await queryInterface.addIndex('users', ['email'])
await queryInterface.addIndex('posts', ['user_id', 'status'])

// Django migration
class Migration:
    operations = [
        migrations.AddIndex(
            model_name='user',
            index=models.Index(fields=['email'])
        )
    ]
```

**Detection in Review:**

- Look for WHERE clauses
- Check if columns have indexes
- Foreign keys without indexes
- Columns in JOIN conditions

### Inefficient Queries

**Pattern to Detect:**

```javascript
// ❌ Fetching all columns when only need a few
const users = await User.findAll();
return users.map((u) => u.id);

// ❌ Loading all records without pagination
const posts = await Post.findAll(); // Could be thousands

// ❌ Multiple separate queries instead of JOIN
const user = await User.findOne({ where: { id } });
const posts = await Post.findAll({ where: { userId: id } });
const comments = await Comment.findAll({ where: { userId: id } });
```

**How to Fix:**

```javascript
// ✅ Select only needed columns
const users = await User.findAll({
  attributes: ['id'],
});

// ✅ Pagination
const posts = await Post.findAll({
  limit: 20,
  offset: page * 20,
});

// ✅ Single query with JOINs
const user = await User.findOne({
  where: { id },
  include: [{ model: Post }, { model: Comment }],
});
```

### Unnecessary Database Roundtrips

**Pattern to Detect:**

```javascript
// ❌ Multiple updates in loop
for (const user of users) {
  await User.update({ lastSeen: new Date() }, { where: { id: user.id } });
}

// ❌ Checking existence before insert
const exists = await User.findOne({ where: { email } });
if (!exists) {
  await User.create({ email });
}
```

**How to Fix:**

```javascript
// ✅ Bulk update
await User.update(
  { lastSeen: new Date() },
  { where: { id: { [Op.in]: users.map((u) => u.id) } } }
);

// ✅ Upsert (insert or update)
await User.upsert({ email });

// ✅ Batch inserts
await User.bulkCreate(users);
```

## Algorithm Complexity

### Inefficient Algorithms

**Pattern to Detect:**

```javascript
// ❌ O(n²) - nested loops on same collection
function findDuplicates(arr) {
  const duplicates = [];
  for (let i = 0; i < arr.length; i++) {
    for (let j = i + 1; j < arr.length; j++) {
      if (arr[i] === arr[j]) {
        duplicates.push(arr[i]);
      }
    }
  }
  return duplicates;
}

// ❌ O(n²) - indexOf in loop
const common = arr1.filter((item) => arr2.indexOf(item) !== -1);
```

**How to Fix:**

```javascript
// ✅ O(n) using Set
function findDuplicates(arr) {
  const seen = new Set();
  const duplicates = new Set();
  for (const item of arr) {
    if (seen.has(item)) {
      duplicates.add(item);
    }
    seen.add(item);
  }
  return Array.from(duplicates);
}

// ✅ O(n) using Set for lookup
const set2 = new Set(arr2);
const common = arr1.filter((item) => set2.has(item));
```

### Unnecessary Iterations

**Pattern to Detect:**

```javascript
// ❌ Multiple passes over array
const positives = numbers.filter((n) => n > 0);
const sum = positives.reduce((a, b) => a + b, 0);
const avg = sum / positives.length;

// ❌ Array operations in loop
for (const item of items) {
  total += item.price;
  names.push(item.name);
  if (item.featured) featured.push(item);
}
```

**How to Fix:**

```javascript
// ✅ Single pass with reduce
const { sum, count } = numbers.reduce(
  (acc, n) => {
    if (n > 0) {
      acc.sum += n;
      acc.count++;
    }
    return acc;
  },
  { sum: 0, count: 0 }
);
const avg = sum / count;

// ✅ Single pass (already optimal, but structured)
const result = items.reduce(
  (acc, item) => {
    acc.total += item.price;
    acc.names.push(item.name);
    if (item.featured) acc.featured.push(item);
    return acc;
  },
  { total: 0, names: [], featured: [] }
);
```

## Memory Management

### Memory Leaks

**Pattern to Detect:**

```javascript
// ❌ Event listeners not removed
componentDidMount() {
  window.addEventListener('resize', this.handleResize)
}
// Component unmounts but listener remains

// ❌ Intervals not cleared
componentDidMount() {
  this.interval = setInterval(this.poll, 1000)
}

// ❌ Growing arrays/caches
const cache = []
function addToCache(item) {
  cache.push(item) // Never cleared, grows forever
}

// ❌ Circular references preventing GC
const obj1 = {}
const obj2 = { ref: obj1 }
obj1.ref = obj2 // Circular reference
```

**How to Fix:**

```javascript
// ✅ Clean up event listeners
componentWillUnmount() {
  window.removeEventListener('resize', this.handleResize)
}

// React hooks
useEffect(() => {
  window.addEventListener('resize', handleResize)
  return () => window.removeEventListener('resize', handleResize)
}, [])

// ✅ Clear intervals
componentWillUnmount() {
  clearInterval(this.interval)
}

// ✅ Bounded cache with LRU
const LRU = require('lru-cache')
const cache = new LRU({ max: 500 })

// ✅ Break circular references
obj1.ref = null
obj2.ref = null
```

### Unnecessary Memory Allocation

**Pattern to Detect:**

```javascript
// ❌ Creating objects in render/loop
render() {
  const style = { color: 'red' } // New object every render
  return <div style={style} />
}

// ❌ Creating functions in render
render() {
  return <button onClick={() => this.handleClick()} />
}

// ❌ Copying large arrays unnecessarily
const sorted = [...largeArray].sort() // Copies entire array
```

**How to Fix:**

```javascript
// ✅ Define outside render
const style = { color: 'red' }
render() {
  return <div style={style} />
}

// ✅ Bind in constructor or use class property
handleClick = () => { /* ... */ }
render() {
  return <button onClick={this.handleClick} />
}

// ✅ Sort in place if mutation is acceptable
largeArray.sort()

// Or if immutability needed, be aware of cost
const sorted = [...largeArray].sort() // Necessary copy
```

## Network Performance

### Too Many HTTP Requests

**Pattern to Detect:**

```javascript
// ❌ Sequential requests that could be parallel
const user = await fetchUser();
const posts = await fetchPosts();
const comments = await fetchComments();

// ❌ Loading resources one by one
for (const id of ids) {
  const item = await fetch(`/api/items/${id}`);
  items.push(item);
}

// ❌ No request batching
ids.forEach((id) => {
  fetch(`/api/delete/${id}`);
});
```

**How to Fix:**

```javascript
// ✅ Parallel requests
const [user, posts, comments] = await Promise.all([
  fetchUser(),
  fetchPosts(),
  fetchComments(),
]);

// ✅ Batch API request
const items = await fetch('/api/items', {
  method: 'POST',
  body: JSON.stringify({ ids }),
});

// ✅ Single delete request with array
await fetch('/api/delete', {
  method: 'POST',
  body: JSON.stringify({ ids }),
});
```

### Large Payloads

**Pattern to Detect:**

```javascript
// ❌ Sending unnecessary data
return res.json({ users: allUsers }); // Includes passwords, tokens, etc.

// ❌ No pagination
const posts = await Post.findAll(); // Returns 10,000 records

// ❌ Including related data unnecessarily
const user = await User.findOne({
  include: [{ all: true, nested: true }],
});
```

**How to Fix:**

```javascript
// ✅ Send only needed fields
return res.json({
  users: allUsers.map((u) => ({
    id: u.id,
    username: u.username,
    email: u.email,
  })),
});

// ✅ Pagination
const posts = await Post.findAll({
  limit: 20,
  offset: req.query.page * 20,
});

// ✅ Include only what's needed
const user = await User.findOne({
  include: [{ model: Profile, attributes: ['bio'] }],
});
```

### Missing Caching

**Pattern to Detect:**

```javascript
// ❌ No caching of expensive operations
app.get('/stats', async (req, res) => {
  const stats = await calculateStats(); // Runs every request
  res.json(stats);
});

// ❌ No HTTP caching headers
res.json(data); // No Cache-Control header

// ❌ Re-fetching unchanged data
useEffect(() => {
  fetchData(); // Fetches on every render
}, []);
```

**How to Fix:**

```javascript
// ✅ In-memory caching
const cache = new Map();
app.get('/stats', async (req, res) => {
  if (cache.has('stats')) {
    return res.json(cache.get('stats'));
  }
  const stats = await calculateStats();
  cache.set('stats', stats);
  setTimeout(() => cache.delete('stats'), 300000); // 5 min TTL
  res.json(stats);
});

// ✅ HTTP caching headers
res.set('Cache-Control', 'public, max-age=300');
res.json(data);

// ✅ React Query or SWR for smart caching
const { data } = useQuery('stats', fetchStats, {
  staleTime: 300000, // 5 minutes
});
```

## React Performance

### Unnecessary Re-renders

**Pattern to Detect:**

```jsx
// ❌ Creating new objects/arrays in render
<Component items={data.filter(d => d.active)} />

// ❌ Inline arrow functions as props
<Child onClick={() => handleClick(id)} />

// ❌ Not memoizing expensive calculations
const sorted = items.sort() // Runs every render

// ❌ Missing React.memo for expensive components
function ExpensiveList({ items }) {
  return items.map(item => <ExpensiveItem key={item.id} item={item} />)
}
```

**How to Fix:**

```jsx
// ✅ Memoize filtered data
const activeItems = useMemo(
  () => data.filter(d => d.active),
  [data]
)
<Component items={activeItems} />

// ✅ useCallback for event handlers
const handleClickItem = useCallback(
  (id) => handleClick(id),
  [handleClick]
)
<Child onClick={handleClickItem} />

// ✅ useMemo for expensive calculations
const sorted = useMemo(
  () => items.sort((a, b) => a.value - b.value),
  [items]
)

// ✅ React.memo to prevent re-renders
const ExpensiveList = React.memo(({ items }) => {
  return items.map(item => <ExpensiveItem key={item.id} item={item} />)
})
```

### Large Lists Without Virtualization

**Pattern to Detect:**

```jsx
// ❌ Rendering thousands of items
function List({ items }) {
  return (
    <div>
      {items.map((item) => (
        <Item key={item.id} item={item} />
      ))}
    </div>
  );
}
// If items.length > 100, consider virtualization
```

**How to Fix:**

```jsx
// ✅ Use react-window or react-virtualized
import { FixedSizeList } from 'react-window';

function List({ items }) {
  return (
    <FixedSizeList height={600} itemCount={items.length} itemSize={50}>
      {({ index, style }) => (
        <div style={style}>
          <Item item={items[index]} />
        </div>
      )}
    </FixedSizeList>
  );
}
```

## Bundle Size

### Large Dependencies

**Pattern to Detect:**

```javascript
// ❌ Importing entire library
import _ from 'lodash';
import moment from 'moment';

// ❌ Importing everything from UI library
import * as MaterialUI from '@material-ui/core';

// ❌ Large bundle in critical path
import HugeChart from 'huge-chart-library';
```

**How to Fix:**

```javascript
// ✅ Import only what you need
import debounce from 'lodash/debounce';
import format from 'date-fns/format';

// ✅ Tree-shakeable imports
import { Button, TextField } from '@material-ui/core';

// ✅ Code splitting for large dependencies
const HugeChart = lazy(() => import('huge-chart-library'));
```

### Missing Code Splitting

**Pattern to Detect:**

```javascript
// ❌ All routes in main bundle
import Home from './Home';
import Admin from './Admin';
import Analytics from './Analytics';

// All loaded even if user never visits these pages
```

**How to Fix:**

```javascript
// ✅ Route-based code splitting
import { lazy, Suspense } from 'react';

const Home = lazy(() => import('./Home'));
const Admin = lazy(() => import('./Admin'));
const Analytics = lazy(() => import('./Analytics'));

function App() {
  return (
    <Suspense fallback={<Loading />}>
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/admin" element={<Admin />} />
        <Route path="/analytics" element={<Analytics />} />
      </Routes>
    </Suspense>
  );
}
```

## Node.js Performance

### Blocking the Event Loop

**Pattern to Detect:**

```javascript
// ❌ Synchronous operations in routes
app.get('/data', (req, res) => {
  const data = fs.readFileSync('large-file.json');
  res.send(data);
});

// ❌ CPU-intensive work in main thread
app.post('/process', (req, res) => {
  const result = processMillionsOfRecords(req.body); // Blocks
  res.json(result);
});

// ❌ Synchronous crypto
const hash = crypto.pbkdf2Sync(password, salt, 100000, 64, 'sha512');
```

**How to Fix:**

```javascript
// ✅ Async file operations
app.get('/data', async (req, res) => {
  const data = await fs.promises.readFile('large-file.json');
  res.send(data);
});

// ✅ Worker threads for CPU-intensive tasks
const { Worker } = require('worker_threads');

app.post('/process', (req, res) => {
  const worker = new Worker('./processor.js', {
    workerData: req.body,
  });
  worker.on('message', (result) => res.json(result));
});

// ✅ Async crypto
crypto.pbkdf2(password, salt, 100000, 64, 'sha512', (err, key) => {
  // Handle result
});
```

## Performance Review Checklist

Quick checklist for performance review:

### Database

- [ ] No N+1 queries
- [ ] Indexes on queried columns
- [ ] Pagination for large datasets
- [ ] Efficient query patterns
- [ ] Connection pooling configured

### Algorithms

- [ ] Appropriate complexity (no O(n²) where O(n) possible)
- [ ] Efficient data structures used
- [ ] No unnecessary iterations
- [ ] Caching for expensive operations

### Memory

- [ ] No memory leaks
- [ ] Event listeners cleaned up
- [ ] Intervals/timeouts cleared
- [ ] Bounded caches
- [ ] Avoid unnecessary allocations

### Network

- [ ] Requests parallelized where possible
- [ ] Batch API calls
- [ ] Pagination implemented
- [ ] Caching headers set
- [ ] Payload size minimized

### Frontend (React)

- [ ] Memoization for expensive calculations
- [ ] No unnecessary re-renders
- [ ] Virtualization for long lists
- [ ] Code splitting for routes
- [ ] Lazy loading for heavy components

### Bundle

- [ ] Tree-shakeable imports
- [ ] Code splitting
- [ ] Compression enabled
- [ ] Only necessary dependencies

---

**Reference Version:** 1.0 **Last Updated:** 2025-11-08
