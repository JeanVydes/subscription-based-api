# API Subscription Based With Account Managing Template

start creating, don't worry about boring stuff

this is a base for start a powerful API using axum, databases and subscriptions, including useful stuff like accounts managing with sessions, just start creating your idea, dont worry about low level boring stuff

i tried to make this the most organized possible, organizing per modules, so you don't have to break your mind trying to comprend the api functionability

also i create a postman api reference, you have to replicate that in your frontend and you are ready, check the [Postman API Reference](https://www.postman.com/jeanservices/workspace/8i/collection/11966073-e41fc689-a391-45b6-8d1e-d2b6176b5615?action=share&creator=11966073) to learn how to use

# Features

* Accounts System
* Login System Based In Session Tokens
* Database Connection with Postgres (misc) using Diesel as ORM, MongoDB (for users) and Redis (for sessions)
* LemonSqueezy Subscription Totally Integration
* The API provide ratelimits per routers, buffering, CORS, compression, fallbacks, and that's boring stuff

# Run

* Fly.io as Hosting Provider (optional)
* PostgresSQL Instance (optional [SupaBase](https://supabase.com/))
* MongoDB Instance ([MongoDB](https://mongodb.com/))
* RedisDB Instance ([upstash.io](https://upstash.io/))
* Set enviroment vars
* `fly launch`

# .env Template

```
HOST=0.0.0.0
PORT=8080

POSTGRES_URI=                       # (optional) fly secrets set POSTGRES_URI=
MONGO_URI=                          # fly secrets set MONGO_URI=
REDIS_URI=                          # fly secrets set REDIS_URI=
MONGO_DB_NAME=

API_TOKENS_SIGNING_KEY=             # fly secrets set API_TOKENS_SIGNING_KEY=
API_TOKENS_EXPIRATION_TIME=

LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY= # fly secrets set LEMONSQUEEZY_WEBHOOK_SIGNATURE_KEY=
PRO_PRODUCT_ID=
PRO_MONTHLY_VARIANT_ID=
PRO_ANNUALLY_VARIANT_ID=
```

# Future Ideas

* Next.js template

# Help

Reach me via email `github@jeanvides.com`