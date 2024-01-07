# Rust API Template

This template include authentication based on session tokens, the framework used is Axum, and has integration with MongoDB and Redis.

# .env Template

```
HOST=0.0.0.0
PORT=3000
MONGO_URI=your_uri
REDIS_URI=your_uri
MONGO_DB_NAME=test
API_TOKENS_SIGNING_KEY=password 
API_TOKENS_EXPIRATION_TIME=86400
```