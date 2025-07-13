Here are visual examples of developer-friendly CLI patterns:

## 1. Interactive Mode (Most User-Friendly)

```
$ myapp

🚀 Welcome to MyApp CLI

? What would you like to do? (Use arrow keys)
❯ Deploy application
  Manage database
  View logs
  Configure settings
  Exit

? Choose environment: (Use arrow keys)  
❯ development (current branch: feature/auth)
  staging
  production

? Select services to deploy: (Space to select, Enter to confirm)
❯ ◉ api-server
  ◯ web-frontend  
  ◉ worker-queue
  ◯ database-migrations

✅ Deploying api-server, worker-queue to development...
```

## 2. Smart Defaults + Discovery

```
$ myapp

🚀 MyApp CLI - Quick Actions

Most common:
  myapp deploy                    # Deploy to dev (auto-detected from git)
  myapp deploy --env staging      # Deploy to specific environment
  myapp logs                      # View recent logs

Setup:
  myapp init                      # Initialize new project (guided)
  myapp --interactive             # Full interactive mode

💡 Tip: Add --help to any command for details
💡 New here? Try: myapp --interactive
```

## 3. Progressive Command Discovery

```
$ myapp deploy --help

Deploy your application

USAGE:
    myapp deploy [OPTIONS]

OPTIONS:
    -e, --env <ENV>        Environment [default: auto-detect from git branch]
    -s, --service <NAME>   Specific service [default: all]
    
EXAMPLES:
    myapp deploy                           # Smart defaults
    myapp deploy --env prod               # Specific environment  
    myapp deploy --service api            # Single service

💡 First time? Try: myapp deploy --interactive
```

## 4. Contextual Suggestions

```
$ myapp deploy --env invalid

❌ Error: Environment 'invalid' not found

Available environments:
  development  (current git branch: feature/auth)
  staging      (last deployed: 2 hours ago)  
  production   (last deployed: yesterday)

💡 Try: myapp deploy --env development
💡 Or: myapp deploy --interactive
```

## 5. Adaptive Output Based on Terminal Width

**Wide Terminal (120+ chars):**
```
┌─────────────────┬──────────┬─────────────────┬─────────────┬─────────────┐
│ Service         │ Status   │ Last Deployed   │ Version     │ Health      │
├─────────────────┼──────────┼─────────────────┼─────────────┼─────────────┤
│ api-server      │ Running  │ 2 minutes ago   │ v1.2.3      │ ✅ Healthy  │
│ web-frontend    │ Running  │ 5 minutes ago   │ v2.1.0      │ ✅ Healthy  │
│ worker-queue    │ Stopped  │ 1 hour ago      │ v1.1.2      │ ❌ Down     │
└─────────────────┴──────────┴─────────────────┴─────────────┴─────────────┘
```

**Narrow Terminal (< 80 chars):**
```
Services Status:

api-server
  Status: ✅ Running (v1.2.3)
  Deployed: 2 minutes ago

web-frontend  
  Status: ✅ Running (v2.1.0)
  Deployed: 5 minutes ago

worker-queue
  Status: ❌ Down (v1.1.2) 
  Deployed: 1 hour ago
```

## 6. Command Building Assistant

```
$ myapp

❓ I want to... (type to search, tab to complete)

> deploy to staging█

Suggestions:
  deploy --env staging                    # Deploy all services to staging
  deploy --env staging --service api     # Deploy specific service
  deploy --env staging --dry-run         # Preview deployment

Press Enter to run, Tab for more options
```

**Key UX Principles:**
- **Default to most common actions**
- **Progressive disclosure** (simple → advanced)
- **Visual hierarchy** with emojis/icons
- **Contextual help** instead of generic manuals
- **Auto-completion** and suggestions
- **Responsive layout** for different terminal sizes

The interactive mode works best for complex tools, while smart defaults work for simpler, frequently-used commands.