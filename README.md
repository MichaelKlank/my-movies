# My Movies

A self-hosted movie and series collection manager with barcode scanning support.

## Features

- **Single Binary Deployment**: Tauri desktop app includes embedded server - just download and run!
- **Multi-User Support**: Each user manages their own collection with isolated data
- **Barcode Scanning**: Scan DVD/Blu-ray EAN codes via browser camera or native Tauri app
- **TMDB Integration**: Automatic metadata fetching from The Movie Database
- **Real-time Sync**: WebSocket-based synchronization across all connected clients
- **Import/Export**: CSV import from existing collection managers (My Movies Pro compatible)
- **Responsive Design**: Works on desktop, tablet, and mobile browsers
- **Native Apps**: Optional Tauri apps for iOS/Android with native barcode scanning
- **Flexible Deployment**: Run as desktop app (embedded server) or web server (for multi-device access)

## Tech Stack

### Backend (Rust)
- **Axum**: HTTP/WebSocket server
- **SQLx**: Compile-time checked SQL queries
- **SQLite + Litestream**: Database with continuous backup
- **JWT**: Built-in authentication

### Frontend (TypeScript/React)
- **React 18**: UI framework
- **TanStack Router**: Type-safe routing
- **TanStack Query**: Server state management with WebSocket invalidation
- **Shadcn/ui**: Component library
- **Tailwind CSS**: Styling
- **html5-qrcode**: Browser-based barcode scanning

### Desktop/Mobile (Tauri 2.0)
- **Tauri**: Cross-platform app framework
- **Embedded Server**: Full Rust backend compiled into single binary
- **Native Barcode Scanner**: Reliable scanning on iOS/Android

## Architecture

The application uses a modular architecture where the server logic is a reusable library. This enables three deployment modes:

| Mode | Frontend | Backend | Use Case |
|------|----------|---------|----------|
| **Tauri Desktop** | Embedded in app | Embedded in app | Personal use, single binary |
| **Docker** | Served by Rust server | Rust server | Server deployment, self-hosted |
| **Development** | Vite dev server | Standalone Rust | Development with hot reload |

```
┌─────────────────────────────────────────────────────────────────┐
│                         Clients                                 │
├─────────────────┬───────────────────────────────────────────────┤
│   Web Browser   │           Tauri Desktop App                   │
│   (React SPA)   │   ┌─────────────────────────────────────────┐ │
│        │        │   │  React SPA + Embedded Rust Server       │ │
│        │        │   │  (Single Binary - No external server!)  │ │
│        │        │   └─────────────────────────────────────────┘ │
└────────┬────────┴───────────────────────────────────────────────┘
         │                             │
         │ HTTP/WS                     │ localhost (internal)
         │                             │
┌────────▼─────────────────────────────▼──────────────────────────┐
│              Server Library (my-movies-server)                  │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │  REST API   │  │  WebSocket  │  │  Background Jobs        │  │
│  │  /api/v1/*  │  │  /ws        │  │  (TMDB fetch, cleanup)  │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                      Core Library (my-movies-core)              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   Models    │  │  Services   │  │  Database (SQLx)        │  │
│  │             │  │  - Auth     │  │  - Migrations           │  │
│  │             │  │  - Movies   │  │  - Queries              │  │
│  │             │  │  - Import   │  │                         │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
         │
         │
┌────────▼────────┐     ┌─────────────────┐
│     SQLite      │────▶│   Litestream    │────▶ S3/MinIO Backup
└─────────────────┘     └─────────────────┘

External APIs:
┌─────────────────┐     ┌─────────────────┐
│  Open EAN DB    │     │      TMDB       │
│  (EAN → Title)  │     │  (Title → Meta) │
└─────────────────┘     └─────────────────┘
```

### Crate Dependencies

```
my-movies-core (library)
       ↑
       │
       ├─────────────────────┐
       │                     │
my-movies-server         my-movies-tauri
    (lib + bin)              (bin)
       │                     │
       ↓                     ↓
  Standalone Server    Desktop App with
  (for web deployment) Embedded Server
```

## Project Structure

```
my-movies/
├── crates/
│   ├── core/                 # Shared Rust library
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── config.rs     # Configuration management
│   │   │   ├── error.rs      # Error types
│   │   │   ├── db/           # Database layer
│   │   │   │   ├── mod.rs
│   │   │   │   └── migrations/
│   │   │   ├── models/       # Domain models
│   │   │   │   ├── mod.rs
│   │   │   │   ├── user.rs
│   │   │   │   ├── movie.rs
│   │   │   │   ├── series.rs
│   │   │   │   └── collection.rs
│   │   │   └── services/     # Business logic
│   │   │       ├── mod.rs
│   │   │       ├── auth.rs
│   │   │       ├── movies.rs
│   │   │       ├── tmdb.rs
│   │   │       ├── ean.rs
│   │   │       └── import.rs
│   │   └── Cargo.toml
│   │
│   └── server/               # Server library + standalone binary
│       ├── src/
│       │   ├── lib.rs        # Server library (used by Tauri)
│       │   ├── main.rs       # Standalone server binary
│       │   ├── routes/       # API endpoints
│       │   │   ├── mod.rs
│       │   │   ├── auth.rs
│       │   │   ├── movies.rs
│       │   │   ├── series.rs
│       │   │   ├── collections.rs
│       │   │   ├── import.rs
│       │   │   └── ws.rs
│       │   └── middleware/   # Auth, logging, etc.
│       └── Cargo.toml
│
├── apps/
│   ├── web/                  # React SPA (browser)
│   │   ├── src/
│   │   │   ├── main.tsx
│   │   │   ├── routes/       # TanStack Router pages
│   │   │   ├── components/   # UI components
│   │   │   ├── hooks/        # Custom hooks
│   │   │   ├── lib/          # Utilities
│   │   │   │   ├── api.ts    # API client
│   │   │   │   ├── ws.ts     # WebSocket client
│   │   │   │   └── scanner.ts# Barcode scanner abstraction
│   │   │   └── styles/
│   │   ├── index.html
│   │   ├── vite.config.ts
│   │   ├── tailwind.config.js
│   │   ├── tsconfig.json
│   │   └── package.json
│   │
│   └── tauri/                # Tauri app (desktop + mobile)
│       ├── src-tauri/
│       │   ├── src/
│       │   │   ├── lib.rs    # Starts embedded server + Tauri window
│       │   │   └── main.rs
│       │   ├── Cargo.toml    # Depends on my-movies-server
│       │   ├── tauri.conf.json
│       │   ├── capabilities/
│       │   └── Info.ios.plist
│       └── package.json
│
├── docker/
│   ├── Dockerfile
│   ├── docker-compose.yml
│   └── litestream.yml
│
├── data/                     # SQLite database (shared by all deployment modes)
│   └── my-movies.db
│
├── Cargo.toml                # Workspace root
├── pnpm-workspace.yaml
├── package.json
├── .env.example
└── README.md
```

## Database Schema

### Users
| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| username | TEXT | Unique username |
| email | TEXT | Unique email |
| password_hash | TEXT | Argon2 hashed password |
| role | TEXT | 'admin' or 'user' |
| created_at | TIMESTAMP | |
| updated_at | TIMESTAMP | |

### Movies
| Column | Type | Description |
|--------|------|-------------|
| id | UUID | Primary key |
| user_id | UUID | Owner (FK → users) |
| tmdb_id | INTEGER | TMDB reference |
| imdb_id | TEXT | IMDB reference |
| barcode | TEXT | EAN/UPC code |
| title | TEXT | Display title |
| original_title | TEXT | Original language title |
| sort_title | TEXT | For sorting |
| personal_title | TEXT | User's custom title |
| description | TEXT | Plot summary |
| tagline | TEXT | Movie tagline |
| production_year | INTEGER | Release year |
| release_date | DATE | Actual release date |
| running_time | INTEGER | Minutes |
| rating | TEXT | MPAA/FSK rating |
| personal_rating | REAL | User's rating (0-10) |
| watched | BOOLEAN | Has user watched it |
| ... | ... | (full schema in migrations) |

### Series
Similar to Movies with additional fields for episodes, seasons, network, etc.

### Collections
For box sets and movie bundles.

## Getting Started

### Prerequisites

- Rust 1.85+ (required for Edition 2024)
- Node.js 20+
- pnpm 8+
- Docker & Docker Compose (for deployment)
- TMDB API Key (free at themoviedb.org)

### Development Setup

1. **Clone and install dependencies**
   ```bash
   git clone https://github.com/yourusername/my-movies.git
   cd my-movies
   pnpm install
   ```

2. **Configure environment**
   ```bash
   cp .env.example .env
   # Edit .env with your TMDB API key
   ```

### Option A: Tauri Desktop App (Recommended for Personal Use)

The easiest way to run the app - everything in one binary!

```bash
cd apps/tauri
pnpm run tauri dev
```

This starts:
- The embedded Rust server (automatically on port 3000)
- The Vite dev server for hot reload
- The Tauri window

**Build for production:**
```bash
cd apps/tauri
pnpm run tauri build
```

This creates a native app bundle (`My Movies.app` on macOS, `.exe` on Windows, etc.) with the server embedded - no separate backend needed!

> **macOS Gatekeeper Warning:** Unsigned builds will show "App can't be opened" warning. To bypass this, run:
> ```bash
> xattr -cr "/Applications/My Movies.app"
> ```
> Or right-click the app → "Open" → "Open" again in the dialog.

### Option B: Standalone Server (For Web/Multi-User Deployment)

For running the server separately (e.g., on a server for multiple users):

1. **Start the backend**
   ```bash
   cargo run --bin my-movies-server
   ```

2. **Start the frontend (in another terminal)**
   ```bash
   cd apps/web
   pnpm dev
   ```

3. **Access the app**
   - Frontend: http://localhost:5173
   - API: http://localhost:3000/api/v1

### Option C: Docker (For Server Deployment)

Docker runs everything in a single container - the server serves both API and frontend!

```bash
cd docker
docker compose up -d
```

Access the app at: http://localhost:3000

The Docker setup:
- Builds the React frontend into static files
- Rust server serves both API (`/api/v1/*`) and frontend (`/`)
- Shares database with local development via bind mount (`../data`)
- Single container, single port (3000)

### Building Mobile Apps (Tauri)

**iOS:**
```bash
cd apps/tauri
pnpm tauri ios build
```

**Android:**
```bash
cd apps/tauri
pnpm tauri android build
```

## API Endpoints

### Authentication
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | /api/v1/auth/register | Register new user |
| POST | /api/v1/auth/login | Login, returns JWT |
| POST | /api/v1/auth/refresh | Refresh JWT token |
| GET | /api/v1/auth/me | Get current user |

### Movies
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/v1/movies | List user's movies |
| POST | /api/v1/movies | Add movie |
| GET | /api/v1/movies/:id | Get movie details |
| PUT | /api/v1/movies/:id | Update movie |
| DELETE | /api/v1/movies/:id | Delete movie |
| POST | /api/v1/movies/scan | Lookup by barcode |

### Series
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/v1/series | List user's series |
| POST | /api/v1/series | Add series |
| ... | ... | (same pattern as movies) |

### Collections
| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | /api/v1/collections | List user's collections |
| POST | /api/v1/collections | Create collection |
| ... | ... | ... |

### Import/Export
| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | /api/v1/import/csv | Import from CSV |
| GET | /api/v1/export/csv | Export to CSV |

### WebSocket
| Endpoint | Description |
|----------|-------------|
| /ws | Real-time updates (requires JWT) |

**WebSocket Message Types:**
```typescript
// Server → Client
{ type: "movie_added", payload: Movie }
{ type: "movie_updated", payload: Movie }
{ type: "movie_deleted", payload: { id: string } }

// Client → Server
{ type: "subscribe", payload: { collections: ["movies", "series"] } }
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| DATABASE_URL | SQLite database path | ./data/my-movies.db |
| JWT_SECRET | Secret for JWT signing | (required) |
| TMDB_API_KEY | TMDB API key | (required) |
| RUST_LOG | Log level | info |
| HOST | Server bind address | 0.0.0.0 |
| PORT | Server port | 3000 |
| STATIC_DIR | Directory with frontend files | (none - API only) |

**Note:** `STATIC_DIR` is only needed for standalone server deployment when you want the server to serve the frontend. In Docker, this is set automatically to `/app/static`. Tauri handles the frontend via its webview, so `STATIC_DIR` is not used there.

## Backup Strategy

The app uses Litestream for continuous SQLite backups to S3-compatible storage.

Configure in `docker/litestream.yml`:
```yaml
dbs:
  - path: /app/data/my-movies.db
    replicas:
      - url: s3://your-bucket/my-movies
        access-key-id: ${AWS_ACCESS_KEY_ID}
        secret-access-key: ${AWS_SECRET_ACCESS_KEY}
```

To enable backups in Docker, run with the backup profile:
```bash
docker compose --profile backup up -d
```

## License

MIT
