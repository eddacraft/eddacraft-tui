# Framework and Library Pattern Library

This file catalogs common patterns for popular frameworks and libraries to help
with pattern detection and analysis.

## React Patterns

### Component Organization

**Feature-Based (Recommended for larger apps)**

```
src/
  features/
    auth/
      components/
        LoginForm.tsx
        SignupForm.tsx
      hooks/
        useAuth.ts
      api/
        authApi.ts
      types/
        auth.types.ts
```

**Type-Based (Common in smaller apps)**

```
src/
  components/
    LoginForm.tsx
    SignupForm.tsx
  hooks/
    useAuth.ts
  api/
    authApi.ts
```

### State Management Patterns

**Context API Pattern**

```typescript
// Context + Provider pattern
export const AuthContext = createContext<AuthContextType>(null!)

export function AuthProvider({ children }: Props) {
  const [user, setUser] = useState<User | null>(null)
  return (
    <AuthContext.Provider value={{ user, setUser }}>
      {children}
    </AuthContext.Provider>
  )
}

export const useAuth = () => useContext(AuthContext)
```

**Redux Pattern**

```typescript
// Slice pattern (Redux Toolkit)
const userSlice = createSlice({
  name: 'user',
  initialState,
  reducers: {
    setUser: (state, action) => {
      state.user = action.payload;
    },
  },
});
```

**Zustand Pattern**

```typescript
// Store pattern
const useStore = create<State>((set) => ({
  user: null,
  setUser: (user) => set({ user }),
}));
```

### Hook Patterns

**Custom Hook Pattern**

```typescript
function useData(id: string) {
  const [data, setData] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  useEffect(() => {
    fetchData(id)
      .then(setData)
      .catch(setError)
      .finally(() => setLoading(false));
  }, [id]);

  return { data, loading, error };
}
```

**React Query Pattern**

```typescript
const { data, isLoading, error } = useQuery(['user', id], () => fetchUser(id));
```

### Styling Patterns

**CSS Modules**

```typescript
import styles from './Button.module.css'
<button className={styles.primary}>Click</button>
```

**Styled Components**

```typescript
const Button = styled.button`
  background: blue;
  color: white;
`;
```

**Tailwind**

```typescript
<button className="bg-blue-500 text-white px-4 py-2">
  Click
</button>
```

## Node.js / Express Patterns

### Route Organization

**Route Files Pattern**

```javascript
// routes/users.js
router.get('/', getAllUsers);
router.get('/:id', getUser);
router.post('/', createUser);
router.put('/:id', updateUser);
router.delete('/:id', deleteUser);
```

**Controller Pattern**

```javascript
// controllers/userController.js
exports.getAllUsers = async (req, res) => {
  const users = await User.findAll();
  res.json(users);
};

// routes/users.js
const controller = require('../controllers/userController');
router.get('/', controller.getAllUsers);
```

### Middleware Pattern

**Standard Middleware**

```javascript
function authenticate(req, res, next) {
  const token = req.headers.authorization;
  if (!token) {
    return res.status(401).json({ error: 'Unauthorized' });
  }
  // Verify token
  req.user = decoded;
  next();
}

app.use('/api/protected', authenticate);
```

**Error Handling Middleware**

```javascript
app.use((err, req, res, next) => {
  logger.error(err);
  res.status(err.status || 500).json({
    error:
      process.env.NODE_ENV === 'production'
        ? 'Internal server error'
        : err.message,
  });
});
```

### Service Layer Pattern

```javascript
// services/userService.js
class UserService {
  async getUser(id) {
    return await User.findByPk(id);
  }

  async createUser(data) {
    return await User.create(data);
  }
}

// controllers/userController.js
const userService = new UserService();

exports.getUser = async (req, res) => {
  const user = await userService.getUser(req.params.id);
  res.json(user);
};
```

### Repository Pattern

```javascript
// repositories/userRepository.js
class UserRepository {
  async findById(id) {
    return await db.query('SELECT * FROM users WHERE id = ?', [id]);
  }

  async create(user) {
    return await db.query('INSERT INTO users SET ?', [user]);
  }
}

// services/userService.js
class UserService {
  constructor(userRepo) {
    this.userRepo = userRepo;
  }

  async getUser(id) {
    return await this.userRepo.findById(id);
  }
}
```

## Python / Django Patterns

### App Organization

**Django App Pattern**

```
myapp/
  models.py         # Database models
  views.py          # View functions/classes
  urls.py           # URL routing
  serializers.py    # DRF serializers
  admin.py          # Admin interface
  tests.py          # Tests
```

### View Patterns

**Function-Based Views**

```python
def user_list(request):
    users = User.objects.all()
    return JsonResponse({'users': list(users.values())})
```

**Class-Based Views**

```python
class UserListView(ListView):
    model = User
    template_name = 'users/list.html'
    context_object_name = 'users'
```

**Django REST Framework ViewSets**

```python
class UserViewSet(viewsets.ModelViewSet):
    queryset = User.objects.all()
    serializer_class = UserSerializer
    permission_classes = [IsAuthenticated]
```

### Model Patterns

**Standard Model**

```python
class User(models.Model):
    username = models.CharField(max_length=100, unique=True)
    email = models.EmailField(unique=True)
    created_at = models.DateTimeField(auto_now_add=True)

    class Meta:
        ordering = ['-created_at']

    def __str__(self):
        return self.username
```

**Abstract Base Model**

```python
class TimestampedModel(models.Model):
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)

    class Meta:
        abstract = True

class User(TimestampedModel):
    username = models.CharField(max_length=100)
```

### Manager Pattern

```python
class UserManager(models.Manager):
    def active(self):
        return self.filter(is_active=True)

    def admins(self):
        return self.filter(is_staff=True)

class User(models.Model):
    objects = UserManager()
```

## Python / FastAPI Patterns

### Router Organization

```python
# routers/users.py
router = APIRouter(prefix="/users", tags=["users"])

@router.get("/")
async def get_users():
    return await user_service.get_all()

@router.get("/{user_id}")
async def get_user(user_id: int):
    return await user_service.get(user_id)
```

### Dependency Injection

```python
# dependencies/auth.py
async def get_current_user(token: str = Depends(oauth2_scheme)):
    user = verify_token(token)
    if not user:
        raise HTTPException(status_code=401)
    return user

# routers/users.py
@router.get("/me")
async def get_me(current_user: User = Depends(get_current_user)):
    return current_user
```

### Pydantic Models

```python
class UserBase(BaseModel):
    username: str
    email: EmailStr

class UserCreate(UserBase):
    password: str

class UserResponse(UserBase):
    id: int
    created_at: datetime

    class Config:
        from_attributes = True
```

## Rust Patterns

### Module Organization

```
src/
  main.rs
  lib.rs
  models/
    mod.rs
    user.rs
  services/
    mod.rs
    user_service.rs
  api/
    mod.rs
    routes.rs
```

### Error Handling Pattern

**Result Type Pattern**

```rust
pub enum AppError {
    NotFound(String),
    Unauthorized,
    DatabaseError(sqlx::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err)
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

pub async fn get_user(id: i32) -> Result<User> {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = ?", id)
        .fetch_one(&pool)
        .await?;
    Ok(user)
}
```

### Builder Pattern

```rust
pub struct User {
    id: i32,
    username: String,
    email: String,
}

impl User {
    pub fn builder() -> UserBuilder {
        UserBuilder::default()
    }
}

pub struct UserBuilder {
    username: Option<String>,
    email: Option<String>,
}

impl UserBuilder {
    pub fn username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    pub fn email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    pub fn build(self) -> Result<User> {
        // Validation and construction
    }
}
```

### Trait Pattern

```rust
pub trait Repository<T> {
    async fn find_by_id(&self, id: i32) -> Result<T>;
    async fn create(&self, entity: T) -> Result<T>;
    async fn update(&self, entity: T) -> Result<T>;
    async fn delete(&self, id: i32) -> Result<()>;
}

pub struct UserRepository {
    pool: PgPool,
}

impl Repository<User> for UserRepository {
    async fn find_by_id(&self, id: i32) -> Result<User> {
        // Implementation
    }
}
```

## Go Patterns

### Package Organization

```
project/
  cmd/
    api/
      main.go
  pkg/
    user/
      user.go
      repository.go
      service.go
  internal/
    auth/
      auth.go
```

### Interface Pattern

```go
type UserRepository interface {
    FindByID(id int) (*User, error)
    Create(user *User) error
    Update(user *User) error
    Delete(id int) error
}

type userRepo struct {
    db *sql.DB
}

func NewUserRepository(db *sql.DB) UserRepository {
    return &userRepo{db: db}
}

func (r *userRepo) FindByID(id int) (*User, error) {
    // Implementation
}
```

### Error Handling Pattern

```go
func GetUser(id int) (*User, error) {
    user, err := repo.FindByID(id)
    if err != nil {
        return nil, fmt.Errorf("failed to get user: %w", err)
    }
    return user, nil
}
```

### Context Pattern

```go
func (s *UserService) GetUser(ctx context.Context, id int) (*User, error) {
    // Check context cancellation
    select {
    case <-ctx.Done():
        return nil, ctx.Err()
    default:
    }

    user, err := s.repo.FindByID(ctx, id)
    if err != nil {
        return nil, err
    }
    return user, nil
}
```

## Database Patterns

### ORM Patterns (Sequelize, TypeORM, SQLAlchemy)

**Model Definition**

```typescript
// TypeORM
@Entity()
export class User {
  @PrimaryGeneratedColumn()
  id: number;

  @Column({ unique: true })
  email: string;

  @OneToMany(() => Post, (post) => post.user)
  posts: Post[];
}
```

**Repository Pattern**

```typescript
const userRepo = getRepository(User);
const user = await userRepo.findOne({
  where: { id },
  relations: ['posts'],
});
```

### Query Builder Patterns

```typescript
// Knex.js
const users = await knex('users')
  .select('*')
  .where('is_active', true)
  .orderBy('created_at', 'desc')
  .limit(10);
```

### Migration Patterns

```typescript
// Sequelize migration
module.exports = {
  up: async (queryInterface, Sequelize) => {
    await queryInterface.createTable('users', {
      id: {
        type: Sequelize.INTEGER,
        primaryKey: true,
        autoIncrement: true,
      },
      email: {
        type: Sequelize.STRING,
        unique: true,
        allowNull: false,
      },
    });
  },

  down: async (queryInterface, Sequelize) => {
    await queryInterface.dropTable('users');
  },
};
```

## Testing Patterns

### Unit Test Patterns

**Jest/Vitest**

```typescript
describe('UserService', () => {
  describe('getUser', () => {
    it('should return user when found', async () => {
      const user = await userService.getUser(1);
      expect(user).toBeDefined();
      expect(user.id).toBe(1);
    });

    it('should throw error when not found', async () => {
      await expect(userService.getUser(999)).rejects.toThrow('User not found');
    });
  });
});
```

**pytest**

```python
class TestUserService:
    def test_get_user_returns_user_when_found(self):
        user = user_service.get_user(1)
        assert user is not None
        assert user.id == 1

    def test_get_user_raises_error_when_not_found(self):
        with pytest.raises(UserNotFound):
            user_service.get_user(999)
```

### Integration Test Patterns

```typescript
describe('User API', () => {
  beforeAll(async () => {
    await setupTestDb();
  });

  afterAll(async () => {
    await teardownTestDb();
  });

  it('should create user', async () => {
    const response = await request(app)
      .post('/api/users')
      .send({ username: 'test', email: 'test@example.com' })
      .expect(201);

    expect(response.body.id).toBeDefined();
  });
});
```

### Mock Patterns

```typescript
// Jest mock
const mockRepo = {
  findByID: jest.fn().mockResolvedValue({ id: 1, name: 'Test' }),
};

const service = new UserService(mockRepo);
const user = await service.getUser(1);

expect(mockRepo.findByID).toHaveBeenCalledWith(1);
```

## Detection Heuristics

When analyzing a repository, use these heuristics to detect patterns:

### React Detection

- Look for `jsx` or `tsx` files
- Check for `react` in dependencies
- Find `useState`, `useEffect` imports
- Identify component file structure

### State Management

- Redux: Look for `store.ts`, `slice.ts`, `createSlice`
- Context: Look for `createContext`, `useContext`
- Zustand: Look for `create` from 'zustand'

### Styling

- CSS Modules: `*.module.css` files
- Styled Components: `styled` imports
- Tailwind: `className` with utility classes

### API Layer

- Express: `app.use`, `router.get`
- FastAPI: `@app.get`, `APIRouter`
- Django: `views.py`, `urls.py`

### Database

- TypeORM: `@Entity`, `@Column` decorators
- Sequelize: `sequelize.define`, `Model.init`
- Prisma: `prisma/schema.prisma` file
- Raw SQL: Direct query strings

---

**Reference Version:** 1.0 **Last Updated:** 2025-11-08
